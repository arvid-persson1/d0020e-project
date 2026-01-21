use super::Field;
use either::Either;
use nameof::{name_of, name_of_type};
use std::fmt::{Debug, Error as FmtError, Formatter};

/// Matches everything.
///
/// This might be useful to fetch all data from a source.
#[derive(Clone)]
pub struct True;

/// Checks if the field specified by `field` is equal to `value`.
#[derive(Clone)]
pub struct Eq<'a, F, U: ?Sized> {
    /// The field to check equality on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub(super) value: &'a U,
}

/// Checks if the field specified by `field` is not equal to `value`.
#[derive(Clone)]
pub struct Ne<'a, F, U: ?Sized> {
    /// The field to check inequality on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub(super) value: &'a U,
}

/// Checks if the field specified by `field` is greater than `value`.
#[derive(Clone)]
pub struct Gt<'a, F, U: ?Sized> {
    /// The field to perform comparison on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub(super) value: &'a U,
}

/// Checks if the field specified by `field` is lesser than `value`.
#[derive(Clone)]
pub struct Lt<'a, F, U: ?Sized> {
    /// The field to perform comparison on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub(super) value: &'a U,
}

/// Performs AND on the two subqueries.
#[derive(Clone)]
pub struct And<L, R>(pub(super) L, pub(super) R);

/// Performs OR on the two subqueries.
#[derive(Clone)]
pub struct Or<L, R>(pub(super) L, pub(super) R);

/// Performs XOR on the two subqueries.
#[derive(Clone)]
pub struct Xor<L, R>(pub(super) L, pub(super) R);

/// Negates a query.
#[derive(Clone)]
pub struct Not<Q>(pub(super) Q);

// TODO: Possible future combinators:
// - Remaining comparators: `Ge`, `Le`.
// - Remaining logic gates: `Nand`, `Nor`, `Xor`, `Xnor`.
// However, since queries are expressed through types, the compiler should be able to optimize
// them well as is. As such, these above combinators would be more of a convenience feature rather
// than new functionality.
// - Variadic logic gates: `All`, `Any`, `One`.
// - Interconnected field equality (e.g. `.foo == .bar`).
// - Type-specific queries (e.g. `StartsWith` for strings).
// - `Limit`.
// - Explore possibilities of nested field access (e.g. `.foo.bar == 5`). Likely includes macro
// work.

/// A query that can be evaluated to check if some data matches a predicate.
pub trait Query<T> {
    /// Try to match `data` to the predicate specified by this query.
    fn evaluate(&self, data: &T) -> bool;
}

impl<T> Query<T> for True {
    /// Returns `true`.
    #[inline]
    fn evaluate(&self, _: &T) -> bool {
        true
    }
}

impl<T, U, const NAME: &'static str> Query<T> for Eq<'_, Field<T, U, NAME>, U>
where
    U: PartialEq + ?Sized,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter },
            value,
        } = self;
        getter(data) == *value
    }
}

impl<T, U, const NAME: &'static str> Query<T> for Ne<'_, Field<T, U, NAME>, U>
where
    U: PartialEq + ?Sized,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter },
            value,
        } = self;
        getter(data) != *value
    }
}

impl<T, U, const NAME: &'static str> Query<T> for Gt<'_, Field<T, U, NAME>, U>
where
    U: PartialOrd + ?Sized,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter },
            value,
        } = self;
        getter(data) > *value
    }
}

impl<T, U, const NAME: &'static str> Query<T> for Lt<'_, Field<T, U, NAME>, U>
where
    U: PartialOrd + ?Sized,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter },
            value,
        } = self;
        getter(data) < *value
    }
}

impl<T, L, R> Query<T> for And<L, R>
where
    L: Query<T>,
    R: Query<T>,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self(lhs, rhs) = self;
        lhs.evaluate(data) && rhs.evaluate(data)
    }
}

impl<T, L, R> Query<T> for Or<L, R>
where
    L: Query<T>,
    R: Query<T>,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self(lhs, rhs) = self;
        lhs.evaluate(data) || rhs.evaluate(data)
    }
}

impl<T, L, R> Query<T> for Xor<L, R>
where
    L: Query<T>,
    R: Query<T>,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self(lhs, rhs) = self;
        lhs.evaluate(data) ^ rhs.evaluate(data)
    }
}

impl<T, Q> Query<T> for Not<Q>
where
    Q: Query<T>,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self(query) = self;
        !query.evaluate(data)
    }
}

impl<T, L, R> Query<T> for Either<L, R>
where
    L: Query<T>,
    R: Query<T>,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        match self {
            Self::Left(query) => query.evaluate(data),
            Self::Right(query) => query.evaluate(data),
        }
    }
}

impl Debug for True {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        if f.alternate() {
            write!(f, "true")
        } else {
            f.debug_struct(name_of_type!(Self)).finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Eq<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{NAME} = {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &NAME)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Ne<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{NAME} != {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &NAME)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Gt<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{NAME} > {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &NAME)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Lt<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{NAME} < {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &NAME)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<L, R> Debug for And<L, R>
where
    L: Debug,
    R: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self(lhs, rhs) = self;
        if f.alternate() {
            write!(f, "{lhs:#?} & {rhs:#?}")
        } else {
            f.debug_tuple(name_of_type!(Self))
                .field(lhs)
                .field(rhs)
                .finish()
        }
    }
}

impl<L, R> Debug for Or<L, R>
where
    L: Debug,
    R: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self(lhs, rhs) = self;
        if f.alternate() {
            write!(f, "{lhs:#?} | {rhs:#?}")
        } else {
            f.debug_tuple(name_of_type!(Self))
                .field(lhs)
                .field(rhs)
                .finish()
        }
    }
}

impl<L, R> Debug for Xor<L, R>
where
    L: Debug,
    R: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self(lhs, rhs) = self;
        if f.alternate() {
            write!(f, "{lhs:#?} ^ {rhs:#?}")
        } else {
            f.debug_tuple(name_of_type!(Self))
                .field(lhs)
                .field(rhs)
                .finish()
        }
    }
}

impl<Q> Debug for Not<Q>
where
    Q: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self(query) = self;
        if f.alternate() {
            write!(f, "!({query:#?})")
        } else {
            f.debug_tuple(name_of_type!(Self)).field(query).finish()
        }
    }
}
