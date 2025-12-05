//! Connector for REST APIs.

use crate::{
    connector::{Sink, Source},
    encode::{Decode, Encode},
    errors::{ConnectionError, FetchError, FetchOneError, SendError, classify_reqwest},
};
use futures::{Stream, StreamExt as _};
use reqwest::{Body, Client, Error as ReqwestError, IntoUrl, Response, Url};
use serde::Serialize;
use std::{io::Error as IoError, marker::PhantomData};

/// A connector to work with REST APIs.
///
/// This makes no assumption about the format used to communicate with the API, but delegates this
/// work to its [`encoder`](Encode) and [`decoder`](Decode).
///
/// Data is assumed to be fetched using GET requests and sent using PUT requests. This should
/// comply with APIs using best practices.
///
/// [`Source`] and [`Sink`] are implemented for `&mut self` to allow for stateful encoders or
/// decoders, see trait documentation for more information.
// TODO: Implement variants for readonly/writeonly?
#[derive(Debug, Clone)]
pub struct Rest<T, Q, E, D> {
    /// The URL to fetch data from.
    source_url: Url,
    /// The URL to send data to.
    sink_url: Url,
    /// The client used to execute requests.
    client: Client,
    /// The encoder used to serialize data to be sent.
    encoder: E,
    /// The decoder used to deserialize received data.
    decoder: D,
    /// Satisfies missing fields using `T` and `Q`.
    // TODO: This may be overly restrictive when considering variance. Improve using unstable
    // `phantom_variance_markers` (#135806)?
    _phantom: PhantomData<(T, Q)>,
}

impl<T, Q, E, D> Rest<T, Q, E, D> {
    /// Construct a new REST connector using a default [`Client`].
    ///
    /// # Errors
    ///
    /// This method fails if a TLS backend cannot be initialized, or the resolver cannot load the
    /// system configuration, or if either [`URL`](Url) fails to parse.
    pub fn new(
        source_url: impl IntoUrl,
        sink_url: impl IntoUrl,
        encoder: E,
        decoder: D,
    ) -> Result<Self, ReqwestError> {
        // `ClientBuilder::build` can fail even with default options if a TLS backend fails to
        // initialize.
        let client = Client::builder()
            .build()
            .inspect_err(|err| debug_assert!(err.is_builder()))?;
        Self::with_client(source_url, sink_url, encoder, decoder, client)
    }

    /// Construct a new REST connector using a provided [`Client`].
    ///
    /// # Errors
    ///
    /// This method fails if either [`URL`](Url) fails to parse.
    pub fn with_client(
        source_url: impl IntoUrl,
        sink_url: impl IntoUrl,
        encoder: E,
        decoder: D,
        client: Client,
    ) -> Result<Self, ReqwestError> {
        Ok(Self {
            // Even `Url::into_url` can fail, so we call it once here during construction so we
            // know it's safe to unwrap later when calling `into_url` on clones.
            source_url: source_url.into_url()?,
            sink_url: sink_url.into_url()?,
            client,
            encoder,
            decoder,
            _phantom: PhantomData,
        })
    }

    /// Get a reference to the [`URL`](Url) used to fetch data.
    pub const fn source_url(&self) -> &Url {
        &self.source_url
    }

    /// Get a reference to the [`URL`](Url) used to send data.
    pub const fn sink_url(&self) -> &Url {
        &self.sink_url
    }
}

impl<T, Q, E, D> Rest<T, Q, E, D>
where
    T: Sync,
    Q: Sync,
    E: Sync,
    D: Sync,
{
    /// Helper to use for [`Source`] implementation.
    ///
    /// # Errors
    ///
    /// If the request fails, returns the error as classified by [`classify_reqwest`].
    async fn fetch_impl(&self, query: Q) -> Result<Response, FetchError>
    where
        Q: Serialize,
        D: Decode<T>,
    {
        // `RequestBuilder::build` also fails is the URL cannot be parsed. Although
        // `<Url as IntoUrl>::into_url` can fail, it has already been validated that this is not
        // the case. Hence, any error here stems from the query.
        let request = self
            .client
            .get(self.source_url.clone())
            .query(&query)
            .build()
            .map_err(|err| FetchError::InvalidQuery(Box::new(err)))?;
        self.client.execute(request).await.map_err(classify_reqwest)
    }

    #[expect(
        clippy::missing_panics_doc,
        reason = "Panic should not occur here. See comment."
    )]
    /// Helper to use for [`Sink`] implementation.
    ///
    /// # Errors
    ///
    /// If the request fails, returns the error as classified by [`classify_reqwest`].
    async fn send_impl<B>(&self, body: B) -> Result<Response, ConnectionError>
    where
        B: Into<Body> + Send,
    {
        // `RequestBuilder::build` fails is the URL cannot be parsed. Although
        // `<Url as IntoUrl>::into_url` can fail, it has already been validated during construction
        // that this is not the case. Hence, this shouldn't fail.
        let request = self
            .client
            .put(self.sink_url.clone())
            .body(body)
            .build()
            .unwrap();
        self.client.execute(request).await.map_err(classify_reqwest)
    }
}

impl<'a, T, Q, E, D> Source<'a, T> for &'a mut Rest<T, Q, E, D>
where
    T: Sync,
    Q: Serialize + Send + Sync,
    E: Send + Sync,
    D: Decode<T> + Send + Sync,
{
    type Query = Q;

    async fn fetch(
        self,
        query: Self::Query,
    ) -> Result<impl Stream<Item = Result<T, FetchError>> + Send + Unpin, FetchError>
    where
        T: Send,
    {
        let input = self.fetch_impl(query).await?.bytes_stream().map(|res| {
            res.map_err(|err| {
                // HTTP errors should be raised by `send`, and already have been returned.
                debug_assert!(err.status().is_none());
                ConnectionError::Io(IoError::other(err))
            })
        });
        self.decoder
            .decode(input)
            .await
            .map(|output| output.map(|res| res.map_err(Into::into)))
            .map_err(Into::into)
    }

    async fn fetch_all(self, query: Self::Query) -> Result<Vec<T>, FetchError> {
        let bytes = self.fetch_impl(query).await?.bytes().await?;
        self.decoder.decode_all(&bytes).map_err(Into::into)
    }

    async fn fetch_one(self, query: Self::Query) -> Result<T, FetchOneError> {
        let bytes = self.fetch_impl(query).await?.bytes().await?;
        self.decoder.decode_one(&bytes).map_err(Into::into)
    }
}

impl<T, Q, E, D> Sink<T> for Rest<T, Q, E, D>
where
    T: Send + Sync,
    Q: Sync,
    E: Encode<T> + Sync,
    D: Sync,
{
    async fn send<'s, I>(&self, entries: I) -> Result<(), SendError>
    where
        T: 's,
        I: IntoIterator<Item = &'s T>,
    {
        let body = self.encoder.encode(entries)?;
        self.send_impl(Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn send_all(&self, entries: &[T]) -> Result<(), SendError>
where {
        let body = self.encoder.encode_all(entries)?;
        self.send_impl(Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn send_one(&self, entry: &T) -> Result<(), SendError> {
        let body = self.encoder.encode_one(entry)?;
        self.send_impl(Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }
}
