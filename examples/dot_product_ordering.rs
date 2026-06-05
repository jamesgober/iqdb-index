//! The `DotProduct` ordering contract — negate at the boundary.
//!
//! `Hit.distance` is **smaller-is-nearer**, and `search` returns hits
//! best-first. Cosine / Euclidean / Manhattan / Hamming already satisfy that.
//! But the dot product is a *similarity*: a larger value means *more* similar.
//! An index using [`DistanceMetric::DotProduct`] must therefore store `-dot` in
//! `Hit.distance`, so that "most similar" sorts to the front like every other
//! metric. This example shows the negation and proves the ordering is correct.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example dot_product_ordering
//! ```

use std::sync::Arc;

use iqdb_index::{Index, IndexCore, IndexStats};
use iqdb_types::{DistanceMetric, Hit, IqdbError, Metadata, Result, SearchParams, VectorId};

/// A dot-product index that honours the ordering contract.
struct DotIndex {
    dim: usize,
    rows: Vec<(VectorId, Arc<[f32]>)>,
}

#[derive(Default, Clone)]
struct DotConfig;

impl IndexCore for DotIndex {
    fn insert(&mut self, id: VectorId, vector: Arc<[f32]>, _m: Option<Metadata>) -> Result<()> {
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
        match self.rows.iter().position(|(e, _)| e == id) {
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
        if params.metric != DistanceMetric::DotProduct {
            return Err(IqdbError::InvalidMetric);
        }
        let mut hits: Vec<Hit> = self
            .rows
            .iter()
            .map(|(id, v)| {
                let dot: f32 = query.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
                // The contract: store the NEGATED similarity so that the most
                // similar vector (largest dot) has the smallest `distance`.
                Hit {
                    id: id.clone(),
                    distance: -dot,
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
        DistanceMetric::DotProduct
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            n_vectors: self.rows.len(),
            index_type: "dot",
            ..IndexStats::default()
        }
    }
}

impl Index for DotIndex {
    type Config = DotConfig;
    fn new(dim: usize, metric: DistanceMetric, _config: Self::Config) -> Result<Self> {
        if metric != DistanceMetric::DotProduct {
            return Err(IqdbError::InvalidConfig {
                reason: "DotIndex only supports DistanceMetric::DotProduct",
            });
        }
        if dim == 0 {
            return Err(IqdbError::InvalidConfig {
                reason: "dim must be greater than zero",
            });
        }
        Ok(Self {
            dim,
            rows: Vec::new(),
        })
    }
}

fn main() -> Result<()> {
    let mut index = DotIndex::new(2, DistanceMetric::DotProduct, DotConfig)?;

    // Query is [1, 0]. Raw dot products: a→0.9, b→0.2, c→0.5.
    // Most similar (largest dot) is `a`, then `c`, then `b`.
    index.insert(VectorId::from(1u64), Arc::from([0.9, 0.4].as_slice()), None)?; // a
    index.insert(VectorId::from(2u64), Arc::from([0.2, 0.9].as_slice()), None)?; // b
    index.insert(VectorId::from(3u64), Arc::from([0.5, 0.5].as_slice()), None)?; // c

    let hits = index.search(
        &[1.0, 0.0],
        &SearchParams::new(3, DistanceMetric::DotProduct),
    )?;

    println!("query [1, 0] under DotProduct (best-first):");
    for (rank, hit) in hits.iter().enumerate() {
        // `distance` is the negated similarity; flip it back for display.
        println!(
            "  #{rank}: id={} similarity={:.2} (stored distance={:.2})",
            hit.id, -hit.distance, hit.distance
        );
    }

    // Most similar first, despite "distance" being the ranking key.
    assert_eq!(hits[0].id, VectorId::U64(1)); // a, dot 0.9
    assert_eq!(hits[1].id, VectorId::U64(3)); // c, dot 0.5
    assert_eq!(hits[2].id, VectorId::U64(2)); // b, dot 0.2
    // Distances are non-decreasing — the universal best-first invariant holds.
    assert!(hits[0].distance <= hits[1].distance && hits[1].distance <= hits[2].distance);
    println!("ordering contract holds: most similar first.");

    Ok(())
}
