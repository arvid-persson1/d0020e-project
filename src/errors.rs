#![allow(
    unused,
    reason = "False positives as consequence of `error_set` macro."
)]
#![allow(
    missing_docs,
    reason = "False positives as consequence of `error_set` macro."
)]

use error_set::error_set;
use reqwest::StatusCode;
use std::error::Error;

/// Type alias for convenience.
type BoxError = Box<dyn Error + Send>;

error_set! {
    /// Errors that may occur when a connection to an external resource is opened or used.
    ConnectionError := {
        /// An HTTP error, with the status code attached.
        Http(BoxError) { status: StatusCode },
        /// A catch-all for miscellaneous errors.
        Other(BoxError),
    }

    /// Errors that may occur when fetching entries. Created by methods of [`Source`].
    FetchError := {
        /// An error while decoding.
        Decode(BoxError),
        /// The query was not valid or did not match the requested operation.
        InvalidQuery(BoxError),
    } || ConnectionError

    /// Errors that may occur when fetching a single entry. Created by [`Source::fetch_one`].
    FetchOneError := {
        /// There was no entry matching the query.
        NoSuchEntry
    } || FetchError

    /// Errors that may occur when sending entries. Created by methods of [`Sink`].
    SendError := {
        /// An error while encoding.
        Encode(BoxError),
        /// The sink did not accept the entry/entries.
        Rejected,
    } || ConnectionError
}
