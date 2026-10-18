//! In-process session kernel for Review Desk window policy.
//!
//! Callers and tests use construction, [`ReviewSession::apply`], and
//! [`ReviewSession::view`] only. I/O is expressed as [`Effect`]s; the GPUI shell
//! runs them and feeds completions back as [`Intent`]s.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use gpui_kit::SharedString;
use gpui_kit::component::diff::{DiffFile, DiffLineRange, DiffMode};
use reviewdesk_core::{
    Patch, ProviderError, PullRequestDetail, PullRequestId, PullRequestListItem,
    PullRequestListSort, View, ViewFilter, ViewGroupBy, ViewId, ViewKind, resolve_launch_view_id,
};

use crate::pull_requests_pane::{
    PullRequestsPaneInputs, PullRequestsPaneView, project_pull_requests_pane,
};

use crate::overview_body::{
    BodySegment, DetailsBlock, OverviewMarkdownInput, compose_overview_markdown,
    details_effective_open, open_details_ids, parse_body_segments,
};
use crate::patch_index::{PatchIndex, index_patch};

/// User gestures, boot, timer ticks, and async I/O completions.
#[derive(Debug)]
pub enum Intent {
    Boot,
    RefreshPullRequests,
    SelectPullRequest(PullRequestId),
    SetReviewTab(ReviewTab),
    ToggleDetails {
        id: usize,
        currently_open: bool,
    },
    SelectFile(SharedString),
    RetryFile(SharedString),
    ToggleDiffMode,
    AddAnnotation(DiffLineRange),
    RetryDetail,
    RetryPatch,
    PullRequestListQueryFinished {
        result: Result<Vec<PullRequestListItem>, ProviderError>,
    },
    PullRequestListSyncFinished {
        generation: u64,
        result: Result<(), ProviderError>,
    },
    PullRequestsViewsBootstrapFinished {
        views: Result<Vec<View>, ProviderError>,
        last_used: Result<Option<ViewId>, ProviderError>,
        scoped_repository: Result<Option<String>, ProviderError>,
    },
    ViewerLoginFinished {
        result: Result<String, ProviderError>,
    },
    SelectView(ViewId),
    SetRepoScope {
        repo: Option<String>,
    },
    SaveActiveView,
    SaveViewAsNew {
        name: String,
    },
    DiscardViewEdits,
    RenameActiveView {
        name: String,
    },
    DeleteActiveView,
    #[allow(dead_code)]
    ReorderViews {
        order: Vec<ViewId>,
    },
    ViewsPersistFinished {
        views: Result<Vec<View>, ProviderError>,
    },
    DetailLocalFinished {
        id: PullRequestId,
        result: Result<Option<PullRequestDetail>, ProviderError>,
    },
    DetailNetworkFinished {
        id: PullRequestId,
        had_cache: bool,
        result: Result<PullRequestDetail, ProviderError>,
    },
    PatchLocalFinished {
        id: PullRequestId,
        result: Result<Option<Patch>, ProviderError>,
    },
    PatchNetworkFinished {
        id: PullRequestId,
        had_cache: bool,
        result: Result<Patch, ProviderError>,
    },
}

