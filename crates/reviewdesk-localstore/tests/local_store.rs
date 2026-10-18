use reviewdesk_core::{
    LocalStore, LocalStoreError, Patch, PullRequestDetail, PullRequestId, PullRequestLabel,
    Reviewer,
};
use reviewdesk_localstore::TursoLocalStore;
use tempfile::tempdir;

fn sample_detail() -> PullRequestDetail {
    PullRequestDetail {
        id: PullRequestId::new("octo/repo#42").expect("id"),
        title: "Fix the thing".into(),
        body: "What changed and why.".into(),
        author_login: "octo".into(),
        author_avatar_url: Some("https://avatars.example/octo.png".into()),
        is_draft: false,
        labels: vec![PullRequestLabel {
            name: "bug".into(),
            color: "ff0000".into(),
        }],
        web_url: "https://github.com/octo/repo/pull/42".into(),
        reviewers: vec![
            Reviewer {
                login: "hubot".into(),
                avatar_url: None,
            },
            Reviewer {
                login: "dev".into(),
                avatar_url: Some("https://avatars.example/dev.png".into()),
            },
        ],
    }
}

#[tokio::test]
async fn open_on_temp_path_migrates_and_reports_schema_version() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");

    let store = TursoLocalStore::open(&path).await.expect("open");
    let version = store.schema_version().await.expect("schema_version");

    assert_eq!(version, 7);
}

#[tokio::test]
async fn store_survives_close_and_reopen() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");

    {
        let store = TursoLocalStore::open(&path).await.expect("open");
        assert_eq!(store.schema_version().await.expect("version"), 7);
    }

    let store = TursoLocalStore::open(&path).await.expect("reopen");
    assert_eq!(store.schema_version().await.expect("version"), 7);
}

#[tokio::test]
async fn migrate_is_idempotent_when_opening_twice() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");

    let first = TursoLocalStore::open(&path).await.expect("first open");
    let second = TursoLocalStore::open(&path).await.expect("second open");

    assert_eq!(first.schema_version().await.expect("v1"), 7);
    assert_eq!(second.schema_version().await.expect("v2"), 7);
}

#[tokio::test]
async fn pull_request_detail_round_trips_through_store() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    let detail = sample_detail();

    store
        .put_pull_request_detail(&detail)
        .await
        .expect("put detail");

    let loaded = store
        .get_pull_request_detail(&detail.id)
        .await
        .expect("get detail")
        .expect("cached detail");

    assert_eq!(loaded, detail);
}

#[tokio::test]
async fn get_pull_request_detail_returns_none_when_missing() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    let id = PullRequestId::new("octo/repo#99").expect("id");

    let loaded = store
        .get_pull_request_detail(&id)
        .await
        .expect("get detail");

    assert!(loaded.is_none());
}

#[tokio::test]
async fn put_pull_request_detail_overwrites_existing_row() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    let mut detail = sample_detail();

    store
        .put_pull_request_detail(&detail)
        .await
        .expect("put detail");
    detail.title = "Updated title".into();
    store
        .put_pull_request_detail(&detail)
        .await
        .expect("put updated detail");

    let loaded = store
        .get_pull_request_detail(&detail.id)
        .await
        .expect("get detail")
        .expect("cached detail");

    assert_eq!(loaded.title, "Updated title");
}

#[tokio::test]
async fn patch_round_trips_through_store() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    let id = PullRequestId::new("octo/repo#7").expect("id");
    let patch = Patch::new("diff --git a/foo b/foo\n");

    store.put_patch(&id, &patch).await.expect("put patch");

    let loaded = store
        .get_patch(&id)
        .await
        .expect("get patch")
        .expect("cached patch");

    assert_eq!(loaded, patch);
}

#[tokio::test]
async fn get_patch_returns_none_when_missing() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    let id = PullRequestId::new("octo/repo#8").expect("id");

    let loaded = store.get_patch(&id).await.expect("get patch");

    assert!(loaded.is_none());
}

#[tokio::test]
async fn unopenable_path_returns_typed_open_error() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().to_path_buf();

    let result = TursoLocalStore::open(&path).await;
    assert!(matches!(result, Err(LocalStoreError::Open { .. })));
}
