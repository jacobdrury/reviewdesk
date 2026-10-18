use chrono::{DateTime, TimeZone, Utc};
use reviewdesk_core::{
    LocalStore, NotificationQuery, NotificationReason, NotificationThread, NotificationThreadId,
    PullRequestId,
};
use reviewdesk_localstore::TursoLocalStore;
use tempfile::tempdir;

fn thread(
    id: &str,
    pull_request_id: &str,
    reason: NotificationReason,
    unread: bool,
    updated_at: DateTime<Utc>,
) -> NotificationThread {
    NotificationThread {
        id: NotificationThreadId::new(id).expect("thread id"),
        pull_request_id: PullRequestId::new(pull_request_id).expect("pr id"),
        title: format!("Thread {id}"),
        repo_label: "acme/widgets".to_string(),
        reason,
        unread,
        updated_at,
    }
}

async fn open_store() -> (tempfile::TempDir, TursoLocalStore) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    (dir, store)
}

#[tokio::test]
async fn sync_notifications_returns_unread_updated_at_desc() {
    let (_dir, store) = open_store().await;

    let older = thread(
        "1",
        "acme/widgets#1",
        NotificationReason::ReviewRequested,
        true,
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
    );
    let newer = thread(
        "2",
        "acme/widgets#2",
        NotificationReason::Mention,
        true,
        Utc.with_ymd_and_hms(2026, 1, 2, 12, 0, 0).unwrap(),
    );

    store
        .sync_notifications(&[older.clone(), newer.clone()])
        .await
        .expect("sync");

    let listed = store
        .query_notifications(&NotificationQuery::default())
        .await
        .expect("query");

    assert_eq!(listed, vec![newer, older]);
}

#[tokio::test]
async fn sync_notifications_reconcile_drops_absent_rows() {
    let (_dir, store) = open_store().await;

    let keep = thread(
        "1",
        "acme/widgets#1",
        NotificationReason::Comment,
        true,
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
    );
    let drop = thread(
        "9",
        "acme/widgets#9",
        NotificationReason::CiActivity,
        true,
        Utc.with_ymd_and_hms(2026, 1, 1, 13, 0, 0).unwrap(),
    );

    store
        .sync_notifications(&[keep.clone(), drop])
        .await
        .expect("sync");
    store
        .sync_notifications(&[keep.clone()])
        .await
        .expect("reconcile");

    let listed = store
        .query_notifications(&NotificationQuery::default())
        .await
        .expect("query");
    assert_eq!(listed, vec![keep]);
}

#[tokio::test]
async fn set_notification_read_removes_from_default_unread_list() {
    let (_dir, store) = open_store().await;

    let row = thread(
        "1",
        "acme/widgets#1",
        NotificationReason::Author,
        true,
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
    );
    store
        .sync_notifications(&[row.clone()])
        .await
        .expect("sync");

    store
        .set_notification_read(&row.id)
        .await
        .expect("mark read");

    let unread = store
        .query_notifications(&NotificationQuery::default())
        .await
        .expect("unread query");
    assert!(unread.is_empty());

    let all = store
        .query_notifications(&NotificationQuery { unread_only: false })
        .await
        .expect("all query");
    assert_eq!(all.len(), 1);
    assert!(!all[0].unread);
}

#[tokio::test]
async fn remove_notification_deletes_row() {
    let (_dir, store) = open_store().await;

    let row = thread(
        "1",
        "acme/widgets#1",
        NotificationReason::StateChange,
        true,
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
    );
    store
        .sync_notifications(&[row.clone()])
        .await
        .expect("sync");

    store.remove_notification(&row.id).await.expect("remove");

    let all = store
        .query_notifications(&NotificationQuery { unread_only: false })
        .await
        .expect("all query");
    assert!(all.is_empty());
}
