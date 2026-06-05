//! The [`IndexCore`] and [`Index`] traits.
//!
//! [`IndexCore`] is the object-safe operational surface every index exposes;
//! the engine stores indexes through `Box<dyn IndexCore>`. [`Index`] is the
//! typed-construction sibling that carries the implementer's associated
//! `Config` and a `Self`-returning `new` — used where the concrete type is
//! known, not through `dyn`.

use std::sync::Arc;

use iqdb_types::{DistanceMetric, Hit, Metadata, Result, SearchParams, VectorId};

use crate::stats::IndexStats;

/// The object-safe operational surface of an index.
///
/// Every concrete index implements this trait so the engine can hold a
/// heterogeneous set of indexes as `Box<dyn IndexCore>`. The methods are
/// the read/write/diagnose surface; construction is a separate concern, in
/// [`Index`].
///
/// ## Required vs. provided
///
/// Implementers must define [`insert`](IndexCore::insert),
/// [`delete`](IndexCore::delete), [`search`](IndexCore::search),
/// [`len`](IndexCore::len), [`dim`](IndexCore::dim),
/// [`metric`](IndexCore::metric), [`flush`](IndexCore::flush), and
/// [`stats`](IndexCore::stats). [`insert_batch`](IndexCore::insert_batch),
/// [`search_batch`](IndexCore::search_batch), and
/// [`is_empty`](IndexCore::is_empty) have default implementations and may
/// be overridden when a vectorized fast path exists.
///
/// ## Concurrency contract
///
/// Implementers MUST be `Send + Sync` — that bound is on the trait
/// itself, and the engine relies on it to share `Box<dyn IndexCore>`
/// across threads inside its per-shard locks. An implementer needs only
/// to be **single-writer-internal**: the engine guards every concrete
/// index with an external `RwLock`, so the index sees either many
/// concurrent shared (`&self`) accesses OR one exclusive (`&mut self`)
/// access at a time. Indexes therefore do not need their own internal
/// locking; correctness against `&self` aliasing is enough.
///
/// In particular, [`insert`](IndexCore::insert),
/// [`delete`](IndexCore::delete), and [`flush`](IndexCore::flush) take
/// `&mut self` and are only ever called while the engine holds the
/// corresponding shard's write lock; [`search`](IndexCore::search),
/// [`len`](IndexCore::len), [`is_empty`](IndexCore::is_empty),
/// [`dim`](IndexCore::dim), [`metric`](IndexCore::metric), and
/// [`stats`](IndexCore::stats) take `&self` and may be called
/// concurrently from many threads while the shard's write lock is
/// unheld.
///
/// ## Ordering contract on `Hit.distance`
///
/// [`Hit`]'s `distance` is documented as **smaller is nearer**. Four of the
/// five metrics (Cosine, Euclidean, Manhattan, Hamming) satisfy that
/// natively. For [`DistanceMetric::DotProduct`], the raw inner product is a
/// similarity (larger is more similar), so an implementation MUST negate it
/// at the boundary — store `-dot` in `Hit.distance` — to keep one ordering
/// invariant across the index family.
///
/// ## Deletion contract
///
/// [`delete`](IndexCore::delete) is specified by **observable behavior**,
/// not by a storage mechanism. After `delete(id)`:
///
/// - `search` MUST NOT return `id` until a subsequent `insert(id, …)`
///   succeeds.
/// - Whether storage is reclaimed immediately, tombstoned, or compacted
///   later is implementation-defined and surfaced via
///   [`IndexStats::extra`].
///
/// An implementation MAY reject re-inserting a deleted id; if so, it MUST
/// document that and return [`iqdb_types::IqdbError::Duplicate`].
///
/// ## Examples
///
/// See the crate-level docs for a runnable mock implementation that
/// exercises every method of this trait.
pub trait IndexCore: Send + Sync {
    /// Insert one vector into the index.
    ///
    /// `vector.len()` MUST equal [`dim`](IndexCore::dim); otherwise the
    /// call returns [`iqdb_types::IqdbError::DimensionMismatch`]. Inserting
    /// an `id` that is already present returns
    /// [`iqdb_types::IqdbError::Duplicate`].
    ///
    /// `vector` is owned via [`Arc<[f32]>`](std::sync::Arc) so the engine
    /// can hand the same payload allocation to the index and its
    /// authoritative record store without copying — the index takes one
    /// strong reference (no `[f32]` data copy), the record store keeps
    /// another. Implementers SHOULD store the `Arc` (or a clone) rather than
    /// allocating a fresh buffer, so the shared-payload guarantee holds.
    fn insert(
        &mut self,
        id: VectorId,
        vector: Arc<[f32]>,
        metadata: Option<Metadata>,
    ) -> Result<()>;

