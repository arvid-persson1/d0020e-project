use super::Query;

/// A best-effort translation of the input query to a single output query.
///
/// In cases where total translation is not possible, [`query`](Self::query) attempts to fetch a
/// minimal superset of the desired items, which are then intended to be filtered using the
/// provided [`residues`](Self::residue). The downside to this approach, of course, is that the
/// translation does not have perfect information and as such said superset of elements may be very
/// large, possibly many times larger than the final output. Care should be taken when time or
/// especially memory usage become concerns. In the worst case, **this may attempt to fetch every
/// element from the resource**. Consult implementation-specific documentation for which query
/// operations have "good" translations.
///
/// Note that this translation is oblivious to any internal operations of the resource. The
/// resource may silently ignore the query or return something entirely different.
// TODO: Implement `Debug`.
#[allow(missing_debug_implementations, reason = "TODO")]
pub struct Single<'a, Q, T> {
    /// The query that when executed should select a superset of the desired elements.
    pub query: Q,
    /// The residue subqueries. Executing these on the data returned by running
    /// [`query`](Self::query) should produce the desired output, assuming the
    pub residue: Vec<&'a dyn Query<T>>,
}

/// Translation to HTTP query parameters.
mod http;
pub use http::*;
