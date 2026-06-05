//! Consumer simulation: the trait fits all three index families.
//!
//! DIRECTIVES §8 requires that `IndexCore` / `Index` be implementable by
//! **graph, clustered, and brute-force** indexes "without awkward
//! abstractions". This suite encodes a stand-in for each real consumer at the
//! *exact* shape it uses — cross-checked against the implementations in
//! `iqdb-flat` (`FlatIndex` / `FlatConfig`), `iqdb-hnsw` (`HnswIndex` /
//! `HnswConfig { m, ef_construction }`), and `iqdb-ivf` (`IvfIndex` /
//! `IvfConfig { n_clusters, n_probes }`):
//!
//! - each carries its **own** associated `Config` (not a shared god-config),
//! - each constructs through `Index::new(dim, metric, config)`,
//! - each reports its own `index_type` and `extra: None` from `stats`,
//! - each overrides `is_empty`,
//! - and all three coexist behind `Box<dyn IndexCore>`.
//!
//! The point is not the search algorithm (every stand-in scans linearly) — it
//! is that the trait surface is sufficient and ergonomic for indexes whose
//! construction parameters differ wildly. If this compiles and passes, the
//! v0.2 surface needs no change to carry the real consumers.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use iqdb_index::{Index, IndexCore, IndexStats};
use iqdb_types::{DistanceMetric, Hit, IqdbError, Metadata, Result, SearchParams, VectorId};

/// Shared linear-scan core the three stand-ins delegate to, so each family's
/// `impl` only has to express its construction shape and `index_type`.
struct Scan {
    dim: usize,
    metric: DistanceMetric,
    rows: Vec<(VectorId, Arc<[f32]>, Option<Metadata>)>,
}

impl Scan {
    fn new(dim: usize, metric: DistanceMetric) -> Result<Self> {
        if dim == 0 {
            return Err(IqdbError::InvalidConfig {
                reason: "dim must be greater than zero",
            });
        }
        Ok(Self {
            dim,
            metric,
            rows: Vec::new(),
        })
    }

    fn insert(&mut self, id: VectorId, v: Arc<[f32]>, m: Option<Metadata>) -> Result<()> {
        if v.len() != self.dim {
            return Err(IqdbError::DimensionMismatch {
                expected: self.dim,
                found: v.len(),
            });
        }
        if self.rows.iter().any(|(e, _, _)| e == &id) {
            return Err(IqdbError::Duplicate);
        }
        self.rows.push((id, v, m));
        Ok(())
    }

    fn delete(&mut self, id: &VectorId) -> Result<()> {
        match self.rows.iter().position(|(e, _, _)| e == id) {
            Some(p) => {
                let _ = self.rows.remove(p);
                Ok(())
            }
            None => Err(IqdbError::NotFound),
        }
    }

