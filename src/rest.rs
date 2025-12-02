//! Connector for REST APIs.

use crate::{
    errors::{
        ConnectionError, DecodeOneError, DecodeStreamError, FetchError, FetchOneError, SendError,
        classify_reqwest,
    },
    source_sink::{Sink, Source},
};
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt as _, TryStreamExt as _, stream::iter as from_iter};
use reqwest::{Body, Client, Error as ReqwestError, IntoUrl, Response, Url};
use serde::{Serialize, de::value::Error as DeserializeError};
use std::{fmt::Error as FmtError, io::Error as IoError, marker::PhantomData};

/// A type that can decode data from bytes.
///
/// This trait provides several ways of decoding data regarding how many bytes are returned at a
/// time. The [`decode`] method is `async` to work with data that is produced in real time while
/// the others expect all bytes in advance.
///
/// A decoder is generally stateless, e.g. one for JSON, meaning they will practically often be
/// ZSTs. However, all methods have conservatively been made to accept `&mut self` to allow for
/// stateful decoders.
#[allow(
    clippy::missing_errors_doc,
    reason = "Default implementations only delegate errors and do not raise their own."
)]
pub trait Decode<T> {
    /// Decode data from a stream of bytes.
    ///
    /// An error may be raised for each item produced, or before any are. Reasonably,
    /// [`DecodeStreamError::Connection`] should only passed on from input; decoding should not
    /// produce new connection errors, though this is not enforced.
    ///
    /// The default implementation collects the bytes and calls [`decode_all`].
    fn decode<S>(
        &mut self,
        bytes: S,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = Result<T, DecodeStreamError>> + Send + Unpin,
            DecodeStreamError,
        >,
    > + Send
    where
        Self: Send,
        T: Send,
        S: Stream<Item = Result<Bytes, ConnectionError>> + Send,
    {
        // TODO: This should be possible to implement without creating an `async` closure.
        async move {
            let buf = bytes.try_collect::<BytesMut>().await?;
            self.decode_all(&buf)
                .map(|vec| from_iter(vec.into_iter().map(Ok)))
                .map_err(Into::into)
        }
    }

    /// Decode data from a slice.
    ///
    /// This method intentionally does not have a default implementation based on [`decode`]. This
    /// is partly because that method is `async` while this one isn't, meaning it would have to
    /// make explicit blocking calls, unexpectedly leading to poor concurrency and possibly
    /// performance. It would also require enforcing the contract on that method about not creating
    /// new connection errors, since this method cannot return one.
    fn decode_all(&mut self, bytes: &[u8]) -> Result<Vec<T>, DeserializeError>;

    /// Decode a single entry from a slice. If the slice is empty or represents an empty
    /// collection, <code>[Err]\([`Empty`](DeserializeError::Empty))</code> is returned.
    ///
    /// One entry is assumed to be fairly small such that collection all bytes into a slice is
    /// acceptable, and as such no stream variant of this method exists.
    ///
    /// The default implementation calls [`decode_optional`].
    fn decode_one(&mut self, bytes: &[u8]) -> Result<T, DecodeOneError> {
        self.decode_optional(bytes)?.ok_or(DecodeOneError::Empty)
    }

    /// Decode a single entry from a slice, if one exists.
    ///
    /// This method poses no restriction on *which* entry should be returned. The format may
    /// however define an ordering.
    ///
    /// One entry is assumed to be fairly small such that collection all bytes into a slice is
    /// acceptable, and as such no stream variant of this method exists.
    fn decode_optional(&mut self, bytes: &[u8]) -> Result<Option<T>, DeserializeError>;
}

/// A type that can encode data as bytes.
///
/// This trait provides several ways of encoding data regarding how many entries are processes at a
/// time.
///
/// An encoder is generally stateless, e.g. one for JSON, meaning they will practically often be
/// ZSTs. However, all methods have conservatively been made to accept `&mut self` to allow for
/// stateful decoders.
#[allow(
    clippy::missing_errors_doc,
    reason = "Default implementations only delegate errors and do not raise their own."
)]
pub trait Encode<T> {
    /// Encode data from an iterator.
    ///
    /// This method intentionally returns all bytes at once, rather than an iterator, as many
    /// formats (e.g. JSON) require not only headers but also footers (such as closing a list in
    /// JSON). Returning header data and waiting for a footer leaves any intermediate state as
    /// invalid encoding, and omitting header data means even the final result is invalid.
    ///
    /// The default implementation collects the entries and calls [`encode_all`].
    fn encode<I>(&mut self, entries: I) -> Result<Box<[u8]>, FmtError>
    where
        I: IntoIterator<Item = T>,
    {
        self.encode_all(&entries.into_iter().collect::<Box<_>>())
    }

