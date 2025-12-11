//! Error types used by connectors.

// TODO: Consider reworking error handling entirely using `pattern_types` to reduce nesting and
// repetition in all of these `enum`s.

use reqwest::Error as ReqwestError;
use std::{error::Error, io::Error as IoError};
use thiserror::Error;
use transitive::Transitive;

/// Convenience alias.
type BoxError = Box<dyn Error + Send>;

/// Errors that may occur when connecting or communicating with external resources.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConnectionError {
    /// An HTTP error with status code attached.
    #[error("HTTP {code}: {source}")]
    Http {
        /// The HTTP status code. It is up to the creator to ensure that this is actually an error,
        /// i.e. not a 2XX code, and the server that it does not return such a code on failure.
        code: u16,
        /// The source error.
        #[source]
        source: BoxError,
    },
    /// An IO error.
    #[error(transparent)]
    Io(#[from] IoError),
    /// Connection timed out.
    #[error("Connection timed out.")]
    TimedOut,
    /// Redirect limit was reached or cyclic redirect detected.
    #[error("Redirect limit was reached or cyclic redirect detected.")]
    Redirect,
    /// A message was received that could not be processed. It might be an issue with encoding,
    /// charset used, or that an external resource communicated using an unknown or unexpected
    /// format.
    #[error(transparent)]
    Process(BoxError),
}

impl From<ReqwestError> for ConnectionError {
    /// Classify a [`reqwest::Error`].
    ///
    /// `reqwest` provides a single error type which is used by the entire crate. It exposes few
    /// methods and all of its fields are private. This means that it is generally difficult to
    /// properly classify it and impossible to extract useful debug information other than possibly
    /// in its error message. That being said, this function handles some cases and maps them to
    /// known [`ConnectionError`]s.
    ///
    /// # Panics
    ///
    /// Panics if for the given error, [`reqwest::Error::is_builder`] is true as those should be
    /// handled in advance, when returned from the corresponding `build` method. It also panics if
    /// [`reqwest::Error::is_status`] is true as `reqwest::Response::error_for_status` (and the
    /// `_ref`) variant should be avoided in favor of passing the error directly to this function
    /// which identifies the HTTP error and maps it to [`ConnectionError::Http`].
    #[inline]
    fn from(value: ReqwestError) -> Self {
        // Builder errors should be handled separately.
        assert!(!value.is_builder());
        // `Response::error_for_status` (`_ref`) shouldn't be used as HTTP errors are included as
        // their own variant.
        assert!(!value.is_status());

        // No guarantee that several of these cases don't match; order by decreasing specificity.
        if let Some(status) = value.status() {
            Self::Http {
                code: status.into(),
                source: Box::new(value),
            }
        } else if value.is_redirect() {
            Self::Redirect
        } else if value.is_timeout() {
            Self::TimedOut
        } else if value.is_body() || value.is_decode() {
            Self::Process(Box::new(value))
        } else {
            // Reqwest allows errors to have none of these properties.
            // This case also covers `ReqwestError::is_connect` and `ReqwestError::is_request`.
            Self::Io(IoError::other(value))
        }
    }
}

/// An error that occured during decoding. The inner error is with many implementations likely to
/// implement [`serde::de::Error`], e.g. [`serde::de::value::Error`], but that trait is not `dyn`
/// compatible.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct DecodeError(#[from] pub BoxError);

/// Errors that may occur when fetching entries. Created by methods of
/// [`Source`](crate::connector::Source).
#[derive(Debug, Error, Transitive)]
#[transitive(from(ReqwestError, ConnectionError))]
pub enum FetchError {
    /// Error occured during decoding.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// Error occurred when processing query. The query was not valid or did not match the
    /// requested operation.
    // TODO: More information should be added here.
    #[error("The query was not valid or did not match the requested operation.")]
    InvalidQuery(#[source] BoxError),
    /// Error occured during connection or communication.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}

/// Errors that may occur when fetching a single entry. Created by
/// [`Source::fetch_one`](crate::connector::Source::fetch_one).
#[derive(Debug, Error, Transitive)]
#[transitive(from(ReqwestError, FetchError))]
pub enum FetchOneError {
    /// Error occured during fetching.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// There was no entry matching the query.
    #[error("There was no entry matching the query.")]
    NoSuchEntry,
}

/// An error that occured during encoding. The inner error is with many implementations likely to
/// implement [`serde::ser::Error`], but that trait is not `dyn` compatible.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct EncodeError(#[from] pub BoxError);

/// Errors that may occur when sending entries. Created by methods of
/// [`Sink`](crate::connector::Sink).
#[derive(Debug, Error)]
pub enum SendError {
    /// Error occured during encoding. This error is likely to implement [`serde::ser::Error`], but
    /// that trait is not `dyn` compatible.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// The entry/entries were rejected.
    #[error("The entry/entries were rejected.")]
    Rejected,
    /// Error occured during connection or communication.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}

/// Errors that may occur when decoding data from a stream. Created by
/// [`decode`](crate::encode::Decode::decode).
#[derive(Debug, Error)]
pub enum DecodeStreamError {
    /// Error occured during decoding.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// A connection was established, but an error occurred before all data was sent.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}

impl From<DecodeStreamError> for FetchError {
    #[inline]
    fn from(value: DecodeStreamError) -> Self {
        match value {
            DecodeStreamError::Decode(err) => Self::Decode(err),
            DecodeStreamError::Connection(err) => Self::Connection(err),
        }
    }
}

/// Errors that may occur when decoding a single entry. Created by
/// [`Decode::decode_one`](crate::encode::Decode::decode_one).
#[derive(Debug, Error)]
pub enum DecodeOneError {
    /// Error occured during decoding. This error is likely to implement [`serde::de::Error`],
    /// e.g. [`serde::de::value::Error`], but that trait is not `dyn` compatible.
    // TODO: More information should be added here.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// No bytes were returned or they represent an empty collection.
    #[error("No bytes were returned or they represent an empty collection.")]
    Empty,
}

impl From<DecodeOneError> for FetchOneError {
    #[inline]
    fn from(value: DecodeOneError) -> Self {
        match value {
            DecodeOneError::Decode(err) => Self::Fetch(FetchError::Decode(err)),
            DecodeOneError::Empty => Self::NoSuchEntry,
        }
    }
}
