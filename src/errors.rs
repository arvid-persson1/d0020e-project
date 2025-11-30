//! Error types used by connectors.

use reqwest::Error as ReqwestError;
use serde::de::value::Error as DeserializeError;
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
}

impl From<ReqwestError> for ConnectionError {
    #[expect(clippy::dbg_macro, reason = "Remove after improving error handling.")]
    fn from(value: ReqwestError) -> Self {
        // TODO: Improve error classification. Can anything useful be extracted from debug
        // information?
        dbg!(&value);
        IoError::other(value).into()
    }
}

/// Errors that may occur when fetching entries. Created by methods of
/// [`Source`](crate::source_sink::Source).
#[derive(Debug, Error, Transitive)]
#[transitive(from(ReqwestError, ConnectionError))]
pub enum FetchError {
    /// Error occured during decoding.
    #[error(transparent)]
    Decode(#[from] DeserializeError),
    /// Error occurred when processing query. The query was not valid or did not match the
    /// requested operation.
    // TODO: Could this be given more fields or variants?
    #[error("The query was not valid or did not match the requested operation.")]
    InvalidQuery(#[source] BoxError),
    /// Error occured during connection or communication.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}

/// Errors that may occur when fetching a single entry. Created by
/// [`Source::fetch_one`](crate::source_sink::Source::fetch_one).
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

/// Errors that may occur when sending entries. Created by methods of
/// [`Sink`](crate::source_sink::Sink).
#[derive(Debug, Error)]
pub enum SendError {
    /// Error occured during encoding. This error is likely to implement [`serde::ser::Error`],
    /// e.g. [`std::fmt::Error`], but that trait is not `dyn` compatible.
    #[error("{0}")]
    Encode(#[source] BoxError),
    /// The entry/entries were rejected.
    #[error("The entry/entries were rejected.")]
    Rejected,
    /// Error occured during connection or communication.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}

/// Errors that may occur when decoding data from a stream. Created by
/// [`decode`](crate::rest::Decode::decode).
#[derive(Debug, Error)]
pub enum DecodeStreamError {
    /// Error occured during decoding.
    #[error(transparent)]
    Decode(#[from] DeserializeError),
    /// A connection was established, but an error occurred before all data was sent.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}

impl From<DecodeStreamError> for FetchError {
    fn from(value: DecodeStreamError) -> Self {
        match value {
            DecodeStreamError::Decode(err) => Self::Decode(err),
            DecodeStreamError::Connection(err) => Self::Connection(err),
        }
    }
}

/// Errors that may occur when decoding a single entry. Created by
/// [`Decode::decode_one`](crate::rest::Decode::decode_one).
#[derive(Debug, Error)]
pub enum DecodeOneError {
    /// Error occurred during decoding.
    #[error(transparent)]
    Decode(#[from] DeserializeError),
    /// No bytes were returned or they represent an empty collection.
    #[error("No bytes were returned or they represent an empty collection.")]
    Empty,
}

impl From<DecodeOneError> for FetchOneError {
    fn from(value: DecodeOneError) -> Self {
        match value {
            DecodeOneError::Decode(err) => Self::Fetch(err.into()),
            DecodeOneError::Empty => Self::NoSuchEntry,
        }
    }
}
