//! The data broker.

use async_trait::async_trait;
use futures::{stream::iter as from_iter, Stream};

/// Trait for a source of data.
///
/// There are many kinds of data sources, and one communicates with them in different ways. For
/// example, some operate on a persistent connection while some simply return all data immediately
/// and finish. One may also need to send along additional data every time we access them.
///
/// This trait makes some choices based on expected use cases. One of these is to restrict both
/// input and output to [`Send`] types, which allows for default implementations of
/// [`fetch`](Self::fetch) and [`stream`](Self::stream) while still functioning in a multithreaded
/// runtime. Another is to accept the performance overhead associated with [`mod@async_trait`] in order to produce more flexible futures.
// async_trait generates some messy signatures, little of it useful.
#[cfg_attr(not(doc), async_trait)]
pub trait Source<T, Q>: Sized
where
    T: Send,
    Q: Send + 'static,
{
    /// The error type associated with this source's operations. Note that for
    /// [`stream`](Self::stream), this is used both for the creation of the stream and for when an
    /// element is polled.
    type Err;

    /// Fetch some data from the source. This may fetch all data, only some available at the time,
    /// or none at all; repeated calls may return the same data again, different data, or none at
    /// all.
    ///
    /// For sources that operate on pages, this might return one page of data, while
    /// [`fetch_all`](Self::fetch_all) would return all pages at once.
    ///
    /// Calls to this function should not block, though this is not a guarantee and should not be
    /// relied upon.
    ///
    /// The default implementation directly calls [`fetch_all`](Self::fetch_all).
    #[inline]
    async fn fetch(&self, query: Q) -> Result<Vec<T>, Self::Err> {
        self.fetch_all(query).await
    }

    /// Try to fetch all remaining data from the source. This may or may not return any data
    /// previously fetched.
    ///
    /// Calls to this function are expected to block if there is expected to be some more data
    /// available at a later time, though not just yet. Behavior for endless streams of data is
    /// left unspecified.
    async fn fetch_all(&self, query: Q) -> Result<Vec<T>, Self::Err>;

    /// Create a stream of data.
    ///
    /// The stream is an opaque type implementing [`Stream`] rather than an associated type to
    /// allow for a default implementation in terms of other functions, at the cost of flexibility.
    /// It is for example not possible for callers to add additional trait bounds to the produced
    /// stream.
    ///
    /// The default implementation calls [`fetch_all`](Self::fetch_all), and turns the returned
    /// vector into a stream. This is inefficient and defeats the point of using a stream, so
    /// implementors able to provide a better implementation should do so.
    #[inline]
    async fn stream(
        &self,
        query: Q,
    ) -> Result<impl Stream<Item = Result<T, Self::Err>>, Self::Err> {
        self.fetch_all(query)
            .await
            .map(|v| from_iter(v.into_iter().map(Ok)))
    }

    /// Approximate the bounds on the number of elements the source would yield, given a query.
    ///
    /// Specifically, returns a tuple `(lower, upper)` where `lower` is the lower bound and `upper`
    /// is the upper bond. A [`None`] value of `upper` means that there either is no upper bound,
    /// or it is larger than [`usize::MAX`].
    ///
    /// The intended use is for optimizations such as reserving capacity in buffers. The bounds are
    /// only approximations and must not be trusted in e.g. unsafe code to omit bound checks;
    /// implementations may return incorrect bounds.
    ///
    /// A query is provided as there may be additional information to be gained by examining the
    /// query in advance, without actually polling the source. As such this function is not
    /// `async`.
    ///
    /// The default implementation returns <code>(0, [None])</code> which is always technically
    /// correct.
    #[allow(
        unused_variables,
        reason = "Satisfies `clippy::renamed_function_params` transparent to implementors."
    )]
    #[inline]
    fn size_hint(&self, query: &Q) -> (usize, Option<usize>) {
        (0, None)
    }
}
