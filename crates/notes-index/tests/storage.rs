use notes_index::{Index, Indexed, RegistryStore, Seen};
use notes_model::CoreError;
#[test]
fn fts_words_are_tokens_and_not_a_query_language() {
    let d = tempfile::tempdir().unwrap();
    let mut index = Index::open(&d.path().join("index.db")).unwrap();
    let seen = Seen {
        path: "a.md".into(),
        size: 42,
        mtime: "1".into(),
    };
    index
        .apply(
            Indexed {
                seen: &seen,
                hash: "one",
                text: "heading\nação tested testing\n",
            },
            true,
        )
        .unwrap();
    let hit = index.words("acao", 10).unwrap();
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].line, 2);
    assert!(index.words("test", 10).unwrap().is_empty());
    assert_eq!(index.words("TESTED", 10).unwrap().len(), 1);
    assert!(index.words("tested OR absent", 10).unwrap().is_empty());
    index.remove(&["a.md".into()]).unwrap();
    assert!(index.words("acao", 10).unwrap().is_empty());
}
#[test]
fn future_schema_is_refused_without_changing_it() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("registry.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.pragma_update(None, "user_version", 99).unwrap();
    drop(db);
    assert!(matches!(
        RegistryStore::open(&path),
        Err(CoreError::SchemaAhead { found: 99, .. })
    ));
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.pragma_query_value::<u32, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        99
    );
}
#[test]
fn index_recreation_does_not_touch_identity() {
    let d = tempfile::tempdir().unwrap();
    let registry = d.path().join("registry.db");
    let mut store = RegistryStore::open(&registry).unwrap();
    store.write(b"identity").unwrap();
    drop(store);
    let path = d.path().join("index.db");
    drop(Index::open(&path).unwrap());
    std::fs::remove_file(&path).unwrap();
    drop(Index::open(&path).unwrap());
    assert_eq!(
        RegistryStore::open(&registry)
            .unwrap()
            .read()
            .unwrap()
            .unwrap(),
        b"identity"
    );
}

#[test]
fn stale_registry_snapshots_merge_unrelated_notes_but_refuse_conflicts() {
    let d = tempfile::tempdir().unwrap();
    let mut store = RegistryStore::open(&d.path().join("registry.db")).unwrap();
    let base = br#"{"notes":{},"schema":1}"#;
    store.write_merged(None, base).unwrap();
    let a = br#"{"notes":{"a":{"path":"a.md","rev":1}},"schema":1}"#;
    let b = br#"{"notes":{"b":{"path":"b.md","rev":1}},"schema":1}"#;
    store.write_merged(Some(base), a).unwrap();
    store.write_merged(Some(base), b).unwrap();
    let result: serde_json::Value =
        serde_json::from_slice(&store.read().unwrap().unwrap()).unwrap();
    assert_eq!(result["notes"].as_object().unwrap().len(), 2);
    assert!(store
        .write_merged(
            Some(base),
            br#"{"notes":{"a":{"path":"changed.md","rev":2}},"schema":1}"#
        )
        .is_err());
}

#[test]
fn upgrading_derived_parser_schema_clears_only_index_data() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("index.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE notes(path TEXT); INSERT INTO notes VALUES('old.md'); CREATE TABLE fts(text TEXT); PRAGMA user_version=1;").unwrap();
    drop(db);
    let index = Index::open(&path).unwrap();
    assert!(index.plan().unwrap().is_empty());
    drop(index);
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.pragma_query_value::<u32, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        2
    );
}
