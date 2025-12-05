//! The `Encode` and `Decode` traits.

use crate::errors::{ConnectionError, DecodeOneError, DecodeStreamError};
use bytes::{Bytes, BytesMut};
use futures::{
    Stream, TryFutureExt as _, TryStreamExt as _, future::ready, stream::iter as from_iter,
};
use serde::de::value::Error as DeserializeError;
use std::{borrow::Borrow, fmt::Error as FmtError};

/// A type that can encode data as bytes.
///
/// This trait provides several ways of encoding data regarding how many entries are processes at a
/// time. It is also conservative and tries to make no assumptions about the data format: for
/// example, it may be the case that several elements encoded and then concatenated are not
/// equivalent to those same elements concatenated then encoded (e.g. JSON, where objects "in
/// sequence" are not equivalent to a list of objects).
///
/// An encoder is generally stateless, e.g. one for JSON, meaning they will practically often be
/// ZSTs.
///
/// Some of this trait's methods all have intraconnected default implementations, which means that
/// although no methods are marked as "required", **an implementation must override at minimum
/// either [`encode`](Self::encode) or [`encode_all`](Self::encode_all)**, otherwise calls will
/// always fail. That being said, often more efficient implementations of the other methods are
/// possible. Check the method documentations for more information.
#[expect(
    clippy::missing_errors_doc,
    reason = "Default implementations only delegate errors and do not raise their own."
)]
pub trait Encode<T> {
    /// Encode data from an iterator.
    ///
    /// Depending on the format, this may or may not be equivalent to calling `encode_one` on
    /// each entry and concatenating the results.
    ///
    /// This method intentionally returns all bytes at once, rather than an iterator, as many
    /// formats (e.g. JSON) require not only headers but also footers (such as closing a list in
    /// JSON). Returning header data and waiting for a footer leaves any intermediate state as
    /// invalid encoding, and omitting header data means even the final result is invalid. The main
    /// advantage of this method, then, is if the data can be generated lazily.
    ///
    /// The default implementation collects the entries and calls [`encode_all`](Self::encode_all).
    fn encode<'a, I>(&self, entries: I) -> Result<Box<[u8]>, FmtError>
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
    {
        self.encode_all(&entries.into_iter().collect::<Box<_>>())
    }

    /// Encode data from a slice.
    ///
    /// Depending on the format, this may or may not be equivalent to calling
    /// [`encode_one`](Self::encode_one) on each entry and concatenating the results.
    ///
    /// The default implementation creates an iterator over the entries and calls
    /// [`encode`](Self::encode).
    fn encode_all<B>(&self, entries: &[B]) -> Result<Box<[u8]>, FmtError>
    where
        B: Borrow<T>,
    {
        self.encode(entries.iter().map(Borrow::borrow))
    }

    /// Encode a single entry.
    ///
    /// Depending on the format, calling this several times and concatenating the results may or
    /// may not be equivalent to calling `encode_all`.
    fn encode_one(&self, entry: &T) -> Result<Box<[u8]>, FmtError>;
}

/// A type that can decode data from bytes.
///
/// This trait provides several ways of decoding data regarding how many bytes are returned at a
/// time. The [`decode`](Self::decode) method is `async` to work with data that is produced in
/// real time while the others expect all bytes in advance. It it also conservative and tries to
/// make no assumptions about the data format: for example, it may be the case that several
/// elements encoded and then concatenated are not equivalent to those same elements concatenated
/// then encoded (e.g. JSON, where objects "in sequence" are not equivalent to a list of objects).
///
/// A decoder is generally stateless, e.g. one for JSON, meaning they will practically often be
/// ZSTs.
#[expect(
    clippy::missing_errors_doc,
    reason = "Default implementations only delegate errors and do not raise their own."
)]
pub trait Decode<T> {
    /// Decode data from a stream of bytes.
    ///
    /// An error may be raised for each item produced, or before any are. Reasonably,
    /// [`DecodeStreamError::Connection`] should only passed on from input; decoding should not
    /// produce new connection errors, though this is not enforced or validated.
    ///
    /// The default implementation collects the bytes and calls [`decode_all`](Self::decode_all).
    fn decode<S>(
        &self,
        bytes: S,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = Result<T, DecodeStreamError>> + Send + Unpin,
            DecodeStreamError,
        >,
    > + Send
    where
        Self: Sync,
        T: Send,
        S: Stream<Item = Result<Bytes, ConnectionError>> + Send,
    {
        bytes
            .try_collect::<BytesMut>()
            .map_err(Into::into)
            .and_then(|buf| {
                let res = self
                    .decode_all(&buf)
                    .map(|vec| from_iter(vec.into_iter().map(Ok)))
                    .map_err(Into::into);
                ready(res)
            })
    }

    /// Decode data from a slice.
    ///
    /// This method intentionally does not have a default implementation based on
    /// [`decode`](Self::decode). This is partly because that method is `async` while this one
    /// isn't, meaning it would have to make explicit blocking calls, unexpectedly leading to poor
    /// concurrency and possibly performance. It would also require enforcing the contract that
    /// that method must not create new connection errors, since this method cannot return one.
    fn decode_all(&self, bytes: &[u8]) -> Result<Vec<T>, DeserializeError>;

    /// Decode a single entry from a slice. If the slice is empty or represents an empty
    /// collection, <code>[Err]\([`Empty`](DeserializeError::Empty))</code> is returned.
    ///
    /// One entry is assumed to be fairly small such that collection all bytes into a slice is
    /// acceptable, and as such no stream variant of this method exists.
    ///
    /// The default implementation calls [`decode_optional`](Self::decode_optional).
    fn decode_one(&self, bytes: &[u8]) -> Result<T, DecodeOneError> {
        self.decode_optional(bytes)?.ok_or(DecodeOneError::Empty)
    }

    /// Decode a single entry from a slice, if one exists.
    ///
    /// This method poses no restriction on *which* entry should be returned. The format may
    /// however define an ordering.
    ///
    /// One entry is assumed to be fairly small such that collection all bytes into a slice is
    /// acceptable, and as such no stream variant of this method exists.
    fn decode_optional(&self, bytes: &[u8]) -> Result<Option<T>, DeserializeError>;
}
