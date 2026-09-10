use notes_sync::{store::Store, Error};
use std::fs;
use uuid::Uuid;
#[test]
fn journal_survives_restart_and_stale_transactions_cannot_replace_it() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let workspace = Uuid::new_v4();
    store.initialize(workspace).unwrap();
    assert_eq!(store.initialize(workspace), Err(Error::Stale));
    let (_, digest) = store.load().unwrap();
    store
        .transact(&digest, |j| {
            j.acknowledge(Uuid::new_v4(), Default::default())?;
            Ok(())
        })
        .unwrap();
    assert_eq!(store.transact(&digest, |_| Ok(())), Err(Error::Stale));
    drop(store);
    let (restored, _) = Store::open(dir.path()).unwrap().load().unwrap();
    assert_eq!(restored.workspace, workspace);
    assert_eq!(restored.acknowledgments.len(), 1);
}
#[test]
fn a_failed_transaction_keeps_exact_previous_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.initialize(Uuid::new_v4()).unwrap();
    let (_, digest) = store.load().unwrap();
    let path = dir.path().join("journal.json");
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        store.transact(&digest, |j| {
            j.schema = 99;
            Ok(())
        }),
        Err(Error::Schema)
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        store.transact(&digest, |_| Err::<(), _>(Error::Collision)),
        Err(Error::Collision)
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
}
#[test]
fn future_and_corrupt_files_are_preserved_and_never_reset() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let path = dir.path().join("journal.json");
    for (bytes, error) in [
        (b"{\"schema\":99}".as_slice(), Error::Schema),
        (b"broken".as_slice(), Error::InvalidState),
    ] {
        fs::write(&path, bytes).unwrap();
        assert_eq!(store.load(), Err(error));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}
#[cfg(unix)]
#[test]
fn symlink_state_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(other.path(), dir.path().join("link")).unwrap();
    assert!(Store::open(&dir.path().join("link")).is_err());
    let store = Store::open(dir.path()).unwrap();
    std::os::unix::fs::symlink(
        other.path().join("journal.json"),
        dir.path().join("journal.json"),
    )
    .unwrap();
    assert!(store.initialize(Uuid::new_v4()).is_err());
    assert!(!other.path().join("journal.json").exists());
}
