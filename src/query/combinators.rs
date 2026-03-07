//! Query primitives and logical combinators.
//!
//! This module defines the core query abstract syntax tree (AST) and the semantics for evaluating
//! queries against in-memory data. Queries are composable, immutable, and evaluated recursively.

#[cfg(feature = "postgres")]
use super::SqlStatement;
use super::{Field, Query};
#[cfg(feature = "rest")]
use super::{HttpQuery, Single};
use either::Either;
use nameof::{name_of, name_of_type};
#[cfg(feature = "rest")]
use std::collections::HashSet;
use std::fmt::{Debug, Display, Error as FmtError, Formatter};

/// Matches everything.
///
/// This might be useful to fetch all data from a source.
#[derive(Clone)]
pub struct True;

// TODO: `F` parameters in combinators should be replaced by parameterized `Field` types.

/// Checks if the field specified by `field` is equal to `value`.
#[derive(Clone)]
pub struct Eq<'a, F, V: ?Sized> {
    /// The field to check equality on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub value: &'a V,
}

/// Checks if the field specified by `field` is not equal to `value`.
///
/// This is a pure query node and does not perform any evaluation by itself.
#[derive(Clone)]
pub struct Ne<'a, F, V: ?Sized> {
    /// The field to check inequality on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub value: &'a V,
}

/// Checks if the field specified by `field` is greater than `value`.
///
/// This is a pure query node and does not perform any evaluation by itself.
#[derive(Clone)]
pub struct Gt<'a, F, V: ?Sized> {
    /// The field to perform comparison on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub value: &'a V,
}

/// Checks if the field specified by `field` is lesser than `value`.
///
/// This is a pure query node and does not perform any evaluation by itself.
#[derive(Clone)]
pub struct Lt<'a, F, V: ?Sized> {
    /// The field to perform comparison on.
    pub(super) field: F,
    /// The value to compare the field to.
    pub value: &'a V,
}

/// Performs AND on the two subqueries.
#[derive(Clone)]
pub struct And<L, R>(pub L, pub R);

/// Performs OR on the two subqueries.
#[derive(Clone)]
pub struct Or<L, R>(pub L, pub R);

/// Performs XOR on the two subqueries.
#[derive(Clone)]
pub struct Xor<L, R>(pub L, pub R);

/// Negates a query.
#[derive(Clone)]
pub struct Not<Q>(pub Q);

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

impl<T> Query<T> for True {
    /// Returns `true`.
    #[inline]
    fn evaluate(&self, _: &T) -> bool {
        true
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        Single {
            query: SqlStatement {
                query_text: String::new(),
                params: Vec::new(),
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![<Self as Query<T>>::to_sql_single(self).query])
    }

    /// Returns a query with no parameters and no residue.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        Single {
            query: HttpQuery::new(),
            residue: Vec::new(),
        }
    }

    /// Returns a single query with no parameters.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        Some(vec![HttpQuery::new()])
    }
}

impl<T, U, V> Query<T> for Eq<'_, Field<T, U>, V>
where
    U: PartialEq<V> + ?Sized,
    // TODO: This bound is not required for `evauluate`, but there will be many situations like
    // this one where translation methods require more bounds. Is adding them to the entire trait
    // implementation acceptable? Should the bound at least be feature gated?
    V: Display + ?Sized,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter, .. },
            value,
        } = self;
        getter(data) == *value
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        Single {
            query: SqlStatement {
                query_text: format!("{} = ?", self.field.name),
                params: vec![self.value.to_string()],
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Returns a query with one parameter, that being the field name and `value.to_string()`, and
    /// no residue.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        let Self { field: _, value } = self;
        Single {
            query: vec![(&self.field.name, value.to_string().into())],
            residue: Vec::new(),
        }
    }

    /// Returns a single query with one parameter, that being the field name and
    /// `value.to_string()`.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        let Self { field: _, value } = self;
        let query = vec![(&*self.field.name, value.to_string().into())];
        Some(vec![query])
    }
}

