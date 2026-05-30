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
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
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
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition). One trait. Many indexes. Per-index config via associated types.
    </p>
    <blockquote>
        <strong>Status: pre-1.0, in active development.</strong> The public API is being designed across the 0.x series and frozen at <code>1.0.0</code>. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

<h2>What it does</h2>

- **`Index` trait** &mdash; the common interface every index implements
- **Associated `Config`** &mdash; each index exposes its own parameter struct, not a god-config enum
- **Full operation set** &mdash; insert/insert_batch, delete, search/search_batch, flush, stats
- **`IndexStats`** &mdash; uniform introspection across index types
- **Stable by design** &mdash; interface code; breaking changes cascade, so it freezes carefully


<br>

## Installation

```toml
[dependencies]
iqdb-index = "0.1"
```

<br>

## Status

This is the <code>v0.1.0</code> scaffold: structure, tooling, and quality gates are in place; the implementation lands across the 0.x series per the <a href="./dev/ROADMAP.md"><code>ROADMAP</code></a> and <a href="./docs/API.md"><code>docs/API.md</code></a>.

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
