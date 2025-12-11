//! The [`Source`] and [`Sink`] traits.

use crate::errors::{FetchError, FetchOneError, SendError};
use futures::{
    FutureExt as _, Stream, StreamExt as _, TryFutureExt as _, TryStreamExt,
    stream::iter as from_iter,
};
use std::array::from_ref;

/// A type that can provide data given some query.
///
/// This trait is intended to be implemented on reference types, and as such most methods consume
/// `self`.
///
/// This trait provides several ways of fetching data regarding how much data is returned at a
/// time. Some of these have intraconnected default implementations, which means that although some
/// mathods are not marked as "required", **an implementation must override at minimum either
/// [`fetch`](Self::fetch) or [`fetch_all`](Self::fetch_all)**, otherwise calls will always fail.
/// That being said, often more efficient implementations of both methods are possible. Check the
/// method documentations for more information.
// TODO: Add support for writing to existing buffers, i.e. accepting `&mut Vec<T>` or `&mut [T]`
// and returning `Result<usize, _>`, similar to `std::io::Read`.
pub trait Source<'a, T>: Sized {
    /// Data to be passed along with a request.
    type Query;

    /// Fetch all data matching the query as a stream.
    ///
    /// The default implementation calls [`fetch_all`] and creates a stream from the vector
    /// consisting either of entirely [`Ok`] values, or a single [`Err`] value. This means that
    /// implementors **must** override this function unless they instead override [`fetch_all`].
    ///
    /// [`fetch_all`]: Self::fetch_all
    // `Unpin` is needed to support this default implementation of `fetch_optional`. This bound is
    // likely to be less restrictive than the alternative, see comment below.
    // TODO: Is a default implementation that imposes neither restriction possible?
    #[inline]
    fn fetch(
        self,
        query: Self::Query,
    ) -> impl Future<
        Output = Result<impl Stream<Item = Result<T, FetchError>> + Send + Unpin, FetchError>,
    > + Send
    where
        T: Send,
    {
        self.fetch_all(query)
            .map(move |res| res.map(|vec| from_iter(vec.into_iter().map(Ok))))
    }

    /// Fetch all data matching the query.
    ///
    /// The default implementation calls [`fetch`] and collects the results. This means that
    /// implementors **must** override this function unless they instead override [`fetch`].
    ///
    /// [`fetch`]: Self::fetch
    #[inline]
    fn fetch_all(
        self,
        query: Self::Query,
    ) -> impl Future<Output = Result<Vec<T>, FetchError>> + Send
    where
        T: Send,
    {
        self.fetch(query).and_then(TryStreamExt::try_collect)
    }

    /// Fetch a single entry matching the query. If no such entry exists,
    /// <code>[Err]\([`NoSuchEntry`](FetchOneError::NoSuchEntry))</code> is returned.
    ///
    /// This method imposes no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however uniquely identify one.
    #[inline]
    fn fetch_one(self, query: Self::Query) -> impl Future<Output = Result<T, FetchOneError>>
    where
        Self: 'a,
        T: Send + 'a,
    {
        self.fetch_optional(query)
            .map(|res| res?.ok_or(FetchOneError::NoSuchEntry))
    }

    /// Fetch a single entry matching the query, if one exists.
    ///
    /// This method imposes no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however uniquely identify one.
    ///
    /// The default implementation calls [`fetch`](Self::fetch) and returns the first item.
    #[inline]
    fn fetch_optional<'s>(
        self,
        query: Self::Query,
    ) -> impl Future<Output = Result<Option<T>, FetchError>>
    where
        'a: 's,
        Self: 's,
        T: Send + 's,
    {
        // The simplest solution using `TryStreamExt::try_next` doesn't work as it borrows
        // `&mut self`, so the value is dropped. It could be circumvented by explicitly
        // creating an `async move` closure, but this introduces additional trait bounds (e.g.
        // `Self` and `Self::Query` requiring `Send`). This solution does however require `Unpin`
        // on the stream returned from `fetch`, see comment above.

        self.fetch(query)
            .and_then(move |stream| stream.into_future().map(|next| next.0.transpose()))
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
    fn size_hint(&self, query: &Self::Query) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// A type that can accept data.
///
/// This trait provides several ways of sending data regarding how much data is sent at a time.
/// These have intraconnected default implementations, which means that although no methods are
/// marked as "required", **an implementation must override at minimum either
/// [`send`](Self::send), [`send_all`](Self::send_all) or [`send_one`](Self::send_one)**,
/// otherwise calls will always fail. That being said, often more efficient implementations of
/// other methods are possible. Check the method documentations for more information.
pub trait Sink<T> {
    /// Send data from a stream.
    ///
    /// The default implementation calls [`send_one`] for each element of the stream. This means
    /// that implementors **must** override this function unless they instead override
    /// [`send_one`] or its dependencies.
    ///
    /// Extra care should be taken when calling this method on a type that does not override the
    /// default implementations, as sending each entry individually may be either very
    /// computationally expensive or may require a lot of waiting (e.g. on network requests). Check
    /// documentation of the concrete implementation for more details.
    ///
    /// [`send_one`]: Self::send_one
    #[inline]
    fn send<'s, I>(&self, entries: I) -> impl Future<Output = Result<(), SendError>> + Send
    where
        Self: Sync,
        // This `Send` bound is not actually required for this default implementation. It has been
        // imposed here to facilitate many implementations, avoiding requiring them to "define the
        // iterator type in advance" (on the `impl` block) to place additional bounds on it. The
        // additional bound is currently considered not too much of a restriction.
        I: IntoIterator<Item = &'s T> + Send,
        I::IntoIter: Send,
        T: Sync + 's,
    {
        from_iter(entries)
            .then(|entry| self.send_one(entry))
            .try_collect()
    }

    /// Send all data from a slice.
    ///
    /// The default implementation calls [`send`] with an iterator created from the slice. This
    /// means that implementors **must** override this function unless they instead override
    /// [`send`] or its dependencies.
    ///
    /// [`send`]: Self::send
    #[inline]
    fn send_all(&self, entries: &[T]) -> impl Future<Output = Result<(), SendError>> + Send
    where
        Self: Sync,
        T: Sync,
    {
        self.send(entries)
    }

    /// Send a single entry.
    ///
    /// The default implementation calls [`send_all`] with a slice containing only `entry`. This
    /// means that implementors **must** override this function unless they instead override
    /// [`send_all`] or its dependencies.
    ///
    /// [`send_all`]: Self::send_all
    #[inline]
    fn send_one(&self, entry: &T) -> impl Future<Output = Result<(), SendError>> + Send
    where
        Self: Sync,
        T: Sync,
    {
        self.send_all(from_ref(entry))
    }
}
