// This is where I always just get lost when trying to create this connector.
// I give up now and need to actually sit down and talk to someone who can understand the project structure properly.

use crate::{
    Query,
    connector::{Sink, Source},
    encode::{Codec, Decode, Encode},
    errors::{ConnectionError, DecodeError, FetchError, FetchOneError, SendError},
    query::{GraphQLQuery, Single},
};
use async_trait::async_trait;
use futures::{StreamExt as _, TryStreamExt as _, future::ready, stream::BoxStream};
use reqwest::{Body, Client, Response, Url};
use std::{io::Error as IoError, marker::PhantomData};

#[derive(Debug, Clone)]
pub struct ReadOnly<T, D> {
    url: Url,
    client: Client,
    decoder: D,
    _phantom: PhantomData<T>,
}

#[derive(Debug, Clone)]
pub struct WriteOnly<T, E> {
    url: Url,
    client: Client,
    encoder: E,
    _phantom: PhantomData<T>,
}

#[derive(Debug, Clone)]
pub struct ReadWrite<T, E, D, C> {
    source_url: Url,
    sink_url: Url,
    client: Client,
    codec: Codec<T, E, D, C>,
    _phantom: PhantomData<T>,
}

/// Helper for Source implementation. NOTE: AI, double check!!!
async fn fetch_impl(
    client: &Client,
    url: Url,
    query: GraphQLQuery<'_>,
) -> Result<Response, FetchError> {
    let body = serde_json::to_vec(&query).map_err(|e| FetchError::InvalidQuery(Box::new(e)))?;
    client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(Into::into)
}

/// Helper for Sink implementation. NOTE: AI, double check!!!
async fn send_impl<B>(client: &Client, url: Url, body: B) -> Result<Response, ConnectionError>
where
    B: Into<Body>,
{
    let request = client.post(url).body(body).build().unwrap(); // safe unwrap – URL already validated
    client.execute(request).await.map_err(Into::into)
}

