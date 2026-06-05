# iqdb-index &mdash; API Reference

> Complete reference for **every** public item in `iqdb-index` as of **v1.0.0**:
> what it is, its parameters and return shape, the contract it carries, and
> worked examples for each use case.
>
> **Status: stable (1.0).** The public API is committed under SemVer for the 1.x
> series — no breaking changes until 2.0 (the frozen surface is recorded in
> [`dev/ROADMAP.md`](../dev/ROADMAP.md)). Only additive, non-breaking changes are
> made within 1.x.

## Table of Contents

- [Overview](#overview)
- [The three tiers](#the-three-tiers)
- [Crate constants](#crate-constants)
  - [`VERSION`](#version)
- [Traits](#traits)
  - [`IndexCore`](#indexcore)
  - [`Index`](#index)
- [Introspection](#introspection)
  - [`IndexStats`](#indexstats)
- [Contracts](#contracts)
  - [Ordering contract](#ordering-contract)
  - [Deletion contract](#deletion-contract)
  - [Concurrency contract](#concurrency-contract)
- [Errors](#errors)
- [Feature flags](#feature-flags)
- [Trait implementation matrix](#trait-implementation-matrix)

---

## Overview

`iqdb-index` is the polymorphism layer of the iQDB vector database. It defines
the shape every concrete index (flat, HNSW, IVF, …) implements, so the engine
can hold a heterogeneous set of indexes and run the right one without being
hardcoded to a single strategy. It is interface code: two traits plus one
introspection struct, depending only on
[`iqdb-types`](https://docs.rs/iqdb-types).

```rust
use std::sync::Arc;
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, SearchParams, VectorId};
# use iqdb_index::IndexStats;
# use iqdb_types::{Hit, IqdbError, Metadata, Result};
# struct Flat { dim: usize, metric: DistanceMetric, rows: Vec<(VectorId, Arc<[f32]>)> }
# #[derive(Default, Clone)] struct FlatConfig;
# impl IndexCore for Flat {
#   fn insert(&mut self, id: VectorId, v: Arc<[f32]>, _m: Option<Metadata>) -> Result<()> { self.rows.push((id, v)); Ok(()) }
#   fn delete(&mut self, id: &VectorId) -> Result<()> { match self.rows.iter().position(|(e,_)| e==id) { Some(p) => { let _ = self.rows.remove(p); Ok(()) }, None => Err(IqdbError::NotFound) } }
#   fn search(&self, q: &[f32], p: &SearchParams) -> Result<Vec<Hit>> {
#     let mut h: Vec<Hit> = self.rows.iter().map(|(id,v)| Hit { id: id.clone(), distance: q.iter().zip(v.iter()).map(|(a,b)|(a-b).powi(2)).sum(), metadata: None }).collect();
#     h.sort_by(|a,b| a.distance.total_cmp(&b.distance)); h.truncate(p.k); Ok(h)
#   }
#   fn len(&self) -> usize { self.rows.len() }
#   fn dim(&self) -> usize { self.dim }
#   fn metric(&self) -> DistanceMetric { self.metric }
#   fn flush(&mut self) -> Result<()> { Ok(()) }
#   fn stats(&self) -> IndexStats { IndexStats { n_vectors: self.rows.len(), index_type: "flat", ..IndexStats::default() } }
# }
# impl Index for Flat { type Config = FlatConfig; fn new(dim: usize, metric: DistanceMetric, _c: Self::Config) -> Result<Self> { Ok(Flat { dim, metric, rows: Vec::new() }) } }
# fn main() -> iqdb_types::Result<()> {
// Construct a concrete index, insert, and search — the common path.
let mut index = Flat::new(3, DistanceMetric::Euclidean, FlatConfig)?;
index.insert(VectorId::from(1u64), Arc::from([1.0, 0.0, 0.0].as_slice()), None)?;
let hits = index.search(&[1.0, 0.0, 0.0], &SearchParams::new(1, DistanceMetric::Euclidean))?;
assert_eq!(hits[0].id, VectorId::U64(1));
# Ok(()) }
```

**One contract, many backends.** Every guarantee a caller relies on — best-first
ordering, deletion visibility, the `Send + Sync` bound — lives on the trait, so
a flat scan and an HNSW graph are interchangeable behind it.

---

## The three tiers

The crate follows the iQDB tiered-API mandate:

| Tier | Surface | When |
|---|---|---|
| **Tier 1** | [`Index::new`] + the [`IndexCore`] operations (`insert`, `search`, `delete`, `stats`) | The common case: build an index and use it. |
| **Tier 2** | [`Index::Config`] — each index's own parameter struct | Tuning a specific backend (graph degree, probe count, …). |
| **Tier 3** | implementing [`IndexCore`] + [`Index`] yourself | Adding a brand-new index strategy. |

Tier 1 and Tier 2 are what consumers of an index use; Tier 3 is what an index
*crate* implements. This crate defines all three seams.

---

## Crate constants

### `VERSION`

```rust
pub const VERSION: &str;
```

The crate's compile-time version (`CARGO_PKG_VERSION`), a `major.minor.patch`
SemVer core. Use it to report the exact `iqdb-index` build a binary links
against — useful in diagnostics and version-skew checks across the family.

```rust
let v = iqdb_index::VERSION;
assert_eq!(v.split('.').count(), 3);
assert!(v.split('.').all(|part| !part.is_empty()));
```

---

## Traits

### `IndexCore`

```rust
pub trait IndexCore: Send + Sync {
    fn insert(&mut self, id: VectorId, vector: Arc<[f32]>, metadata: Option<Metadata>) -> Result<()>;
    fn delete(&mut self, id: &VectorId) -> Result<()>;
    fn search(&self, query: &[f32], params: &SearchParams) -> Result<Vec<Hit>>;
    fn len(&self) -> usize;
    fn dim(&self) -> usize;
    fn metric(&self) -> DistanceMetric;
    fn flush(&mut self) -> Result<()>;
    fn stats(&self) -> IndexStats;

    // Provided (overridable) methods:
    fn insert_batch(&mut self, items: Vec<(VectorId, Arc<[f32]>, Option<Metadata>)>) -> Result<()>;
    fn search_batch(&self, queries: &[&[f32]], params: &SearchParams) -> Result<Vec<Vec<Hit>>>;
    fn is_empty(&self) -> bool;
}
```

The **object-safe** operational surface every index exposes. The engine stores
indexes as `Box<dyn IndexCore>` and operates them without naming a concrete
type. The supertrait bound `Send + Sync` is mandatory — the engine shares index
objects across threads inside its per-shard locks.

**Object safety.** `IndexCore` has no generic methods, no associated types, and
no `Self`-by-value returns, so `Box<dyn IndexCore>` compiles. Construction
(which is *not* object-safe) is split out into [`Index`].

#### Required methods

##### `insert`

```rust
fn insert(&mut self, id: VectorId, vector: Arc<[f32]>, metadata: Option<Metadata>) -> Result<()>;
```

Insert one vector.

- **`id`** — the [`VectorId`] naming this vector.
- **`vector`** — the components, owned as an [`Arc<[f32]>`](std::sync::Arc) so the
  engine can share one allocation between the index and its record store
  without copying. Implementers should store the `Arc` (or a clone), not a fresh
  buffer.
- **`metadata`** — optional [`Metadata`] to return alongside hits.
- **Returns** `Ok(())`, or [`IqdbError::DimensionMismatch`] if
  `vector.len() != dim()`, or [`IqdbError::Duplicate`] if `id` is already
  present.

```rust
// `index` implements `Index + IndexCore`; see examples/custom_index.rs for a
// complete, runnable backend.
index.insert(VectorId::from(1u64), Arc::from([0.1, 0.2, 0.3].as_slice()), None)?;
assert_eq!(index.len(), 1);
```

##### `delete`

```rust
fn delete(&mut self, id: &VectorId) -> Result<()>;
```

Remove `id` from the search space. Returns [`IqdbError::NotFound`] if no vector
with that id is searchable. The *mechanism* (true removal, tombstone, deferred
compaction) is the implementation's choice; the *observable* result is fixed by
the [deletion contract](#deletion-contract).

##### `search`

```rust
fn search(&self, query: &[f32], params: &SearchParams) -> Result<Vec<Hit>>;
```

Run a top-`k` similarity search.

- **`query`** — the query components; `query.len()` must equal `dim()`.
- **`params`** — a [`SearchParams`]: `k`, the `metric`, and the optional `ef`
  and `filter`.
- **Returns** up to `params.k` [`Hit`]s, best-first (smallest distance first;
  see the [ordering contract](#ordering-contract)). Returns
  [`IqdbError::DimensionMismatch`] on a query-length mismatch, or
  [`IqdbError::InvalidMetric`] if `params.metric` does not match `metric()`.

```rust
// index holds ids 1 @ [0,0] and 2 @ [9,0]; query the origin for the nearest.
let hits = index.search(&[0.0, 0.0], &SearchParams::new(1, DistanceMetric::Euclidean))?;
assert_eq!(hits[0].id, VectorId::U64(1)); // nearest first
```

##### `len`, `dim`, `metric`, `flush`, `stats`

```rust
fn len(&self) -> usize;             // searchable vectors (excludes tombstones)
fn dim(&self) -> usize;             // configured dimensionality
fn metric(&self) -> DistanceMetric; // configured metric
fn flush(&mut self) -> Result<()>;  // commit buffered/persistent state
fn stats(&self) -> IndexStats;      // runtime snapshot
```

`len` counts only searchable vectors — a deleted-but-not-yet-reclaimed entry is
not counted. `flush` is `Ok(())` for purely in-memory indexes. `stats` returns
an [`IndexStats`] snapshot.

#### Provided methods

These ship with default implementations and may be overridden when a backend has
a vectorized fast path.

##### `insert_batch`

```rust
fn insert_batch(&mut self, items: Vec<(VectorId, Arc<[f32]>, Option<Metadata>)>) -> Result<()>;
```

Insert many vectors. The default loops over `items` calling [`insert`](#insert).
It is **fail-fast**: the first error returns immediately, and inserts that
already succeeded remain. An override may apply a vectorized or atomic path and
should document which.

```rust
index.insert_batch(vec![
    (VectorId::from(1u64), Arc::from([0.0, 0.0].as_slice()), None),
    (VectorId::from(2u64), Arc::from([1.0, 0.0].as_slice()), None),
])?;
assert_eq!(index.len(), 2);
```

##### `search_batch`

```rust
fn search_batch(&self, queries: &[&[f32]], params: &SearchParams) -> Result<Vec<Vec<Hit>>>;
```

Run several searches with shared `params`. The default loops over `queries`
calling [`search`](#search), preserving input order in the outer `Vec`.

##### `is_empty`

```rust
fn is_empty(&self) -> bool;
```

`true` when the index holds no searchable vectors. The default is
`self.len() == 0`; override only if you can answer it faster than counting.

---

### `Index`

```rust
pub trait Index: IndexCore {
    type Config: Default + Clone;
    fn new(dim: usize, metric: DistanceMetric, config: Self::Config) -> Result<Self>
    where
        Self: Sized;
}
```

Typed construction. `Index` adds an associated [`Config`](#configuration) and a
`Self`-returning [`new`](#new) on top of [`IndexCore`]. It is **deliberately not
object-safe** — `Box<dyn Index>` will not compile, and that is the point: the
engine constructs a concrete index through `Index::new`, then stores it as
`Box<dyn IndexCore>`. Every concrete index implements **both** traits.

#### Configuration

```rust
type Config: Default + Clone;
```

Each index exposes its own parameter struct rather than a shared god-config
enum. `Default` lets a caller construct with zero tuning; `Clone` lets the
engine reuse a config across rebuilds. An index with nothing to tune uses a
zero-sized unit config.

#### `new`

```rust
fn new(dim: usize, metric: DistanceMetric, config: Self::Config) -> Result<Self>;
```

Build a fresh index of `dim`-dimensional vectors under `metric` with `config`.
Returns [`IqdbError::InvalidConfig`] when `dim == 0` or `config` does not
describe a working index. Implementations should reject invalid `metric`/`config`
combinations here, not at first use.

```rust
// `Flat` is a concrete index implementing `Index` (see examples/custom_index.rs).
let ok = Flat::new(128, DistanceMetric::Cosine, FlatConfig);
assert!(ok.is_ok());

// Zero dimensions is rejected at construction.
let bad = Flat::new(0, DistanceMetric::Cosine, FlatConfig);
assert!(matches!(bad.unwrap_err(), IqdbError::InvalidConfig { .. }));
```

---

## Introspection

### `IndexStats`

```rust
pub struct IndexStats {
    pub n_vectors: usize,
    pub memory_bytes: usize,
    pub disk_bytes: Option<usize>,
    pub index_type: &'static str,
    pub extra: Option<HashMap<String, String>>,
}
```

A runtime snapshot returned by [`IndexCore::stats`]. The first four fields are
the shape shared by every index; `extra` carries index-specific counters
(tombstone counts, graph-layer histograms, training progress) without bloating
the trait.

- **`n_vectors`** — searchable vectors (excludes tombstones).
- **`memory_bytes`** — best-effort resident footprint, for dashboards, not
  accounting.
- **`disk_bytes`** — on-disk footprint, or [`None`] for a purely in-memory index
  (`None`, not `0`).
- **`index_type`** — a short, stable identifier (`"flat"`, `"hnsw"`, `"ivf"`).
- **`extra`** — per-kind counters, or [`None`] when there are none. It is an
  `Option` so the common `stats()` call allocates no empty map.

**Derives / traits:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`.

```rust
use iqdb_index::IndexStats;

// Start from `default()` and set only what you have.
let stats = IndexStats {
    n_vectors: 42,
    memory_bytes: 4_096,
    index_type: "flat",
    ..IndexStats::default()
};
assert_eq!(stats.n_vectors, 42);
assert_eq!(stats.disk_bytes, None);
assert!(stats.extra.is_none());
```

```rust
use std::collections::HashMap;
use iqdb_index::IndexStats;

// An index with per-kind detail populates `extra`.
let mut extra = HashMap::new();
extra.insert("tombstones".to_string(), "7".to_string());
let stats = IndexStats {
    n_vectors: 100,
    memory_bytes: 1 << 20,
    disk_bytes: Some(2 << 20),
    index_type: "hnsw",
    extra: Some(extra),
};
assert_eq!(stats.extra.unwrap().get("tombstones").map(String::as_str), Some("7"));
```

---

## Contracts

These are guarantees on the trait, not on any one implementation. A backend is
correct only if it honours all three.

### Ordering contract

[`Hit::distance`](iqdb_types::Hit) is **smaller-is-nearer**, and `search`
returns hits best-first. Four of the five metrics (Cosine, Euclidean, Manhattan,
Hamming) satisfy this natively. For
[`DistanceMetric::DotProduct`](iqdb_types::DistanceMetric) the raw inner product
is a *similarity* (larger is more similar), so an index **must negate it at the
boundary** — store `-dot` in `Hit.distance` — so one ordering invariant holds
across the whole family.

### Deletion contract

After `delete(id)`:

- `search` **must not** return `id` until a later `insert(id, …)` succeeds.
- Whether storage is reclaimed immediately, tombstoned, or compacted later is
  implementation-defined and surfaced through [`IndexStats::extra`].

An implementation **may** reject re-inserting a deleted id; if it does, it must
document that and return [`IqdbError::Duplicate`].

#### Per-implementation deletion semantics

The contract fixes the *observable* result; each consumer reaches it differently.
Recorded here (v0.5.0) for the three real consumers:

| Crate | Mechanism | Storage | `len()` | Notes |
|---|---|---|---|---|
| `iqdb-flat` | **true removal** | reclaimed immediately (`swap_remove`) | drops by 1 | order-independent topk, so the swap is safe; re-insert allowed |
| `iqdb-hnsw` | **tombstone** | retained (node kept for graph connectivity) | drops by 1 (live count) | tombstoned nodes are traversal-only and never returned by `search` |
| `iqdb-ivf` | **true removal** | reclaimed (`swap_remove` from the cluster's inverted list) | drops by 1 | id removed from its cluster's posting list |

That flat/ivf reclaim while hnsw tombstones — all behind one unchanged contract —
is exactly why the trait specifies *observable behaviour*, not a storage
mechanism. A graph index cannot cheaply excise a node without breaking
connectivity, so it tombstones; a flat or clustered index can drop a row in O(1).
Both honour "a deleted id never reappears in `search` until re-inserted."

### Concurrency contract

`IndexCore: Send + Sync` is required. An index needs only to be
**single-writer-internal**: the engine guards each concrete index with an
external `RwLock`, so the index sees either many concurrent `&self` reads
(`search`, `len`, `stats`, …) *or* one exclusive `&mut self` write (`insert`,
`delete`, `flush`) at a time. Indexes therefore do not need their own internal
locking.

---

## Synchronous by design

The trait is **synchronous** — every method returns `Result<T>`, not a future
— and that is a frozen decision (as of v0.4.0), for three reasons:

1. **Object safety on the hot path.** `IndexCore` must stay `dyn`-compatible so
   the engine can hold `Box<dyn IndexCore>`. An `async fn` in the trait is not
   `dyn`-compatible without boxing the returned future — a heap allocation on
   every `search`, which the query hot path cannot afford.
2. **The work is CPU-bound.** A nearest-neighbour search is an in-memory scan
   or graph walk, not I/O; a future buys nothing and costs a state machine.
3. **Async belongs at the engine boundary.** The engine already guards each
   index with an `RwLock` and can offload a blocking call (e.g. via
   `spawn_blocking`) if it wants an async API edge.

There is no async trait, no `async` feature, and no `futures` dependency. Async
is optional *wrapping* by a consumer, not part of this crate's surface.

---

## Errors

Every fallible method returns [`iqdb_types::Result<T>`](iqdb_types::Result),
whose error is the shared [`IqdbError`](iqdb_types::IqdbError). The variants this
crate's contract refers to:

| Variant | Raised when |
|---|---|
| [`DimensionMismatch`](iqdb_types::IqdbError::DimensionMismatch) | A vector or query length does not match `dim()`. |
| [`Duplicate`](iqdb_types::IqdbError::Duplicate) | `insert` collided with an id already present. |
| [`NotFound`](iqdb_types::IqdbError::NotFound) | `delete` named an id that is not searchable. |
| [`InvalidMetric`](iqdb_types::IqdbError::InvalidMetric) | `params.metric` did not match the index's `metric()`. |
| [`InvalidConfig`](iqdb_types::IqdbError::InvalidConfig) | `new` got `dim == 0` or an unworkable config. |

`IqdbError` is `#[non_exhaustive]`, so a `match` on it must carry a wildcard arm.

---

## Feature flags

`iqdb-index` has **no** feature flags. It is a pure, std-only trait crate with a
single dependency ([`iqdb-types`](https://docs.rs/iqdb-types)); there is nothing
to gate. The default build is the whole surface.

**Feature freeze (v0.4.0):** the feature set is frozen at *empty*. There is no
`std`/`no_std` split (the trait uses `std::sync::Arc` and
`std::collections::HashMap`), no `serde` gate (the surface is behaviour, not
data to serialize), and no `async` gate (see [Synchronous by
design](#synchronous-by-design)). Any feature added in the 1.x series would be
purely additive.

---

## Trait implementation matrix

| Item | Kind | Object-safe | Key bound |
|---|---|:---:|---|
| [`IndexCore`](#indexcore) | trait | ✅ (`Box<dyn IndexCore>`) | `Send + Sync` |
| [`Index`](#index) | trait | — (by design) | `Index: IndexCore`, `Config: Default + Clone` |
| [`IndexStats`](#indexstats) | struct | n/a | `Debug + Clone + Default + PartialEq + Eq` |

---

## Validation

As of v0.3.0 the surface is cross-checked against the three real consumers, each
of which implements `IndexCore` + `Index` verbatim with its own associated
`Config`:

| Crate | Type | `Config` | Family |
|---|---|---|---|
| `iqdb-flat` | `FlatIndex` | `FlatConfig` (unit) | brute-force |
| `iqdb-hnsw` | `HnswIndex` | `HnswConfig { m, ef_construction }` | graph |
| `iqdb-ivf` | `IvfIndex` | `IvfConfig { n_clusters, n_probes }` | clustered |

`tests/consumer_simulation.rs` encodes a stand-in for each at its exact
construction shape and proves all three coexist behind `Box<dyn IndexCore>` —
DIRECTIVES §8 (graph, clustered, and brute-force fit without awkward
abstractions). No trait change was required to carry them.

---

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
