//! The [`Source`] and [`Sink`] traits.

use crate::{
    errors::{FetchError, FetchOneError, SendError},
    query::Query,
};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt as _, iter as from_iter};
use std::slice::from_ref;

/// A type that can provide data given some query.
///
/// This trait provides several ways of fetching data regarding how much data is returned at a
/// time.
// TODO: Add support for writing to existing buffers, i.e. accepting `&mut Vec<T>` or `&mut [T]`
// and returning `Result<usize, _>`, similar to `std::io::Read`.
#[async_trait]
pub trait Source<T>
where
    T: Send,
{
    /// Fetch all data matching the query as a stream.
    ///
    /// The default implementation calls [`fetch_all`](Self::fetch_all) and creates a stream from
    /// the vector consisting either of entirely [`Ok`] values, or a single [`Err`] value.
    /// Crucially, this means that it is *not lazy*.
    #[inline]
    async fn fetch<'s>(
        &'s mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<BoxStream<'s, Result<T, FetchError>>, FetchError>
    where
        T: 's,
    {
        let vec = self.fetch_all(query).await?;
        Ok(from_iter(vec.into_iter().map(Ok)).boxed())
    }

    /// Fetch all data matching the query.
    async fn fetch_all(&mut self, query: &(dyn Query<T> + Sync)) -> Result<Vec<T>, FetchError>;

    /// Fetch a single entry matching the query. If no such entry exists,
    /// <code>[Err]\([`NoSuchEntry`](FetchOneError::NoSuchEntry))</code> is returned.
    ///
    /// This method imposes no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however uniquely identify one.
    #[inline]
    async fn fetch_one(&mut self, query: &(dyn Query<T> + Sync)) -> Result<T, FetchOneError> {
        self.fetch_all(query)
            .await?
            .into_iter()
            .next()
            .ok_or(FetchOneError::NoSuchEntry)
    }

    /// Fetch a single entry matching the query, if one exists.
    ///
    /// This method imposes no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however uniquely identify one.
    #[inline]
    async fn fetch_optional(
        &mut self,
        query: &(dyn Query<T> + Sync),
    ) -> Result<Option<T>, FetchError> {
        Ok(self.fetch_all(query).await?.into_iter().next())
    }

    /// Approximate the bounds on the number of elements that would be returned from the given
    /// query.
    ///
    /// Specifically, on success, returns a tuple `(lower, upper)` where `lower` is the lower bound
    /// and `upper` is the upper bound. A [`None`] value of `upper` means that there either is no
    /// upper bound, or it is larger than [`usize::MAX`].
    ///
    /// The intended use is for optimizations such as reserving capacity in buffers. The bounds
    /// should be interpreted only as optimizations, meaning they may not be relied upon for
    /// correctness or omitting bounds checks in usane code; implementations may return incorrect
    /// bounds.
    ///
    /// This method is not intended to actually connect to the source, and as such does not return
    /// a [`Result`] nor a [`Future`]. It also shouldn't need to consume neither `self` (as it
    /// would render the method pointless), nor `query`, so they are both borrowed.
    ///
    /// The default implementation returns <code>(0, [None])</code> which is always correct though
    /// minimally specific.
    #[expect(
        unused_variables,
        reason = "Avoids raising `clippy::renamed_function_params` in implementors."
    )]
    #[inline]
    fn size_hint(&self, query: &dyn Query<T>) -> (usize, Option<usize>) {
        Default::default()
    }
}

/// A type that can accept data.
///
/// This trait provides several ways of sending data regarding how much data is sent at a time.
#[async_trait]
pub trait Sink<T>
where
    T: Sync,
{
    /// Send all data from a slice.
    async fn send_all(&self, entries: &[T]) -> Result<(), SendError>;

    /// Send a single entry.
    #[inline]
    async fn send_one(&self, entry: &T) -> Result<(), SendError> {
        self.send_all(from_ref(entry)).await
    }
}
