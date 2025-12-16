//! Connector for REST APIs.

use crate::{
    connector::{Sink, Source},
    encode::{Codec, Decode, Encode},
    errors::{ConnectionError, DecodeError, FetchError, FetchOneError, SendError},
};
use futures::{Stream, StreamExt as _};
use reqwest::{Body, Client, Method, Response, Url};
use serde::Serialize;
use std::{io::Error as IoError, marker::PhantomData};

/// The [`Builder`](builder::Builder), used to construct REST connectors more flexibly.
mod builder;
pub use builder::*;

/// A source to work with REST APIs.
///
/// This makes no assumption about the format used to communicate with the API, but delegates this
/// work to its [`decoder`](Decode).
///
/// [`Source`] is implemented for `&mut self` to allow for stateful decoders, see trait
/// documentation for more information. Note that the type `(&str, &str)` and some similar types
/// **cannot** be serialized to query parameters, but an array or a slice like `&[(&str, &str)]`
/// can.
#[derive(Debug, Clone)]
pub struct ReadOnly<T, Q, D> {
    /// The URL to fetch data from.
    url: Url,
    /// The HTTP method to use when fetching data.
    method: Method,
    /// The client used to execute requests.
    client: Client,
    /// The decoder used to deserialize received data.
    decoder: D,
    /// Satisfies missing fields using `T` and `Q`.
    // TODO: This may be overly restrictive when considering variance. Improve using unstable
    // `phantom_variance_markers` (#135806)?
    _phantom: PhantomData<(T, Q)>,
}

/// A sink to work with REST APIs.
///
/// This makes no assumption about the format used to communicate with the API, but delegates this
/// work to its [`encoder`](Encode).
///
/// [`Sink`] is implemented for `&mut self` to allow for stateful decoders, see trait
/// documentation for more information.
#[derive(Debug, Clone)]
pub struct WriteOnly<T, E> {
    /// The URL to send data to.
    url: Url,
    /// The HTTP method to use when sending data.
    method: Method,
    /// The client used to execute requests.
    client: Client,
    /// The encoder used to serialize data to be sent.
    encoder: E,
    /// Satisfies missing fields using `T` and `Q`.
    // TODO: This may be overly restrictive when considering variance. Improve using unstable
    // `phantom_variance_markers` (#135806)?
    _phantom: PhantomData<T>,
}

/// A connector to work with REST APIs.
///
/// This makes no assumption about the format used to communicate with the API, but delegates this
/// work to its [`encoder`](Encode) and [`decoder`](Decode).
///
/// [`Source`] and [`Sink`] are implemented for `&mut self` to allow for stateful encoders or
/// decoders, see trait documentation for more information. Note that the type `(&str, &str)` and
/// some similar types **cannot** be serialized to query parameters, but an array or a slice like
/// `&[(&str, &str)]` can.
#[derive(Debug, Clone)]
pub struct ReadWrite<T, Q, E, D, C> {
    /// The URL to fetch data from.
    source_url: Url,
    /// The HTTP method to use when fetching data.
    source_method: Method,
    /// The URL to send data to.
    sink_url: Url,
    /// The HTTP method to use when sending data.
    sink_method: Method,
    /// The client used to execute requests.
    client: Client,
    /// The codec used to serialize and deserialize data.
    codec: Codec<T, E, D, C>,
    /// Satisfies missing fields using `T` and `Q`.
    // TODO: This may be overly restrictive when considering variance. Improve using unstable
    // `phantom_variance_markers` (#135806)?
    _phantom: PhantomData<(T, Q)>,
}