impl<T, U, V> Query<T> for Ne<'_, Field<T, U>, V>
where
    U: PartialEq<V> + ?Sized,
    V: Sync + ?Sized + ToString,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter, .. },
            value,
        } = self;
        getter(data) != *value
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        Single {
            query: SqlStatement {
                query_text: format!("{} != ?", self.field.name),
                params: vec![self.value.to_string()],
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Returns a single query with no parameters, meaning **this entire (sub)query remains as
    /// residue**.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        None
    }
}

impl<T, U, V> Query<T> for Gt<'_, Field<T, U>, V>
where
    U: PartialOrd<V> + ?Sized,
    V: Sync + ?Sized + ToString,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter, .. },
            value,
        } = self;
        getter(data) > *value
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        Single {
            query: SqlStatement {
                query_text: format!("{} > ?", self.field.name),
                params: vec![self.value.to_string()],
            },
            // Empty residue. Postgres handles the logic natively
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Returns a single query with no paramqters, meaning **this entire (sub)query remains as
    /// residue**.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        None
    }
}

impl<T, U, V> Query<T> for Lt<'_, Field<T, U>, V>
where
    U: PartialOrd<V> + ?Sized,
    V: Sync + ?Sized + ToString,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self {
            field: Field { getter, .. },
            value,
        } = self;
        getter(data) < *value
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        Single {
            query: SqlStatement {
                query_text: format!("{} < ?", self.field.name),
                params: vec![self.value.to_string()],
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Returns a single query with no parameters, meaning **this entire (sub)query remains as
    /// residue**.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        None
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

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        let Self(lhs, rhs) = self;
        let mut l_single = lhs.to_sql_single();
        let mut r_single = rhs.to_sql_single();

        l_single.query.params.append(&mut r_single.query.params);

        Single {
            query: SqlStatement {
                query_text: format!(
                    "({}) AND ({})",
                    l_single.query.query_text, r_single.query.query_text
                ),
                params: l_single.query.params,
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Combines both the parameter lists and the residues of both subqueries.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        let Self(lhs, rhs) = self;
        let Single {
            mut query,
            mut residue,
        } = lhs.to_http_single();
        let mut rhs = rhs.to_http_single();

        query.append(&mut rhs.query);
        residue.append(&mut rhs.residue);

        Single { query, residue }
    }

    /// Creates the cartesian product of all parts from both subqueries. Although this is
    /// theoretically a "perfect" translation, **the number of output queries grows very quickly
    /// for complex queries**.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        let Self(lhs, rhs) = self;
        let lhs = lhs.to_http_multi()?;
        let rhs = rhs.to_http_multi()?;
        let mut result = Vec::new();

        for l in &lhs {
            for r in &rhs {
                let mut l = l.clone();
                l.extend_from_slice(r);
                result.push(l);
            }
        }

        Some(result)
    }
}

/// Backing implementation for [`Or::to_http_single`] and [`Xor::to_http_single`].
#[cfg(feature = "rest")]
fn or_to_single_impl<'a, T>(
    lhs: &'a impl Query<T>,
    rhs: &'a impl Query<T>,
) -> Single<'a, HttpQuery<'a>, T> {
    let Single {
        mut query,
        mut residue,
    } = lhs.to_http_single();
    let mut rhs = rhs.to_http_single();

    // TODO: Is this needlessly complex?
    let mut rhs_query = HashSet::with_capacity(rhs.query.len());
    rhs_query.extend(rhs.query);
    query.retain(|z| rhs_query.contains(z));

    residue.append(&mut rhs.residue);

    Single { query, residue }
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

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        let Self(lhs, rhs) = self;
        let mut l_single = lhs.to_sql_single();
        let mut r_single = rhs.to_sql_single();

        l_single.query.params.append(&mut r_single.query.params);

        Single {
            query: SqlStatement {
                query_text: format!(
                    "({}) OR ({})",
                    l_single.query.query_text, r_single.query.query_text
                ),
                params: l_single.query.params,
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Retains only the parameters specified in both subqueries, and combines the residues.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        let Self(lhs, rhs) = self;
        or_to_single_impl(lhs, rhs)
    }

    /// Combines the partial queries.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        let Self(lhs, rhs) = self;
        let mut lhs = lhs.to_http_multi()?;
        let mut rhs = rhs.to_http_multi()?;

        lhs.append(&mut rhs);
        Some(lhs)
    }
}

