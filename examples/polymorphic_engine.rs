//! Hold many index kinds behind one trait object.
//!
//! The whole point of `IndexCore` is that an engine can store a heterogeneous
//! set of indexes as `Box<dyn IndexCore>` and operate them without knowing
//! which concrete strategy is behind each one. This example builds two indexes
//! configured differently, stores them as trait objects, and dispatches the
//! same operations across both.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example polymorphic_engine
//! ```

use std::sync::Arc;

use iqdb_index::{Index, IndexCore, IndexStats};
use iqdb_types::{DistanceMetric, Hit, IqdbError, Metadata, Result, SearchParams, VectorId};

/// A minimal brute-force index, enough to stand in for a real strategy.
struct MiniFlat {
    dim: usize,
    metric: DistanceMetric,
    rows: Vec<(VectorId, Arc<[f32]>)>,
}

#[derive(Default, Clone)]
struct MiniConfig;

impl IndexCore for MiniFlat {
    fn insert(
        &mut self,
        id: VectorId,
        vector: Arc<[f32]>,
        _metadata: Option<Metadata>,
    ) -> Result<()> {
        if vector.len() != self.dim {
            return Err(IqdbError::DimensionMismatch {
                expected: self.dim,
                found: vector.len(),
            });
        }
        self.rows.push((id, vector));
        Ok(())
    }

    fn delete(&mut self, id: &VectorId) -> Result<()> {
        match self.rows.iter().position(|(existing, _)| existing == id) {
            Some(pos) => {
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
        let mut hits: Vec<Hit> = self
            .rows
            .iter()
            .map(|(id, vector)| {
                let distance = query
                    .iter()
                    .zip(vector.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum();
                Hit {
                    id: id.clone(),
                    distance,
                    metadata: None,
                }
            })
            .collect();
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
        Ok(())
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            n_vectors: self.rows.len(),
            index_type: "mini-flat",
            ..IndexStats::default()
        }
    }
}

impl Index for MiniFlat {
    type Config = MiniConfig;

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

fn main() -> Result<()> {
    // Two indexes, different metrics — the kind of heterogeneity an engine
    // holds across shards or collections.
    let mut cosine = MiniFlat::new(2, DistanceMetric::Cosine, MiniConfig)?;
    let mut euclid = MiniFlat::new(2, DistanceMetric::Euclidean, MiniConfig)?;
    cosine.insert(VectorId::from(1u64), Arc::from([1.0, 0.0].as_slice()), None)?;
    euclid.insert(VectorId::from(2u64), Arc::from([0.0, 1.0].as_slice()), None)?;

    // Erase the concrete type: store both behind the object-safe trait.
    let engine: Vec<Box<dyn IndexCore>> = vec![Box::new(cosine), Box::new(euclid)];

    // Operate over the whole set without naming a concrete index type.
    let query = [0.5_f32, 0.5];
    for index in &engine {
        let params = SearchParams::new(1, index.metric());
        let hits = index.search(&query, &params)?;
        let best = hits.first().map(|h| h.id.clone());
        println!(
            "index type={:<10} metric={:?} len={} best={:?}",
            index.stats().index_type,
            index.metric(),
            index.len(),
            best,
        );
    }

    Ok(())
}
