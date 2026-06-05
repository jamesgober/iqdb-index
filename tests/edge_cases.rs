//! Edge-case coverage for the trait contract.
//!
//! Boundary conditions the happy-path and property suites do not pin down
//! explicitly: empty inputs, `k` larger than the index, exhaustive deletion,
//! re-insertion after deletion, and the `IndexStats` defaults. Driven through
//! the shared brute-force `MockIndex` (the reference implementation) so the
//! crate's own default shims are exercised at the boundaries.

#![allow(clippy::unwrap_used)]

mod common;

use common::{MockConfig, MockIndex, arc};
use iqdb_index::{Index, IndexCore, IndexStats};
use iqdb_types::{DistanceMetric, IqdbError, SearchParams, VectorId};

fn euclid(dim: usize) -> MockIndex {
    MockIndex::new(dim, DistanceMetric::Euclidean, MockConfig).unwrap()
}

#[test]
fn search_on_empty_index_returns_empty() {
    let idx = euclid(2);
    let hits = idx
        .search(
            &[0.0, 0.0],
            &SearchParams::new(5, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert!(hits.is_empty());
}

#[test]
fn search_k_greater_than_len_returns_all() {
    let mut idx = euclid(2);
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    idx.insert(VectorId::from(2u64), arc(&[1.0, 0.0]), None)
        .unwrap();
    let hits = idx
        .search(
            &[0.0, 0.0],
            &SearchParams::new(100, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn delete_all_then_index_is_empty() {
    let mut idx = euclid(2);
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    idx.insert(VectorId::from(2u64), arc(&[1.0, 0.0]), None)
        .unwrap();
    idx.delete(&VectorId::from(1u64)).unwrap();
    idx.delete(&VectorId::from(2u64)).unwrap();
    assert!(idx.is_empty());
    assert_eq!(idx.len(), 0);
    let hits = idx
        .search(
            &[0.0, 0.0],
            &SearchParams::new(5, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert!(hits.is_empty());
}

#[test]
fn reinsert_after_delete_succeeds_for_true_removal() {
    // The reference index does true removal, so a deleted id may be re-inserted.
    let mut idx = euclid(2);
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    idx.delete(&VectorId::from(1u64)).unwrap();
    idx.insert(VectorId::from(1u64), arc(&[9.0, 9.0]), None)
        .unwrap();
    assert_eq!(idx.len(), 1);
    let hits = idx
        .search(
            &[9.0, 9.0],
            &SearchParams::new(1, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert_eq!(hits[0].id, VectorId::U64(1));
}

#[test]
fn empty_insert_batch_is_ok_and_noop() {
    let mut idx = euclid(2);
    idx.insert_batch(Vec::new()).unwrap();
    assert!(idx.is_empty());
}

#[test]
fn empty_search_batch_returns_empty_outer_vec() {
    let idx = euclid(2);
    let queries: &[&[f32]] = &[];
    let results = idx
        .search_batch(queries, &SearchParams::new(3, DistanceMetric::Euclidean))
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_k_zero_on_populated_index_is_empty() {
    let mut idx = euclid(2);
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    let hits = idx
        .search(
            &[0.0, 0.0],
            &SearchParams::new(0, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert!(hits.is_empty());
}

#[test]
fn search_batch_fails_fast_on_bad_query() {
    let mut idx = euclid(2);
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    let good: &[f32] = &[0.0, 0.0];
    let bad: &[f32] = &[0.0, 0.0, 0.0]; // wrong dim
    let queries: &[&[f32]] = &[good, bad];
    let err = idx
        .search_batch(queries, &SearchParams::new(1, DistanceMetric::Euclidean))
        .unwrap_err();
    assert_eq!(
        err,
        IqdbError::DimensionMismatch {
            expected: 2,
            found: 3
        }
    );
}

#[test]
fn default_index_stats_field_values() {
    let stats = IndexStats::default();
    assert_eq!(stats.n_vectors, 0);
    assert_eq!(stats.memory_bytes, 0);
    assert_eq!(stats.disk_bytes, None);
    assert_eq!(stats.index_type, "");
    assert!(stats.extra.is_none());
}

#[test]
fn index_stats_equality_is_structural() {
    let a = IndexStats {
        n_vectors: 3,
        index_type: "flat",
        ..IndexStats::default()
    };
    let b = IndexStats {
        n_vectors: 3,
        index_type: "flat",
        ..IndexStats::default()
    };
    let c = IndexStats {
        n_vectors: 4,
        index_type: "flat",
        ..IndexStats::default()
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn delete_twice_returns_not_found_the_second_time() {
    let mut idx = euclid(2);
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    idx.delete(&VectorId::from(1u64)).unwrap();
    let err = idx.delete(&VectorId::from(1u64)).unwrap_err();
    assert_eq!(err, IqdbError::NotFound);
}

#[test]
fn bytes_vector_id_round_trips_through_search() {
    let mut idx = euclid(2);
    let id = VectorId::try_from(vec![0xde, 0xad, 0xbe, 0xef]).unwrap();
    idx.insert(id.clone(), arc(&[1.0, 2.0]), None).unwrap();
    let hits = idx
        .search(
            &[1.0, 2.0],
            &SearchParams::new(1, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert_eq!(hits[0].id, id);
}
