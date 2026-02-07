use crate::{
    encode::Codec,
    rest::{ReadOnly, ReadWrite, WriteOnly},
};
use reqwest::{Client, IntoUrl, Method, Url};
use std::marker::PhantomData;
use thiserror::Error;

/// A builder used to construct a [`ReadOnly`], [`WriteOnly`] or [`ReadWrite`] REST connector.
///
/// The type produced and methods available depends on which fields are set:
/// - A [`ReadOnly`] requires a [source URL](Self::source_url) and a [decoder](Self::decoder), and
///   optionally allows setting a [source method](Self::source_method) and [client](Self::client).
/// - A [`WriteOnly`] requires a [sink URL](Self::sink_url) and an [encoder](Self::encoder), and
///   optionally allows setting a [sink method](Self::sink_method) and [client](Self::client).
/// - A [`ReadWrite`] requires a [source URL](Self::source_url), a [sink URL](Self::sink_url) and
///   either both an [encoder](Self::encoder) and a [decoder](Self::decoder), or a single
///   [codec](Self::codec). It also optionally allows setting a [client](Self::client).
///
/// If none of these cases match, there is no output type and no `build` method exists.
///
/// The builder uses the typestate pattern to accomplish this. The downside is that the method
/// documentations can be quite messy with the type signatures. It is advised to consult the guide
/// above instead.
// TODO: Add support for more fields of `reqwest::RequestBuilder`, e.g. HTTP headers.
// TODO: This feels like a realization of a more general pattern that would be useful to create a
// macro to generate.
// TODO: Can be optimized by replacing usage of `Option` with `MaybeUninit` since constant
// parameters already tell which fields are "initialized".
#[derive(Clone, Debug)]
pub struct Builder<
    T,
    Q,
    E = !,
    D = !,
    C = !,
    const SOURCE_URL: bool = false,
    const SOURCE_METHOD: bool = false,
    const SINK_URL: bool = false,
    const SINK_METHOD: bool = false,
    const CLIENT: bool = false,
    const ENCODER: bool = false,
    const DECODER: bool = false,
    const COMBINED: bool = false,
> {
    /// The [URL](IntoUrl) to use when fetching data.
    // INVARIANT: `source_url.is_some() == SOURCE_URL`.
    // INVARIANT: `source_url.map_or(true, |url| url.into_url().is_ok())`. This is validated during
    // construction.
    source_url: Option<Url>,
    /// The [HTTP method](Method) to use when fetching data. Defaults to [`GET`](Method::GET).
    // INVARIANT: `source_method.is_some() == SOURCE_METHOD`.
    source_method: Option<Method>,
    /// The [URL](IntoUrl) to use when sending data.
    // INVARIANT: `sink_url.is_some() == SINK_URL`.
    // INVARIANT: `sink_url.map_or(true, |url| url.into_url().is_ok())`. This is validated during
    // construction.
    sink_url: Option<Url>,
    /// The [HTTP method](Method) to use when sending data. Defaults to [`PUT`](Method::PUT).
    // INVARIANT: `sink_method.is_some() == SINK_METHOD`.
    sink_method: Option<Method>,
    /// The [`Client`] to use when making requests.
    // INVARIANT: `client.is_some() == CLIENT`.
    client: Option<Client>,
    /// The [encoder](crate::Encode) to use when sending data.
    // INVARIANT: `encoder.is_some() == ENCODER`.
    encoder: Option<E>,
    /// The [decoder](crate::Decode) to use when fetching data.
    // INVARIANT: `decoder.is_some() == DECODER`.
    decoder: Option<D>,
    /// The combined [encoder](crate::Encode) and [decoder](crate::Decode) to use when sending and
    /// fetching data respestively.
    // INVARIANT: `combined.is_some() == COMBINED`.
    // INVARIANT: `!(combined.is_some() && encoder.is_some())`.
    // INVARIANT: `!(combined.is_some() && decoder.is_some())`.
    combined: Option<C>,
    /// Satisfies missing fields using `T` and `Q`.
    // TODO: This may be overly restrictive when considering variance. Improve using unstable
    // `phantom_variance_markers` (#135806)?
    _phantom: PhantomData<(T, Q)>,
}

impl<T, Q> Builder<T, Q> {
    /// Construct a [`Builder`] with no fields set.
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self {
            source_url: None,
            source_method: None,
            sink_url: None,
            sink_method: None,
            client: None,
            encoder: None,
            decoder: None,
            combined: None,
            _phantom: PhantomData,
        }
    }
}

