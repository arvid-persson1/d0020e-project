#![feature(type_changing_struct_update)]
#![feature(never_type)]

//! The data broker.

// TODO: Rework module visibility, nesting, public exports.

// Currently, `tokio` is only used by tests. It will be used more later, so instead of making
// it a test-only dependency for the time being, this is added temporarily to suppress warnings.
// TODO: Remove.
use crate::connector::MemorySource;
use crate::connector::Sink as _;
use crate::connector::Source;
use crate::errors::SendError;
use async_trait::async_trait;
use futures::{
    StreamExt as _,
    future::try_join_all,
    stream::{BoxStream, FuturesUnordered, select_all},
};
use serde::Serialize;
use std::any::Any;
use std::{collections::HashSet, hash::Hash};
use tokio as _;

#[allow(
    unused_extern_crates,
    reason = "Our custom procedural macros (like Queryable) hardcode the `broker::` path. This alias allows those macros to compile when used internally within this crate."
)]
extern crate self as broker;

use diesel as _;
use diesel_derive_enum as _;

pub mod errors;
use crate::errors::{FetchError, FetchOneError};

pub mod connector;

pub mod query;
pub use query::Query;

pub mod encode;
pub use encode::{Codec, Decode, Encode};

#[cfg(feature = "rest")]
pub mod rest;

#[cfg(feature = "postgres")]
pub mod postgres;

/// struct for sending sourcename to frontend
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult<T> {
    /// Source as item
    pub item: T,
    /// Source as String
    pub source: String,
}

/// The broker.
#[expect(missing_debug_implementations, reason = "TODO")]
pub struct Broker<T> {
    // TODO: Add names?
    /// Sources added to the broker.
    sources: Vec<(String, Box<dyn Source<T> + Send>)>,
}

impl<T> Broker<T>
where
    T: Send,
{
    /// Constructs a broker with no sources.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add a source to the broker.
    #[inline]
    pub fn add_source(&mut self, name: impl Into<String>, source: Box<dyn Source<T> + Send>) {
        self.sources.push((name.into(), source));
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
            .map(|source| source.1.fetch(query))
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

impl<T> Default for Broker<T>
where
    T: Send,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> Source<T> for Broker<T>
where
    T: Eq + Hash + Send + 'static,
{
    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    async fn fetch<'s>(
        &'s mut self,
        query: &'s (dyn Query<T> + Sync),
    ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
    where
        T: 's,
    {
        // TODO: Don't require awaiting all streams immediately. Might require own `Stream`
        // implementation.
        let futures = self
            .sources
            .iter_mut()
            .map(|source| source.1.fetch(query))
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
            .map(|source| source.1.size_hint(query).0)
            .sum();
        let mut out = HashSet::with_capacity(min_capacity);

        for per_source in self
            .sources
            .iter_mut()
            .map(|source| source.1.fetch_all(query))
        {
            out.extend(per_source.await?);
        }

        Ok(out.into_iter().collect())
    }

    #[inline]
    async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
        let mut futures = self
            .sources
            .iter_mut()
            .map(|source| source.1.fetch_one(query))
            .collect::<FuturesUnordered<_>>();

        // TODO: Fix this messy code.

        let mut error = None;

        while let Some(result) = futures.next().await {
            match result {
                Ok(entry) => return Ok(entry),
                Err(FetchOneError::NoSuchEntry) => {},
                // Rather than terminate on first error, we try other sources. Maybe the first
                // one is just temporarily down. Due to the API, we can't return all errors, so
                // we arbitrarily return the first in the case that all sources fail.
                Err(FetchOneError::Fetch(err)) => {
                    error = Some(err);
                },
            }
        }

        Err(error.map_or(FetchOneError::NoSuchEntry, FetchOneError::Fetch))
    }

    #[inline]
    async fn fetch_optional(
        &mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<Option<T>, FetchError> {
        let mut futures = self
            .sources
            .iter_mut()
            .map(|source| source.1.fetch_optional(query))
            .collect::<FuturesUnordered<_>>();

        // TODO: Fix this messy code.

        let mut error = None;

        while let Some(result) = futures.next().await {
            match result {
                Ok(Some(entry)) => return Ok(Some(entry)),
                Ok(None) => {},
                // Rather than terminate on first error, we try other sources. Maybe the first
                // one is just temporarily down. Due to the API, we can't return all errors, so
                // we arbitrarily return the first in the case that all sources fail.
                Err(err) => {
                    error = Some(err);
                },
            }
        }

        error.map_or_else(|| Ok(None), Err)
    }

    #[inline]
    fn size_hint(&self, query: &dyn Query<T>) -> (usize, Option<usize>) {
        self.sources
            .iter()
            .map(|source| source.1.size_hint(query))
            .reduce(|(lower_acc, upper_acc): (_, _), (lower, upper)| {
                let lower = lower_acc + lower;
                let upper = upper_acc
                    .zip(upper)
                    .and_then(|(a, b): (_, _)| a.checked_add(b));
                (lower, upper)
            })
            .unwrap_or_default()
    }
}

impl<T> Broker<T>
where
    T: Send + Sync + Clone + 'static,
{
    /// Returns all sources currently registered with the broker.
    ///
    /// Sources are returned as `(name, source)` pairs, where the
    /// name uniquely identifies the source within the broker.
    /// The returned slice is read-only and reflects the broker's
    /// current in-memory state.
    #[must_use]
    #[inline]
    pub fn sources(&self) -> &[(String, Box<dyn Source<T> + Send>)] {
        &self.sources
    }

    /// Adds an item to a specific named source.
    ///
    /// This method only works for sources that implement [`Sink`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The source does not exist.
    /// - The source is not writable.
    /// - The write operation fails.
    #[inline]
    pub async fn add_to_source(&mut self, name: &str, item: T) -> Result<(), SendError>
    where
        T: Clone + 'static,
    {
        for (source_name, source) in &mut self.sources {
            if source_name == name
                && let Some(sink) = source.as_any_mut().downcast_mut::<MemorySource<T>>()
            {
                return sink.send_one(&item).await;
            }
        }
        Err(SendError::Rejected)
    }

    /// Fetch all items from all registered sources, including their source name.
    ///
    /// # Errors
    ///
    /// Returns [`FetchError`] if any underlying source fails to fetch
    /// or decode its results.
    #[inline]
    pub async fn fetch_all_with_source(
        &mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<Vec<SearchResult<T>>, FetchError> {
        let mut out = Vec::new();

        for (name, source) in &mut self.sources {
            let results = source.fetch_all(query).await?;

            for item in results {
                out.push(SearchResult {
                    item,
                    source: name.clone(),
                });
            }
        }

        Ok(out)
    }
}
