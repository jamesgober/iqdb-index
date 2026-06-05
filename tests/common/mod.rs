//! Shared test fixture: a naive brute-force `MockIndex`.
//!
//! A single in-test index that implements both `IndexCore` and `Index` with a
//! linear scan. It is the smallest faithful implementation of the trait
//! contract — exact search, true (non-tombstone) deletion, fail-fast batch
//! inserts — so the trait-shape and property suites exercise the real default
//! shims against a known-correct backend.

#![allow(dead_code)]
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::Arc;

use iqdb_index::{Index, IndexCore, IndexStats};
use iqdb_types::{DistanceMetric, Hit, IqdbError, Metadata, Result, SearchParams, VectorId};

/// Zero-sized configuration for [`MockIndex`].
#[derive(Default, Clone)]
pub struct MockConfig;

/// A brute-force index used only by the test suites.
pub struct MockIndex {
    dim: usize,
    metric: DistanceMetric,
    entries: Vec<(VectorId, Arc<[f32]>, Option<Metadata>)>,
}

impl MockIndex {
    /// Squared Euclidean distance — monotonic in the true distance, so it
    /// orders results identically while avoiding a `sqrt` per candidate.
    fn squared_distance(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
    }
}

/// Wrap a slice literal in the `Arc<[f32]>` the trait takes at the insert
/// boundary, mirroring how the engine shares a vector payload.
pub fn arc(v: &[f32]) -> Arc<[f32]> {
    Arc::from(v)
}

impl IndexCore for MockIndex {
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
        if self.entries.iter().any(|(existing, _, _)| existing == &id) {
            return Err(IqdbError::Duplicate);
        }
        self.entries.push((id, vector, metadata));
        Ok(())
    }

    fn delete(&mut self, id: &VectorId) -> Result<()> {
        match self
            .entries
            .iter()
            .position(|(existing, _, _)| existing == id)
        {
            Some(pos) => {
                let _removed = self.entries.remove(pos);
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
        let mut scored: Vec<Hit> = self
            .entries
            .iter()
            .map(|(id, vector, metadata)| Hit {
                id: id.clone(),
                distance: Self::squared_distance(query, vector),
                metadata: metadata.clone(),
            })
            .collect();
        scored.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        scored.truncate(params.k);
        Ok(scored)
    }

    fn len(&self) -> usize {
        self.entries.len()
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
        let _previous = extra.insert("backend".to_string(), "mock".to_string());
        IndexStats {
            n_vectors: self.entries.len(),
            memory_bytes: self.entries.len() * self.dim * size_of::<f32>(),
            disk_bytes: None,
            index_type: "mock",
            extra: Some(extra),
        }
    }
}

impl Index for MockIndex {
    type Config = MockConfig;

    fn new(dim: usize, metric: DistanceMetric, _config: Self::Config) -> Result<Self> {
        if dim == 0 {
            return Err(IqdbError::InvalidConfig {
                reason: "MockIndex dim must be greater than zero",
            });
        }
        Ok(Self {
            dim,
            metric,
            entries: Vec::new(),
        })
    }
}
