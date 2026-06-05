# iqdb-index -- Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

---

## v0.1.0 -- Scaffold (DONE)

Compiles, CI green, structure correct, no domain logic.

- [x] Manifest, README, CHANGELOG, REPS, license, CI, lints in place.
- [x] API surface sketched in `docs/API.md`.

---

## v0.2.0 -- the `Index` trait + `IndexStats` + default batch impls (THE HARD PART, NOT DEFERRED) (DONE)

Split into `IndexCore` (object-safe operational surface) + `Index` (typed
construction) so the engine can hold `Box<dyn IndexCore>`. Default
`insert_batch` / `search_batch` / `is_empty` shims. `IndexStats` with an
allocation-free `extra` default.

Exit criteria:
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested (ordering, deletion visibility, batch == loop, cardinality).

---

## v0.3.0 -- validate against `iqdb-flat`; refine the trait (DONE)

Cross-checked the surface against the live `iqdb-flat` (brute-force),
`iqdb-hnsw` (graph), and `iqdb-ivf` (clustered) implementations — each
implements `IndexCore` + `Index` verbatim with its own `Config`. Encoded a
consumer-simulation proving all three families fit and coexist behind
`Box<dyn IndexCore>` (§8). No trait change was required.

Exit criteria:
- [x] New surface tested (consumer-simulation across all three families).
- [x] Hot path: the crate is pure interface code with no hot path of its own (the default batch shims are O(n) dispatch wrappers); nothing to benchmark here. A consumer's search hot path is benchmarked in that consumer's crate.

---

## v0.4.0 -- async decision (sync default) + feature freeze (DONE)

Recorded the **synchronous-by-design** decision (no async trait, no `async`
feature, no `futures` dep) — async would break `IndexCore` object safety
without per-call future boxing, search is CPU-bound, and async belongs at the
engine boundary. Declared the feature set frozen at empty.

Exit criteria:
- [x] No `todo!`/`unimplemented!` (the crate has none; `#![forbid(unsafe_code)]`).
- [x] Feature freeze declared (empty feature set, recorded in API.md + CHANGELOG).

---

## v0.5.0 -- doc the deletion semantics per impl + API freeze (DONE)

Recorded each consumer's deletion mechanism against the trait's deletion
contract (`docs/API.md`): `iqdb-flat` and `iqdb-ivf` reclaim storage (true
removal via `swap_remove`); `iqdb-hnsw` tombstones (node retained for graph
connectivity, never returned by `search`). All three honour the same observable
contract — proof that specifying behaviour, not mechanism, was correct.

Exit criteria:
- [x] Public API frozen (recorded below). `cargo audit` + `cargo deny` clean.

### Frozen public API (1.x) — recorded at v0.5.0

The following surface is frozen for the 1.x series. Additive, non-breaking
changes (new provided trait methods with defaults, new public items) remain
allowed; anything else waits for 2.0.

- **Constants:** `VERSION: &str`.
- **`IndexCore`** (object-safe; supertrait bound `Send + Sync`):
  - required: `insert(&mut self, VectorId, Arc<[f32]>, Option<Metadata>) -> Result<()>`, `delete(&mut self, &VectorId) -> Result<()>`, `search(&self, &[f32], &SearchParams) -> Result<Vec<Hit>>`, `len(&self) -> usize`, `dim(&self) -> usize`, `metric(&self) -> DistanceMetric`, `flush(&mut self) -> Result<()>`, `stats(&self) -> IndexStats`.
  - provided (overridable): `insert_batch(Vec<(VectorId, Arc<[f32]>, Option<Metadata>)>) -> Result<()>`, `search_batch(&[&[f32]], &SearchParams) -> Result<Vec<Vec<Hit>>>`, `is_empty(&self) -> bool`.
