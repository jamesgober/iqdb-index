//! Property-based coverage of the `IndexCore` trait contract.
//!
//! Each test drives the shared brute-force `MockIndex` — the reference
//! implementation of the trait — and asserts a contract invariant over many
//! generated inputs. Because the batch and `is_empty` paths use the crate's
//! own default shims, these properties exercise `iqdb-index`'s real code, not
//! just the mock.
//!
//! Invariants covered (dev/DIRECTIVES.md §8):
//! - **Deletion visibility** — after `delete(id)`, `search` never returns `id`.
//! - **Best-first ordering** — results are non-decreasing in distance and
//!   number `min(k, len)`.
//! - **Batch equivalence** — `insert_batch` equals a sequential `insert` loop;
//!   `search_batch` equals mapping `search` over each query.
//! - **Cardinality** — `len` tracks inserts minus deletes; `is_empty` agrees.

#![allow(clippy::unwrap_used)]

mod common;

use common::{MockConfig, MockIndex, arc};
use iqdb_index::{Index, IndexCore};
use iqdb_types::{DistanceMetric, SearchParams, VectorId};
use proptest::prelude::*;

/// A point set: a dimensionality plus per-point component vectors. Ids are the
/// 1-based position, which keeps them unique (the trait rejects duplicates).
fn point_set() -> impl Strategy<Value = (usize, Vec<Vec<f32>>)> {
    (1usize..=6).prop_flat_map(|dim| {
        let points = prop::collection::vec(
            prop::collection::vec(-100.0f32..100.0f32, dim..=dim),
            0..=24,
        );
        (Just(dim), points)
    })
}

fn build(dim: usize, points: &[Vec<f32>]) -> MockIndex {
    let mut idx = MockIndex::new(dim, DistanceMetric::Euclidean, MockConfig).unwrap();
    for (i, p) in points.iter().enumerate() {
        idx.insert(VectorId::from(i as u64 + 1), arc(p), None)
            .unwrap();
    }
    idx
}

proptest! {
    /// Results come back smallest-distance-first and number exactly
    /// `min(k, len)`.
    #[test]
    fn search_is_best_first_and_capped((dim, points) in point_set(), k in 0usize..30) {
        let idx = build(dim, &points);
        let query = vec![0.0f32; dim];
        let hits = idx
            .search(&query, &SearchParams::new(k, DistanceMetric::Euclidean))
            .unwrap();

        prop_assert_eq!(hits.len(), k.min(points.len()));
        for win in hits.windows(2) {
            prop_assert!(win[0].distance <= win[1].distance);
        }
    }

    /// After deleting a subset of ids, none of them appear in a wide search.
    #[test]
    fn deleted_ids_never_resurface((dim, points) in point_set(), seed in any::<u64>()) {
        prop_assume!(!points.is_empty());
        let mut idx = build(dim, &points);

        // Delete every id whose (id XOR seed) is even — a data-dependent subset.
        let mut deleted = Vec::new();
        for i in 0..points.len() {
            let id = i as u64 + 1;
            if (id ^ seed).is_multiple_of(2) {
                idx.delete(&VectorId::from(id)).unwrap();
                deleted.push(VectorId::from(id));
            }
        }

        prop_assert_eq!(idx.len(), points.len() - deleted.len());

        let query = vec![0.0f32; dim];
        let hits = idx
            .search(&query, &SearchParams::new(points.len(), DistanceMetric::Euclidean))
            .unwrap();
        for hit in &hits {
            prop_assert!(!deleted.contains(&hit.id), "deleted id resurfaced: {:?}", hit.id);
        }
    }

    /// `insert_batch` leaves the index identical to a sequential `insert` loop,
    /// observed through `search`.
    #[test]
    fn insert_batch_equals_insert_loop((dim, points) in point_set()) {
        let looped = build(dim, &points);

        let mut batched = MockIndex::new(dim, DistanceMetric::Euclidean, MockConfig).unwrap();
        let items: Vec<_> = points
            .iter()
            .enumerate()
            .map(|(i, p)| (VectorId::from(i as u64 + 1), arc(p), None))
            .collect();
        batched.insert_batch(items).unwrap();

        prop_assert_eq!(looped.len(), batched.len());

        let query = vec![0.0f32; dim];
        let params = SearchParams::new(points.len(), DistanceMetric::Euclidean);
        prop_assert_eq!(
            looped.search(&query, &params).unwrap(),
            batched.search(&query, &params).unwrap(),
        );
    }

    /// `search_batch` equals mapping `search` over each query independently,
    /// in input order.
    #[test]
    fn search_batch_equals_search_map((dim, points) in point_set()) {
        let idx = build(dim, &points);
        let params = SearchParams::new(5, DistanceMetric::Euclidean);

        let q_owned = [vec![0.0f32; dim], vec![1.0f32; dim], vec![-1.0f32; dim]];
        let queries: Vec<&[f32]> = q_owned.iter().map(Vec::as_slice).collect();

        let batched = idx.search_batch(&queries, &params).unwrap();
        let mapped: Vec<_> = queries
            .iter()
            .map(|q| idx.search(q, &params).unwrap())
            .collect();

        prop_assert_eq!(batched, mapped);
    }

    /// `is_empty` agrees with `len == 0` after an arbitrary insert count.
    #[test]
    fn is_empty_agrees_with_len((dim, points) in point_set()) {
        let idx = build(dim, &points);
        prop_assert_eq!(idx.is_empty(), idx.len() == 0);
        prop_assert_eq!(idx.is_empty(), points.is_empty());
    }
}
