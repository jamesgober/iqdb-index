//! Batch operations and `IndexStats` introspection.
//!
//! Two facets of the trait that the other examples only touch:
//!
//! - the default `insert_batch` (fail-fast) and `search_batch`
//!   (order-preserving) shims — you get them for free without overriding,
//! - and `IndexStats`, including the open-ended `extra` map an index uses to
//!   report per-kind counters (here, a tombstone count) without changing the
//!   trait.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example batch_and_stats
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use iqdb_index::{Index, IndexCore, IndexStats};
use iqdb_types::{DistanceMetric, Hit, IqdbError, Metadata, Result, SearchParams, VectorId};

/// A tombstoning index — like a graph index, `delete` marks rather than
/// removes, so it has a per-kind counter to report through `IndexStats::extra`.
struct TombstoneIndex {
    dim: usize,
    metric: DistanceMetric,
    rows: Vec<(VectorId, Arc<[f32]>, bool)>, // (id, vector, tombstoned)
}

#[derive(Default, Clone)]
struct TombstoneConfig;

impl TombstoneIndex {
    fn tombstone_count(&self) -> usize {
        self.rows.iter().filter(|(_, _, dead)| *dead).count()
    }
}

impl IndexCore for TombstoneIndex {
    fn insert(&mut self, id: VectorId, vector: Arc<[f32]>, _m: Option<Metadata>) -> Result<()> {
        if vector.len() != self.dim {
            return Err(IqdbError::DimensionMismatch {
                expected: self.dim,
                found: vector.len(),
            });
        }
        if self.rows.iter().any(|(e, _, dead)| e == &id && !*dead) {
            return Err(IqdbError::Duplicate);
        }
        self.rows.push((id, vector, false));
        Ok(())
    }

    fn delete(&mut self, id: &VectorId) -> Result<()> {
        match self.rows.iter_mut().find(|(e, _, dead)| e == id && !*dead) {
            Some(row) => {
                row.2 = true; // mark, do not remove
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
            .filter(|(_, _, dead)| !*dead) // tombstones are never returned
            .map(|(id, v, _)| Hit {
                id: id.clone(),
                distance: query
                    .iter()
                    .zip(v.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum(),
                metadata: None,
            })
            .collect();
        hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        hits.truncate(params.k);
        Ok(hits)
    }

    fn len(&self) -> usize {
        // Live rows only — tombstones are not searchable.
        self.rows.iter().filter(|(_, _, dead)| !*dead).count()
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
        let mut extra = HashMap::new();
        let _ = extra.insert("tombstones".to_string(), self.tombstone_count().to_string());
        IndexStats {
            n_vectors: self.len(),
            memory_bytes: self.rows.len() * self.dim * size_of::<f32>(), // includes tombstones
            disk_bytes: None,
            index_type: "tombstone",
            extra: Some(extra),
        }
    }
}

impl Index for TombstoneIndex {
    type Config = TombstoneConfig;
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

fn vec2(x: f32, y: f32) -> Arc<[f32]> {
    Arc::from([x, y].as_slice())
}

fn main() -> Result<()> {
    let mut index = TombstoneIndex::new(2, DistanceMetric::Euclidean, TombstoneConfig)?;

    // Default `insert_batch`: one call, fail-fast on the first bad item.
    index.insert_batch(vec![
        (VectorId::from(1u64), vec2(0.0, 0.0), None),
        (VectorId::from(2u64), vec2(1.0, 0.0), None),
        (VectorId::from(3u64), vec2(5.0, 0.0), None),
        (VectorId::from(4u64), vec2(9.0, 0.0), None),
    ])?;
    println!("inserted batch: len={}", index.len());

    // Default `search_batch`: shared params, results in input order.
    let q1: &[f32] = &[0.0, 0.0];
    let q2: &[f32] = &[9.0, 0.0];
    let results =
        index.search_batch(&[q1, q2], &SearchParams::new(1, DistanceMetric::Euclidean))?;
    println!(
        "search_batch nearest: q1->{} q2->{}",
        results[0][0].id, results[1][0].id
    );

    // Tombstone one id, then introspect.
    index.delete(&VectorId::from(3u64))?;
    let stats = index.stats();
    println!(
        "stats: type={} live={} tombstones={} memory_bytes={} disk_bytes={:?}",
        stats.index_type,
        stats.n_vectors,
        stats
            .extra
            .as_ref()
            .and_then(|m| m.get("tombstones"))
            .map(String::as_str)
            .unwrap_or("0"),
        stats.memory_bytes,
        stats.disk_bytes,
    );

    // The tombstoned id is gone from search but still occupies memory.
    assert_eq!(stats.n_vectors, 3);
    assert_eq!(
        stats
            .extra
            .as_ref()
            .and_then(|m| m.get("tombstones"))
            .map(String::as_str),
        Some("1"),
    );
    let after = index.search(q2, &SearchParams::new(5, DistanceMetric::Euclidean))?;
    assert!(after.iter().all(|h| h.id != VectorId::U64(3)));
    println!("tombstoned id 3 no longer appears in search.");

    Ok(())
}