    fn search(&self, query: &[f32], params: &SearchParams) -> Result<Vec<Hit>> {
        if query.len() != self.dim {
            return Err(IqdbError::DimensionMismatch {
                expected: self.dim,
                found: query.len(),
            });
        }
        if params.metric != self.metric {
            return Err(IqdbError::InvalidMetric);
        }
        let mut hits: Vec<Hit> = self
            .rows
            .iter()
            .map(|(id, v, m)| Hit {
                id: id.clone(),
                distance: query
                    .iter()
                    .zip(v.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum(),
                metadata: m.clone(),
            })
            .collect();
        hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        hits.truncate(params.k);
        Ok(hits)
    }
}

// --- Brute-force family (mirrors iqdb-flat: FlatIndex / FlatConfig) ----------

#[derive(Default, Clone)]
struct FlatConfig;

struct FlatLike(Scan);

impl IndexCore for FlatLike {
    fn insert(&mut self, id: VectorId, v: Arc<[f32]>, m: Option<Metadata>) -> Result<()> {
        self.0.insert(id, v, m)
    }
    fn delete(&mut self, id: &VectorId) -> Result<()> {
        self.0.delete(id)
    }
    fn search(&self, q: &[f32], p: &SearchParams) -> Result<Vec<Hit>> {
        self.0.search(q, p)
    }
    fn len(&self) -> usize {
        self.0.rows.len()
    }
    fn is_empty(&self) -> bool {
        self.0.rows.is_empty()
    }
    fn dim(&self) -> usize {
        self.0.dim
    }
    fn metric(&self) -> DistanceMetric {
        self.0.metric
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
    fn stats(&self) -> IndexStats {
        IndexStats {
            n_vectors: self.0.rows.len(),
            index_type: "flat",
            ..IndexStats::default()
        }
    }
}

impl Index for FlatLike {
    type Config = FlatConfig;
    fn new(dim: usize, metric: DistanceMetric, _config: Self::Config) -> Result<Self> {
        Ok(Self(Scan::new(dim, metric)?))
    }
}

// --- Graph family (mirrors iqdb-hnsw: HnswIndex / HnswConfig) ----------------

#[derive(Clone)]
struct HnswConfig {
    m: usize,
    ef_construction: usize,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
        }
    }
}

struct HnswLike {
    scan: Scan,
    m: usize,
}