#[async_trait]
impl<T, D> Source<T> for ReadOnly<T, D>
where
    T: Send,
    D: Decode<T> + Send + Sync,
{
    async fn fetch<'s>(
        &'s mut self,
        query: &'s (dyn Query<T> + Sync),
    ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
    where
        T: 's,
    {
        let Single { query, residue } = query.to_graphql_single();

        let bytes = fetch_impl(&self.client, self.url.clone(), query)
            .await?
            .bytes_stream()
            .map_err(|e| ConnectionError::Io(IoError::other(e)));

        let apply_residue = move |res| {
            ready(match res {
                Ok(entry) if residue.iter().all(|part| part.evaluate(&entry)) => Some(Ok(entry)),
                Ok(_) => None,
                Err(err) => Some(Err(FetchError::from(err))),
            })
        };

        self.decoder
            .decode(bytes)
            .await
            .map(|output| output.filter_map(apply_residue).boxed())
            .map_err(Into::into)
    }

    async fn fetch_all(&mut self, query: &(dyn Query<T> + Sync)) -> Result<Vec<T>, FetchError> {
        let Single { query, residue } = query.to_graphql_single();

        let bytes = fetch_impl(&self.client, self.url.clone(), query).await?;
        let bytes = bytes.bytes().await?;

        self.decoder
            .decode_all(&bytes)
            .map(|mut entries| {
                entries.retain(|entry| residue.iter().all(|part| part.evaluate(entry)));
                entries
            })
            .map_err(|err| DecodeError(Box::new(err)).into())
    }

    async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
        let Single { query, residue } = query.to_graphql_single();

        let bytes = fetch_impl(&self.client, self.url.clone(), query)
            .await?
            .bytes_stream()
            .map_err(|err| {
                // HTTP errors should be raised by `fetch_impl`, and already have been returned.
                debug_assert!(err.status().is_none());
                ConnectionError::Io(IoError::other(err))
            });

        let mut error = None;

        let mut stream = self.decoder.decode(bytes).await?;
        while let Some(result) = stream.next().await {
            match result {
                Ok(entry) if residue.iter().all(|part| part.evaluate(&entry)) => return Ok(entry),
                Ok(_) => {},
                Err(err) => error = Some(err),
            }
        }

        Err(error.map_or(FetchOneError::NoSuchEntry, Into::into))
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
        send_impl(&self.client, self.url.clone(), Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    #[inline]
    async fn send_one(&self, entry: &T) -> Result<(), SendError> {
        let body = self.encoder.encode_one(entry).map_err(SendError::Encode)?;
        send_impl(&self.client, self.url.clone(), Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }
}

#[async_trait]
impl<T, E, D, C> Source<T> for ReadWrite<T, E, D, C>
where
    T: Send + Sync,
    E: Send + Sync,
    D: Decode<T> + Send + Sync,
    C: Decode<T> + Send + Sync,
{
    #[inline]
    async fn fetch<'s>(
        &'s mut self,
        query: &'s (dyn Query<T> + Sync),
    ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
    where
        T: 's,
    {
        let Single { query, residue } = query.to_graphql_single();

        let bytes = fetch_impl(&self.client, self.source_url.clone(), query)
            .await?
            .bytes_stream()
            .map_err(|err| {
                // HTTP errors should be raised by `fetch_impl`, and already have been returned.
                debug_assert!(err.status().is_none());
                ConnectionError::Io(IoError::other(err))
            });

        let apply_residue = move |res| {
            ready(match res {
                Ok(entry) if residue.iter().all(|part| part.evaluate(&entry)) => Some(Ok(entry)),
                Ok(_) => None,
                Err(err) => Some(Err(FetchError::from(err))),
            })
        };

        self.codec
            .decode(bytes)
            .await
            .map(|output| output.filter_map(apply_residue).boxed())
            .map_err(Into::into)
    }

    #[inline]
    async fn fetch_all(&mut self, query: &(dyn Query<T> + Sync)) -> Result<Vec<T>, FetchError> {
        let Single { query, residue } = query.to_graphql_single();

        let bytes = fetch_impl(&self.client, self.source_url.clone(), query)
            .await?
            .bytes()
            .await?;

        self.codec
            .decode_all(&bytes)
            .map(|mut entries| {
                entries.retain(|entry| residue.iter().all(|part| part.evaluate(entry)));
                entries
            })
            .map_err(|err| DecodeError(Box::new(err)).into())
    }

    #[inline]
    async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
        let Single { query, residue } = query.to_graphql_single();

        let bytes = fetch_impl(&self.client, self.source_url.clone(), query)
            .await?
            .bytes_stream()
            .map_err(|err| {
                // HTTP errors should be raised by `fetch_impl`, and already have been returned.
                debug_assert!(err.status().is_none());
                ConnectionError::Io(IoError::other(err))
            });

        // TODO: Fix this messy code.

        let mut error = None;

        let mut stream = self.codec.decode(bytes).await?;
        while let Some(result) = stream.next().await {
            match result {
                Ok(entry) if residue.iter().all(|part| part.evaluate(&entry)) => return Ok(entry),
                Ok(_) => {},
                Err(err) => error = Some(err),
            }
        }

        Err(error.map_or(FetchOneError::NoSuchEntry, Into::into))
    }
}

#[async_trait]
impl<T, E, D, C> Sink<T> for ReadWrite<T, E, D, C>
where
    T: Sync,
    E: Encode<T> + Sync,
    D: Sync,
    C: Encode<T> + Sync,
{
    #[inline]
    async fn send_all(&self, entries: &[T]) -> Result<(), SendError> {
        let body = self.codec.encode_all(entries).map_err(SendError::Encode)?;
        send_impl(&self.client, self.sink_url.clone(), Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    #[inline]
    async fn send_one(&self, entry: &T) -> Result<(), SendError> {
        let body = self.codec.encode_one(entry).map_err(SendError::Encode)?;
        send_impl(&self.client, self.sink_url.clone(), Vec::from(body))
            .await
            .map(|_| ())
            .map_err(Into::into)
    }
}
