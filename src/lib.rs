//! # iqdb-index
//!
//! The index-trait layer for the HiveDB **iqdb** vector-database spine. It
//! defines the shape every concrete index (flat, HNSW, IVF, …) implements so
//! the engine can hold them polymorphically, and exposes [`IndexStats`] for
//! runtime introspection. Its only dependency is
//! [`iqdb-types`](iqdb_types) — the shared vocabulary.
//!
//! ## Why the trait is split
//!
//! [`Index`] carries an associated `type Config: Default + Clone` and a
//! `Self`-returning `fn new(...) -> Result<Self>`. That combination makes a
//! single trait non-object-safe — `Box<dyn Index>` would not compile. The
//! engine needs to hold a heterogeneous set of indexes, so the trait is
//! split:
//!
//! - [`IndexCore`] — the object-safe operational surface: `insert`,
//!   `delete`, `search`, `len`, `dim`, `metric`, `flush`, `stats` (plus the
//!   default batch shims). The engine stores `Box<dyn IndexCore>` through
//!   this trait.
//! - [`Index`] — adds the associated `Config` and `new`. Used where the
//!   concrete index type is known.
//!
//! Every concrete index implements **both**.
//!
//! ## Ordering contract on `Hit.distance`
//!
//! [`iqdb_types::Hit::distance`] is documented as **smaller is nearer**.
//! Four of the five metrics (Cosine, Euclidean, Manhattan, Hamming) already
//! satisfy that contract under [`iqdb_types::DistanceMetric`]. For
//! `DotProduct` the raw value is a similarity (larger is more similar), so
//! every index MUST negate it at the boundary — store `-dot` in
//! `Hit.distance` — to keep one ordering invariant across the index family.
//!
//! ## Example
//!
//! ```
//! use std::sync::Arc;
//!
//! use iqdb_index::{Index, IndexCore, IndexStats};
//! use iqdb_types::{
//!     DistanceMetric, Hit, IqdbError, Metadata, Result, SearchParams, VectorId,
//! };
//!
//! /// A toy in-memory index used only to document the trait shape.
//! struct Toy {
//!     dim: usize,
//!     metric: DistanceMetric,
//!     ids: Vec<VectorId>,
//! }
//!
//! #[derive(Default, Clone)]
//! struct ToyConfig;
//!
//! impl IndexCore for Toy {
//!     fn insert(&mut self, id: VectorId, _v: Arc<[f32]>, _m: Option<Metadata>) -> Result<()> {
//!         self.ids.push(id);
//!         Ok(())
//!     }
//!     fn delete(&mut self, id: &VectorId) -> Result<()> {
//!         match self.ids.iter().position(|x| x == id) {
//!             Some(pos) => {
//!                 let _removed = self.ids.remove(pos);
//!                 Ok(())
//!             }
//!             None => Err(IqdbError::NotFound),
//!         }
//!     }
//!     fn search(&self, _q: &[f32], _p: &SearchParams) -> Result<Vec<Hit>> {
//!         Ok(Vec::new())
//!     }
//!     fn len(&self) -> usize { self.ids.len() }
//!     fn dim(&self) -> usize { self.dim }
//!     fn metric(&self) -> DistanceMetric { self.metric }
//!     fn flush(&mut self) -> Result<()> { Ok(()) }
//!     fn stats(&self) -> IndexStats {
//!         IndexStats {
//!             n_vectors: self.ids.len(),
//!             index_type: "toy",
//!             ..IndexStats::default()
//!         }
//!     }
//! }
//!
//! impl Index for Toy {
//!     type Config = ToyConfig;
//!     fn new(dim: usize, metric: DistanceMetric, _c: Self::Config) -> Result<Self> {
//!         Ok(Toy { dim, metric, ids: Vec::new() })
//!     }
//! }
//!
//! # fn main() -> Result<()> {
//! let mut idx = Toy::new(3, DistanceMetric::Cosine, ToyConfig)?;
//! assert!(idx.is_empty());
//! idx.insert(VectorId::from(1u64), Arc::<[f32]>::from(&[1.0, 0.0, 0.0][..]), None)?;
//! assert_eq!(idx.len(), 1);
//! idx.delete(&VectorId::from(1u64))?;
//! assert!(idx.is_empty());
//! # Ok(())
//! # }
//! ```

#![deny(warnings)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unused_must_use)]
#![deny(unused_results)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::unreachable)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod index;
mod stats;

pub use crate::index::{Index, IndexCore};
pub use crate::stats::IndexStats;

/// The version of this crate, taken from `Cargo.toml` at compile time.
///
/// Exposed so a consumer can report the exact `iqdb-index` build it links
/// against — useful in diagnostics and version-skew checks across the iqdb
/// crate family.
///
/// # Examples
///
/// ```
/// // Carries a `major.minor.patch` SemVer core.
/// let version = iqdb_index::VERSION;
/// assert_eq!(version.split('.').count(), 3);
/// assert!(version.split('.').all(|part| !part.is_empty()));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