/// Work the shell must perform against LocalStore / Provider / timers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    QueryPullRequests,
    SyncPullRequests {
        generation: u64,
    },
    FetchDetail {
        id: PullRequestId,
    },
    FetchPatch {
        id: PullRequestId,
    },
    ArmPullRequestsRefreshTimer,
    BootstrapPullRequestsViews,
    FetchViewerLogin,
    PersistViews {
        views: Vec<View>,
        last_used: Option<ViewId>,
        scoped_repository: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewTab {
    Overview,
    Files,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState<T> {
    Idle,
    Loading,
    Ready(T),
    Error(ProviderError),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullRequestsSyncView {
    pub syncing: bool,
    pub last_success: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl PullRequestsSyncView {
    pub fn status_label(&self) -> String {
        if self.syncing {
            return "Syncing pull requests…".into();
        }
        if let Some(error) = &self.error {
            return format!("Sync failed: {error}");
        }
        if let Some(at) = self.last_success {
            return format!("Pull requests synced {}", format_sync_time(at));
        }
        "Pull requests not synced yet".into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationView {
    pub id: String,
    pub range: DiffLineRange,
    pub body: SharedString,
}

/// Paint projection. No LocalStore / Provider access from here.
#[derive(Clone)]
pub struct SessionView {
    pub pull_requests_pane: PullRequestsPaneView,
    pub selected_id: Option<PullRequestId>,
    pub review_tab: ReviewTab,
    pub detail: LoadState<PullRequestDetail>,
    pub overview_markdown: Option<SharedString>,
    pub patch_index: LoadState<()>,
    pub file_paths: Arc<[SharedString]>,
    pub selected_file: Option<SharedString>,
    pub selected_diff_file: Option<DiffFile>,
    pub selected_file_error: Option<ProviderError>,
    pub diff_mode: DiffMode,
    /// Bumps when Diff surface inputs change (mode, selected file, materialization).
    pub diff_revision: u64,
    pub annotations: Vec<AnnotationView>,
}

#[derive(Clone, PartialEq, Eq)]
struct OverviewComposeCache {
    pr_id: PullRequestId,
    open_details: Vec<usize>,
    markdown: SharedString,
}

struct Annotation {
    id: String,
    range: DiffLineRange,
    body: SharedString,
}

/// Sole external seam for window session policy.
pub struct ReviewSession {
    pull_request_cache: LoadState<Arc<[PullRequestListItem]>>,
    pull_requests_sync: PullRequestsSyncView,
    pull_requests_sync_generation: u64,
    views: LoadState<Arc<[View]>>,
    active_view_id: Option<ViewId>,
    working_filter: ViewFilter,
    working_group_by: ViewGroupBy,
    working_sort: PullRequestListSort,
    saved_snapshot: Option<(ViewFilter, ViewGroupBy, PullRequestListSort)>,
    viewer_login: Option<String>,
    repo_scope: Option<String>,
    next_user_view_seq: u64,
    selected_id: Option<PullRequestId>,
    review_tab: ReviewTab,
    detail: LoadState<PullRequestDetail>,
    patch_index: LoadState<PatchIndex>,
    file_cache: HashMap<SharedString, DiffFile>,
    file_errors: HashMap<SharedString, ProviderError>,
    selected_file: Option<SharedString>,
    selected_diff_file: Option<DiffFile>,
    diff_mode: DiffMode,
    diff_revision: u64,
    annotations: HashMap<String, Vec<Annotation>>,
    annotation_seq: usize,
    overview_segments: Vec<BodySegment>,
    details_open: HashMap<usize, bool>,
    overview_compose_cache: Option<OverviewComposeCache>,
    file_list_paths: Arc<[SharedString]>,
}

impl ReviewSession {
    pub fn new() -> Self {
        Self {
            pull_request_cache: LoadState::Loading,
            pull_requests_sync: PullRequestsSyncView::default(),
            pull_requests_sync_generation: 0,
            views: LoadState::Loading,
            active_view_id: None,
            working_filter: ViewFilter::default(),
            working_group_by: ViewGroupBy::default(),
            working_sort: PullRequestListSort::default(),
            saved_snapshot: None,
            viewer_login: None,
            repo_scope: None,
            next_user_view_seq: 1,
            selected_id: None,
            review_tab: ReviewTab::Overview,
            detail: LoadState::Idle,
            patch_index: LoadState::Idle,
            file_cache: HashMap::new(),
            file_errors: HashMap::new(),
            selected_file: None,
            selected_diff_file: None,
            diff_mode: DiffMode::Split,
            diff_revision: 0,
            annotations: HashMap::new(),
            annotation_seq: 0,
            overview_segments: Vec::new(),
            details_open: HashMap::new(),
            overview_compose_cache: None,
            file_list_paths: Arc::from([]),
        }
    }

    pub fn apply(&mut self, intent: Intent) -> Vec<Effect> {
        match intent {
            Intent::Boot => self.boot(),
            Intent::RefreshPullRequests => self.begin_pull_requests_sync(),
            Intent::SelectPullRequest(id) => self.select_pull_request(id),
            Intent::SetReviewTab(tab) => {
                self.review_tab = tab;
                Vec::new()
            }
            Intent::ToggleDetails { id, currently_open } => {
                self.details_open.insert(id, !currently_open);
                Vec::new()
            }
            Intent::SelectFile(path) => {
                self.select_file(path);
                Vec::new()
            }
            Intent::RetryFile(path) => {
                self.file_errors.remove(&path);
                self.file_cache.remove(&path);
                self.materialize_file(path);
                Vec::new()
            }
            Intent::ToggleDiffMode => {
                self.diff_mode = match self.diff_mode {
                    DiffMode::Split => DiffMode::Unified,
                    DiffMode::Unified => DiffMode::Split,
                    other => other,
                };
                self.bump_diff_revision();
                Vec::new()
            }
            Intent::AddAnnotation(range) => {
                self.add_annotation(range);
                Vec::new()
            }
            Intent::RetryDetail => self.retry_detail(),
            Intent::RetryPatch => self.retry_patch(),
            Intent::PullRequestListQueryFinished { result } => {
                self.on_pull_request_list_query_finished(result);
                Vec::new()
            }
            Intent::PullRequestListSyncFinished { generation, result } => {
                self.on_pull_requests_sync_finished(generation, result)
            }
            Intent::PullRequestsViewsBootstrapFinished {
                views,
                last_used,
                scoped_repository,
            } => self.on_views_bootstrap_finished(views, last_used, scoped_repository),
            Intent::ViewerLoginFinished { result } => {
                if let Ok(login) = result {
                    self.viewer_login = Some(login);
                }
                Vec::new()
            }
            Intent::SelectView(id) => self.select_view(id),
            Intent::SetRepoScope { repo } => self.set_repo_scope(repo),
            Intent::SaveActiveView => self.save_active_view(),
            Intent::SaveViewAsNew { name } => self.save_view_as_new(name),
            Intent::DiscardViewEdits => {
                self.discard_view_edits();
                Vec::new()
            }
            Intent::RenameActiveView { name } => self.rename_active_view(name),
            Intent::DeleteActiveView => self.delete_active_view(),
            Intent::ReorderViews { order } => self.reorder_views(order),
            Intent::ViewsPersistFinished { views } => {
                self.on_views_persist_finished(views);
                Vec::new()
            }
            Intent::DetailLocalFinished { id, result } => {
                self.on_detail_local(id, result);
                Vec::new()
            }
            Intent::DetailNetworkFinished {
                id,
                had_cache,
                result,
            } => {
                self.on_detail_network(id, had_cache, result);
                Vec::new()
            }
            Intent::PatchLocalFinished { id, result } => {
                self.on_patch_local(id, result);
                Vec::new()
            }
            Intent::PatchNetworkFinished {
                id,
                had_cache,
                result,
            } => {
                self.on_patch_network(id, had_cache, result);
                Vec::new()
            }
        }
    }

    pub fn view(&mut self) -> SessionView {
        let ready_detail = match &self.detail {
            LoadState::Ready(detail) => Some(detail.clone()),
            _ => None,
        };
        let overview_markdown = ready_detail.map(|detail| self.cached_overview_markdown(&detail));
        let selected_file_error = self
            .selected_file
            .as_ref()
            .and_then(|path| self.file_errors.get(path).cloned());
        let annotations = self.current_annotations();
        let patch_index = match &self.patch_index {
            LoadState::Idle => LoadState::Idle,
            LoadState::Loading => LoadState::Loading,
            LoadState::Ready(_) => LoadState::Ready(()),
            LoadState::Error(error) => LoadState::Error(error.clone()),
        };

        let pull_requests_pane = project_pull_requests_pane(&PullRequestsPaneInputs {
            sync: self.pull_requests_sync.clone(),
            cache: &self.pull_request_cache,
            views: &self.views,
            active_view_id: &self.active_view_id,
            working_filter: &self.working_filter,
            working_group_by: self.working_group_by,
            working_sort: &self.working_sort,
            saved_snapshot: &self.saved_snapshot,
            viewer_login: &self.viewer_login,
            repo_scope: &self.repo_scope,
        });

        SessionView {
            pull_requests_pane,
            selected_id: self.selected_id.clone(),
            review_tab: self.review_tab,
            detail: self.detail.clone(),
            overview_markdown,
            patch_index,
            file_paths: self.file_list_paths.clone(),
            selected_file: self.selected_file.clone(),
            selected_diff_file: self.selected_diff_file.clone(),
            selected_file_error,
            diff_mode: self.diff_mode,
            diff_revision: self.diff_revision,
            annotations,
        }
    }

    fn bump_diff_revision(&mut self) {
        self.diff_revision = self.diff_revision.wrapping_add(1);
    }

    fn boot(&mut self) -> Vec<Effect> {
        self.pull_request_cache = LoadState::Loading;
        self.views = LoadState::Loading;
        let mut effects = self.begin_pull_requests_sync();
        effects.insert(0, Effect::QueryPullRequests);
        effects.push(Effect::BootstrapPullRequestsViews);
        effects.push(Effect::FetchViewerLogin);
        effects.push(Effect::ArmPullRequestsRefreshTimer);
        effects
    }

    fn begin_pull_requests_sync(&mut self) -> Vec<Effect> {
        self.pull_requests_sync_generation += 1;
        let generation = self.pull_requests_sync_generation;
        self.pull_requests_sync.syncing = true;
        self.pull_requests_sync.error = None;
        vec![Effect::SyncPullRequests { generation }]
    }

    fn on_pull_request_list_query_finished(
        &mut self,
        result: Result<Vec<PullRequestListItem>, ProviderError>,
    ) {
        match result {
            Ok(items) => {
                self.pull_request_cache = LoadState::Ready(Arc::from(items));
            }
            Err(error) => {
                self.pull_request_cache = LoadState::Error(error);
            }
        }
        self.reconcile_selection_after_list_change();
    }

    fn reconcile_selection_after_list_change(&mut self) {
        let LoadState::Ready(items) = &self.pull_request_cache else {
            return;
        };
        let Some(selected) = &self.selected_id else {
            return;
        };
        if !items.iter().any(|item| &item.id == selected) {
            self.clear_selection();
        }
    }

    fn on_pull_requests_sync_finished(
        &mut self,
        generation: u64,
        result: Result<(), ProviderError>,
    ) -> Vec<Effect> {
        if self.pull_requests_sync_generation != generation {
            return Vec::new();
        }
        self.pull_requests_sync.syncing = false;
        match result {
            Ok(()) => {
                self.pull_requests_sync.last_success = Some(Utc::now());
                self.pull_requests_sync.error = None;
            }
            Err(error) => {
                self.pull_requests_sync.error = Some(error.to_string());
            }
        }
        vec![Effect::QueryPullRequests]
    }

    fn select_pull_request(&mut self, id: PullRequestId) -> Vec<Effect> {
        if self.selected_id.as_ref() == Some(&id) {
            return Vec::new();
        }
        self.selected_id = Some(id.clone());
        self.review_tab = ReviewTab::Overview;
        self.reset_subordinate_state();
        vec![
            Effect::FetchDetail { id: id.clone() },
            Effect::FetchPatch { id },
        ]
    }

    fn clear_selection(&mut self) {
        self.selected_id = None;
        self.reset_subordinate_state();
    }

    fn reset_subordinate_state(&mut self) {
        self.detail = LoadState::Idle;
        self.patch_index = LoadState::Idle;
        self.file_cache.clear();
        self.file_errors.clear();
        self.selected_file = None;
        self.selected_diff_file = None;
        self.overview_segments.clear();
        self.details_open.clear();
        self.overview_compose_cache = None;
        self.file_list_paths = Arc::from([]);
        self.bump_diff_revision();
    }

    fn retry_detail(&mut self) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        self.detail = LoadState::Loading;
        self.overview_segments.clear();
        self.details_open.clear();
        self.overview_compose_cache = None;
        vec![Effect::FetchDetail { id }]
    }

    fn retry_patch(&mut self) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        self.patch_index = LoadState::Loading;
        self.file_cache.clear();
        self.file_errors.clear();
        self.selected_file = None;
        self.selected_diff_file = None;
        self.file_list_paths = Arc::from([]);
        self.bump_diff_revision();
        vec![Effect::FetchPatch { id }]
    }

    fn on_detail_local(
        &mut self,
        id: PullRequestId,
        result: Result<Option<PullRequestDetail>, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        match result {
            Ok(Some(detail)) => self.apply_detail(detail),
            Ok(None) => {
                self.detail = LoadState::Loading;
            }
            Err(_) => {
                self.detail = LoadState::Loading;
            }
        }
    }

    fn on_detail_network(
        &mut self,
        id: PullRequestId,
        had_cache: bool,
        result: Result<PullRequestDetail, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        if had_cache {
            if let Ok(detail) = result {
                self.apply_detail(detail);
            }
            return;
        }
        match result {
            Ok(detail) => self.apply_detail(detail),
            Err(error) => {
                self.details_open.clear();
                self.overview_compose_cache = None;
                self.overview_segments.clear();
                self.detail = LoadState::Error(error);
            }
        }
    }

    fn apply_detail(&mut self, detail: PullRequestDetail) {
        self.details_open.clear();
        self.overview_compose_cache = None;
        self.overview_segments = parse_body_segments(&detail.body);
        self.detail = LoadState::Ready(detail);
    }

    fn on_patch_local(&mut self, id: PullRequestId, result: Result<Option<Patch>, ProviderError>) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        match result {
            Ok(Some(patch)) => {
                self.apply_indexed_patch(patch);
            }
            Ok(None) => {
                self.patch_index = LoadState::Loading;
            }
            Err(_) => {
                self.patch_index = LoadState::Loading;
            }
        }
    }

    fn on_patch_network(
        &mut self,
        id: PullRequestId,
        had_cache: bool,
        result: Result<Patch, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        if had_cache {
            if let Ok(patch) = result {
                self.file_cache.clear();
                self.file_errors.clear();
                self.apply_indexed_patch(patch);
            }
            return;
        }
        match result {
            Ok(patch) => self.apply_indexed_patch(patch),
            Err(error) => {
                self.file_list_paths = Arc::from([]);
                self.selected_file = None;
                self.selected_diff_file = None;
                self.patch_index = LoadState::Error(error);
                self.bump_diff_revision();
            }
        }
    }

    fn apply_indexed_patch(&mut self, patch: Patch) {
        match index_patch(patch) {
            Ok(index) => {
                self.file_list_paths = index.paths().cloned().collect::<Vec<_>>().into();
                if let Some(path) = index.first_path() {
                    self.selected_file = Some(path.clone());
                    self.patch_index = LoadState::Ready(index);
                    self.materialize_file(path);
                    return;
                }
                self.selected_file = None;
                self.selected_diff_file = None;
                self.patch_index = LoadState::Ready(index);
                self.bump_diff_revision();
            }
            Err(error) => {
                self.file_list_paths = Arc::from([]);
                self.selected_file = None;
                self.selected_diff_file = None;
                self.patch_index = LoadState::Error(error);
                self.bump_diff_revision();
            }
        }
    }

    fn select_file(&mut self, path: SharedString) {
        let LoadState::Ready(index) = &self.patch_index else {
            return;
        };
        if !index.paths().any(|entry| entry.as_str() == path.as_str()) {
            return;
        }
        self.selected_file = Some(path.clone());
        self.materialize_file(path);
    }

    fn materialize_file(&mut self, path: SharedString) {
        if let Some(file) = self.file_cache.get(&path) {
            self.selected_diff_file = Some(file.clone());
            self.bump_diff_revision();
            return;
        }
        if self.file_errors.contains_key(&path) {
            self.selected_diff_file = None;
            self.bump_diff_revision();
            return;
        }
        let LoadState::Ready(index) = &self.patch_index else {
            return;
        };
        match index.materialize(path.as_str()) {
            Ok(file) => {
                self.file_errors.remove(&path);
                self.file_cache.insert(path, file.clone());
                self.selected_diff_file = Some(file);
                self.bump_diff_revision();
            }
            Err(error) => {
                self.file_cache.remove(&path);
                self.file_errors.insert(path, error);
                self.selected_diff_file = None;
                self.bump_diff_revision();
            }
        }
    }

    fn on_views_bootstrap_finished(
        &mut self,
        views: Result<Vec<View>, ProviderError>,
        last_used: Result<Option<ViewId>, ProviderError>,
        scoped_repository: Result<Option<String>, ProviderError>,
    ) -> Vec<Effect> {
        let views = views.unwrap_or_default();
        let last_used = last_used.ok().flatten();
        self.repo_scope = scoped_repository.ok().flatten();

        let mut views = views;
        if views.is_empty() {
            views = View::seed_set();
            let launch = resolve_launch_view_id(&views, last_used.as_ref());
            self.active_view_id = Some(launch.clone());
            self.load_working_from_active_view();
            return vec![Effect::PersistViews {
                views,
                last_used: Some(launch),
                scoped_repository: self.repo_scope.clone(),
            }];
        }

        self.views = LoadState::Ready(Arc::from(views.clone()));
        let launch = resolve_launch_view_id(&views, last_used.as_ref());
        self.activate_view(launch);
        Vec::new()
    }

    fn on_views_persist_finished(&mut self, views: Result<Vec<View>, ProviderError>) {
        match views {
            Ok(list) => {
                self.views = LoadState::Ready(Arc::from(list));
                if self.saved_snapshot.is_none() {
                    self.load_working_from_active_view();
                }
            }
            Err(_) => {
                self.views = LoadState::Error(ProviderError::other("views persist failed"));
            }
        }
    }

    fn select_view(&mut self, id: ViewId) -> Vec<Effect> {
        if self.active_view_id.as_ref() == Some(&id) {
            return Vec::new();
        }
        self.activate_view(id.clone());
        vec![Effect::PersistViews {
            views: self.current_views_vec(),
            last_used: Some(id),
            scoped_repository: self.repo_scope.clone(),
        }]
    }

    fn set_repo_scope(&mut self, repo: Option<String>) -> Vec<Effect> {
        self.repo_scope = repo;
        vec![Effect::PersistViews {
            views: self.current_views_vec(),
            last_used: self.active_view_id.clone(),
            scoped_repository: self.repo_scope.clone(),
        }]
    }

    fn save_active_view(&mut self) -> Vec<Effect> {
        let Some(active) = self.active_view_id.clone() else {
            return Vec::new();
        };
        let LoadState::Ready(views) = &self.views else {
            return Vec::new();
        };
        let Some(mut view) = views.iter().find(|v| v.id == active).cloned() else {
            return Vec::new();
        };
        if !view.can_save_in_place() {
            return Vec::new();
        }
        view.filter = self.working_filter.clone();
        view.group_by = self.working_group_by;
        view.sort = self.working_sort.clone();
        let mut updated: Vec<View> = views.to_vec();
        if let Some(slot) = updated.iter_mut().find(|v| v.id == active) {
            *slot = view;
        }
        self.saved_snapshot = Some((
            self.working_filter.clone(),
            self.working_group_by,
            self.working_sort.clone(),
        ));
        vec![Effect::PersistViews {
            views: updated,
            last_used: Some(active),
            scoped_repository: self.repo_scope.clone(),
        }]
    }

    fn save_view_as_new(&mut self, name: String) -> Vec<Effect> {
        let LoadState::Ready(views) = &self.views else {
            return Vec::new();
        };
        let name = name.trim();
        if name.is_empty() {
            return Vec::new();
        }
        let id = ViewId::new(format!("user-{}", self.next_user_view_seq));
        self.next_user_view_seq += 1;
        let position = views.len() as i64;
        let view = View {
            id: id.clone(),
            name: name.to_string(),
            kind: ViewKind::User,
            position,
            filter: self.working_filter.clone(),
            group_by: self.working_group_by,
            sort: self.working_sort.clone(),
        };
        let mut updated = views.to_vec();
        updated.push(view);
        self.active_view_id = Some(id.clone());
        self.saved_snapshot = Some((
            self.working_filter.clone(),
            self.working_group_by,
            self.working_sort.clone(),
        ));
        vec![Effect::PersistViews {
            views: updated,
            last_used: Some(id),
            scoped_repository: self.repo_scope.clone(),
        }]
    }

    fn discard_view_edits(&mut self) {
        self.load_working_from_active_view();
    }

    fn rename_active_view(&mut self, name: String) -> Vec<Effect> {
        let Some(active) = self.active_view_id.clone() else {
            return Vec::new();
        };
        let LoadState::Ready(views) = &self.views else {
            return Vec::new();
        };
        let name = name.trim();
        if name.is_empty() {
            return Vec::new();
        }
        let mut updated: Vec<View> = views.to_vec();
        let Some(slot) = updated.iter_mut().find(|v| v.id == active) else {
            return Vec::new();
        };
        if !slot.can_rename() {
            return Vec::new();
        }
        slot.name = name.to_string();
        vec![Effect::PersistViews {
            views: updated,
            last_used: Some(active),
            scoped_repository: self.repo_scope.clone(),
        }]
    }

    fn delete_active_view(&mut self) -> Vec<Effect> {
        let Some(active) = self.active_view_id.clone() else {
            return Vec::new();
        };
        let LoadState::Ready(views) = &self.views else {
            return Vec::new();
        };
        let Some(view) = views.iter().find(|v| v.id == active) else {
            return Vec::new();
        };
        if !view.can_delete() {
            return Vec::new();
        }
        let mut updated: Vec<View> = views.iter().filter(|v| v.id != active).cloned().collect();
        for (ix, view) in updated.iter_mut().enumerate() {
            view.position = ix as i64;
        }
        let fallback = resolve_launch_view_id(&updated, None);
        self.views = LoadState::Ready(Arc::from(updated.clone()));
        self.activate_view(fallback.clone());
        vec![Effect::PersistViews {
            views: updated,
            last_used: Some(fallback),
            scoped_repository: self.repo_scope.clone(),
        }]
    }

    fn reorder_views(&mut self, order: Vec<ViewId>) -> Vec<Effect> {
        let LoadState::Ready(views) = &self.views else {
            return Vec::new();
        };
        if order.len() != views.len() {
            return Vec::new();
        }
        let mut by_id: std::collections::HashMap<&ViewId, &View> =
            views.iter().map(|v| (&v.id, v)).collect();
        let mut reordered = Vec::with_capacity(order.len());
        for (position, id) in order.iter().enumerate() {
            let Some(view) = by_id.remove(id) else {
                return Vec::new();
            };
            let mut copy = (*view).clone();
            copy.position = position as i64;
            reordered.push(copy);
        }
        if !by_id.is_empty() {
            return Vec::new();
        }
        vec![Effect::PersistViews {
            views: reordered,
            last_used: self.active_view_id.clone(),
            scoped_repository: self.repo_scope.clone(),
        }]
    }

    fn activate_view(&mut self, id: ViewId) {
        self.active_view_id = Some(id);
        self.load_working_from_active_view();
    }

    fn current_views_vec(&self) -> Vec<View> {
        match &self.views {
            LoadState::Ready(views) => views.to_vec(),
            _ => Vec::new(),
        }
    }

    fn load_working_from_active_view(&mut self) {
        let Some(active) = &self.active_view_id else {
            self.working_filter = ViewFilter::default();
            self.working_group_by = ViewGroupBy::default();
            self.working_sort = PullRequestListSort::default();
            self.saved_snapshot = None;
            return;
        };
        let LoadState::Ready(views) = &self.views else {
            return;
        };
        let Some(view) = views.iter().find(|v| &v.id == active) else {
            return;
        };
        self.working_filter = view.filter.clone();
        self.working_group_by = view.group_by;
        self.working_sort = view.sort.clone();
        self.saved_snapshot = Some((view.filter.clone(), view.group_by, view.sort.clone()));
    }

    fn add_annotation(&mut self, range: DiffLineRange) {
        let Some(pr_id) = self.selected_id.as_ref().map(|id| id.as_str().to_string()) else {
            return;
        };
        let (start, end) = (
            range.start().min(range.end()),
            range.start().max(range.end()),
        );
        let lines = if start == end {
            format!("line {end}")
        } else {
            format!("lines {start}-{end}")
        };
        self.annotation_seq += 1;
        let id = format!("annotation-{}", self.annotation_seq);
        self.annotations.entry(pr_id).or_default().push(Annotation {
            id,
            range,
            body: format!("Annotation on {lines}.").into(),
        });
    }

    fn current_annotations(&self) -> Vec<AnnotationView> {
        let Some(pr_id) = self.selected_id.as_ref() else {
            return Vec::new();
        };
        self.annotations
            .get(pr_id.as_str())
            .map(|items| {
                items
                    .iter()
                    .map(|a| AnnotationView {
                        id: a.id.clone(),
                        range: a.range.clone(),
                        body: a.body.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn details_is_open(&self, block: &DetailsBlock) -> bool {
        details_effective_open(block, self.details_open.get(&block.id).copied())
    }

    fn overview_compose_cache_key(&self, pr_id: &PullRequestId) -> (PullRequestId, Vec<usize>) {
        let open_details = open_details_ids(&self.overview_segments, &|block| {
            self.details_is_open(block)
        });
        (pr_id.clone(), open_details)
    }

    fn overview_cache_matches(&self, pr_id: &PullRequestId) -> bool {
        let key = self.overview_compose_cache_key(pr_id);
        self.overview_compose_cache
            .as_ref()
            .is_some_and(|cache| cache.pr_id == key.0 && cache.open_details == key.1)
    }

    fn cached_overview_markdown(&mut self, detail: &PullRequestDetail) -> SharedString {
        if self.overview_cache_matches(&detail.id)
            && let Some(cache) = &self.overview_compose_cache
        {
            return cache.markdown.clone();
        }

        let key = self.overview_compose_cache_key(&detail.id);
        let reviewers = if detail.reviewers.is_empty() {
            "None".to_string()
        } else {
            detail
                .reviewers
                .iter()
                .map(|r| r.login.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let labels = if detail.labels.is_empty() {
            "None".to_string()
        } else {
            detail
                .labels
                .iter()
                .map(|l| l.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let is_open = |block: &DetailsBlock| self.details_is_open(block);
        let markdown = SharedString::from(compose_overview_markdown(OverviewMarkdownInput {
            title: &detail.title,
            author: &detail.author_login,
            is_draft: detail.is_draft,
            web_url: &detail.web_url,
            labels: &labels,
            reviewers: &reviewers,
            body: &detail.body,
            segments: &self.overview_segments,
            is_open: &is_open,
        }));
        self.overview_compose_cache = Some(OverviewComposeCache {
            pr_id: key.0,
            open_details: key.1,
            markdown: markdown.clone(),
        });
        markdown
    }
}

impl Default for ReviewSession {
    fn default() -> Self {
        Self::new()
    }
}

fn format_sync_time(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use gpui_kit::component::diff::DiffSide;
    use reviewdesk_core::ReviewState;

    const MULTI_FILE: &str = "diff --git a/new b/new\nnew file mode 100644\n--- /dev/null\n+++ b/new\n@@ -0,0 +1 @@\n+added\ndiff --git a/old b/old\ndeleted file mode 100644\n--- a/old\n+++ /dev/null\n@@ -1 +0,0 @@\n-deleted\n";

    fn pr_id(raw: &str) -> PullRequestId {
        PullRequestId::new(raw).unwrap()
    }

    fn sample_item(id: &str) -> PullRequestListItem {
        PullRequestListItem {
            id: pr_id(id),
            title: format!("Title {id}"),
            web_url: format!("https://example.test/{id}"),
            updated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            author_login: "alice".into(),
            author_avatar_url: None,
            repo_label: "acme/widgets".into(),
            labels: Vec::new(),
            review_state: ReviewState::Pending,
            is_draft: false,
        }
    }

    fn sample_detail(id: &str, body: &str) -> PullRequestDetail {
        PullRequestDetail {
            id: pr_id(id),
            title: format!("Title {id}"),
            body: body.into(),
            author_login: "alice".into(),
            author_avatar_url: None,
            is_draft: false,
            labels: Vec::new(),
            web_url: format!("https://example.test/{id}"),
            reviewers: Vec::new(),
        }
    }

    #[test]
    fn boot_emits_query_sync_and_timer() {
        let mut session = ReviewSession::new();
        let effects = session.apply(Intent::Boot);
        assert_eq!(
            effects,
            vec![
                Effect::QueryPullRequests,
                Effect::SyncPullRequests { generation: 1 },
                Effect::BootstrapPullRequestsViews,
                Effect::FetchViewerLogin,
                Effect::ArmPullRequestsRefreshTimer,
            ]
        );
        assert!(matches!(
            session.view().pull_requests_pane.cache,
            LoadState::Loading
        ));
        assert!(session.view().pull_requests_pane.sync.syncing);
    }

    #[test]
    fn selection_emits_parallel_fetches_and_resets_review_state() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::Boot);
        let _ = session.apply(Intent::PullRequestListQueryFinished {
            result: Ok(vec![sample_item("pr-1"), sample_item("pr-2")]),
        });
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(sample_detail("pr-1", "hello"))),
        });
        let _ = session.apply(Intent::PatchLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(Patch::new(MULTI_FILE))),
        });
        assert!(matches!(session.view().detail, LoadState::Ready(_)));
        assert_eq!(session.view().file_paths.len(), 2);

        let effects = session.apply(Intent::SelectPullRequest(pr_id("pr-2")));
        assert_eq!(
            effects,
            vec![
                Effect::FetchDetail { id: pr_id("pr-2") },
                Effect::FetchPatch { id: pr_id("pr-2") },
            ]
        );
        let view = session.view();
        assert_eq!(view.selected_id, Some(pr_id("pr-2")));
        assert_eq!(view.review_tab, ReviewTab::Overview);
        assert!(matches!(view.detail, LoadState::Idle));
        assert!(matches!(view.patch_index, LoadState::Idle));
        assert!(view.file_paths.is_empty());
        assert!(view.selected_file.is_none());
        assert!(view.overview_markdown.is_none());
    }

    #[test]
    fn missing_selection_from_pull_requests_reload_clears_review_pane() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(sample_detail("pr-1", "body"))),
        });
        let _ = session.apply(Intent::PullRequestListQueryFinished {
            result: Ok(vec![sample_item("pr-other")]),
        });
        let view = session.view();
        assert!(view.selected_id.is_none());
        assert!(matches!(view.detail, LoadState::Idle));
        assert!(matches!(view.patch_index, LoadState::Idle));
    }

    #[test]
    fn stale_pull_requests_sync_finish_is_ignored() {
        let mut session = ReviewSession::new();
        let first = session.apply(Intent::RefreshPullRequests);
        assert_eq!(first, vec![Effect::SyncPullRequests { generation: 1 }]);
        let second = session.apply(Intent::RefreshPullRequests);
        assert_eq!(second, vec![Effect::SyncPullRequests { generation: 2 }]);

        let stale = session.apply(Intent::PullRequestListSyncFinished {
            generation: 1,
            result: Ok(()),
        });
        assert!(stale.is_empty());
        assert!(session.view().pull_requests_pane.sync.syncing);

        let fresh = session.apply(Intent::PullRequestListSyncFinished {
            generation: 2,
            result: Ok(()),
        });
        assert_eq!(fresh, vec![Effect::QueryPullRequests]);
        assert!(!session.view().pull_requests_pane.sync.syncing);
        assert!(
            session
                .view()
                .pull_requests_pane
                .sync
                .last_success
                .is_some()
        );
    }

    #[test]
    fn detail_cache_then_refresh_keeps_ready_on_network_failure() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(sample_detail("pr-1", "cached"))),
        });
        assert!(matches!(session.view().detail, LoadState::Ready(_)));

        let _ = session.apply(Intent::DetailNetworkFinished {
            id: pr_id("pr-1"),
            had_cache: true,
            result: Err(ProviderError::other("network blip")),
        });
        let LoadState::Ready(detail) = session.view().detail else {
            panic!("expected cached detail to remain");
        };
        assert_eq!(detail.body, "cached");
    }

    #[test]
    fn detail_cache_miss_shows_loading_then_error_without_cache() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(None),
        });
        assert!(matches!(session.view().detail, LoadState::Loading));

        let _ = session.apply(Intent::DetailNetworkFinished {
            id: pr_id("pr-1"),
            had_cache: false,
            result: Err(ProviderError::NotFound),
        });
        assert!(matches!(
            session.view().detail,
            LoadState::Error(ProviderError::NotFound)
        ));
    }

    #[test]
    fn stale_detail_completion_after_reselection_is_ignored() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-2")));
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(sample_detail("pr-1", "stale"))),
        });
        assert!(matches!(session.view().detail, LoadState::Idle));
        assert_eq!(session.view().selected_id, Some(pr_id("pr-2")));
    }

    #[test]
    fn stale_patch_completion_after_reselection_is_ignored() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-2")));
        let _ = session.apply(Intent::PatchLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(Patch::new(MULTI_FILE))),
        });
        assert!(matches!(session.view().patch_index, LoadState::Idle));
        assert!(session.view().file_paths.is_empty());
    }

    #[test]
    fn overview_markdown_changes_when_details_toggled() {
        let body = "<details><summary>Notes</summary>\n\nsecret\n\n</details>\n";
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(sample_detail("pr-1", body))),
        });
        let closed = session
            .view()
            .overview_markdown
            .expect("markdown")
            .to_string();
        assert!(closed.contains("Notes"));
        assert!(!closed.contains("secret"));

        let _ = session.apply(Intent::ToggleDetails {
            id: 0,
            currently_open: false,
        });
        let open = session
            .view()
            .overview_markdown
            .expect("markdown")
            .to_string();
        assert!(open.contains("secret"));
        assert_ne!(closed, open);
    }

    #[test]
    fn successful_patch_index_auto_selects_first_path() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::PatchLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(Patch::new(MULTI_FILE))),
        });
        let view = session.view();
        assert!(matches!(view.patch_index, LoadState::Ready(())));
        assert_eq!(view.selected_file.as_deref(), Some("new"));
        assert!(view.selected_diff_file.is_some());
        assert!(view.selected_file_error.is_none());
    }

    #[test]
    fn select_file_and_retry_surface_materialize_errors() {
        let mut broken = MULTI_FILE.to_owned();
        broken.push_str("diff --git a/bad b/bad\n--- a/bad\n+++ b/bad\n@@ not a hunk @@\n");
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::PatchLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(Patch::new(broken))),
        });
        let _ = session.apply(Intent::SelectFile("bad".into()));
        assert!(session.view().selected_file_error.is_some());
        assert!(session.view().selected_diff_file.is_none());

        // Retry clears error slot then re-materializes; bad file stays an error.
        let _ = session.apply(Intent::RetryFile("bad".into()));
        assert!(session.view().selected_file_error.is_some());
    }

    #[test]
    fn patch_refresh_failure_with_cache_keeps_ready() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::PatchLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(Patch::new(MULTI_FILE))),
        });
        let _ = session.apply(Intent::PatchNetworkFinished {
            id: pr_id("pr-1"),
            had_cache: true,
            result: Err(ProviderError::other("blip")),
        });
        assert!(matches!(session.view().patch_index, LoadState::Ready(())));
        assert_eq!(session.view().selected_file.as_deref(), Some("new"));
    }

    #[test]
    fn views_bootstrap_seeds_when_store_empty() {
        let mut session = ReviewSession::new();
        let effects = session.apply(Intent::PullRequestsViewsBootstrapFinished {
            views: Ok(Vec::new()),
            last_used: Ok(None),
            scoped_repository: Ok(None),
        });
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], Effect::PersistViews { .. }));
        assert_eq!(
            session.active_view_id.as_ref().map(|id| id.as_str()),
            Some(reviewdesk_core::VIEW_ID_NEEDS_MY_REVIEW)
        );
    }

    #[test]
    fn reorder_views_emits_persist_effect() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::PullRequestsViewsBootstrapFinished {
            views: Ok(View::seed_set()),
            last_used: Ok(None),
            scoped_repository: Ok(None),
        });
        let order: Vec<ViewId> = View::seed_set().into_iter().rev().map(|v| v.id).collect();
        let effects = session.apply(Intent::ReorderViews { order });
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], Effect::PersistViews { .. }));
    }

    #[test]
    fn empty_store_vs_zero_match_in_pane() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::PullRequestsViewsBootstrapFinished {
            views: Ok(View::seed_set()),
            last_used: Ok(None),
            scoped_repository: Ok(None),
        });
        let _ = session.apply(Intent::PullRequestListQueryFinished {
            result: Ok(Vec::new()),
        });
        assert_eq!(
            session.view().pull_requests_pane.empty,
            Some(reviewdesk_core::PullRequestsEmptyKind::EmptyStore)
        );

        let _ = session.apply(Intent::PullRequestListQueryFinished {
            result: Ok(vec![sample_item("acme/widgets#1")]),
        });
        let _ = session.apply(Intent::SelectView(ViewId::new(
            reviewdesk_core::VIEW_ID_MINE,
        )));
        let _ = session.apply(Intent::ViewerLoginFinished {
            result: Ok("not-author".into()),
        });
        assert_eq!(
            session.view().pull_requests_pane.empty,
            Some(reviewdesk_core::PullRequestsEmptyKind::ZeroMatch)
        );
    }

    #[test]
    fn annotations_are_session_scoped_per_pull_request() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let range = DiffLineRange::new("new", DiffSide::Modified, 1, 1);
        let _ = session.apply(Intent::AddAnnotation(range));
        assert_eq!(session.view().annotations.len(), 1);
        assert!(
            session.view().annotations[0]
                .body
                .as_str()
                .contains("line 1")
        );

        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-2")));
        assert!(session.view().annotations.is_empty());
    }
}
