//! The `Encode` and `Decode` traits.

use crate::errors::{ConnectionError, DecodeError, DecodeOneError, DecodeStreamError, EncodeError};
use bytes::{Bytes, BytesMut};
use futures::{
    Stream, TryFutureExt as _, TryStreamExt as _,
    future::{Either, ready},
    stream::iter as from_iter,
};
use std::marker::PhantomData;

pub mod json;

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
// TODO: Add support for formatters, allowing things like "pretty printing".
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
    /// formats require not only "header data" but also "footer data" (such as closing a list in
    /// JSON). Returning header data and waiting for a footer leaves any intermediate state as
    /// invalid encoding, and omitting header data means even the final result is invalid.
    ///
    /// The main advantage of this method, then, is if the data can be generated lazily: it may be
    /// impractical or infeasible to pull all data into memory at once even though the resulting
    /// encoded data is considerably smaller, or the encoding might be finished before having
    /// processed all data due to properties of the format.
    fn encode<'a, I>(&self, entries: I) -> Result<Box<[u8]>, EncodeError>
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>;

    /// Encode data from a slice.
    ///
    /// Depending on the format, this may or may not be equivalent to calling
    /// [`encode_one`](Self::encode_one) on each entry and concatenating the results.
    fn encode_all(&self, entries: &[T]) -> Result<Box<[u8]>, EncodeError> {
        self.encode(entries)
    }

    /// Encode a single entry.
    ///
    /// Depending on the format, calling this several times and concatenating the results may or
    /// may not be equivalent to calling `encode_all`.
    fn encode_one(&self, entry: &T) -> Result<Box<[u8]>, EncodeError>;
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
    // TODO: Can this be simplified using `tokio::io::AsyncRead`?
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
                    .map_err(DecodeStreamError::Decode);
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
    fn decode_all(&self, bytes: &[u8]) -> Result<Vec<T>, DecodeError>;

    /// Decode a single entry from a slice. If the slice is empty or represents an empty
    /// collection, <code>[Err]\([`Empty`](DecodeOneError::Empty))</code> is returned.
    ///
    /// One entry is assumed to be fairly small such that collection all bytes into a slice is
    /// acceptable, and as such no stream variant of this method exists.
    fn decode_one(&self, bytes: &[u8]) -> Result<T, DecodeOneError> {
        self.decode_optional(bytes)
            .map_err(DecodeOneError::Decode)?
            .ok_or(DecodeOneError::Empty)
    }

    /// Decode a single entry from a slice, if one exists.
    ///
    /// This method poses no restriction on *which* entry should be returned. The format may
    /// however define an ordering.
    ///
    /// One entry is assumed to be fairly small such that collection all bytes into a slice is
    /// acceptable, and as such no stream variant of this method exists.
    fn decode_optional(&self, bytes: &[u8]) -> Result<Option<T>, DecodeError>;
}

/// A type used for encoding and decoding, with the option to use the same value for both.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Codec<T, E = (), D = (), C = ()>(CodecImpl<T, E, D, C>);

/// Implementation of [`Codec`].
// TODO: `PhantomData` usage here may be overly restrictive when considering variance. Improve
// using unstable `phantom_variance_markers` (#135806)?
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum CodecImpl<T, E, D, C> {
    /// Separate encoder and decoder.
    Separate(E, D, PhantomData<T>),
    /// One value serving as both encoder and decoder.
    Combined(C, PhantomData<T>),
}

impl<T, E, D, C> Codec<T, E, D, C> {
    /// Construct a [`Codec`] using a separate encoder and decoder.
    pub const fn separate(encoder: E, decoder: D) -> Self {
        Self(CodecImpl::Separate(encoder, decoder, PhantomData))
    }

    /// Construct a [`Codec`] using one value as both encoder and decoder.
    pub const fn combined(combined: C) -> Self {
        Self(CodecImpl::Combined(combined, PhantomData))
    }
}

impl<T, E, D, C> From<C> for Codec<T, E, D, C>
where
    C: Encode<T> + Decode<T>,
{
    fn from(value: C) -> Self {
        Self::combined(value)
    }
}

impl<T, E, D, C> Encode<T> for Codec<T, E, D, C>
where
    E: Encode<T>,
    C: Encode<T>,
{
    fn encode<'a, I>(&self, entries: I) -> Result<Box<[u8]>, EncodeError>
    where
        T: 'a,
        I: IntoIterator<Item = &'a T>,
    {
        match &self.0 {
            CodecImpl::Separate(encoder, ..) => encoder.encode(entries),
            CodecImpl::Combined(combined, ..) => combined.encode(entries),
        }
    }

    fn encode_all(&self, entries: &[T]) -> Result<Box<[u8]>, EncodeError> {
        match &self.0 {
            CodecImpl::Separate(encoder, ..) => encoder.encode_all(entries),
            CodecImpl::Combined(combined, ..) => combined.encode_all(entries),
        }
    }

    fn encode_one(&self, entry: &T) -> Result<Box<[u8]>, EncodeError> {
        match &self.0 {
            CodecImpl::Separate(encoder, ..) => encoder.encode_one(entry),
            CodecImpl::Combined(combined, ..) => combined.encode_one(entry),
        }
    }
}

impl<T, E, D, C> Decode<T> for Codec<T, E, D, C>
where
    D: Decode<T> + Sync,
    C: Decode<T> + Sync,
{
    async fn decode<S>(
        &self,
        bytes: S,
    ) -> Result<impl Stream<Item = Result<T, DecodeStreamError>> + Send + Unpin, DecodeStreamError>
    where
        Self: Sync,
        T: Send,
        S: Stream<Item = Result<Bytes, ConnectionError>> + Send,
    {
        match &self.0 {
            CodecImpl::Separate(_, decoder, ..) => decoder.decode(bytes).await.map(Either::Left),
            CodecImpl::Combined(combined, ..) => combined.decode(bytes).await.map(Either::Right),
        }
    }

    fn decode_all(&self, bytes: &[u8]) -> Result<Vec<T>, DecodeError> {
        match &self.0 {
            CodecImpl::Separate(_, decoder, ..) => decoder.decode_all(bytes),
            CodecImpl::Combined(combined, ..) => combined.decode_all(bytes),
        }
    }

    fn decode_one(&self, bytes: &[u8]) -> Result<T, DecodeOneError> {
        match &self.0 {
            CodecImpl::Separate(_, decoder, ..) => decoder.decode_one(bytes),
            CodecImpl::Combined(combined, ..) => combined.decode_one(bytes),
        }
    }

    fn decode_optional(&self, bytes: &[u8]) -> Result<Option<T>, DecodeError> {
        match &self.0 {
            CodecImpl::Separate(_, decoder, ..) => decoder.decode_optional(bytes),
            CodecImpl::Combined(combined, ..) => combined.decode_optional(bytes),
        }
    }
}
