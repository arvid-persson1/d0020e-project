use super::{
    super::{
        Field,
        combinators::{And, Eq, Gt, Lt, Ne, Not, Or, Query, True, Xor},
    },
    Single,
};
use std::{collections::HashSet, fmt::Display};

/// Key-value pairs ready to be serialized as HTTP parameters.
///
/// Note that while many endpoints ignore duplicate keys and key order, this is not actually part
/// of any particular specification, so this implementation conservatively does not check for
/// duplicate keys (regardless of their values) and maintains input key order.
//
// This is `Vec<_>` instead of `Box<[_]>` both because implementation becomes easier, but also
// because callers may choose to add extra parameters beyond what is provided by the query API.
// TODO: Feature flag to disable preserving order, if this would be more performant?
pub type HttpQuery = Vec<(&'static str, Box<str>)>;

/// Translate queries into HTTP format (HTTP query parameters) for use with
/// [REST connectors](crate::rest).
pub trait ToHttp<T>: Query<T> {
    /// Translate into a single [`HttpQuery`]. See [`Single`] documentation for caveats, and
    /// primitive- or combinator-specific documentation for details.
    fn to_http_single(&self) -> Single<'_, HttpQuery, T>;

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
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>>;
}

impl<T> ToHttp<T> for True {
    /// Returns a query with no parameters and no residue.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        Single {
            query: HttpQuery::new(),
            residue: Vec::new(),
        }
    }

    /// Returns a single query with no parameters.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        Some(vec![HttpQuery::new()])
    }
}

impl<T, U, const NAME: &'static str> ToHttp<T> for Eq<'_, Field<T, U, NAME>, U>
where
    U: PartialEq + Display + ?Sized,
{
    /// Returns a query with one parameter, that being the field name and `value.to_string()`, and
    /// no residue.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        let Self { field: _, value } = self;
        Single {
            query: vec![(NAME, value.to_string().into())],
            residue: Vec::new(),
        }
    }

    /// Returns a single query with one parameter, that being the field name and
    /// `value.to_string()`.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        let Self { field: _, value } = self;
        let query = vec![(NAME, value.to_string().into())];
        Some(vec![query])
    }
}

impl<T, U, const NAME: &'static str> ToHttp<T> for Ne<'_, Field<T, U, NAME>, U>
where
    U: PartialEq + ?Sized,
{
    /// Returns a single query with no parameters, meaning **this entire (sub)query remains as
    /// residue**.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        None
    }
}

impl<T, U, const NAME: &'static str> ToHttp<T> for Gt<'_, Field<T, U, NAME>, U>
where
    U: PartialOrd + ?Sized,
{
    /// Returns a single query with no parameters, meaning **this entire (sub)query remains as
    /// residue**.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        None
    }
}

impl<T, U, const NAME: &'static str> ToHttp<T> for Lt<'_, Field<T, U, NAME>, U>
where
    U: PartialOrd + ?Sized,
{
    /// Returns a single query with no parameters, meaning **this entire (sub)query remains as
    /// residue**.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        None
    }
}

impl<T, L, R> ToHttp<T> for And<L, R>
where
    L: ToHttp<T>,
    R: ToHttp<T>,
{
    /// Combines both the parameter lists and the residues of both subqueries.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
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
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
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
fn or_to_single_impl<'a, T, L, R>(lhs: &'a L, rhs: &'a R) -> Single<'a, HttpQuery, T>
where
    L: ToHttp<T>,
    R: ToHttp<T>,
{
    let Single {
        mut query,
        mut residue,
    } = lhs.to_http_single();
    let mut rhs = rhs.to_http_single();

    let mut rhs_query = HashSet::with_capacity(rhs.query.len());
    rhs_query.extend(rhs.query);
    query.retain(|z| rhs_query.contains(z));

    residue.append(&mut rhs.residue);

    Single { query, residue }
}

impl<T, L, R> ToHttp<T> for Or<L, R>
where
    L: ToHttp<T>,
    R: ToHttp<T>,
{
    /// Retains only the parameters specified in both subqueries, and combines the residues.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        let Self(lhs, rhs) = self;
        or_to_single_impl(lhs, rhs)
    }

    /// Combines the partial queries.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        let Self(lhs, rhs) = self;
        let mut lhs = lhs.to_http_multi()?;
        let mut rhs = rhs.to_http_multi()?;

        lhs.append(&mut rhs);
        Some(lhs)
    }
}

impl<T, L, R> ToHttp<T> for Xor<L, R>
where
    L: ToHttp<T>,
    R: ToHttp<T>,
{
    /// Retains only the parameters specified in both subqueries, and combines the residues. The
    /// XOR itself also remains as residue.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        let Self(lhs, rhs) = self;
        // For this purpose, XOR is just an OR that might exclude some more results after the
        // local filtering step.
        let mut res = or_to_single_impl(lhs, rhs);
        res.residue.push(self);
        res
    }

    /// Translation is impossible.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        None
    }
}

impl<T, Q> ToHttp<T> for Not<Q>
where
    Q: Query<T>,
{
    /// Returns a single query with no parameters, meaning **this entire (sub)query remains as
    /// residue**.
    #[inline]
    fn to_http_single(&self) -> Single<'_, HttpQuery, T> {
        Single {
            query: HttpQuery::new(),
            residue: vec![self],
        }
    }

    /// Translation is impossible.
    #[inline]
    fn to_http_multi(&self) -> Option<Vec<HttpQuery>> {
        None
    }
}
