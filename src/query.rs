//! Typed query construction API.
//!
//! This module provides the core building blocks for constructing,
//! combining, and translating queries in a type-safe manner.
//!
//! Queries can be:
//! - Evaluated locally via [`Query`]
//! - Translated into HTTP parameters via [`ToHttp`]

pub use query_macro::Queryable;
use std::fmt;
use std::sync::Arc;

/// Query primitives and combinators.
mod combinators;
pub use combinators::*;

/// Translation of queries into other formats.
mod translate;
pub use crate::query::translate::ToHttp;
pub use translate::*;

/// Concatenate two field-name segments into a dotted path.
/// Example: "address" + "city" -> "address.city"
fn concat_names(a: &'static str, b: &'static str) -> &'static str {
    Box::leak(format!("{a}.{b}").into_boxed_str())
}

/// A typed handle to a struct field.
///
/// A `Field<T, U>` represents:
/// - how to access a field of type `U` from a value of type `T`
/// - the field’s logical name, used for debugging and translation
///
/// Fields are typically constructed via `#[derive(Queryable)]`.
#[derive(Clone)]
pub struct Field<T: ?Sized, U: ?Sized> {
    /// Field name (used for debug / translation)
    pub name: &'static str,
    /// Getter function
    pub getter: Arc<dyn for<'b> Fn(&'b T) -> &'b U + Send + Sync>,
}

// Manual Debug impl (required by project lint rules)
impl<T: ?Sized, U: ?Sized> fmt::Debug for Field<T, U> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}
impl<T: ?Sized + 'static, U: ?Sized + 'static> Field<T, U> {
    /// Construct a new Field from a name and getter function.
    #[inline]
    pub fn new(name: &'static str, getter: impl Fn(&T) -> &U + Send + Sync + 'static) -> Self {
        Self {
            name,
            getter: Arc::new(getter),
        }
    }

    /// Compose two fields to access a nested value.
    ///
    /// This allows queries over nested structures while preserving
    /// a flat, dotted field name for translation purposes.
    #[must_use]
    #[inline]
    pub fn then<V: ?Sized + 'static>(self, next: &Field<U, V>) -> Field<T, V> {
        let name = concat_names(self.name, next.name);
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
