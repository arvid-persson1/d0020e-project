#![feature(type_changing_struct_update)]
#![feature(never_type)]
#![allow(trivial_casts, reason = "Necessary for dyn casts.")]

//! The data broker.

// TODO: Rework module visibility, nesting, public exports.

// Currently, `tokio` is only used by tests. It will be used more later, so instead of making
// it a test-only dependency for the time being, this is added temporarily to suppress warnings.
// TODO: Remove.
use async_trait::async_trait;
use futures::{
    FutureExt as _, StreamExt as _,
    future::{join_all, try_join_all},
    stream::{BoxStream, FuturesUnordered, select_all},
};
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
use crate::errors::{FetchError, FetchOneError, SendError};

pub mod connector;
use connector::{Sink, Source};

pub mod query;
pub use query::Query;

pub mod encode;
pub use encode::{Codec, Decode, Encode};

#[cfg(feature = "rest")]
pub mod rest;

#[cfg(feature = "postgres")]
pub mod postgres;

/// A "full" connector; one that is both a [`Source`] and a [`Sink`].
trait Full<T>: Source<T> + Sink<T> + Send + Sync
where
    T: Send + Sync,
{
}
impl<C, T> Full<T> for C
where
    C: Source<T> + Sink<T> + Send + Sync,
    T: Send + Sync,
{
}

/// The broker.
#[expect(missing_debug_implementations, reason = "TODO")]
pub struct Broker<T> {
    /// Complete connectors ([`Source`] and [`Sink`]) added to the broker.
    connectors: Vec<(Box<str>, Box<dyn Full<T>>)>,
    /// Sources added to the broker.
    sources: Vec<(Box<str>, Box<dyn Source<T> + Send + Sync>)>,
    /// Sinks added to the broker.
    sinks: Vec<(Box<str>, Box<dyn Sink<T> + Send + Sync>)>,
}

impl<T> Broker<T> {
    /// Constructs a broker with no sources.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self {
            connectors: Vec::new(),
            sources: Vec::new(),
            sinks: Vec::new(),
        }
    }

    /// Add a source to the broker.
    #[inline]
    pub fn add_source(&mut self, name: Box<str>, source: Box<dyn Source<T> + Send + Sync>) {
        self.sources.push((name, source));
    }
}

impl<T> Broker<T>
where
    T: Send,
{
    /// Fetch some data matching a query, selecting only up to a given amount for each source.
    ///
    /// In other words, this returns up to a number of entries up to the number of sources times
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
        let mut out = Vec::with_capacity((self.sources.len() + self.connectors.len()) * per_source);
        let mut futures = self
            .sources
            .iter_mut()
            .map(|(name, source)| (&**name, &mut **source))
            .chain(self.connectors.iter_mut().map(|(name, source)| {
                (&**name, &mut **source as &mut (dyn Source<T> + Send + Sync))
            }))
            .map(|(_, source)| source.fetch(query))
            .collect::<FuturesUnordered<_>>();

        while let Some(sample) = futures.next().await {
            let mut sample = sample?;
            while let Some(entry) = sample.next().await {
                out.push(entry?);
            }
        }

        Ok(out)
    }

    /// Fetch some data matching a query, selecting only up to a given amount for each source.
    ///
    /// In other words, this returns up to a number of entries up to the number of sources times
    /// `per_source` (the actual number might be lower if any source fetches fewer than
    /// `per_source` entries).
    ///
    /// Includes names of sources.
    ///
    /// # Errors
    ///
    /// Delegates errors returned by [`fetch`](Self::fetch).
    #[inline]
    pub async fn sample_named(
        &mut self,
        query: &(dyn Query<T> + Sync),
        per_source: usize,
    ) -> Result<Vec<(&str, T)>, FetchError> {
        let mut out = Vec::with_capacity((self.sources.len() + self.connectors.len()) * per_source);
        let mut futures = self
            .sources
            .iter_mut()
            .map(|(name, source)| (&**name, &mut **source))
            .chain(self.connectors.iter_mut().map(|(name, source)| {
                (&**name, &mut **source as &mut (dyn Source<T> + Send + Sync))
            }))
            .map(|(name, source)| source.fetch(query).map(move |fut| (name, fut)))
            .collect::<FuturesUnordered<_>>();

        while let Some((name, sample)) = futures.next().await {
            let mut sample = sample?;
            while let Some(entry) = sample.next().await {
                out.push((name, entry?));
            }
        }

        Ok(out)
    }
}

impl<T> Broker<T>
where
    T: Sync,
{
    /// Send all data from a slice to all sinks.
    ///
    /// Returns an error for each sink that fails, along with the name of the sink. An empty output
    /// means all sinks succeeded.
    #[inline]
    #[must_use]
    pub async fn send_all_unchecked(&self, entries: &[T]) -> Box<[(&str, SendError)]> {
        let futures = self
            .sinks
            .iter()
            .map(|(name, sink)| (&**name, &**sink))
            .chain(
                self.connectors
                    .iter()
                    .map(|(name, sink)| (&**name, &**sink as &(dyn Sink<T> + Send + Sync))),
            )
            .map(|(name, sink)| {
                sink.send_all(entries)
                    .map(move |res| res.map_err(|err| (name, err)))
            });
        join_all(futures)
            .await
            .into_iter()
            .filter_map(Result::err)
            .collect()
    }

    /// Send a single entry to all sinks.
    ///
    /// Returns an error for each sink that fails, along with the name of the sink. An empty output
    /// means all sinks succeeded.
    #[inline]
    #[must_use]
    pub async fn send_one_unchecked(&self, entry: &T) -> Box<[(&str, SendError)]> {
        let futures = self
            .sinks
            .iter()
            .map(|(name, sink)| (&**name, &**sink))
            .chain(
                self.connectors
                    .iter()
                    .map(|(name, sink)| (&**name, &**sink as &(dyn Sink<T> + Send + Sync))),
            )
            .map(|(name, sink)| {
                sink.send_one(entry)
                    .map(move |res| res.map_err(|err| (name, err)))
            });
        join_all(futures)
            .await
            .into_iter()
            .filter_map(Result::err)
            .collect()
    }
}

