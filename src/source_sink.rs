//! The [`Source`] and [`Sink`] traits.

use crate::errors::{FetchError, FetchOneError, SendError};
use futures::{
    FutureExt as _, Stream, StreamExt as _, TryFutureExt as _, TryStreamExt,
    stream::iter as from_iter,
};

/// A type that can provide data given some query.
///
/// This trait provides several ways of fetching data regarding how much data is returned at a
/// time. These all have intraconnected default implementations, which means that although no
/// methods are marked as "required", **an implementation must override at minimum either `fetch`
/// or `fetch_all`**, otherwise calls will always fail. That being said, often more efficient
/// implementations of the other methods are possible. Check the method documentations for more
/// information.
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
    fn fetch<'s>(
        self,
        query: Self::Query,
    ) -> impl Future<
        Output = Result<impl Stream<Item = Result<T, FetchError>> + Send + Unpin, FetchError>,
    > + Send
    + 's
    where
        'a: 's,
        Self: 's,
        T: Send + 's,
    {
        self.fetch_all(query)
            .map(move |res| res.map(|vec| from_iter(vec.into_iter().map(Ok))))
    }

    /// Fetch all data matching the query.
    ///
    /// The default implementation calls [`fetch`] and collects the results. This means that
    /// **must** override this function unless they instead override [`fetch`].
    ///
    /// [`fetch`]: Self::fetch
    fn fetch_all<'s>(
        self,
        query: Self::Query,
    ) -> impl Future<Output = Result<Vec<T>, FetchError>> + Send + 's
    where
        'a: 's,
        Self: 's,
        T: Send + 's,
    {
        self.fetch(query).and_then(TryStreamExt::try_collect)
    }

    /// Fetch a single entry matching the query. If no such entry exists,
    /// <code>[Err]\([`NoSuchEntry`](FetchOneError::NoSuchEntry))</code> is returned.
    ///
    /// This method poses no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however define an ordering.
    ///
    /// The default implementation calls [`fetch_optional`](Self::fetch_optional).
    fn fetch_one<'s>(
        self,
        query: Self::Query,
    ) -> impl Future<Output = Result<T, FetchOneError>> + Send + 's
    where
        'a: 's,
        Self: 's,
        T: Send + 's,
    {
        self.fetch_optional(query)
            .map(|res| res?.ok_or(FetchOneError::NoSuchEntry))
    }

    /// Fetch a single entry matching the query, if one exists.
    ///
    /// This method poses no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however uniquely identify one.
    ///
    /// The default implementation calls [`fetch`](Self::fetch) and returns the first item.
    fn fetch_optional<'s>(
        self,
        query: Self::Query,
    ) -> impl Future<Output = Result<Option<T>, FetchError>> + Send + 's
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
    /// a [`Result`] nor a [`Future`].
    ///
    /// The default implementation returns <code>(0, [None])</code> which is always correct though
    /// minimally specific.
    #[allow(
        unused_variables,
        reason = "Avoids raising `clippy::renamed_function_params` in implementors."
    )]
    fn size_hint(&self, query: Self::Query) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// A type that can accept data.
///
/// This trait provides several ways of sending data regarding how much data is sent at a time.
pub trait Sink<'a, T>: Sized {
    /// Send data from a stream.
    ///
    /// The default implementation collects the entries and calls [`send_all`](Self::send_all).
    fn send<'s, I>(self, entries: I) -> impl Future<Output = Result<(), SendError>> + Send + 's
    where
        Self: Send + 's,
        I: IntoIterator<Item = T> + Send + 's,
        T: Send,
    {
        async move {
            let buf = entries.into_iter().collect::<Vec<_>>();
            self.send_all(&buf).await
        }
        // entries.collect::<Vec<_>>().then(|v| self.send_all(&v))
    }

    /// Send all data from a slice.
    // `send_all` cannot have a default implementation based on `send` or `send_one` as they
    // capture `T` by value while `send_all` can only produce `&T` without placing a `Clone`
    // restriction on `T`. Additionally, `send` couldn't accept a stream of `&T` as `Stream`
    // doesn't define a lifetime parameter.
    fn send_all<'s>(
        self,
        entries: &'s [T],
    ) -> impl Future<Output = Result<(), SendError>> + Send + 's
    where
        'a: 's;

    /// Send a single entry.
    ///
    /// The default implementation calls [`send_all`](Self::send_all) with a slice containing only
    /// `entry`.
    fn send_one<'s>(self, entry: T) -> impl Future<Output = Result<(), SendError>> + Send + 's
    where
        Self: Send + 's,
        T: Send + 's,
    {
        async move {
            let buf = vec![entry];
            self.send_all(&buf).await
        }
        // self.send_all(&[entry])
    }
}