    /// Encode data from a slice.
    ///
    /// Depending on the format, this may or may not be equivalent to calling `encode_one` on
    /// several entries and concatenating the results.
    // `encode_all` cannot have a default implementation based on `encode` as it
    // captures `T` by value while `encode_all` can only produce `&T` without placing a `Clone`
    // restriction on `T`. Additionally, `encode` couldn't accept an iterator of `&T` as `Iterator`
    // doesn't define a lifetime parameter.
    fn encode_all(&mut self, entries: &[T]) -> Result<Box<[u8]>, FmtError>;

    /// Encode a single entry.
    ///
    /// Depending on the format, calling this several times and concatenating the results may or
    /// may not be equivalent to calling `encode_all`.
    fn encode_one(&mut self, entry: T) -> Result<Box<[u8]>, FmtError>;
}

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
        Q: Serialize + Send,
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

    #[expect(clippy::missing_panics_doc, reason = "Panic should not occur here.")]
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
        // `<Url as IntoUrl>::into_url` can fail, it has already been validated that this is not
        // the case. Hence, this shouldn't fail.
        let request = self
            .client
            .put(self.sink_url.clone())
            .body(body)
            .build()
            .unwrap();
        self.client.execute(request).await.map_err(classify_reqwest)
    }
}

#[expect(
    unused_lifetimes,
    reason = "Lifetimes required by trait declaration, but unnecessary for current implementation where outputs do not borrow `self`."
)]
impl<'a, T, Q, E, D> Source<'a, T> for &'a mut Rest<T, Q, E, D>
where
    T: Sync,
    Q: Serialize + Send + Sync,
    E: Send + Sync,
    D: Decode<T> + Send + Sync,
{
    type Query = Q;

    async fn fetch<'s>(
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

    async fn fetch_all<'s>(self, query: Self::Query) -> Result<Vec<T>, FetchError> {
        let bytes = self.fetch_impl(query).await?.bytes().await?;
        self.decoder.decode_all(&bytes).map_err(Into::into)
    }

    async fn fetch_one<'s>(self, query: Self::Query) -> Result<T, FetchOneError> {
        let bytes = self.fetch_impl(query).await?.bytes().await?;
        self.decoder.decode_one(&bytes).map_err(Into::into)
    }
}

#[expect(
    unused_lifetimes,
    reason = "Lifetimes required by trait declaration, but unnecessary for current implementation where outputs do not borrow `self`."
)]
impl<'a, T, Q, E, D> Sink<'a, T> for &'a mut Rest<T, Q, E, D>
where
    T: Send + Sync,
    Q: Send + Sync,
    E: Encode<T> + Send + Sync,
    D: Send + Sync,
{
    async fn send<'s, I>(self, entries: I) -> Result<(), SendError>
    where
        I: IntoIterator<Item = T>,
    {
        let body = self.encoder.encode(entries)?;
        self.send_impl(Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn send_all<'s>(self, entries: &'s [T]) -> Result<(), SendError>
    where
        'a: 's,
    {
        let body = self.encoder.encode_all(entries)?;
        self.send_impl(Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn send_one<'s>(self, entry: T) -> Result<(), SendError> {
        let body = self.encoder.encode_one(entry)?;
        self.send_impl(Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }
}