/// Helper to use for [`Source`] implementation.
///
/// # Errors
///
/// If the request fails, returns the error as classified by [`classify_reqwest`].
async fn fetch_impl<Q>(
    client: &Client,
    url: Url,
    method: Method,
    query: Q,
) -> Result<Response, FetchError>
where
    Q: Serialize,
{
    // `RequestBuilder::build` also fails is the URL cannot be parsed. Although
    // `<Url as IntoUrl>::into_url` can fail, it has already been validated that this is not
    // the case. Hence, any error here stems from the query.
    let request = client
        .request(method, url)
        .query(&query)
        .build()
        .map_err(|err| FetchError::InvalidQuery(Box::new(err)))?;
    client.execute(request).await.map_err(Into::into)
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
async fn send_impl<B>(
    client: &Client,
    url: Url,
    method: Method,
    body: B,
) -> Result<Response, ConnectionError>
where
    B: Into<Body>,
{
    // `RequestBuilder::build` fails is the URL cannot be parsed. Although
    // `<Url as IntoUrl>::into_url` can fail, it has already been validated during construction
    // that this is not the case. Hence, this shouldn't fail.
    let request = client
        .request(method, url)
        .body(body)
        .build()
        .expect("URL failed to parse.");
    client.execute(request).await.map_err(Into::into)
}

impl<'a, T, Q, D> Source<'a, T> for &'a mut ReadOnly<T, Q, D>
where
    T: Send,
    Q: Serialize + Send,
    D: Decode<T> + Send + Sync,
{
    type Query = Q;

    #[inline]
    async fn fetch(
        self,
        query: Self::Query,
    ) -> Result<impl Stream<Item = Result<T, FetchError>> + Send + Unpin, FetchError> {
        let input = fetch_impl(&self.client, self.url.clone(), self.method.clone(), query)
            .await?
            .bytes_stream()
            .map(|res| {
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

    #[inline]
    async fn fetch_all(self, query: Self::Query) -> Result<Vec<T>, FetchError> {
        let bytes = fetch_impl(&self.client, self.url.clone(), self.method.clone(), query)
            .await?
            .bytes()
            .await?;
        self.decoder
            .decode_all(&bytes)
            .map_err(|err| DecodeError(Box::new(err)).into())
    }

    #[inline]
    async fn fetch_one(self, query: Self::Query) -> Result<T, FetchOneError> {
        let bytes = fetch_impl(&self.client, self.url.clone(), self.method.clone(), query)
            .await?
            .bytes()
            .await?;
        self.decoder.decode_one(&bytes).map_err(Into::into)
    }
}

impl<'a, T, Q, E, D, C> Source<'a, T> for &'a mut ReadWrite<T, Q, E, D, C>
where
    T: Send + Sync,
    Q: Serialize + Send,
    E: Send + Sync,
    D: Decode<T> + Send + Sync,
    C: Decode<T> + Send + Sync,
{
    type Query = Q;

    #[inline]
    async fn fetch(
        self,
        query: Self::Query,
    ) -> Result<impl Stream<Item = Result<T, FetchError>> + Send + Unpin, FetchError> {
        let input = fetch_impl(
            &self.client,
            self.source_url.clone(),
            self.source_method.clone(),
            query,
        )
        .await?
        .bytes_stream()
        .map(|res| {
            res.map_err(|err| {
                // HTTP errors should be raised by `send`, and already have been returned.
                debug_assert!(err.status().is_none());
                ConnectionError::Io(IoError::other(err))
            })
        });
        self.codec
            .decode(input)
            .await
            .map(|output| output.map(|res| res.map_err(Into::into)))
            .map_err(Into::into)
    }

    #[inline]
    async fn fetch_all(self, query: Self::Query) -> Result<Vec<T>, FetchError> {
        let bytes = fetch_impl(
            &self.client,
            self.source_url.clone(),
            self.source_method.clone(),
            query,
        )
        .await?
        .bytes()
        .await?;
        self.codec
            .decode_all(&bytes)
            .map_err(|err| DecodeError(Box::new(err)).into())
    }

    #[inline]
    async fn fetch_one(self, query: Self::Query) -> Result<T, FetchOneError> {
        let bytes = fetch_impl(
            &self.client,
            self.source_url.clone(),
            self.source_method.clone(),
            query,
        )
        .await?
        .bytes()
        .await?;
        self.codec.decode_one(&bytes).map_err(Into::into)
    }
}

impl<T, E> Sink<T> for WriteOnly<T, E>
where
    T: Sync,
    E: Encode<T> + Sync,
{
    #[inline]
    async fn send<'s, I>(&self, entries: I) -> Result<(), SendError>
    where
        T: 's,
        I: IntoIterator<Item = &'s T>,
    {
        let body = self.encoder.encode(entries).map_err(SendError::Encode)?;
        send_impl(
            &self.client,
            self.url.clone(),
            self.method.clone(),
            Vec::from(body),
        )
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    #[inline]
    async fn send_all(&self, entries: &[T]) -> Result<(), SendError> {
        let body = self
            .encoder
            .encode_all(entries)
            .map_err(SendError::Encode)?;
        send_impl(
            &self.client,
            self.url.clone(),
            self.method.clone(),
            Vec::from(body),
        )
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    #[inline]
    async fn send_one(&self, entry: &T) -> Result<(), SendError> {
        let body = self.encoder.encode_one(entry).map_err(SendError::Encode)?;
        send_impl(
            &self.client,
            self.url.clone(),
            self.method.clone(),
            Vec::from(body),
        )
        .await
        .map(|_| ())
        .map_err(Into::into)
    }
}

impl<T, Q, E, D, C> Sink<T> for ReadWrite<T, Q, E, D, C>
where
    T: Sync,
    Q: Sync,
    E: Encode<T> + Sync,
    D: Sync,
    C: Encode<T> + Sync,
{
    #[inline]
    async fn send<'s, I>(&self, entries: I) -> Result<(), SendError>
    where
        T: 's,
        I: IntoIterator<Item = &'s T>,
    {
        let body = self.codec.encode(entries).map_err(SendError::Encode)?;
        send_impl(
            &self.client,
            self.sink_url.clone(),
            self.sink_method.clone(),
            Vec::from(body),
        )
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    #[inline]
    async fn send_all(&self, entries: &[T]) -> Result<(), SendError> {
        let body = self.codec.encode_all(entries).map_err(SendError::Encode)?;
        send_impl(
            &self.client,
            self.sink_url.clone(),
            self.sink_method.clone(),
            Vec::from(body),
        )
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    #[inline]
    async fn send_one(&self, entry: &T) -> Result<(), SendError> {
        let body = self.codec.encode_one(entry).map_err(SendError::Encode)?;
        send_impl(
            &self.client,
            self.sink_url.clone(),
            self.sink_method.clone(),
            Vec::from(body),
        )
        .await
        .map(|_| ())
        .map_err(Into::into)
    }
}

#[cfg(test)]
#[allow(
    clippy::missing_panics_doc,
    reason = "Panics simply indicate failed tests."
)]
#[allow(clippy::unwrap_used, reason = "Panics simply indicate failed tests.")]
mod tests {
    use super::*;
    use crate::encode::json::Json;
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn cat_aas() {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct Cat {
            id: String,
            tags: Vec<String>,
            created_at: String,
            url: String,
            mimetype: String,
        }

        // Endpoint: `https://cataas.com/cat?json=true`.

        let mut rest = Builder::new()
            .source_url("https://cataas.com/cat")
            .unwrap()
            .decoder(Json)
            .build();

        let _cat: Cat = rest.fetch_one([("json", "true")]).await.unwrap();
    }
}
