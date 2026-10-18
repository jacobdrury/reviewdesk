use chrono::{TimeZone, Utc};
use reviewdesk_core::{
    DraftFilter, InboxFilter, InboxItem, InboxLabel, InboxOptions, InboxQuery, InboxSort,
    InboxSortField, LocalStore, ReviewState, SortDirection,
};
use reviewdesk_core::PullRequestId;
use reviewdesk_localstore::TursoLocalStore;
use tempfile::tempdir;

fn pr(id: &str) -> PullRequestId {
    PullRequestId::new(id).expect("id")
}

fn item(
    id: &str,
    title: &str,
    updated_at: chrono::DateTime<Utc>,
    repo_label: &str,
    author_login: &str,
    is_draft: bool,
    review_state: ReviewState,
    labels: Vec<InboxLabel>,
) -> InboxItem {
    InboxItem {
        id: pr(id),
        title: title.to_string(),
        web_url: format!("https://example.com/{id}"),
        updated_at,
        author_login: author_login.to_string(),
        author_avatar_url: Some(format!("https://avatars.example/{author_login}")),
        repo_label: repo_label.to_string(),
        labels,
        review_state,
        is_draft,
    }
}

async fn open_store() -> (tempfile::TempDir, TursoLocalStore) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    (dir, store)
}

#[tokio::test]
async fn sync_inbox_upserts_and_query_returns_default_order() {
    let (_dir, store) = open_store().await;

    let older = item(
        "acme/widgets#1",
        "First",
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
        "acme/widgets",
        "alice",
        false,
        ReviewState::Pending,
        vec![InboxLabel {
            name: "bug".to_string(),
            color: "ff0000".to_string(),
        }],
    );
    let newer = item(
        "acme/widgets#2",
        "Second",
        Utc.with_ymd_and_hms(2026, 1, 2, 12, 0, 0).unwrap(),
        "acme/widgets",
        "bob",
        false,
        ReviewState::Approved,
        vec![],
    );

    store
        .sync_inbox(&[older.clone(), newer.clone()])
        .await
        .expect("sync");

    let listed = store
        .query_inbox(&InboxQuery::default())
        .await
        .expect("query");

    assert_eq!(listed, vec![newer, older]);
}

#[tokio::test]
async fn sync_inbox_updates_existing_row() {
    let (_dir, store) = open_store().await;

    let original = item(
        "acme/widgets#1",
        "Old title",
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
        "acme/widgets",
        "alice",
        false,
        ReviewState::Pending,
        vec![],
    );
    store.sync_inbox(&[original]).await.expect("sync");

    let updated = item(
        "acme/widgets#1",
        "New title",
        Utc.with_ymd_and_hms(2026, 1, 3, 12, 0, 0).unwrap(),
        "acme/widgets",
        "alice",
        false,
        ReviewState::ChangesRequested,
        vec![InboxLabel {
            name: "enhancement".to_string(),
            color: "00ff00".to_string(),
        }],
    );
    store.sync_inbox(&[updated.clone()]).await.expect("resync");

    let listed = store
        .query_inbox(&InboxQuery::default())
        .await
        .expect("query");

    assert_eq!(listed, vec![updated]);
}

#[tokio::test]
async fn sync_inbox_reconcile_removes_stale_rows() {
    let (_dir, store) = open_store().await;

    let keep = item(
        "acme/widgets#1",
        "Keep",
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
        "acme/widgets",
        "alice",
        false,
        ReviewState::Pending,
        vec![],
    );
    let drop = item(
        "acme/widgets#9",
        "Drop",
        Utc.with_ymd_and_hms(2026, 1, 1, 13, 0, 0).unwrap(),
        "acme/widgets",
        "bob",
        false,
        ReviewState::Pending,
        vec![],
    );
    store.sync_inbox(&[keep.clone(), drop]).await.expect("sync");

    store.sync_inbox(&[keep.clone()]).await.expect("reconcile");

    let listed = store
        .query_inbox(&InboxQuery::default())
        .await
        .expect("query");

    assert_eq!(listed, vec![keep]);
}