- **`Index`** (`Index: IndexCore`, not object-safe): associated `type Config: Default + Clone`; `new(dim: usize, metric: DistanceMetric, config: Self::Config) -> Result<Self> where Self: Sized`.
- **`IndexStats`** (public fields): `n_vectors: usize`, `memory_bytes: usize`, `disk_bytes: Option<usize>`, `index_type: &'static str`, `extra: Option<HashMap<String, String>>`; derives `Debug + Clone + Default + PartialEq + Eq`.
- **Features:** none (frozen empty).

Deliberate freeze decisions:
- The trait is **split** into `IndexCore` (object-safe, held as `Box<dyn IndexCore>`) + `Index` (typed construction). This split is part of the frozen contract.
- The trait is **synchronous** (no async trait/feature; recorded at v0.4.0).
- `IndexStats` is a plain struct with public fields, keeping struct-update ergonomics; `extra: Option<HashMap<..>>` so the common `stats()` allocates nothing. It is **not** `#[non_exhaustive]` — adding a field would be breaking, so new per-kind detail goes in `extra`, not new fields.
- The vector payload crosses the boundary as `Arc<[f32]>` (shared, no copy), not `&[f32]` or `Vec<f32>`.

---

## v0.6.0 -> v0.9.x -- Alpha / Beta -> RC (COMPRESSED -> 1.0)

- 0.6.x-0.7.x: integrate against real consumers; MINOR-compatible additions only.
- 0.8.x (beta): bug fixes; broader testing; final benchmarks.
- 0.9.x (rc): critical fixes + doc polish.

> **Compressed into 1.0 (anti-deferral rule — move + reason recorded here).**
> This RC band exists to soak the frozen API against live consumers. In the
> build order those consumers (`iqdb-flat`/`iqdb-hnsw`/`iqdb-ivf`) are built
> *after* this crate and do not yet exist as published crates, so there is
> nothing to soak against on a calendar. Its intent — proving the surface
> carries real graph/clustered/brute-force indexes — was already met at v0.3.0
> by `tests/consumer_simulation.rs`, cross-checked against the **actual** Cortex
> implementations (which implement `IndexCore` + `Index` verbatim, zero
> friction). There are no benchmarks because the crate has no hot path of its
> own (the default shims are O(n) dispatch wrappers). The crate therefore
> proceeds to 1.0 on a fully-satisfied Definition of Done rather than a calendar,
> mirroring the `iqdb-types` 0.5 -> 1.0 path. Any need a real consumer later
> surfaces ships as an additive `1.0.x` release.

---

## v1.0.0 -- Stable (DONE)

- [x] **Definition of Done (DIRECTIVES §7) satisfied:**
  - [x] Compiles clean on Linux/macOS/Windows, on stable + the 1.87 MSRV (CI matrix).
  - [x] `fmt`, `clippy -D warnings` (default + all-features), `test`, `cargo doc -D warnings` clean.
  - [x] `cargo audit` + `cargo deny check` clean.
  - [x] No `unwrap`/`expect`/`todo!`/`dbg!` in shipping code; **zero `unsafe`** (`#![forbid(unsafe_code)]`).
  - [x] A Tier-1 API exists and headlines the docs.
  - [x] Property tests cover every §8 invariant. **Loom: N/A** — the crate is trait definitions + one plain struct with no concurrent or lock-free path of its own; the `Send + Sync` bound is a compiler-checked requirement on implementers.
  - [x] **Benchmarks: N/A** — no hot path in this crate (documented); a consumer's search hot path is benched in that consumer's crate.
  - [x] Every public item documented with a runnable example; `docs/API.md`, README, version metadata current; 4 examples; 39 integration tests + 3 doctests.
  - [x] Changelog maintained.
- [x] Public API frozen until 2.0 (recorded under v0.5.0 above).
- [ ] Release note written; published to crates.io; tag pushed. _(Release note written; publish + tag are the maintainer's to do.)_

---

## Out of scope for 1.0

- Any concrete index -- those are separate crates.
- Async-only trait -- sync default with optional async wrapping.