    /// Insert many vectors in a single call.
    ///
    /// The default implementation loops over `items` and calls
    /// [`insert`](IndexCore::insert) for each one. It is **fail-fast**: the
    /// first error returns immediately, and any inserts that succeeded
    /// before that point remain in the index. Implementers MAY override
    /// with a vectorized path; if they do, they SHOULD document whether
    /// they preserve fail-fast or apply atomically.
    fn insert_batch(&mut self, items: Vec<(VectorId, Arc<[f32]>, Option<Metadata>)>) -> Result<()> {
        for (id, vector, metadata) in items {
            self.insert(id, vector, metadata)?;
        }
        Ok(())
    }

    /// Remove the vector identified by `id` from the search space.
    ///
    /// Returns [`iqdb_types::IqdbError::NotFound`] if no vector with that
    /// `id` is searchable. See the trait-level "Deletion contract" for the
    /// observable behavior implementations must guarantee.
    fn delete(&mut self, id: &VectorId) -> Result<()>;

    /// Run a top-`k` similarity search.
    ///
    /// Returns up to `params.k` [`Hit`]s ordered best-first (smallest
    /// `Hit.distance` first; see the trait-level "Ordering contract"). If
    /// `params.metric` does not match [`metric`](IndexCore::metric),
    /// implementations return [`iqdb_types::IqdbError::InvalidMetric`]. A
    /// `query.len()` that does not match [`dim`](IndexCore::dim) returns
    /// [`iqdb_types::IqdbError::DimensionMismatch`].
    fn search(&self, query: &[f32], params: &SearchParams) -> Result<Vec<Hit>>;

    /// Run a batch of top-`k` searches with shared `params`.
    ///
    /// The default implementation loops over `queries` and calls
    /// [`search`](IndexCore::search) for each one, preserving input order in
    /// the returned outer `Vec`. Implementers MAY override with a vectorized
    /// path.
    fn search_batch(&self, queries: &[&[f32]], params: &SearchParams) -> Result<Vec<Vec<Hit>>> {
        let mut out = Vec::with_capacity(queries.len());
        for query in queries {
            out.push(self.search(query, params)?);
        }
        Ok(out)
    }

    /// The number of vectors currently searchable in the index.
    ///
    /// Excludes tombstoned or otherwise logically-deleted entries.
    fn len(&self) -> usize;

    /// Returns `true` when the index holds no searchable vectors.
    ///
    /// Default implementation: `self.len() == 0`. Implementers MAY override
    /// if they can answer this faster than counting.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The dimensionality of vectors this index was configured for.
    fn dim(&self) -> usize;

    /// The distance metric this index was configured for.
    fn metric(&self) -> DistanceMetric;

    /// Flush any pending state to durable storage.
    ///
    /// Purely in-memory indexes return `Ok(())` immediately. Indexes that
    /// buffer writes or persist to disk MUST commit those changes here.
    fn flush(&mut self) -> Result<()>;

    /// A runtime snapshot of the index's state.
    ///
    /// See [`IndexStats`] for the shape; index-specific counters live in
    /// [`IndexStats::extra`].
    fn stats(&self) -> IndexStats;
}

/// Typed construction for an index.
///
/// Adds an associated [`Config`](Index::Config) and a `Self`-returning
/// [`new`](Index::new) to the operational surface from [`IndexCore`]. This
/// trait is **not object-safe** — that is by design; the engine holds
/// indexes as `Box<dyn IndexCore>` after they have been constructed
/// through [`Index::new`].
///
/// Every concrete index implements both [`IndexCore`] and `Index`.
///
/// ## Examples
///
/// See the crate-level docs for a runnable mock that implements both
/// traits.
pub trait Index: IndexCore {
    /// Configuration consumed at construction time.
    ///
    /// `Default` lets a caller spin up an index with zero configuration;
    /// `Clone` lets the engine reuse a config across builds (for example,
    /// when rebalancing or rebuilding shards).
    type Config: Default + Clone;

    /// Build a fresh index of `dim` vectors under `metric` with `config`.
    ///
    /// Returns [`iqdb_types::IqdbError::InvalidConfig`] when `dim == 0` or
    /// when `config` does not describe a working index; implementations
    /// SHOULD reject invalid combinations of `metric` and `config` here
    /// rather than at first use.
    fn new(dim: usize, metric: DistanceMetric, config: Self::Config) -> Result<Self>
    where
        Self: Sized;
}
