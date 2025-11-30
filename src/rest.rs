//! Connector for REST APIs.

use crate::{
    errors::{ConnectionError, DecodeOneError, DecodeStreamError, FetchError, FetchOneError},
    source_sink::{Sink as _, Source},
};
use bytes::{Bytes, BytesMut};
use futures::{
    FutureExt as _, Stream, StreamExt as _, TryStreamExt as _, stream::iter as from_iter,
};
use reqwest::{Client, Response, Url};
use serde::{Serialize, de::value::Error as DeserializeError};
use std::{fmt::Error as FmtError, io::Error as IoError, marker::PhantomData};

pub trait Decode<T> {
    fn decode<I>(
        &mut self,
        bytes: I,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = Result<T, DecodeStreamError>> + Send + Unpin,
            DecodeStreamError,
        >,
    > + Send
    where
        Self: Send,
        T: Send,
        I: Stream<Item = Result<Bytes, ConnectionError>> + Send,
    {
        async move {
            let buf = bytes.try_collect::<BytesMut>().await?;
            self.decode_all(&buf)
                .map(|vec| from_iter(vec.into_iter().map(Ok)))
                .map_err(Into::into)
        }
    }

    // TODO: Document why no default implementation exists.
    fn decode_all(&mut self, bytes: &[u8]) -> Result<Vec<T>, DeserializeError>;
    // As per the method contract, `decode` should never create new connection errors, only
    // forward those provided. The stream is created locally and is known to not yield any
    // errors. Hence, we never encounter a connection error. Nevertheless, we should panic in
    // the case that we do to avoid UB as a consequence of a violated contract.
    // ```
    // fn as_deserialize(err: DecodeStreamError) -> DeserializeError {
    //     match err {
    //         DecodeStreamError::Decode(err) => err,
    //         DecodeStreamError::Connection(_) => unreachable!(),
    //     }
    // }
    //
    // let input = once(ok(Bytes::copy_from_slice(bytes)));
    // let output = block_on(self.decode(input)).map_err(as_deserialize)?;
    // block_on(output.try_collect()).map_err(as_deserialize)
    // ```

    // TODO: Document why default implementation can't use `decode`.
    // Assume `bytes` small, hence only slice variant.
    fn decode_one(&mut self, bytes: &[u8]) -> Result<T, DecodeOneError> {
        self.decode_optional(bytes)?.ok_or(DecodeOneError::Empty)
    }

    fn decode_optional(&mut self, bytes: &[u8]) -> Result<Option<T>, DeserializeError>;
}

pub trait Encode<T> {
    type Err;

    fn encode<I>(&mut self, entries: I) -> Result<Box<[u8]>, FmtError>
    where
        I: IntoIterator<Item = T>,
    {
        self.encode_all(&entries.into_iter().collect::<Box<_>>())
    }

    // `encode_all` cannot have a default implementation based on `encode` as it
    // captures `T` by value while `encode_all` can only produce `&T` without placing a `Clone`
    // restriction on `T`. Additionally, `encode` couldn't accept an iterator of `&T` as `Iterator`
    // doesn't define a lifetime parameter.
    fn encode_all(&mut self, entries: &[T]) -> Result<Box<[u8]>, FmtError>;

    fn encode_one(&mut self, entry: T) -> Result<Box<[u8]>, FmtError>;
}

struct Rest<T, Q, E, D> {
    // TODO: URL fields. Separate for fetching/sending. Specify send method. Possibly different
    // variants for readonly/writeonly?
    source_url: Url,
    client: Client,
    encoder: E,
    decoder: D,
    _phantom: PhantomData<(T, Q)>,
}

impl<T, Q, E, D> Rest<T, Q, E, D>
where
    Q: Serialize,
    D: Decode<T>,
{
    async fn send(&self, query: Q) -> Result<Response, FetchError>
    where
        T: Sync,
        Q: Send + Sync,
        E: Sync,
        D: Sync,
    {
        // `RequestBuilder::build` also fails is the URL cannot be parsed. Although
        // `<Url as IntoUrl>::into_url` can fail, it has already been validated that this is not
        // the case.
        let request = self
            .client
            .get(self.source_url.clone())
            .query(&query)
            .build()
            .map_err(|err| FetchError::InvalidQuery(Box::new(err)))?;
        self.client.execute(request).await.map_err(|err| {
            if let Some(status) = err.status() {
                ConnectionError::Http {
                    code: status.into(),
                    source: Box::new(err),
                }
                .into()
            } else {
                err.into()
            }
        })
    }
}

#[expect(
    unused_lifetimes,
    reason = "Required by trait declaration, but unnecessary for current implementation where outputs do not borrow `self`."
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
        let input = self.send(query).await?.bytes_stream().map(|res| {
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
        let bytes = self.send(query).await?.bytes().await?;
        self.decoder.decode_all(&bytes).map_err(Into::into)
    }

    async fn fetch_one<'s>(self, query: Self::Query) -> Result<T, FetchOneError> {
        let bytes = self.send(query).await?.bytes().await?;
        self.decoder.decode_one(&bytes).map_err(Into::into)
    }
}

// TODO: Implement `Sink` for `&mut Rest`.