impl IndexCore for HnswLike {
    fn insert(&mut self, id: VectorId, v: Arc<[f32]>, m: Option<Metadata>) -> Result<()> {
        self.scan.insert(id, v, m)
    }
    fn delete(&mut self, id: &VectorId) -> Result<()> {
        self.scan.delete(id)
    }
    fn search(&self, q: &[f32], p: &SearchParams) -> Result<Vec<Hit>> {
        self.scan.search(q, p)
    }
    fn len(&self) -> usize {
        self.scan.rows.len()
    }
    fn is_empty(&self) -> bool {
        self.scan.rows.is_empty()
    }
    fn dim(&self) -> usize {
        self.scan.dim
    }
    fn metric(&self) -> DistanceMetric {
        self.scan.metric
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
    fn stats(&self) -> IndexStats {
        IndexStats {
            n_vectors: self.scan.rows.len(),
            index_type: "hnsw",
            ..IndexStats::default()
        }
    }
}

impl Index for HnswLike {
    type Config = HnswConfig;
    fn new(dim: usize, metric: DistanceMetric, config: Self::Config) -> Result<Self> {
        // A graph index validates its construction knobs up front.
        if config.m == 0 || config.ef_construction == 0 {
            return Err(IqdbError::InvalidConfig {
                reason: "HNSW m and ef_construction must be greater than zero",
            });
        }
        Ok(Self {
            scan: Scan::new(dim, metric)?,
            m: config.m,
        })
    }
}

// --- Clustered family (mirrors iqdb-ivf: IvfIndex / IvfConfig) ---------------

#[derive(Clone)]
struct IvfConfig {
    n_clusters: usize,
    n_probes: usize,
}

impl Default for IvfConfig {
    fn default() -> Self {
        Self {
            n_clusters: 100,
            n_probes: 8,
        }
    }
}

struct IvfLike {
    scan: Scan,
    n_clusters: usize,
}

impl IndexCore for IvfLike {
    fn insert(&mut self, id: VectorId, v: Arc<[f32]>, m: Option<Metadata>) -> Result<()> {
        self.scan.insert(id, v, m)
    }
    fn delete(&mut self, id: &VectorId) -> Result<()> {
        self.scan.delete(id)
    }
    fn search(&self, q: &[f32], p: &SearchParams) -> Result<Vec<Hit>> {
        self.scan.search(q, p)
    }
    fn len(&self) -> usize {
        self.scan.rows.len()
    }
    fn is_empty(&self) -> bool {
        self.scan.rows.is_empty()
    }
    fn dim(&self) -> usize {
        self.scan.dim
    }
    fn metric(&self) -> DistanceMetric {
        self.scan.metric
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
    fn stats(&self) -> IndexStats {
        IndexStats {
            n_vectors: self.scan.rows.len(),
            index_type: "ivf",
            ..IndexStats::default()
        }
    }
}

impl Index for IvfLike {
    type Config = IvfConfig;
    fn new(dim: usize, metric: DistanceMetric, config: Self::Config) -> Result<Self> {
        if config.n_clusters == 0 || config.n_probes == 0 {
            return Err(IqdbError::InvalidConfig {
                reason: "IVF n_clusters and n_probes must be greater than zero",
            });
        }
        Ok(Self {
            scan: Scan::new(dim, metric)?,
            n_clusters: config.n_clusters,
        })
    }
}

fn vec2(x: f32, y: f32) -> Arc<[f32]> {
    Arc::from([x, y].as_slice())
}

#[test]
fn each_family_constructs_with_its_own_config() {
    // Brute-force: empty config.
    let flat = FlatLike::new(2, DistanceMetric::Euclidean, FlatConfig).unwrap();
    assert_eq!(flat.stats().index_type, "flat");

    // Graph: degree + build breadth.
    let hnsw = HnswLike::new(
        2,
        DistanceMetric::Cosine,
        HnswConfig {
            m: 32,
            ef_construction: 128,
        },
    )
    .unwrap();
    assert_eq!(hnsw.stats().index_type, "hnsw");
    assert_eq!(hnsw.m, 32); // the config knob reached the index

    // Clustered: cluster count + probe count.
    let ivf = IvfLike::new(
        2,
        DistanceMetric::Euclidean,
        IvfConfig {
            n_clusters: 64,
            n_probes: 4,
        },
    )
    .unwrap();
    assert_eq!(ivf.stats().index_type, "ivf");
    assert_eq!(ivf.n_clusters, 64);
}

#[test]
fn each_family_rejects_its_own_invalid_config() {
    assert!(matches!(
        HnswLike::new(
            2,
            DistanceMetric::Cosine,
            HnswConfig {
                m: 0,
                ef_construction: 1
            }
        ),
        Err(IqdbError::InvalidConfig { .. })
    ));
    assert!(matches!(
        IvfLike::new(
            2,
            DistanceMetric::Euclidean,
            IvfConfig {
                n_clusters: 0,
                n_probes: 1
            }
        ),
        Err(IqdbError::InvalidConfig { .. })
    ));
    // The shared dim check still applies to every family.
    assert!(matches!(
        FlatLike::new(0, DistanceMetric::Euclidean, FlatConfig),
        Err(IqdbError::InvalidConfig { .. })
    ));
}

#[test]
fn all_three_families_coexist_behind_one_trait_object() {
    // The engine's real shape: a heterogeneous set, type-erased.
    let mut engine: Vec<Box<dyn IndexCore>> = vec![
        Box::new(FlatLike::new(2, DistanceMetric::Euclidean, FlatConfig).unwrap()),
        Box::new(HnswLike::new(2, DistanceMetric::Euclidean, HnswConfig::default()).unwrap()),
        Box::new(IvfLike::new(2, DistanceMetric::Euclidean, IvfConfig::default()).unwrap()),
    ];

    // Drive the same operations across all of them, type unknown.
    for index in &mut engine {
        index
            .insert_batch(vec![
                (VectorId::from(1u64), vec2(0.0, 0.0), None),
                (VectorId::from(2u64), vec2(9.0, 9.0), None),
            ])
            .unwrap();
        index.delete(&VectorId::from(2u64)).unwrap();
        let hits = index
            .search(
                &[0.0, 0.0],
                &SearchParams::new(5, DistanceMetric::Euclidean),
            )
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, VectorId::U64(1));
        index.flush().unwrap();
    }

    // Each still reports its own identity through the erased trait.
    let kinds: Vec<&str> = engine.iter().map(|i| i.stats().index_type).collect();
    assert_eq!(kinds, ["flat", "hnsw", "ivf"]);
}