impl<T, L, R> Query<T> for Xor<L, R>
where
    L: Query<T> + Sync,
    R: Query<T> + Sync,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self(lhs, rhs) = self;
        lhs.evaluate(data) ^ rhs.evaluate(data)
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        let Self(lhs, rhs) = self;
        let mut l_single = lhs.to_sql_single();
        let mut r_single = rhs.to_sql_single();

        l_single.query.params.append(&mut r_single.query.params);

        Single {
            query: SqlStatement {
                query_text: format!(
                    "({}) != ({})",
                    l_single.query.query_text, r_single.query.query_text
                ),
                params: l_single.query.params,
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Retains only the parameters specified in both subqueries, and combines the residues. The
    /// XOR itself also remains as residue.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        let Self(lhs, rhs) = self;
        // For this purpose, XOR is just an OR that might exclude some more results after the
        // local filtering step.
        let mut res = or_to_single_impl(lhs, rhs);
        res.residue.push(self);
        res
    }

    /// Translation is impossible.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        None
    }
}

impl<T, Q> Query<T> for Not<Q>
where
    Q: Query<T> + Sync,
{
    #[inline]
    fn evaluate(&self, data: &T) -> bool {
        let Self(query) = self;
        !query.evaluate(data)
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        let Self(inner_query) = self;
        let single = inner_query.to_sql_single();

        Single {
            query: SqlStatement {
                query_text: format!("NOT ({})", single.query.query_text),
                params: single.query.params,
            },
            residue: Vec::new(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    /// Returns a single query with no parameters, meaning **this entire (sub)query remains as
    /// residue**.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        None
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

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_single(&self) -> Single<'_, SqlStatement, T> {
        match self {
            Self::Left(query) => query.to_sql_single(),
            Self::Right(query) => query.to_sql_single(),
        }
    }

    #[cfg(feature = "postgres")]
    #[inline]
    fn to_sql_multi(&self) -> Option<Vec<SqlStatement>> {
        Some(vec![self.to_sql_single().query])
    }

    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery<'_>, T> {
        match self {
            Self::Left(query) => query.to_http_single(),
            Self::Right(query) => query.to_http_single(),
        }
    }

    /// Translation is impossible.
    #[cfg(feature = "rest")]
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery<'_>>> {
        match self {
            Self::Left(query) => query.to_http_multi(),
            Self::Right(query) => query.to_http_multi(),
        }
    }
}

// Debug implementations support two formats:
// - Standard (`{:?}`): structured, machine-readable
// - Alternate (`{:#?}`): compact, human-readable query syntax

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

impl<T, V, U> Debug for Eq<'_, Field<T, V>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{} = {:#?}", self.field.name, value)
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &self.field.name)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U> Debug for Ne<'_, Field<T, V>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{} != {:#?}", self.field.name, value)
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &self.field.name)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U> Debug for Gt<'_, Field<T, V>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{} > {:#?}", self.field.name, value)
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &self.field.name)
                .field(name_of!(value in Self), value)
                .finish()
        }
    }
}

impl<T, V, U> Debug for Lt<'_, Field<T, V>, U>
where
    U: Debug + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        let Self { field: _, value } = self;
        if f.alternate() {
            write!(f, "{} < {:#?}", self.field.name, value)
        } else {
            f.debug_struct(name_of_type!(Self))
                .field(name_of!(field in Self), &self.field.name)
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
            write!(f, "({lhs:#?}) & ({rhs:#?})")
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
            write!(f, "({lhs:#?}) | ({rhs:#?})")
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
            write!(f, "({lhs:#?}) ^ ({rhs:#?})")
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
