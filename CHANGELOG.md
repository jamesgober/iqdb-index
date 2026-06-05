<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>iqdb-index</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [0.4.0] - 2026-06-05

The async decision and the feature freeze. No code or public-surface changes — this milestone records two design commitments ahead of the API freeze.

### Added

- Documented the **synchronous-by-design** decision in the crate docs and `docs/API.md`: the trait stays sync (no async trait, no `async` feature, no `futures` dependency) because (1) an `async fn` would break `IndexCore` object safety without per-call future boxing on the search hot path, (2) search is CPU-bound, and (3) async belongs at the engine boundary (`spawn_blocking`), not the index. Async is optional consumer-side wrapping.

### Changed

- Declared the **feature set frozen** at empty: no `std`/`no_std` split, no `serde` gate, no `async` gate. Any 1.x feature would be purely additive.

---

## [0.3.0] - 2026-06-05

Trait validated against the real index families. No API changes — the v0.2.0 surface proved sufficient for graph, clustered, and brute-force indexes alike.

### Added

- `tests/consumer_simulation.rs` — a stand-in for each real consumer at its exact construction shape (`iqdb-flat` `FlatConfig`, `iqdb-hnsw` `HnswConfig { m, ef_construction }`, `iqdb-ivf` `IvfConfig { n_clusters, n_probes }`), proving DIRECTIVES §8: the trait is implementable by graph, clustered, and brute-force indexes without awkward abstractions, and all three coexist behind `Box<dyn IndexCore>`.

### Changed

- Documented the validation outcome in the README and `docs/API.md`: cross-checked against the live `iqdb-flat` / `iqdb-hnsw` / `iqdb-ivf` implementations (each implements `IndexCore` + `Index` verbatim, carries its own `Config`, reports `extra: None`, and overrides `is_empty`); no trait change was required.

---

## [0.2.0] - 2026-06-05

The load-bearing trait surface. Turns the scaffold into the contract every iQDB index implements.

### Added

- `IndexCore` &mdash; the object-safe operational trait (`insert`, `delete`, `search`, `len`, `dim`, `metric`, `flush`, `stats`) with `Send + Sync` bound, held by the engine as `Box<dyn IndexCore>`.
- `Index` &mdash; the typed-construction sibling trait: associated `Config: Default + Clone` and `new(dim, metric, config) -> Result<Self>`, deliberately split out so `IndexCore` stays object-safe.
- Default `insert_batch` (fail-fast) and `search_batch` (order-preserving) shims, plus a default `is_empty`, all overridable.
- `IndexStats` &mdash; runtime introspection snapshot (`n_vectors`, `memory_bytes`, `disk_bytes`, `index_type`, optional `extra` map) with an allocation-free `None` default for `extra`.
- Documented trait contracts: best-first ordering (with the `DotProduct` negation rule), deletion visibility, and the single-writer-internal concurrency model.
- Property-test suite (`proptest`) covering ordering, deletion visibility, batch/loop equivalence, and cardinality, plus trait-shape and `IndexStats` integration tests.
- Runnable examples: `custom_index` (implement the traits by hand) and `polymorphic_engine` (`Box<dyn IndexCore>` dispatch).
- `iqdb-types` `1.0.0` as the crate's single dependency.
- Complete `docs/API.md` reference and an expanded `README.md` quick start.

### Changed

- `Cargo.toml`: added co-author, bumped to `0.2.0`, dropped the unused `std`/`serde` scaffold features (the crate is std-only with no optional surface).

---

## [0.1.0] - 2026-05-30

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.87.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).
[Unreleased]: https://github.com/jamesgober/iqdb-index/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/jamesgober/iqdb-index/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/jamesgober/iqdb-index/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/jamesgober/iqdb-index/releases/tag/v0.2.0