#[tokio::test]
async fn query_inbox_filters_drafts_repo_author_state_and_label() {
    let (_dir, store) = open_store().await;

    let ready = item(
        "acme/a#1",
        "Ready",
        Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
        "acme/a",
        "alice",
        false,
        ReviewState::Approved,
        vec![InboxLabel {
            name: "bug".to_string(),
            color: "a".to_string(),
        }],
    );
    let draft = item(
        "acme/a#2",
        "Draft",
        Utc.with_ymd_and_hms(2026, 2, 2, 0, 0, 0).unwrap(),
        "acme/a",
        "alice",
        true,
        ReviewState::Pending,
        vec![],
    );
    let other_repo = item(
        "acme/b#1",
        "Other repo",
        Utc.with_ymd_and_hms(2026, 2, 3, 0, 0, 0).unwrap(),
        "acme/b",
        "carol",
        false,
        ReviewState::Commented,
        vec![InboxLabel {
            name: "bug".to_string(),
            color: "b".to_string(),
        }],
    );

    store
        .sync_inbox(&[ready.clone(), draft, other_repo.clone()])
        .await
        .expect("sync");

    let drafts_only = InboxQuery {
        filter: InboxFilter {
            options: InboxOptions {
                include_authored: false,
                drafts: DraftFilter::DraftsOnly,
            },
            repo_label: None,
            author_login: None,
            review_state: None,
            has_label: None,
        },
        sort: InboxSort::default(),
    };
    let draft_rows = store.query_inbox(&drafts_only).await.expect("drafts");
    assert_eq!(draft_rows.len(), 1);
    assert_eq!(draft_rows[0].title, "Draft");

    let by_repo = InboxQuery {
        filter: InboxFilter {
            options: InboxOptions::default(),
            repo_label: Some("acme/b".to_string()),
            author_login: None,
            review_state: None,
            has_label: None,
        },
        sort: InboxSort::default(),
    };
    assert_eq!(
        store.query_inbox(&by_repo).await.expect("repo"),
        vec![other_repo.clone()]
    );

    let by_author = InboxQuery {
        filter: InboxFilter {
            options: InboxOptions::default(),
            repo_label: None,
            author_login: Some("alice".to_string()),
            review_state: None,
            has_label: None,
        },
        sort: InboxSort::default(),
    };
    assert_eq!(
        store.query_inbox(&by_author).await.expect("author"),
        vec![ready.clone()]
    );

    let by_state = InboxQuery {
        filter: InboxFilter {
            options: InboxOptions::default(),
            repo_label: None,
            author_login: None,
            review_state: Some(ReviewState::Commented),
            has_label: None,
        },
        sort: InboxSort::default(),
    };
    assert_eq!(
        store.query_inbox(&by_state).await.expect("state"),
        vec![other_repo.clone()]
    );

    let by_label = InboxQuery {
        filter: InboxFilter {
            options: InboxOptions::default(),
            repo_label: None,
            author_login: None,
            review_state: None,
            has_label: Some("bug".to_string()),
        },
        sort: InboxSort::default(),
    };
    let labeled = store.query_inbox(&by_label).await.expect("label");
    assert_eq!(labeled.len(), 2);
    assert_eq!(labeled[0].id, other_repo.id);
    assert_eq!(labeled[1].id, ready.id);
}

#[tokio::test]
async fn query_inbox_sorts_updated_at_ascending() {
    let (_dir, store) = open_store().await;

    let a = item(
        "x#1",
        "A",
        Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap(),
        "x",
        "a",
        false,
        ReviewState::Pending,
        vec![],
    );
    let b = item(
        "x#2",
        "B",
        Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap(),
        "x",
        "b",
        false,
        ReviewState::Pending,
        vec![],
    );
    store.sync_inbox(&[b.clone(), a.clone()]).await.expect("sync");

    let query = InboxQuery {
        filter: InboxFilter::default(),
        sort: InboxSort {
            field: InboxSortField::UpdatedAt,
            direction: SortDirection::Asc,
        },
    };
    assert_eq!(
        store.query_inbox(&query).await.expect("asc"),
        vec![a, b]
    );
}
