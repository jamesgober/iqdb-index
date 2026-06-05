//! Implement a custom index by hand.
//!
//! Builds a minimal brute-force index that satisfies the full `iqdb-index`
//! contract — typed construction, insert, batch insert, top-`k` search,
//! deletion, and stats — then exercises it end to end. This is the Tier-3
//! surface: the seam a new index strategy plugs into.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example custom_index
//! ```

use std::sync::Arc;

use iqdb_index::{Index, IndexCore, IndexStats};
use iqdb_types::{DistanceMetric, Hit, IqdbError, Metadata, Result, SearchParams, VectorId};

/// A linear-scan index: correct, exact, and the baseline every approximate
/// index is measured against.
struct FlatIndex {
    dim: usize,
    metric: DistanceMetric,
    rows: Vec<(VectorId, Arc<[f32]>, Option<Metadata>)>,
}

/// Flat search has nothing to tune, so its config is empty.
#[derive(Default, Clone)]
struct FlatConfig;

impl FlatIndex {
    /// Squared Euclidean distance — monotonic in true distance, so it orders
    /// candidates identically while skipping a per-candidate `sqrt`.
    fn distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum()
    }
}

impl IndexCore for FlatIndex {
    fn insert(
        &mut self,
        id: VectorId,
        vector: Arc<[f32]>,
        metadata: Option<Metadata>,
    ) -> Result<()> {
        if vector.len() != self.dim {
            return Err(IqdbError::DimensionMismatch {
                expected: self.dim,
                found: vector.len(),
            });
        }
        if self.rows.iter().any(|(existing, _, _)| existing == &id) {
            return Err(IqdbError::Duplicate);
        }
        self.rows.push((id, vector, metadata));
        Ok(())
    }

    fn delete(&mut self, id: &VectorId) -> Result<()> {
        match self.rows.iter().position(|(existing, _, _)| existing == id) {
            Some(pos) => {
                // True deletion: the row is removed, so it cannot resurface.
                let _ = self.rows.remove(pos);
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
            .map(|(id, vector, metadata)| Hit {
                id: id.clone(),
                distance: Self::distance(query, vector),
                metadata: metadata.clone(),
            })
            .collect();
        // Ordering contract: smaller distance is nearer, best first.
        hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        hits.truncate(params.k);
        Ok(hits)
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn metric(&self) -> DistanceMetric {
        self.metric
    }

    fn flush(&mut self) -> Result<()> {
        Ok(()) // purely in-memory: nothing to persist
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            n_vectors: self.rows.len(),
            memory_bytes: self.rows.len() * self.dim * size_of::<f32>(),
            index_type: "flat",
            ..IndexStats::default()
        }
    }
}

impl Index for FlatIndex {
    type Config = FlatConfig;

    fn new(dim: usize, metric: DistanceMetric, _config: Self::Config) -> Result<Self> {
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
}

fn vec3(x: f32, y: f32, z: f32) -> Arc<[f32]> {
    Arc::from([x, y, z].as_slice())
}

fn main() -> Result<()> {
    // Tier 1: construct with a default config in one call.
    let mut index = FlatIndex::new(3, DistanceMetric::Euclidean, FlatConfig)?;

    // Batch insert uses the trait's default fail-fast shim.
    index.insert_batch(vec![
        (VectorId::from(1u64), vec3(1.0, 0.0, 0.0), None),
        (VectorId::from(2u64), vec3(0.0, 1.0, 0.0), None),
        (VectorId::from(3u64), vec3(0.9, 0.1, 0.0), None),
    ])?;
    println!("inserted {} vectors", index.len());

    // Top-2 search near the first vector.
    let params = SearchParams::new(2, DistanceMetric::Euclidean);
    let hits = index.search(&[1.0, 0.0, 0.0], &params)?;
    println!("nearest two to [1, 0, 0]:");
    for hit in &hits {
        println!("  id={} distance={:.3}", hit.id, hit.distance);
    }

    // Delete id 1 and confirm it no longer appears.
    index.delete(&VectorId::from(1u64))?;
    let after = index.search(
        &[1.0, 0.0, 0.0],
        &SearchParams::new(3, DistanceMetric::Euclidean),
    )?;
    println!("after deleting id 1, nearest is id={}", after[0].id);

    // Introspect.
    let stats = index.stats();
    println!(
        "stats: type={} n_vectors={} memory_bytes={}",
        stats.index_type, stats.n_vectors, stats.memory_bytes
    );

    Ok(())
}
