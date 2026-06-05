//! Index introspection.
//!
//! [`IndexStats`] is a runtime snapshot every implementer returns from
//! [`crate::IndexCore::stats`]. It carries the counters the engine and the
//! operator need to reason about an index without reaching into its
//! internals — how many vectors it holds, how much memory it occupies, what
//! type of index it is, and an open-ended `extra` map for index-specific
//! detail (tombstone counts, graph layer sizes, training state, …).

use std::collections::HashMap;

/// A runtime snapshot of an index's state.
///
/// Returned by [`crate::IndexCore::stats`]. The first four fields are the
/// shared shape; `extra` is where index-specific counters live (for example,
/// a tombstone count, or an HNSW layer histogram) without cluttering the
/// trait. Construct one with the public fields directly, or start from
/// [`IndexStats::default`] and override only what you have.
///
/// `disk_bytes` is [`Option`] because purely in-memory indexes have nothing
/// on disk — they report [`None`], not zero. An index that does spill to
/// disk reports the on-disk footprint in bytes.
///
/// `extra` is [`Option`] (not a bare [`HashMap`]) so an implementer with no
/// per-kind counters reports [`None`] without allocating an empty
/// [`HashMap`] on every `stats()` call. The default value is [`None`]. A
/// typical dashboard reads `n_vectors`, `memory_bytes`, and `index_type`
/// without touching `extra`; the occasional inspector that needs the
/// per-kind detail unwraps the `Option`.
///
/// # Examples
///
/// ```
/// use iqdb_index::IndexStats;
///
/// let stats = IndexStats {
///     n_vectors: 42,
///     memory_bytes: 4_096,
///     index_type: "flat",
///     ..IndexStats::default()
/// };
///
/// assert_eq!(stats.n_vectors, 42);
/// assert_eq!(stats.memory_bytes, 4_096);
/// assert_eq!(stats.disk_bytes, None);
/// assert_eq!(stats.index_type, "flat");
/// assert!(stats.extra.is_none());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexStats {
    /// The number of vectors currently searchable in the index.
    ///
    /// Excludes tombstoned entries: a vector that has been
    /// [`crate::IndexCore::delete`]d is not counted here even if the
    /// implementation has not yet reclaimed its storage.
    pub n_vectors: usize,
    /// Approximate resident-memory footprint of the index, in bytes.
    ///
    /// A best-effort number for the in-memory state of the index. It is
    /// suitable for capacity dashboards, not for accounting.
    pub memory_bytes: usize,
    /// On-disk footprint of the index in bytes, when the index persists to
    /// disk; [`None`] for purely in-memory indexes.
    pub disk_bytes: Option<usize>,
    /// A short, stable identifier for the index implementation (for
    /// example, `"flat"`, `"hnsw"`, `"ivf"`). Used in logs, metrics, and
    /// diagnostics.
    pub index_type: &'static str,
    /// Index-specific counters that do not fit the shared shape.
    ///
    /// Keys and values are both [`String`]s so this map can carry any
    /// implementer's bookkeeping (tombstone counts, build progress, graph
    /// statistics) without changing the trait surface. Wrapped in
    /// [`Option`] so implementers with no per-kind counters do not
    /// allocate an empty map on every `stats()` call.
    pub extra: Option<HashMap<String, String>>,
}