impl<T, Q> Default for Builder<T, Q> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Error that is raised when a URL fails to be processed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Error)]
#[error("The URL was invalid or not a HTTP URI.")]
pub struct InvalidUrl;

impl<
    T,
    Q,
    E,
    D,
    C,
    const SOURCE_METHOD: bool,
    const SINK_URL: bool,
    const SINK_METHOD: bool,
    const CLIENT: bool,
    const ENCODER: bool,
    const DECODER: bool,
    const COMBINED: bool,
>
    Builder<
        T,
        Q,
        E,
        D,
        C,
        false, // SOURCE_URL
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        ENCODER,
        DECODER,
        COMBINED,
    >
{
    /// Adds a URL to use when fetching data. Required to construct a [`ReadOnly`] and a
    /// [`ReadWrite`].
    ///
    /// # Errors
    ///
    /// This method fails with [`InvalidUrl`] if the URL fails to parse.
    #[expect(
        clippy::map_err_ignore,
        reason = "`reqwest::Error` exposes no useful information about the error."
    )]
    #[inline]
    pub fn source_url<U: IntoUrl>(
        self,
        url: U,
    ) -> Result<
        Builder<
            T,
            Q,
            E,
            D,
            C,
            true, // SOURCE_URL
            SOURCE_METHOD,
            SINK_URL,
            SINK_METHOD,
            CLIENT,
            ENCODER,
            DECODER,
            COMBINED,
        >,
        InvalidUrl,
    > {
        Ok(Builder {
            source_url: Some(url.into_url().map_err(|_| InvalidUrl)?),
            ..self
        })
    }
}

impl<
    T,
    Q,
    E,
    D,
    C,
    const SOURCE_URL: bool,
    const SINK_URL: bool,
    const SINK_METHOD: bool,
    const CLIENT: bool,
    const ENCODER: bool,
    const DECODER: bool,
    const COMBINED: bool,
>
    Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        false, // SOURCE_METHOD
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        ENCODER,
        DECODER,
        COMBINED,
    >
{
    /// Specifies the HTTP method to use when fetching data. Defaults to [`GET`](Method::GET).
    #[inline]
    pub fn source_method(
        self,
        method: Method,
    ) -> Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        true, // SOURCE_METHOD
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        ENCODER,
        DECODER,
        COMBINED,
    > {
        Builder {
            source_method: Some(method),
            ..self
        }
    }
}

impl<
    T,
    Q,
    E,
    D,
    C,
    const SOURCE_URL: bool,
    const SOURCE_METHOD: bool,
    const SINK_METHOD: bool,
    const CLIENT: bool,
    const ENCODER: bool,
    const DECODER: bool,
    const COMBINED: bool,
>
    Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        false, // SINK_URL
        SINK_METHOD,
        CLIENT,
        ENCODER,
        DECODER,
        COMBINED,
    >
{
    /// Adds a URL to use when sending data. Required to construct a [`WriteOnly`] and a
    /// [`ReadWrite`].
    ///
    /// # Errors
    ///
    /// This method fails with [`InvalidUrl`] if the URL fails to parse.
    #[expect(
        clippy::map_err_ignore,
        reason = "`reqwest::Error` exposes no useful information about the error."
    )]
    #[inline]
    pub fn sink_url<U: IntoUrl>(
        self,
        url: U,
    ) -> Result<
        Builder<
            T,
            Q,
            E,
            D,
            C,
            SOURCE_URL,
            SOURCE_METHOD,
            true, // SINK_URL
            SINK_METHOD,
            CLIENT,
            ENCODER,
            DECODER,
            COMBINED,
        >,
        InvalidUrl,
    > {
        Ok(Builder {
            source_url: Some(url.into_url().map_err(|_| InvalidUrl)?),
            ..self
        })
    }
}

impl<
    T,
    Q,
    E,
    D,
    C,
    const SOURCE_URL: bool,
    const SOURCE_METHOD: bool,
    const SINK_URL: bool,
    const CLIENT: bool,
    const ENCODER: bool,
    const DECODER: bool,
    const COMBINED: bool,
