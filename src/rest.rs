//! Connector for REST APIs.

use crate::{
    Query,
    connector::{Sink, Source},
    encode::{Codec, Decode, Encode},
    errors::{ConnectionError, DecodeError, FetchError, FetchOneError, SendError},
    query::translate,
};
use async_trait::async_trait;
use futures::{StreamExt as _, stream::BoxStream};
use reqwest::{Body, Client, Method, Response, Url};
use serde::Serialize;
use std::{io::Error as IoError, marker::PhantomData};

/// The [`Builder`], used to construct REST connectors more flexibly.
mod builder;
pub use builder::*;

/// A source to work with REST APIs.
///
/// This makes no assumption about the format used to communicate with the API, but delegates this
/// work to its [`decoder`](Decode).
///
/// Note that the type `(&str, &str)` and some similar types **cannot** be serialized to query
/// parameters, but alternatives like `&[(&str, &str)]` can.
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
/// Note that the type `(&str, &str)` and some similar types **cannot** be serialized to query
/// parameters, but an array or a slice like `&[(&str, &str)]` can.
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
/// Fails if an error occurs during connection or if the query fails to serialize.
async fn fetch_impl(
    client: &Client,
    url: Url,
    method: Method,
    query: impl Serialize,
) -> Result<Response, FetchError> {
    // `RequestBuilder::build` also fails is the URL cannot be parsed. Although
    // `<Url as IntoUrl>::into_url` can fail, it has already been validated that this will not
    // happen here. Hence, any error here stems from the query.
    let request = client
        .request(method, url)
        .query(&query)
        .build()
        .map_err(|err| FetchError::InvalidQuery(Box::new(err)))?;
    client.execute(request).await.map_err(Into::into)
}

#[expect(clippy::missing_panics_doc, reason = "See implementation.")]
/// Helper to use for [`Sink`] implementation.
///
/// # Errors
///
/// Fails if an error occurs during connection.
async fn send_impl<B>(
    client: &Client,
    url: Url,
    method: Method,
    body: B,
) -> Result<Response, ConnectionError>
where
    B: Into<Body>,
{
    #[allow(
        clippy::unwrap_used,
        reason = "
            `RequestBuilder::build` fails is the URL cannot be parsed. Although
            `<Url as IntoUrl>::into_url` can fail, it has already been validated during
            construction that this will not happen here.
        "
    )]
    let request = client.request(method, url).body(body).build().unwrap();
    client.execute(request).await.map_err(Into::into)
}

#[async_trait]
impl<T, Q, D> Source<T> for ReadOnly<T, Q, D>
where
    T: Send,
    Q: Send,
    D: Decode<T> + Send + Sync,
{
    #[inline]
    async fn fetch<'s>(
        &'s mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
    where
        T: 's,
    {
        let translated = translate(query);

        let bytes = fetch_impl(
            &self.client,
            self.url.clone(),
            self.method.clone(),
            translated,
        )
        .await?
        .bytes_stream()
        .map(|res| {
            res.map_err(|err| {
                // HTTP errors should be raised by `fetch_impl`, and already have been returned.
                debug_assert!(err.status().is_none());
                ConnectionError::Io(IoError::other(err))
            })
        });

        self.decoder
            .decode(bytes)
            .await
            .map(|output| output.map(|res| res.map_err(Into::into)).boxed())
            .map_err(Into::into)
    }

    #[inline]
    async fn fetch_all(&mut self, query: &(dyn Query<T> + Sync)) -> Result<Vec<T>, FetchError> {
        let translated = translate(query);

        let bytes = fetch_impl(
            &self.client,
            self.url.clone(),
            self.method.clone(),
            translated,
        )
        .await?
        .bytes()
        .await?;

        self.decoder
            .decode_all(&bytes)
            .map_err(|err| DecodeError(Box::new(err)).into())
    }

    #[inline]
    async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
        let translated = translate(query);

        let bytes = fetch_impl(
            &self.client,
            self.url.clone(),
            self.method.clone(),
            translated,
        )
        .await?
        .bytes()
        .await?;

        self.decoder.decode_one(&bytes).map_err(Into::into)
    }
}

#[async_trait]
impl<T, Q, E, D, C> Source<T> for ReadWrite<T, Q, E, D, C>
where
    T: Send + Sync,
    Q: Send,
    E: Send + Sync,
    D: Decode<T> + Send + Sync,
    C: Decode<T> + Send + Sync,
{
    #[inline]
    async fn fetch<'s>(
        &'s mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
    where
        T: 's,
    {
        let translated = translate(query);

        let bytes = fetch_impl(
            &self.client,
            self.source_url.clone(),
            self.source_method.clone(),
            translated,
        )
        .await?
        .bytes_stream()
        .map(|res| {
            res.map_err(|err| {
                // HTTP errors should be raised by `fetch_impl`, and already have been returned.
                debug_assert!(err.status().is_none());
                ConnectionError::Io(IoError::other(err))
            })
        });

        self.codec
            .decode(bytes)
            .await
            .map(|output| output.map(|res| res.map_err(Into::into)).boxed())
            .map_err(Into::into)
    }

    #[inline]
    async fn fetch_all(&mut self, query: &(dyn Query<T> + Sync)) -> Result<Vec<T>, FetchError> {
        let translated = translate(query);

        let bytes = fetch_impl(
            &self.client,
            self.source_url.clone(),
            self.source_method.clone(),
            translated,
        )
        .await?
        .bytes()
        .await?;

        self.codec
            .decode_all(&bytes)
            .map_err(|err| DecodeError(Box::new(err)).into())
    }

    #[inline]
    async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
        let translated = translate(query);

        let bytes = fetch_impl(
            &self.client,
            self.source_url.clone(),
            self.source_method.clone(),
            translated,
        )
        .await?
        .bytes()
        .await?;

        self.codec.decode_one(&bytes).map_err(Into::into)
    }
}

#[async_trait]
impl<T, E> Sink<T> for WriteOnly<T, E>
where
    T: Sync,
    E: Encode<T> + Sync,
{
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

#[async_trait]
impl<T, Q, E, D, C> Sink<T> for ReadWrite<T, Q, E, D, C>
where
    T: Sync,
    Q: Sync,
    E: Encode<T> + Sync,
    D: Sync,
    C: Encode<T> + Sync,
{
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
    /*
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
    */
}
