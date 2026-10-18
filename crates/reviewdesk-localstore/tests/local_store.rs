use reviewdesk_core::{LocalStore, LocalStoreError};
use reviewdesk_localstore::TursoLocalStore;
use tempfile::tempdir;

#[tokio::test]
async fn open_on_temp_path_migrates_and_reports_schema_version() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");

    let store = TursoLocalStore::open(&path).await.expect("open");
    let version = store.schema_version().await.expect("schema_version");

    assert_eq!(version, 1);
}

#[tokio::test]
async fn store_survives_close_and_reopen() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");

    {
        let store = TursoLocalStore::open(&path).await.expect("open");
        assert_eq!(store.schema_version().await.expect("version"), 1);
    }

    let store = TursoLocalStore::open(&path).await.expect("reopen");
    assert_eq!(store.schema_version().await.expect("version"), 1);
}

#[tokio::test]
async fn migrate_is_idempotent_when_opening_twice() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");

    let first = TursoLocalStore::open(&path).await.expect("first open");
    let second = TursoLocalStore::open(&path).await.expect("second open");

    assert_eq!(first.schema_version().await.expect("v1"), 1);
    assert_eq!(second.schema_version().await.expect("v2"), 1);
}

#[tokio::test]
async fn unopenable_path_returns_typed_open_error() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().to_path_buf();

    let result = TursoLocalStore::open(&path).await;
    assert!(matches!(result, Err(LocalStoreError::Open { .. })));
}
