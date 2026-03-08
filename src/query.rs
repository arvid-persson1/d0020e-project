//! Typed query construction API.
//!
//! This module provides the core building blocks for constructing,
//! combining, and translating queries in a type-safe manner.

use nameof::{name_of, name_of_type};
pub use query_macro::Queryable;
use std::fmt;
use std::sync::Arc;

/// Query primitives and combinators.
pub mod combinators;
use combinators::{Eq, Gt, Lt, Ne};

/// A query that can be evaluated to check if some data matches a predicate.
pub trait Query<T> {
    /// Try to match `data` to the predicate specified by this query.
    fn evaluate(&self, data: &T) -> bool;

    /// Translate into a single [`HttpQuery`]. See [`Single`] documentation for caveats, and
    /// primitive- or combinator-specific documentation for details.
    #[cfg(feature = "rest")]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T>;

    /// Translate into multiple [`HttpQuery`] instances. This avoids some of the caveats of
    /// [`Single`], but has the obvious downside of requiring multiple requests to be made.
    /// Although they could be made in parallel for negligible time cost, it could put more load on
    /// network traffic or the server if the number of parts is large. See primitive- or
    /// combinator-specific documentation for details.
    ///
    /// Executing all of these queries should not find any undesired elements. It could however be
    /// the case that one element matches several parts of the query, which would result in
    /// duplication. If duplicates are possible, the results should be collected and deduplicated.
    ///
    /// [`None`] is returned if no translation specific enough to select only desired elements is
    /// possible.
    #[cfg(feature = "rest")]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>>;

    // NOTE: In case I choose to use this function
    // #[cfg(feature = "graphql")]
    // fn to_grahpql_single(&self) -> Single<'_, HttpQuery<'_>, T>;
}
/// A best-effort translation of the input query to a single output query.
///
/// In cases where total translation is not possible, [`query`](Self::query) attempts to fetch a
/// minimal superset of the desired items, which are then intended to be filtered using the
/// provided [`residues`](Self::residue). The downside to this approach, of course, is that the
/// translation does not have perfect information and as such said superset of elements may be very
/// large, possibly many times larger than the final output. Care should be taken when time or
/// especially memory usage become concerns. In the worst case, **this may attempt to fetch every
/// element from the source**. Consult implementation-specific documentation for which query
/// operations have "good" translations.
///
/// Note that this translation is oblivious to any internal operations of the source. The source
/// might silently ignore the query or return something entirely different.
#[expect(missing_debug_implementations, reason = "TODO")]
pub struct Single<'a, Q, T> {
    /// The query that when executed should select a superset of the desired elements.
    pub query: Q,
    /// The residue subqueries. Executing these on the data returned by running
    /// [`query`](Self::query) should produce the desired output, assuming the source produced the
    /// values requested by the query.
    pub residue: Vec<&'a (dyn Query<T> + Sync)>,
}

/// A typed handle to a struct field.
///
/// A `Field<T, U>` represents:
/// - How to access a field of type `U` from a value of type `T`.
/// - The field’s logical name, used for debugging and translation.
///
/// Fields constructors are generated from [`#[derive(Queryable)]`].
// TODO: Can these fields be simplified?
#[derive(Clone)]
pub struct Field<T: ?Sized, U: ?Sized> {
    /// Field name.
    name: Arc<str>,
    /// Getter function
    getter: Arc<dyn for<'b> Fn(&'b T) -> &'b U + Send + Sync>,
}

impl<T: ?Sized, U: ?Sized> Field<T, U> {
    /// Constructs a `Field`  from a field name and associated getter function.
    #[inline]
    pub fn new(name: Arc<str>, getter: impl Fn(&T) -> &U + Send + Sync + 'static) -> Self {
        Self {
            name,
            getter: Arc::new(getter),
        }
    }
}

impl<T: ?Sized, U: ?Sized> fmt::Debug for Field<T, U> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { name, getter: _ } = self;
        f.debug_struct(name_of_type!(Self))
            .field(name_of!(name in Self), name)
            .finish_non_exhaustive()
    }
}

impl<T: ?Sized + 'static, U: ?Sized + 'static> Field<T, U> {
    /// Compose two fields to access a nested value.
    ///
    /// This allows queries over nested structures while preserving a flat, dotted field name for
    /// translation purposes.
    #[inline]
    #[must_use]
    pub fn then<V: ?Sized + 'static>(self, next: &Field<U, V>) -> Field<T, V> {
        let name = Arc::<str>::from(format!("{}.{}", self.name, next.name));
        let g1 = Arc::clone(&self.getter);
        let g2 = Arc::clone(&next.getter);

        Field {
            name,
            getter: Arc::new(move |t: &T| g2(g1(t))),
        }
    }
}

impl<T, U: ?Sized> Field<T, U> {
    /// Specifies that the field should be equal to `value`.
    #[inline]
    pub const fn eq<V: ?Sized>(self, value: &V) -> Eq<'_, Self, V> {
        Eq { field: self, value }
    }

    /// Specifies that the field should not be equal to `value`.
    #[inline]
    pub const fn ne<V: ?Sized>(self, value: &V) -> Ne<'_, Self, V> {
        Ne { field: self, value }
    }

    /// Specifies that the field should be greater than `value`.
    #[inline]
    pub const fn gt<V: ?Sized>(self, value: &V) -> Gt<'_, Self, V> {
        Gt { field: self, value }
    }

    /// Specifies that the field should be lesser than `value`.
    #[inline]
    pub const fn lt<V: ?Sized>(self, value: &V) -> Lt<'_, Self, V> {
        Lt { field: self, value }
    }
}

// TODO: Add compilation tests besides normal unit tests.

/// Key-value pairs ready to be serialized as HTTP parameters.
///
/// Note that while many endpoints ignore duplicate keys and key order, this is not actually part
/// of any particular specification, so this implementation conservatively does not check for
/// duplicate keys (regardless of their values) and maintains input key order.
//
// This is `Vec<_>` instead of `Box<[_]>` both because implementation becomes easier, but also
// because callers may choose to add extra parameters beyond what is provided by the query API.
// TODO: Feature flag to disable preserving order, if this would be more performant?
#[cfg(feature = "rest")]
#[expect(clippy::module_name_repetitions, reason = "Established terminology.")]
pub type HttpQuery<'a> = Vec<(&'a str, Box<str>)>;

// Type for making graphql queries simpler???
// #[cfg(feature = "graphql")]
// #[expect(clippy::module_name_repetitions, reason = "Established terminology.")]
// pub type GraphqlQuery<'a> = Vec<(&'a str, Box<str>)>;
