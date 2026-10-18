//! Pull requests list pane projection (Views, scope, grouped rows, empty states).

use std::sync::Arc;

use chrono::Utc;
use reviewdesk_core::{
    PullRequestListItem, PullRequestListRow, PullRequestListSort, PullRequestsEmptyKind, View,
    ViewFilter, ViewGroupBy, ViewId, build_pull_request_list_rows, distinct_repo_labels,
    group_by_display_label,
};

use crate::review_session::{LoadState, PullRequestsSyncView};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestsPaneView {
    pub sync: PullRequestsSyncView,
    pub cache: LoadState<()>,
    pub empty: Option<PullRequestsEmptyKind>,
    pub rows: Arc<[PullRequestListRow]>,
    pub views: Arc<[View]>,
    pub active_view_id: Option<ViewId>,
    pub active_view_name: Option<String>,
    pub view_dirty: bool,
    pub group_by_label: String,
    pub repo_scope: Option<String>,
    pub repositories: Arc<[String]>,
    pub working_filter: ViewFilter,
}

impl Default for PullRequestsPaneView {
    fn default() -> Self {
        Self {
            sync: PullRequestsSyncView::default(),
            cache: LoadState::Loading,
            empty: None,
            rows: Arc::from([]),
            views: Arc::from([]),
            active_view_id: None,
            active_view_name: None,
            view_dirty: false,
            group_by_label: group_by_display_label(ViewGroupBy::None).to_string(),
            repo_scope: None,
            repositories: Arc::from([]),
            working_filter: ViewFilter::default(),
        }
    }
}

pub struct PullRequestsPaneInputs<'a> {
    pub sync: PullRequestsSyncView,
    pub cache: &'a LoadState<Arc<[PullRequestListItem]>>,
    pub views: &'a LoadState<Arc<[View]>>,
    pub active_view_id: &'a Option<ViewId>,
    pub working_filter: &'a ViewFilter,
    pub working_group_by: ViewGroupBy,
    pub working_sort: &'a PullRequestListSort,
    pub saved_snapshot: &'a Option<(ViewFilter, ViewGroupBy, PullRequestListSort)>,
    pub viewer_login: &'a Option<String>,
    pub repo_scope: &'a Option<String>,
}

pub fn project_pull_requests_pane(inputs: &PullRequestsPaneInputs<'_>) -> PullRequestsPaneView {
    let cache_state = match inputs.cache {
        LoadState::Idle | LoadState::Loading => LoadState::Loading,
        LoadState::Error(_) => LoadState::Error(reviewdesk_core::ProviderError::other(
            "pull request cache failed",
        )),
        LoadState::Ready(_) => LoadState::Ready(()),
    };

    let views_arc = match inputs.views {
        LoadState::Ready(v) => v.clone(),
        _ => Arc::from([] as [View; 0]),
    };

    let active_name = inputs.active_view_id.as_ref().and_then(|id| {
        views_arc
            .iter()
            .find(|v| &v.id == id)
            .map(|v| v.name.clone())
    });

    let view_dirty = inputs.saved_snapshot.as_ref().is_some_and(|(f, g, s)| {
        f != inputs.working_filter || *g != inputs.working_group_by || s != inputs.working_sort
    });

    let (rows, empty, repositories) = match inputs.cache {
        LoadState::Ready(items) => {
            let repos = distinct_repo_labels(items);
            let (rows, empty) = build_pull_request_list_rows(
                items,
                inputs.working_filter,
                inputs.working_group_by,
                inputs.working_sort,
                inputs.viewer_login.as_deref(),
                Utc::now(),
                inputs.repo_scope.as_deref(),
            );
            (Arc::from(rows), empty, Arc::from(repos))
        }
        _ => (
            Arc::from([] as [PullRequestListRow; 0]),
            None,
            Arc::from([]),
        ),
    };

    PullRequestsPaneView {
        sync: inputs.sync.clone(),
        cache: cache_state,
        empty,
        rows,
        views: views_arc,
        active_view_id: inputs.active_view_id.clone(),
        active_view_name: active_name,
        view_dirty,
        group_by_label: group_by_display_label(inputs.working_group_by).to_string(),
        repo_scope: inputs.repo_scope.clone(),
        repositories,
        working_filter: inputs.working_filter.clone(),
    }
}