impl<T> Broker<T>
where
    T: Send + Eq + Hash,
{
    /// Fetch all data matching the query.
    ///
    /// Includes, for each entry, which source it came from.
    ///
    /// # Errors
    ///
    /// Fails if any source fails.
    #[inline]
    pub async fn fetch_all_named(
        &mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<Vec<(&str, T)>, FetchError> {
        let min_capacity = self
            .sources
            .iter()
            .map(|(name, source)| (&**name, &**source))
            .chain(
                self.connectors
                    .iter()
                    .map(|(name, source)| (&**name, &**source as &(dyn Source<T> + Send + Sync))),
            )
            .map(|(_, source)| source.size_hint(query).0)
            .sum();
        let mut out = HashSet::with_capacity(min_capacity);

        for (name, per_source) in self
            .sources
            .iter_mut()
            .map(|(name, source)| (&**name, &mut **source))
            .chain(self.connectors.iter_mut().map(|(name, source)| {
                (&**name, &mut **source as &mut (dyn Source<T> + Send + Sync))
            }))
            .map(|(name, source)| (name, source.fetch_all(query)))
        {
            out.extend(per_source.await?.into_iter().map(|entry| (&*name, entry)));
        }

        Ok(out.into_iter().collect())
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
    T: Eq + Hash + Send,
{
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
            .map(|(name, source)| (&**name, &mut **source))
            .chain(self.connectors.iter_mut().map(|(name, source)| {
                let source: &mut (dyn Source<T> + Send + Sync) = &mut **source;
                (&**name, source)
            }))
            .map(|(_, source)| source.fetch(query))
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
            .map(|(name, source)| (&**name, &**source))
            .chain(self.connectors.iter().map(|(name, source)| {
                let source: &(dyn Source<T> + Send + Sync) = &**source;
                (&**name, source)
            }))
            .map(|(_, source)| source.size_hint(query).0)
            .sum();
        let mut out = HashSet::with_capacity(min_capacity);

        for per_source in self
            .sources
            .iter_mut()
            .map(|(name, source)| (&**name, &mut **source))
            .chain(self.connectors.iter_mut().map(|(name, source)| {
                let source: &mut (dyn Source<T> + Send + Sync) = &mut **source;
                (&**name, source)
            }))
            .map(|(_, source)| source.fetch_all(query))
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
            .map(|(name, source)| (&**name, &mut **source))
            .chain(self.connectors.iter_mut().map(|(name, source)| {
                let source: &mut (dyn Source<T> + Send + Sync) = &mut **source;
                (&**name, source)
            }))
            .map(|(_, source)| source.fetch_one(query))
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
            .map(|(name, source)| (&**name, &mut **source))
            .chain(self.connectors.iter_mut().map(|(name, source)| {
                let source: &mut (dyn Source<T> + Send + Sync) = &mut **source;
                (&**name, source)
            }))
            .map(|(_, source)| source.fetch_optional(query))
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
            .map(|(name, source)| (&**name, &**source))
            .chain(
                self.connectors
                    .iter()
                    .map(|(name, source)| (&**name, &**source as &(dyn Source<T> + Send + Sync))),
            )
            .map(|(_, source)| source.size_hint(query))
            .reduce(|(lower_acc, upper_acc), (lower, upper)| {
                let lower = lower_acc + lower;
                let upper = upper_acc.zip(upper).and_then(|(a, b)| a.checked_add(b));
                (lower, upper)
            })
            .unwrap_or_default()
    }
}

#[async_trait]
impl<T> Sink<T> for Broker<T>
where
    T: Sync,
{
    /// Send all data from a slice to all sinks.
    ///
    /// Sends to each sink in a sequential fashion, stopping on first error. To run in parallell
    /// and keep going on error, see [`send_all_unchecked`](Self::send_all_unchecked).
    #[inline]
    async fn send_all(&self, entries: &[T]) -> Result<(), SendError> {
        for (_, sink) in &self.sinks {
            sink.send_all(entries).await?;
        }

        for (_, sink) in &self.connectors {
            sink.send_all(entries).await?;
        }

        Ok(())
    }

    /// Send a single entry to all sinks.
    ///
    /// Sends to each sink in a sequential fashion, stopping on first error. To run in parallell
    /// and keep going on error, see [`send_one_unchecked`](Self::send_one_unchecked).
    #[inline]
    async fn send_one(&self, entry: &T) -> Result<(), SendError> {
        for (_, sink) in &self.sinks {
            sink.send_one(entry).await?;
        }

        for (_, sink) in &self.connectors {
            sink.send_one(entry).await?;
        }

        Ok(())
    }
}
