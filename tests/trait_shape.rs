//! Trait-shape coverage for `iqdb-index`.
//!
//! Exercises every required method of the shared `MockIndex`, the three
//! default-method shims (`insert_batch`, `search_batch`, `is_empty`), and
//! proves the object-safe trait can be held as `Box<dyn IndexCore>`.

#![allow(clippy::unwrap_used)]

mod common;

use common::{MockConfig, MockIndex, arc};
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, IqdbError, Metadata, SearchParams, Value, VectorId};

#[test]
fn new_rejects_zero_dim() {
    let result = MockIndex::new(0, DistanceMetric::Euclidean, MockConfig);
    let err = result.err().expect("expected error");
    assert!(
        matches!(err, IqdbError::InvalidConfig { .. }),
        "expected InvalidConfig, got {err:?}",
    );
}

#[test]
fn is_empty_default_matches_len_zero() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    assert!(idx.is_empty());
    assert_eq!(idx.len(), 0);
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    assert!(!idx.is_empty());
    assert_eq!(idx.len(), 1);
}

#[test]
fn insert_rejects_dimension_mismatch() {
    let mut idx = MockIndex::new(3, DistanceMetric::Euclidean, MockConfig).unwrap();
    let err = idx
        .insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap_err();
    assert_eq!(
        err,
        IqdbError::DimensionMismatch {
            expected: 3,
            found: 2,
        }
    );
}

#[test]
fn insert_rejects_duplicate_id() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    let err = idx
        .insert(VectorId::from(1u64), arc(&[1.0, 1.0]), None)
        .unwrap_err();
    assert_eq!(err, IqdbError::Duplicate);
}

#[test]
fn delete_absent_returns_not_found() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    let err = idx.delete(&VectorId::from(99u64)).unwrap_err();
    assert_eq!(err, IqdbError::NotFound);
}

#[test]
fn delete_then_search_excludes_id() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    idx.insert(VectorId::from(2u64), arc(&[1.0, 0.0]), None)
        .unwrap();
    idx.delete(&VectorId::from(1u64)).unwrap();
    let hits = idx
        .search(
            &[0.0, 0.0],
            &SearchParams::new(5, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, VectorId::U64(2));
}

#[test]
fn search_rejects_metric_mismatch() {
    let idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    let err = idx
        .search(&[0.0, 0.0], &SearchParams::new(1, DistanceMetric::Cosine))
        .unwrap_err();
    assert_eq!(err, IqdbError::InvalidMetric);
}

#[test]
fn search_returns_best_first_up_to_k() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    idx.insert(VectorId::from(1u64), arc(&[10.0, 0.0]), None)
        .unwrap();
    idx.insert(VectorId::from(2u64), arc(&[1.0, 0.0]), None)
        .unwrap();
    idx.insert(VectorId::from(3u64), arc(&[5.0, 0.0]), None)
        .unwrap();
    let hits = idx
        .search(
            &[0.0, 0.0],
            &SearchParams::new(2, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].id, VectorId::U64(2));
    assert_eq!(hits[1].id, VectorId::U64(3));
    assert!(hits[0].distance <= hits[1].distance);
}

#[test]
fn search_with_k_zero_returns_empty() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
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
fn insert_batch_default_loops_per_item() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    let items = vec![
        (VectorId::from(1u64), arc(&[0.0, 0.0]), None),
        (VectorId::from(2u64), arc(&[1.0, 0.0]), None),
        (VectorId::from(3u64), arc(&[2.0, 0.0]), None),
    ];
    idx.insert_batch(items).unwrap();
    assert_eq!(idx.len(), 3);
}

#[test]
fn insert_batch_is_fail_fast_and_persists_prior_inserts() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    let items = vec![
        (VectorId::from(1u64), arc(&[0.0, 0.0]), None),
        (VectorId::from(2u64), arc(&[1.0]), None),
        (VectorId::from(3u64), arc(&[2.0, 0.0]), None),
    ];
    let err = idx.insert_batch(items).unwrap_err();
    assert_eq!(
        err,
        IqdbError::DimensionMismatch {
            expected: 2,
            found: 1,
        }
    );
    assert_eq!(idx.len(), 1);
}

#[test]
fn search_batch_default_loops_per_query() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    idx.insert(VectorId::from(2u64), arc(&[10.0, 0.0]), None)
        .unwrap();
    let q1: &[f32] = &[0.0, 0.0];
    let q2: &[f32] = &[10.0, 0.0];
    let queries: &[&[f32]] = &[q1, q2];
    let results = idx
        .search_batch(queries, &SearchParams::new(1, DistanceMetric::Euclidean))
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0][0].id, VectorId::U64(1));
    assert_eq!(results[1][0].id, VectorId::U64(2));
}

#[test]
fn flush_is_ok_for_in_memory_index() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    idx.flush().unwrap();
}

#[test]
fn stats_reports_index_type_and_counts() {
    let mut idx = MockIndex::new(4, DistanceMetric::Euclidean, MockConfig).unwrap();
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0, 0.0, 0.0]), None)
        .unwrap();
    let stats = idx.stats();
    assert_eq!(stats.n_vectors, 1);
    assert_eq!(stats.index_type, "mock");
    assert_eq!(stats.disk_bytes, None);
    let extra = stats.extra.as_ref().expect("MockIndex populates extra");
    assert_eq!(extra.get("backend").map(String::as_str), Some("mock"));
}

#[test]
fn metadata_round_trips_through_search() {
    let mut idx = MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap();
    let meta: Metadata = [
        ("year".to_string(), Value::Int(2026)),
        ("flag".to_string(), Value::Bool(true)),
    ]
    .into_iter()
    .collect();
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), Some(meta.clone()))
        .unwrap();
    let hits = idx
        .search(
            &[0.0, 0.0],
            &SearchParams::new(1, DistanceMetric::Euclidean),
        )
        .unwrap();
    assert_eq!(hits[0].metadata, Some(meta));
}

#[test]
fn index_core_is_object_safe() {
    let mut idx: Box<dyn IndexCore> =
        Box::new(MockIndex::new(2, DistanceMetric::Euclidean, MockConfig).unwrap());
    assert_eq!(idx.dim(), 2);
    assert_eq!(idx.metric(), DistanceMetric::Euclidean);
    assert!(idx.is_empty());
    idx.insert(VectorId::from(1u64), arc(&[0.0, 0.0]), None)
        .unwrap();
    assert_eq!(idx.len(), 1);
    idx.flush().unwrap();
}
