use crate::{
    Query,
    connector::{Sink, Source},
    encode::{Codec, Decode, Encode},
    errors::{ConnectionError, DecodeError, FetchError, FetchOneError, SendError},
    query::{HttpQuery, Single},
};
use async_trait::async_trait;
use futures::{StreamExt as _, TryStreamExt as _, future::ready, stream::BoxStream};
use reqwest::{Body, Client, Response, Url};
use serde_json::{Value, json};
use std::{io::Error as IoError, marker::PhantomData};

// NOTE: Don't make things harder for not reason, keep it simple to avoid confusion!
//
// #[derive(Debug, Clone)]
// pub struct ReadOnly<T, D> {
//     url: Url,
//     client: Client,
//     decoder: D,
//     _phantom: PhantomData<T>,
// }
//
// #[derive(Debug, Clone)]
// pub struct WriteOnly<T, E> {
//     url: Url,
//     client: Client,
//     encoder: E,
//     _phantom: PhantomData<T>,
// }

#[derive(Debug, Clone)]
pub struct ReadWrite<T, E, D, C> {
    url: Url,
    client: Client,
    codec: Codec<T, E, D, C>,
    _phantom: PhantomData<T>,
}

// NOTE: I think the fetch and send helpers should be REALLY similar.

/// Helper that sends query and fetches result.
// TODO: COMPLETE THIS!!!
async fn fetch_impl(
    client: &Client,
    url: Url,
    resolver: &str,
    resolver_input: &str,
    value: &str,
    fields: Vec<&str>,
    // Figure out the return value
) -> Result<Response, FetchError> {
    // Create the body depending on resolver name, input and what you want to show.
    let body = body_builder(("query", resolver, resolver_input, value, fields));
    // TODO: ACTUALLY HANDLE THE POSSIBLE ERROR!
    client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(Into::into)
}

/// Helper that sends data using reqwest.
async fn send_impl<B>(client: &Client, url: Url, body: B) -> Result<Response, ConnectionError>
where
    B: Into<Body>,
{
    // safe unwrap – URL already validated
    let request = client.post(url).body(body).build().unwrap();
    client.execute(request).await.map_err(Into::into)
}

// As far as I've gathered this should be synchronous.
// NOTE:The check if there is an input might not be needed
fn body_builder(
    (method, resolver, resolver_input, value, fields): (&str, &str, &str, &str, Vec<&str>),
) -> Value {
    let fields_formatted = fields.join(", ");
    let mut query;
    if resolver_input == "" {
        query = format!("{method} {{ {resolver} {{ {fields_formatted} }} }}");
    } else {
        query = format!(
            "{method} {{ {resolver}({resolver_input}: {value}) {{ {fields_formatted} }} }}"
        );
    }
    json!({"query": query})
}

// TODO: I have three main solutions to solve the current problem:
// 1 - I use the existing httpQuery and just have a different fetch_impl and send_impl that
//   restructures for graphql. I think this would be fastest and least painful, but I don't get how
//   this would work with the "graphql" featuer
// 2 - I can add the function to_graphql_single to the query trait and make it use the
//   to_http_single function within the default solution, then use it here in graphql.rs. This
//   would make using the feature easy, but I think it would force the "graphql" feature to also
//   use "rest" automatically.
// 3 - I can add the function to_graphql_single to the query trait and implement it properly. This
//   though, I think would be VERY time consuming since it's needed for EVERY SINGLE primitive.
// -------------------------------------------------------------------------------------------------
// All three of these solutions *should* be able to reuse most things like the builder though I'm
// unsure which would be worth to spring for at this point. I also believe that the graphql.rs file
// here would be needed. though I don't get how it would be linked to the feature "graphql".

// #[async_trait]
// impl<T, D> Source<T> for ReadOnly<T, D>
// where
//     T: Send,
//     D: Decode<T> + Send + Sync,
// {
//     async fn fetch<'s>(
//         &'s mut self,
//         query: &'s (dyn Query<T> + Sync),
//     ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
//     where
//         T: 's,
//     {
//         let Single { query, residue } = query.to_graphql_single();
//
//         let bytes = fetch_impl(&self.client, self.url.clone(), query)
//             .await?
//             .bytes_stream()
//             .map_err(|e| ConnectionError::Io(IoError::other(e)));
//
//         let apply_residue = move |res| {
//             ready(match res {
//                 Ok(entry) if residue.iter().all(|part| part.evaluate(&entry)) => Some(Ok(entry)),
//                 Ok(_) => None,
//                 Err(err) => Some(Err(FetchError::from(err))),
//             })
//         };
//
//         self.decoder
//             .decode(bytes)
//             .await
//             .map(|output| output.filter_map(apply_residue).boxed())
//             .map_err(Into::into)
//     }
//
//     async fn fetch_all(&mut self, query: &(dyn Query<T> + Sync)) -> Result<Vec<T>, FetchError> {
//         let Single { query, residue } = query.to_graphql_single();
//
//         let bytes = fetch_impl(&self.client, self.url.clone(), query).await?;
//         let bytes = bytes.bytes().await?;
//
//         self.decoder
//             .decode_all(&bytes)
//             .map(|mut entries| {
//                 entries.retain(|entry| residue.iter().all(|part| part.evaluate(entry)));
//                 entries
//             })
//             .map_err(|err| DecodeError(Box::new(err)).into())
//     }
//
//     async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
//         let Single { query, residue } = query.to_graphql_single();
//
//         let bytes = fetch_impl(&self.client, self.url.clone(), query)
//             .await?
//             .bytes_stream()
//             .map_err(|err| {
//                 // HTTP errors should be raised by `fetch_impl`, and already have been returned.
//                 debug_assert!(err.status().is_none());
//                 ConnectionError::Io(IoError::other(err))
//             });
//
//         let mut error = None;
//
//         let mut stream = self.decoder.decode(bytes).await?;
//         while let Some(result) = stream.next().await {
//             match result {
//                 Ok(entry) if residue.iter().all(|part| part.evaluate(&entry)) => return Ok(entry),
//                 Ok(_) => {},
//                 Err(err) => error = Some(err),
//             }
//         }
//
//         Err(error.map_or(FetchOneError::NoSuchEntry, Into::into))
//     }
// }
//
// #[async_trait]
// impl<T, E> Sink<T> for WriteOnly<T, E>
// where
//     T: Sync,
//     E: Encode<T> + Sync,
// {
//     #[inline]
//     async fn send_all(&self, entries: &[T]) -> Result<(), SendError> {
//         let body = self
//             .encoder
//             .encode_all(entries)
//             .map_err(SendError::Encode)?;
//         send_impl(&self.client, self.url.clone(), Vec::from(body))
//             .await
//             .map(|_| ())
//             .map_err(Into::into)
//     }
//
//     #[inline]
//     async fn send_one(&self, entry: &T) -> Result<(), SendError> {
//         let body = self.encoder.encode_one(entry).map_err(SendError::Encode)?;
//         send_impl(&self.client, self.url.clone(), Vec::from(body))
//             .await
//             .map(|_| ())
//             .map_err(Into::into)
//     }
// }

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