>
    Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        false, // SINK_METHOD
        CLIENT,
        ENCODER,
        DECODER,
        COMBINED,
    >
{
    /// Specifies the HTTP method to use when sending data. Defaults to [`PUT`](Method::PUT).
    #[inline]
    pub fn sink_method(
        self,
        method: Method,
    ) -> Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        true, // SINK_METHOD
        CLIENT,
        ENCODER,
        DECODER,
        COMBINED,
    > {
        Builder {
            sink_method: Some(method),
            ..self
        }
    }
}

impl<
    T,
    Q,
    E,
    D,
    C,
    const SOURCE_URL: bool,
    const SOURCE_METHOD: bool,
    const SINK_URL: bool,
    const SINK_METHOD: bool,
    const ENCODER: bool,
    const DECODER: bool,
    const COMBINED: bool,
>
    Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        false, // CLIENT
        ENCODER,
        DECODER,
        COMBINED,
    >
{
    /// Add a [`Client`] to the connector. If none is specified, a default is used.
    #[inline]
    pub fn client(
        self,
        client: Client,
    ) -> Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        true, // CLIENT
        ENCODER,
        DECODER,
        COMBINED,
    > {
        Builder {
            client: Some(client),
            ..self
        }
    }
}

impl<
    T,
    Q,
    D,
    C,
    const SOURCE_URL: bool,
    const SOURCE_METHOD: bool,
    const SINK_URL: bool,
    const SINK_METHOD: bool,
    const CLIENT: bool,
    const DECODER: bool,
>
    Builder<
        T,
        Q,
        !,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        false, // ENCODER
        DECODER,
        false, // COMBINED
    >
{
    /// Add an [encoder](crate::encode::Encode) to the connector. One is needed to construct a
    /// [`ReadOnly`], and one alternative needed to construct a [`ReadWrite`].
    #[expect(
        clippy::missing_panics_doc,
        reason = "Assertions will not fail if invariants are upheld."
    )]
    #[inline]
    pub fn encoder<E>(
        self,
        encoder: E,
    ) -> Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        true, // ENCODER
        DECODER,
        false, // COMBINED
    > {
        assert!(self.combined.is_none());
        Builder {
            encoder: Some(encoder),
            ..self
        }
    }
}

impl<
    T,
    Q,
    E,
    C,
    const SOURCE_URL: bool,
    const SOURCE_METHOD: bool,
    const SINK_URL: bool,
    const SINK_METHOD: bool,
    const CLIENT: bool,
    const ENCODER: bool,
>
    Builder<
        T,
        Q,
        E,
        !,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        ENCODER,
        false, // DECODER
        false, // COMBINED
    >
{
    /// Add an [decoder](crate::encode::Decode) to the connector. One is needed to construct a
    /// [`WriteOnly`], and one alternative needed to construct a [`ReadWrite`].
    #[expect(
        clippy::missing_panics_doc,
        reason = "Assertions will not fail if invariants are upheld."
    )]
    #[inline]
    pub fn decoder<D>(
        self,
        decoder: D,
    ) -> Builder<
        T,
        Q,
        E,
        D,
        C,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        ENCODER,
        true,  // DECODER
        false, // COMBINED
    > {
        assert!(self.combined.is_none());
        Builder {
            decoder: Some(decoder),
            ..self
        }
    }
}

impl<
    T,
    Q,
    const SOURCE_URL: bool,
    const SOURCE_METHOD: bool,
    const SINK_URL: bool,
    const SINK_METHOD: bool,
    const CLIENT: bool,
>
    Builder<
        T,
        Q,
        !,
        !,
        !,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        false, // ENCODER
        false, // DECODER
        false, // COMBINED
    >
{
    /// Add a [`Codec`] to the connector. One is needed to construct a [`ReadWrite`], and is
    /// incompatible with any other [encoder](Self::encoder) or [decoder](Self::decoder)
    #[expect(
        clippy::missing_panics_doc,
        reason = "Assertions will not fail if invariants are upheld."
    )]
    #[inline]
    pub fn codec<C>(
        self,
        codec: C,
    ) -> Builder<
        T,
        Q,
        !,
        !,
        Codec<T, !, !, C>,
        SOURCE_URL,
        SOURCE_METHOD,
        SINK_URL,
        SINK_METHOD,
        CLIENT,
        false, // ENCODER
        false, // DECODER
        true,  // COMBINED
    > {
        assert!(self.encoder.is_none());
        assert!(self.decoder.is_none());
        Builder {
            combined: Some(Codec::combined(codec)),
            ..self
        }
    }
}

