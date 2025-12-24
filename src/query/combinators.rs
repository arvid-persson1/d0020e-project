use super::Field;
use either::Either;
use nameof::{name_of, name_of_type};
use std::fmt::{Debug, Error as FmtError, Formatter};

#[derive(Clone)]
pub struct True;

#[derive(Clone)]
pub struct False;

#[derive(Clone)]
pub struct Eq<'a, F, U: ?Sized> {
    pub(super) getter: F,
    pub(super) value: &'a U,
}

#[derive(Clone)]
pub struct Ne<'a, F, U: ?Sized> {
    pub(super) getter: F,
    pub(super) value: &'a U,
}

#[derive(Clone)]
pub struct Gt<'a, F, U: ?Sized> {
    pub(super) getter: F,
    pub(super) value: &'a U,
}

#[derive(Clone)]
pub struct Lt<'a, F, U: ?Sized> {
    pub(super) getter: F,
    pub(super) value: &'a U,
}

#[derive(Clone)]
pub struct And<L, R>(pub(super) L, pub(super) R);

#[derive(Clone)]
pub struct Or<L, R>(pub(super) L, pub(super) R);

#[derive(Clone)]
pub struct Xor<L, R>(pub(super) L, pub(super) R);

#[derive(Clone)]
pub struct Not<Q>(pub(super) Q);

// TODO: Possible future combinators:
// - Remaining comparators: `Ge`, `Le`.
// - Remaining logic gates: `Nand`, `Nor`, `Xor`, `Xnor`.
// - Variadic logic gates: `All`, `Any`, `One`.
// - Interconnected field equality (e.g. `.foo == .bar`).
// - Type-specific queries (e.g. `StartsWith` for strings).
// However, since queries are expressed through types, the compiler should be able to optimize
// them well as is. As such, some of these combinators would be more of a convencience feature
// rather than new functionality.

pub trait Query<T> {
    fn evaluate(&self, data: &T) -> bool;
}

impl<T> Query<T> for True {
    fn evaluate(&self, _: &T) -> bool {
        true
    }
}

impl<T> Query<T> for False {
    fn evaluate(&self, _: &T) -> bool {
        false
    }
}

impl<F, T, U, V> Query<T> for Eq<'_, F, V>
where
    F: Fn(&T) -> &U,
    U: PartialEq<V> + ?Sized,
    V: ?Sized,
{
    fn evaluate(&self, data: &T) -> bool {
        let Self { getter, value } = self;
        getter(data) == *value
    }
}

impl<F, T, U, V> Query<T> for Ne<'_, F, V>
where
    F: Fn(&T) -> &U,
    U: PartialEq<V> + ?Sized,
    V: ?Sized,
{
    fn evaluate(&self, data: &T) -> bool {
        let Self { getter, value } = self;
        getter(data) != *value
    }
}

impl<F, T, U, V> Query<T> for Gt<'_, F, V>
where
    F: Fn(&T) -> &U,
    U: PartialOrd<V> + ?Sized,
    V: ?Sized,
{
    fn evaluate(&self, data: &T) -> bool {
        let Self { getter, value } = self;
        getter(data) > *value
    }
}

impl<F, T, U, V> Query<T> for Lt<'_, F, V>
where
    F: Fn(&T) -> &U,
    U: PartialOrd<V> + ?Sized,
    V: ?Sized,
{
    fn evaluate(&self, data: &T) -> bool {
        let Self { getter, value } = self;
        getter(data) < *value
    }
}

impl<T, L, R> Query<T> for And<L, R>
where
    L: Query<T>,
    R: Query<T>,
{
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
    fn evaluate(&self, data: &T) -> bool {
        let Self(lhs, rhs) = self;
        lhs.evaluate(data) ^ rhs.evaluate(data)
    }
}

impl<T, Q> Query<T> for Not<Q>
where
    Q: Query<T>,
{
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
    fn evaluate(&self, data: &T) -> bool {
        match self {
            Self::Left(query) => query.evaluate(data),
            Self::Right(query) => query.evaluate(data),
        }
    }
}

impl Debug for True {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        if f.alternate() {
            write!(f, "true")
        } else {
            f.debug_struct(name_of_type!(Self)).finish()
        }
    }
}

impl Debug for False {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        if f.alternate() {
            write!(f, "false")
        } else {
            f.debug_struct(name_of_type!(Self)).finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Eq<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { getter, value } = self;
        if f.alternate() {
            write!(f, "{NAME} = {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(getter in Self), &NAME)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Ne<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { getter, value } = self;
        if f.alternate() {
            write!(f, "{NAME} != {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(getter in Self), &NAME)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Gt<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { getter, value } = self;
        if f.alternate() {
            write!(f, "{NAME} > {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(getter in Self), &NAME)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U, const NAME: &'static str> Debug for Lt<'_, Field<T, V, NAME>, U>
where
    U: Debug + ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { getter, value } = self;
        if f.alternate() {
            write!(f, "{NAME} < {value:#?}")
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(getter in Self), &NAME)
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self(query) = self;
        if f.alternate() {
            write!(f, "!({query:#?})")
        } else {
            f.debug_tuple(name_of_type!(Self)).field(query).finish()
        }
    }
}
