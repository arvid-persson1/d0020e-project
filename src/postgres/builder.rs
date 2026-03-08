//! Builder for `PostgreSQL` connectors.

use crate::postgres::{ReadOnly, ReadWrite, WriteOnly};
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
/// Builder struct
pub struct Builder<
    T,
    E = (),
    D = (),
    const URL: bool = false,
    const ENCODER: bool = false,
    const DECODER: bool = false,
> {
    /// The url
    url: Option<String>,
    /// Field for encoder
    encoder: Option<E>,
    /// Field for decoder
    decoder: Option<D>,
    /// Phantomdata field
    _phantom: PhantomData<T>,
}

impl<T> Builder<T> {
    /// Constructs a `Builder` with no fields set.
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            url: None,
            encoder: None,
            decoder: None,
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for Builder<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// State transitions (Setting the fields)
impl<T, E, D, const ENCODER: bool, const DECODER: bool> Builder<T, E, D, false, ENCODER, DECODER> {
    /// Adds a connection string to use for the database pool.
    #[inline]
    pub fn url(self, url: impl Into<String>) -> Builder<T, E, D, true, ENCODER, DECODER> {
        Builder {
            url: Some(url.into()),
            encoder: self.encoder,
            decoder: self.decoder,
            _phantom: PhantomData,
        }
    }
}

impl<T, D, const URL: bool, const DECODER: bool> Builder<T, (), D, URL, false, DECODER> {
    /// Add an encoder to the connector.
    #[inline]
    pub fn encoder<E>(self, encoder: E) -> Builder<T, E, D, URL, true, DECODER> {
        Builder {
            url: self.url,
            encoder: Some(encoder),
            decoder: self.decoder,
            _phantom: PhantomData,
        }
    }
}

impl<T, E, const URL: bool, const ENCODER: bool> Builder<T, E, (), URL, ENCODER, false> {
    /// Add a decoder to the connector.
    #[inline]
    pub fn decoder<D>(self, decoder: D) -> Builder<T, E, D, URL, ENCODER, true> {
        Builder {
            url: self.url,
            encoder: self.encoder,
            decoder: Some(decoder),
            _phantom: PhantomData,
        }
    }
}

impl<T, const URL: bool> Builder<T, (), (), URL, false, false> {
    /// Add a combined codec to the connector, serving as both encoder and decoder.
    #[inline]
    pub fn codec<C: Clone>(self, codec: C) -> Builder<T, C, C, URL, true, true> {
        Builder {
            url: self.url,
            encoder: Some(codec.clone()), // Plugs into encoder
            decoder: Some(codec),         // Plugs into decoder
            _phantom: PhantomData,
        }
    }
}

/// Build trait and implementations
pub trait Build {
    /// The output type
    type Output;
    /// Consume the builder, initializing the connection pool and returning the output.
    fn build(self) -> Self::Output;
}

// `ReadOnly` builder
impl<T, D> Build for Builder<T, (), D, true, false, true> {
    type Output = Result<ReadOnly<T, D>, PoolError>;

    #[inline]
    fn build(self) -> Self::Output {
        // expect() satisfies Clippy. This will never fail due to the `true` type state.
        let url = self.url.expect("Type-state guarantees URL is present");
        let manager = ConnectionManager::<PgConnection>::new(url);

        // Use `?` instead of `.expect()`! If the DB is down, it safely returns a PoolError.
        let pool = Pool::builder().build(manager)?;

        Ok(ReadOnly {
            pool,
            decoder: self
                .decoder
                .expect("Type-state guarantees decoder is present"),
            _phantom: PhantomData,
        })
    }
}

// `WriteOnly` builder
impl<T, E> Build for Builder<T, E, (), true, true, false> {
    type Output = Result<WriteOnly<T, E>, PoolError>;

    #[inline]
    fn build(self) -> Self::Output {
        let url = self.url.expect("Type-state guarantees URL is present");
        let manager = ConnectionManager::<PgConnection>::new(url);

        let pool = Pool::builder().build(manager)?; // Safe error propagation

        Ok(WriteOnly {
            pool,
            encoder: self
                .encoder
                .expect("Type-state guarantees encoder is present"),
            _phantom: PhantomData,
        })
    }
}

// `ReadWrite` builder
// Handles both separate encoders/decoders and the `.codec()` method automatically
impl<T, E, D> Build for Builder<T, E, D, true, true, true> {
    type Output = Result<ReadWrite<T, E, D>, PoolError>;

    #[inline]
    fn build(self) -> Self::Output {
        let url = self.url.expect("Type-state guarantees URL is present");
        let manager = ConnectionManager::<PgConnection>::new(url);

        let pool = Pool::builder().build(manager)?; // Safe error propagation

        Ok(ReadWrite {
            pool,
            encoder: self
                .encoder
                .expect("Type-state guarantees encoder is present"),
            decoder: self
                .decoder
                .expect("Type-state guarantees decoder is present"),
            _phantom: PhantomData,
        })
    }
}