/// A trait indicating that a builder is ready to be built into its output type.
///
/// Depending on the builder, this trait may only be available under certain conditions. That is,
/// it might only be implemented on builders fulfilling various type constraints.
pub trait Build {
    /// The output produced when building.
    type Output;

    /// Consume the builder, returning its output.
    fn build(self) -> Self::Output;
}

// `source_url` and `decoder` set: `ReadOnly`.
impl<T, Q, D, const SOURCE_METHOD: bool, const CLIENT: bool> Build
    for Builder<T, Q, !, D, !, true, SOURCE_METHOD, false, false, CLIENT, false, true, false>
{
    type Output = ReadOnly<T, Q, D>;

    #[expect(clippy::unreachable, reason = "Invariants guarantee correctness.")]
    #[inline]
    fn build(self) -> Self::Output {
        let Self {
            source_url: Some(url),
            source_method,
            client,
            decoder: Some(decoder),
            ..
        } = self
        else {
            unreachable!()
        };

        Self::Output {
            url,
            method: source_method.unwrap_or(Method::GET),
            client: client.unwrap_or_default(),
            decoder,
            _phantom: PhantomData,
        }
    }
}

// `sink_url` and `encoder` set: `WriteOnly`.
impl<T, Q, E, const SINK_METHOD: bool, const CLIENT: bool> Build
    for Builder<T, Q, E, !, !, false, false, true, SINK_METHOD, CLIENT, true, false, false>
{
    type Output = WriteOnly<T, E>;

    #[expect(clippy::unreachable, reason = "Invariants guarantee correctness.")]
    #[inline]
    fn build(self) -> Self::Output {
        let Self {
            sink_url: Some(url),
            sink_method,
            client,
            encoder: Some(encoder),
            ..
        } = self
        else {
            unreachable!()
        };

        Self::Output {
            url,
            method: sink_method.unwrap_or(Method::GET),
            client: client.unwrap_or_default(),
            encoder,
            _phantom: PhantomData,
        }
    }
}

// `source_url`, `sink_url`, `encoder` and `decoder` set: `ReadWrite`.
impl<T, Q, E, D, const SOURCE_METHOD: bool, const SINK_METHOD: bool, const CLIENT: bool> Build
    for Builder<T, Q, E, D, !, true, SOURCE_METHOD, true, SINK_METHOD, CLIENT, true, true, false>
{
    type Output = ReadWrite<T, Q, E, D, !>;

    #[expect(clippy::unreachable, reason = "Invariants guarantee correctness.")]
    #[inline]
    fn build(self) -> Self::Output {
        let Self {
            source_url: Some(source_url),
            source_method,
            sink_url: Some(sink_url),
            sink_method,
            client,
            encoder: Some(encoder),
            decoder: Some(decoder),
            combined: None,
            ..
        } = self
        else {
            unreachable!()
        };

        Self::Output {
            source_url,
            source_method: source_method.unwrap_or(Method::GET),
            sink_url,
            sink_method: sink_method.unwrap_or(Method::PUT),
            client: client.unwrap_or_default(),
            codec: Codec::separate(encoder, decoder),
            _phantom: PhantomData,
        }
    }
}

// `source_url`, `sink_url`, and `combined` set: `ReadWrite`.
impl<T, Q, C, const SOURCE_METHOD: bool, const SINK_METHOD: bool, const CLIENT: bool> Build
    for Builder<T, Q, !, !, C, true, SOURCE_METHOD, true, SINK_METHOD, CLIENT, false, false, true>
{
    type Output = ReadWrite<T, Q, !, !, C>;

    #[expect(clippy::unreachable, reason = "Invariants guarantee correctness.")]
    #[inline]
    fn build(self) -> Self::Output {
        let Self {
            source_url: Some(source_url),
            source_method,
            sink_url: Some(sink_url),
            sink_method,
            client,
            encoder: None,
            decoder: None,
            combined: Some(combined),
            ..
        } = self
        else {
            unreachable!()
        };

        Self::Output {
            source_url,
            source_method: source_method.unwrap_or(Method::GET),
            sink_url,
            sink_method: sink_method.unwrap_or(Method::PUT),
            client: client.unwrap_or_default(),
            codec: Codec::combined(combined),
            _phantom: PhantomData,
        }
    }
}
