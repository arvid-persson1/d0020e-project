//! The data broker.

use futures::{
    FutureExt as _, StreamExt as _, TryFutureExt as _, TryStreamExt as _,
    future::{BoxFuture, err},
    stream::{BoxStream, iter as from_iter, once},
};
use std::io::Error as IoError;
use thiserror::Error;

/// The error type used by connectors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConnectorError {
    /// An [IO error](`IoError`) occured.
    #[error("An IO error occurred: {0}")]
    Io(#[from] IoError),
    /// No entry matching the given query exists.
    #[error("No entry matching the given query exists.")]
    NoSuchEntry,
}

/// A type that can provide data given some query.
///
/// This trait provides several ways of fetching data regarding how much data is returned at a
/// time. These all have intraconnected default implementations, which means that although no
/// methods are marked as "required", **an implementation must override at minimum either `fetch`
/// or `fetch_all`**, otherwise calls will always fail due to stack overflow or out-of-memory
/// errors. That being said, often more efficient implementations of the other methods are
/// possible. Check the method documentations for more information.
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
    fn fetch<'s>(self, query: Self::Query) -> BoxStream<'s, Result<T, ConnectorError>>
    where
        T: Send + 's,
    {
        self.fetch_all(query)
            .map(|r| match r {
                Ok(v) => from_iter(v.into_iter().map(Ok)).boxed(),
                Err(e) => once(err(e)).boxed(),
            })
            .flatten_stream()
            .boxed()
    }

    /// Fetch all data matching the query.
    ///
    /// The default implementation calls [`fetch`](Self::fetch) and collects the results. This
    /// means that implementors **must** override this function unless they instead override
    /// [`fetch`].
    ///
    /// [`fetch`]: Self::fetch
    fn fetch_all<'s>(self, query: Self::Query) -> BoxFuture<'s, Result<Vec<T>, ConnectorError>>
    where
        T: Send + 's,
    {
        self.fetch(query).try_collect().boxed()
    }

    /// Fetch a single entry matching the query. If no such entry exists, <code>[Err]\([`NoSuchEntry`])</code>
    /// is returned.
    ///
    /// This method poses no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however define an ordering.
    ///
    /// The default implementation calls [`fetch_optional`].
    ///
    /// [`NoSuchEntry`]: ConnectorError::NoSuchEntry
    /// [`fetch_optional`]: Self::fetch_optional
    fn fetch_one<'s>(self, query: Self::Query) -> BoxFuture<'s, Result<T, ConnectorError>>
    where
        'a: 's,
        Self: 's,
        T: Send + 's,
    {
        self.fetch_optional(query)
            .and_then(async |t| t.ok_or(ConnectorError::NoSuchEntry))
            .boxed()
    }

    /// Fetch a single entry matching the query, if one exists.
    ///
    /// This method poses no restriction on *which* entry should be returned, only that it should
    /// be one matching the query. The query might however define an ordering.
    ///
    /// The defualt implementation calls [`fetch`] and returns the first item.
    ///
    /// [`fetch`]: Self::fetch
    fn fetch_optional<'s>(
        self,
        query: Self::Query,
    ) -> BoxFuture<'s, Result<Option<T>, ConnectorError>>
    where
        'a: 's,
        Self: 's,
        T: Send + 's,
    {
        // The simplest solution using `TryStreamExt::try_next` doesn't work as it borrows
        // `&mut self`, so the value is dropped. It could be circumvented by explicitly
        // creating an `async move` closure.

        self.fetch(query)
            .into_future()
            .map(|x| x.0.transpose())
            .boxed()
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
    fn size_hint(self, query: Self::Query) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// A type that can accept data.
///
/// This trait provides several ways of sending data regarding how much data is sent at a time.
pub trait Sink<'a, T>: Sized {
    /// Send data from a stream.
    ///
    /// The default implementation collects the entries and calls [`send_all`].
    ///
    /// [`send_all`]: Self::send_all
    fn send<'s, B>(self, entries: BoxStream<'s, T>) -> BoxFuture<'s, Result<(), ConnectorError>>
    where
        Self: Send + 's,
        T: Send + 's,
    {
        entries
            .collect::<Vec<_>>()
            .then(move |v| self.send_all(&v))
            .boxed()
    }

    /// Send all data from a slice.
    fn send_all<'s>(self, entries: &[T]) -> BoxFuture<'s, Result<(), ConnectorError>>;

    /// Send a single entry.
    ///
    /// The default implementation calls [`send_all`] with a slice containing only `entry`.
    ///
    /// [`send_all`]: Self::send_all
    fn send_one<'s>(self, entry: T) -> BoxFuture<'s, Result<(), ConnectorError>> {
        self.send_all(&[entry])
    }
}
