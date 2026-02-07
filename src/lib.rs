#![feature(never_type)]
#![feature(type_changing_struct_update)]

//! The data broker.

// TODO: Rework module visibility, nesting, public exports.

// Currently, `tokio` is only used by tests. It will be used more later, so instead of making
// it a test-only dependency for the time being, this is added temporarily to suppress warnings.
// TODO: Remove.
use async_trait::async_trait;
use bytes as _;
use futures::{
    StreamExt as _,
    future::try_join_all,
    stream::{BoxStream, FuturesUnordered, select_all},
};
use serde as _;
use serde_json as _;
use tokio as _;

pub mod errors;
use crate::errors::{FetchError, FetchOneError};

pub mod connector;

pub mod query;
use query::Query;

pub mod encode;
pub use encode::{Codec, Decode, Encode};

pub mod rest;

use connector::Source;

/// The broker.
#[expect(missing_debug_implementations, reason = "TODO")]
pub struct Broker<T> {
    // TODO: Add names?
    /// Sources added to the broker.
    sources: Vec<Box<dyn Source<T> + Send>>,
}

impl<T> Broker<T>
where
    T: Send,
{
    /// Add a source to the broker.
    #[inline]
    pub fn add_source(&mut self, source: Box<dyn Source<T> + Send>) {
        self.sources.push(source);
    }

    /// Fetch some data matching a query, selecting only up to a given amount for each source. In
    /// other words, this returns up to a number of entries up to the number of sources times
    /// `per_source` (the actual number might be lower if any source fetches fewer than
    /// `per_source` entries).
    ///
    /// # Errors
    ///
    /// Delegates errors returned by [`fetch`](Self::fetch).
    #[inline]
    pub async fn sample(
        &mut self,
        query: &(dyn Query<T> + Sync),
        per_source: usize,
    ) -> Result<Vec<T>, FetchError> {
        let mut out = Vec::with_capacity(self.sources.len() * per_source);
        let mut futures = self
            .sources
            .iter_mut()
            .map(|source| source.fetch(query))
            .collect::<FuturesUnordered<_>>();

        while let Some(sample) = futures.next().await {
            let mut sample = sample?;
            while let Some(entry) = sample.next().await {
                out.push(entry?);
            }
        }

        Ok(out)
    }
}

#[async_trait]
impl<T> Source<T> for Broker<T>
where
    T: Send,
{
    #[inline]
    async fn fetch<'s>(
        &'s mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
    where
        T: 's,
    {
        // TODO: Don't require awaiting all streams immediately. Might require own `Stream`
        // implementation.
        let futures = self
            .sources
            .iter_mut()
            .map(|source| source.fetch(query))
            .collect::<Vec<_>>();
        try_join_all(futures)
            .await
            .map(|streams| select_all(streams).boxed())
    }

    #[inline]
    async fn fetch_all(&mut self, query: &(dyn Query<T> + Sync)) -> Result<Vec<T>, FetchError> {
        let min_capacity = self
            .sources
            .iter()
            .map(|source| source.size_hint(query).0)
            .sum();
        let mut out = Vec::with_capacity(min_capacity);

        let futures = self
            .sources
            .iter_mut()
            .map(|source| source.fetch_all(query))
            .collect::<Vec<_>>();
        for vec in try_join_all(futures).await? {
            out.extend(vec);
        }

        Ok(out)
    }

    #[inline]
    async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
        let mut futures = self
            .sources
            .iter_mut()
            .map(|source| source.fetch_one(query))
            .collect::<FuturesUnordered<_>>();

        // TODO: Fix this messy code.

        let mut first_error = None;

        while let Some(result) = futures.next().await {
            match result {
                Ok(entry) => return Ok(entry),
                Err(FetchOneError::NoSuchEntry) => {},
                // Rather than terminate on first error, we try other sources. Maybe the first
                // one is just temporarily down. Due to the API, we can't return all errors, so
                // we arbitrarily return the first in the case that all sources fail.
                Err(FetchOneError::Fetch(err)) => {
                    first_error = Some(err);
                },
            }
        }

        Err(first_error.map_or(FetchOneError::NoSuchEntry, FetchOneError::Fetch))
    }

    #[inline]
    async fn fetch_optional(
        &mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<Option<T>, FetchError> {
        let mut futures = self
            .sources
            .iter_mut()
            .map(|source| source.fetch_optional(query))
            .collect::<FuturesUnordered<_>>();

        // TODO: Fix this messy code.

        let mut first_error = None;

        while let Some(result) = futures.next().await {
            match result {
                Ok(Some(entry)) => return Ok(Some(entry)),
                Ok(None) => {},
                // Rather than terminate on first error, we try other sources. Maybe the first
                // one is just temporarily down. Due to the API, we can't return all errors, so
                // we arbitrarily return the first in the case that all sources fail.
                Err(err) => {
                    first_error = Some(err);
                },
            }
        }

        first_error.map_or_else(|| Ok(None), Err)
    }

    #[inline]
    fn size_hint(&self, query: &dyn Query<T>) -> (usize, Option<usize>) {
        self.sources
            .iter()
            .map(|source| source.size_hint(query))
            .reduce(|(lower_acc, upper_acc), (lower, upper)| {
                let lower = lower_acc + lower;
                let upper = upper_acc.zip(upper).and_then(|(a, b)| a.checked_add(b));
                (lower, upper)
            })
            .unwrap_or_default()
    }
}
