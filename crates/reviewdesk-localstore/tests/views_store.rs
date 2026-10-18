use reviewdesk_core::{DraftFilter, LocalStore, View, ViewFilter, ViewGroupBy, ViewId, ViewKind};
use reviewdesk_localstore::TursoLocalStore;
use tempfile::tempdir;

async fn open_store() -> (tempfile::TempDir, TursoLocalStore) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("reviewdesk.db");
    let store = TursoLocalStore::open(&path).await.expect("open");
    (dir, store)
}

#[tokio::test]
async fn saved_views_round_trip_in_position_order() {
    let (_dir, store) = open_store().await;

    assert!(store.list_views().await.expect("empty").is_empty());

    for view in View::seed_set() {
        store.save_view(&view).await.expect("save");
    }

    let listed = store.list_views().await.expect("list");
    assert_eq!(listed, View::seed_set());
}

#[tokio::test]
async fn save_view_replaces_existing_by_id() {
    let (_dir, store) = open_store().await;

    let original = View {
        id: ViewId::new("custom"),
        name: "Hot PRs".to_string(),
        kind: ViewKind::User,
        position: 7,
        filter: ViewFilter::default(),
        group_by: ViewGroupBy::Repo,
        sort: Default::default(),
    };
    store.save_view(&original).await.expect("save");

    let renamed = View {
        name: "Renamed".to_string(),
        filter: ViewFilter {
            i_am_author: true,
            drafts: DraftFilter::DraftsOnly,
            ..ViewFilter::default()
        },
        ..original.clone()
    };
    store.save_view(&renamed).await.expect("resave");

    let listed = store.list_views().await.expect("list");
    assert_eq!(listed, vec![renamed]);
}

#[tokio::test]
async fn delete_view_removes_only_that_row() {
    let (_dir, store) = open_store().await;
    for view in View::seed_set() {
        store.save_view(&view).await.expect("save");
    }

    store
        .delete_view(&ViewId::new(reviewdesk_core::VIEW_ID_MINE))
        .await
        .expect("delete");

    let listed = store.list_views().await.expect("list");
    assert_eq!(listed.len(), View::seed_set().len() - 1);
    assert!(
        !listed
            .iter()
            .any(|v| v.id.as_str() == reviewdesk_core::VIEW_ID_MINE)
    );
}

#[tokio::test]
async fn reorder_views_rewrites_positions() {
    let (_dir, store) = open_store().await;
    for view in View::seed_set() {
        store.save_view(&view).await.expect("save");
    }

    let reversed: Vec<ViewId> = View::seed_set().into_iter().rev().map(|v| v.id).collect();
    store.reorder_views(&reversed).await.expect("reorder");

    let listed = store.list_views().await.expect("list");
    let order: Vec<&str> = listed.iter().map(|v| v.id.as_str()).collect();
    let expected: Vec<&str> = reversed.iter().map(|id| id.as_str()).collect();
    assert_eq!(order, expected);
}

#[tokio::test]
async fn last_used_view_preference_persists() {
    let (_dir, store) = open_store().await;
    assert!(store.get_last_used_view().await.expect("none").is_none());

    let id = ViewId::new(reviewdesk_core::VIEW_ID_DRAFTS);
    store.set_last_used_view(&id).await.expect("set");
    assert_eq!(store.get_last_used_view().await.expect("get"), Some(id));

    let other = ViewId::new(reviewdesk_core::VIEW_ID_ALL_PRS);
    store.set_last_used_view(&other).await.expect("overwrite");
    assert_eq!(store.get_last_used_view().await.expect("get"), Some(other));
}
