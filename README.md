<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>iqdb-index</b>
    <br>
    <sub><sup>iQDB INDEX TRAIT</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/iqdb-index"><img alt="Crates.io" src="https://img.shields.io/crates/v/iqdb-index"></a>
    <a href="https://crates.io/crates/iqdb-index"><img alt="Downloads" src="https://img.shields.io/crates/d/iqdb-index?color=%230099ff"></a>
    <a href="https://docs.rs/iqdb-index"><img alt="docs.rs" src="https://img.shields.io/docsrs/iqdb-index"></a>
    <a href="https://github.com/jamesgober/iqdb-index/actions"><img alt="CI" src="https://github.com/jamesgober/iqdb-index/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.87%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        <strong>iqdb-index</strong> is the polymorphism layer of the database. The database calls `index.search(query, params)` and the right implementation runs, so iQDB supports plug-and-play index strategies instead of being hardcoded to one.
    </p>
    <p>
        It is almost entirely trait definitions plus a small amount of shared utility, and it depends only on `iqdb-types`.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.87+</strong> (Rust 2024 edition). One contract, many indexes &mdash; an object-safe operational trait plus typed construction, with per-index config via associated types.
    </p>
    <blockquote>
        <strong>Status: stable (1.0).</strong> The public API is committed under SemVer for the 1.x series — no breaking changes until 2.0. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

<h2>What it does</h2>

- **`IndexCore` trait** &mdash; the object-safe operational surface (`insert`, `delete`, `search`, `len`, `dim`, `metric`, `flush`, `stats`). The engine holds indexes as `Box<dyn IndexCore>`.
- **`Index` trait** &mdash; typed construction: an associated `Config` and a `new(dim, metric, config)`. Split out from `IndexCore` because a `Self`-returning constructor is not object-safe.
- **Associated `Config`** &mdash; each index exposes its own parameter struct, not a god-config enum.
- **Default batch shims** &mdash; `insert_batch` / `search_batch` ship for free and are overridable when a backend has a vectorized fast path.
- **`IndexStats`** &mdash; uniform, allocation-light introspection across index types.
- **Documented contracts** &mdash; best-first ordering (with the `DotProduct` negation rule), deletion visibility, and the `Send + Sync` concurrency model live on the trait, so every backend agrees.

<br>

## Installation

```toml
[dependencies]
iqdb-index = "1.0"
```

<br>

## Quick Start

Implement the two traits for your index, then construct, insert, and search
through one uniform surface. The full runnable version is in
[`examples/custom_index.rs`](./examples/custom_index.rs).

```rust
use std::sync::Arc;
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, SearchParams, VectorId};

// `FlatIndex` implements `IndexCore` + `Index` (see the example).
let mut index = FlatIndex::new(3, DistanceMetric::Euclidean, FlatConfig)?;

index.insert_batch(vec![
    (VectorId::from(1u64), Arc::from([1.0, 0.0, 0.0].as_slice()), None),
    (VectorId::from(2u64), Arc::from([0.0, 1.0, 0.0].as_slice()), None),
])?;

let hits = index.search(&[1.0, 0.0, 0.0], &SearchParams::new(1, DistanceMetric::Euclidean))?;
assert_eq!(hits[0].id, VectorId::U64(1)); // best-first

// Hold any backend behind the object-safe trait:
let engine: Vec<Box<dyn IndexCore>> = vec![Box::new(index)];
assert_eq!(engine[0].len(), 2);
```

### The three tiers

| Tier | Surface | When |
|---|---|---|
| **Tier 1** | `Index::new` + the `IndexCore` operations | Build an index and use it. |
| **Tier 2** | `Index::Config` | Tune a specific backend. |
| **Tier 3** | implement `IndexCore` + `Index` | Add a new index strategy. |

<br>

## Examples

Runnable, each covering a distinct facet of the surface (`cargo run --example <name>`):

| Example | Shows |
|---|---|
| [`custom_index`](./examples/custom_index.rs) | Implement `IndexCore` + `Index` by hand — the Tier-3 seam. |
| [`polymorphic_engine`](./examples/polymorphic_engine.rs) | Hold several index kinds as `Box<dyn IndexCore>` and dispatch over them. |
| [`dot_product_ordering`](./examples/dot_product_ordering.rs) | The `DotProduct` negation contract — store `-dot` so "most similar" sorts first. |
| [`batch_and_stats`](./examples/batch_and_stats.rs) | The default `insert_batch` / `search_batch` shims and `IndexStats::extra`. |

<br>

## Status

<code>v1.0.0</code> &mdash; **stable.** The public API is committed under SemVer for the 1.x series (no breaking changes until 2.0; the frozen surface is recorded in the <a href="./dev/ROADMAP.md"><code>ROADMAP</code></a>). `IndexCore`, `Index`, `IndexStats`, and the default batch shims are property-tested (best-first ordering, deletion visibility, batch&nbsp;==&nbsp;loop) and documented with four runnable examples and a complete <a href="./docs/API.md"><code>API reference</code></a>. The surface is validated against the live `iqdb-flat` (brute-force, true removal), `iqdb-hnsw` (graph, tombstone), and `iqdb-ivf` (clustered, true removal) implementations — each implements the traits verbatim with its own `Config` — and a consumer-simulation proves all three coexist behind `Box<dyn IndexCore>` (DIRECTIVES §8). Synchronous by design; empty, frozen feature set; `cargo audit` + `cargo deny` clean; verified on Windows + Linux across stable and the 1.87 MSRV.

<hr>
<br>

## Where It Fits

`iqdb-index` is the interface every index speaks. It is implemented by:

- `iqdb-types` &mdash; the only dependency
- `iqdb-flat` / `iqdb-hnsw` / `iqdb-ivf` &mdash; implement this trait
- `iqdb-build` / `iqdb-eval` / `iqdb` &mdash; are generic over it

Designing this trait so flat, HNSW, and IVF all fit cleanly is the crate's whole job.

<br>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>JAMES GOBER.</strong></sup>
</div>
