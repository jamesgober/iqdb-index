//! `IndexStats::extra` does not allocate when empty.
//!
//! The common `stats()` call reports no per-kind counters, so `extra` is
//! `Option<HashMap<String, String>>` with a `None` default — no empty-map
//! allocation on the hot dashboard path. An implementer that does have
//! per-kind detail populates `Some(map)` explicitly.

#![allow(clippy::unwrap_used)]

use std::collections::HashMap;

use iqdb_index::IndexStats;

#[test]
fn default_extra_is_none() {
    let stats = IndexStats::default();
    assert!(
        stats.extra.is_none(),
        "Default IndexStats must not allocate an empty HashMap for extra",
    );
}

#[test]
fn extra_some_with_explicit_entries_is_preserved() {
    let mut map = HashMap::new();
    let _ = map.insert("tombstones".to_string(), "42".to_string());

    let stats = IndexStats {
        n_vectors: 100,
        memory_bytes: 1024,
        disk_bytes: None,
        index_type: "test",
        extra: Some(map),
    };

    let entries = stats.extra.as_ref().expect("extra is Some");
    assert_eq!(entries.get("tombstones"), Some(&"42".to_string()));
}

#[test]
fn struct_update_syntax_against_default_yields_none_extra() {
    // Doc-example shape: `..IndexStats::default()` keeps `extra` at the
    // type's default value, which is `None`. This locks the ergonomics so the
    // documented construction path stays one line.
    let stats = IndexStats {
        n_vectors: 42,
        memory_bytes: 4096,
        index_type: "flat",
        ..IndexStats::default()
    };
    assert_eq!(stats.n_vectors, 42);
    assert_eq!(stats.disk_bytes, None);
    assert!(stats.extra.is_none());
}
