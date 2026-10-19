//! In-process session kernel for Review Desk window policy.
//!
//! Callers and tests use construction, [`ReviewSession::apply`], and
//! [`ReviewSession::view`] only. I/O is expressed as [`Effect`]s; the GPUI shell
//! runs them and feeds completions back as [`Intent`]s.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use gpui_kit::SharedString;
use gpui_kit::component::diff::{DiffFile, DiffLineRange, DiffMode, DiffSide};
use reviewdesk_core::{
    DiffAnchor, DiffLineSide, NotificationReason, NotificationThread, NotificationThreadId, Patch,
    PipelinesSnapshot, ProviderError, PullRequestDetail, PullRequestId, PullRequestListItem,
    PullRequestListSort, PullRequestThreads, RerunAction, RerunTarget, ReviewSubmitEvent,
    ThreadStatus, View, ViewFilter, ViewGroupBy, ViewId, ViewKind, resolve_launch_view_id,
};

use crate::pull_requests_pane::{
    PullRequestsPaneInputs, PullRequestsPaneView, project_pull_requests_pane,
};

use crate::metadata_write::MetadataWrite;

/// Fallback Inbox poll cadence when the Provider sends no `X-Poll-Interval`.
pub const DEFAULT_INBOX_POLL: Duration = Duration::from_secs(60);

/// Operator copy when Inbox sync hits classic PAT / scope failures.
pub const INBOX_AUTH_FIX_COPY: &str =
    "Inbox needs a classic PAT with `notifications` or `repo`. See docs/auth-github.md.";

use crate::overview_body::{
    BodySegment, DetailsBlock, OverviewMarkdownInput, compose_overview_markdown,
    details_effective_open, open_details_ids, parse_body_segments,
};
use crate::patch_index::{PatchIndex, index_patch};

/// User gestures, boot, timer ticks, and async I/O completions.
#[derive(Debug)]
pub enum Intent {
    Boot,
    SelectSidebarTab(SidebarTab),
    RefreshPullRequests,
    RefreshInbox,
    SelectNotification(NotificationThreadId),
    NotificationDone(NotificationThreadId),
    SelectPullRequest(PullRequestId),
    BackToPullRequests,
    SetReviewTab(ReviewTab),
    ActivatePipelines,
    RerunPipeline {
        pipeline_id: String,
    },
    ToggleMetadataDock,
    FoldMetadata,
    HideMetadata,
    ToggleDetails {
        id: usize,
        currently_open: bool,
    },
    SelectFile(SharedString),
    RetryFile(SharedString),
    ToggleDiffMode,
    AddDiffReviewComment(DiffLineRange),
    SubmitPendingReview {
        event: ReviewSubmitEvent,
        summary_body: Option<String>,
    },
    DiscardPendingReview,
    ToggleHideResolvedThreads,
    SetConversationReplyThread(Option<SharedString>),
    SetThreadStatus {
        thread_id: String,
        status: ThreadStatus,
    },
    PublishTimelineComment {
        body: String,
    },
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
    /// Reset to All PRs + clear repo scope (Linear “Clear filters”).
    ClearListFilters,
    SaveActiveView,
    #[allow(dead_code)] // View CRUD chrome deferred; intents kept for session API.
    SaveViewAsNew {
        name: String,
    },
    DiscardViewEdits,
    RenameActiveView {
        name: String,
    },
    #[allow(dead_code)]
    RequestRenameActiveView,
    #[allow(dead_code)]
    DeleteActiveView,
    /// Drag-reorder / programmatic order; UI today uses left/right moves.
    #[allow(dead_code)]
    ReorderViews {
        order: Vec<ViewId>,
    },
    #[allow(dead_code)]
    MoveActiveViewLeft,
    #[allow(dead_code)]
    MoveActiveViewRight,
    ToggleWorkingFilterAuthor,
    ToggleWorkingFilterReviewer,
    CycleWorkingFilterDrafts,
    CycleWorkingFilterUpdatedWithin,
    ToggleWorkingFilterReviewState {
        state: reviewdesk_core::ReviewState,
    },
    ViewsPersistFinished {
        views: Result<Vec<View>, ProviderError>,
    },
    InboxQueryFinished {
        result: Result<Vec<NotificationThread>, ProviderError>,
    },
    InboxSyncFinished {
        generation: u64,
        result: Result<InboxSyncOutcome, ProviderError>,
    },
    NotificationReadFinished {
        id: NotificationThreadId,
        result: Result<(), ProviderError>,
    },
    NotificationDoneFinished {
        id: NotificationThreadId,
        result: Result<(), ProviderError>,
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
    MetadataWrite(MetadataWrite),
    MetadataWriteFinished {
        id: PullRequestId,
        result: Result<PullRequestDetail, ProviderError>,
    },
    ThreadsLocalFinished {
        id: PullRequestId,
        result: Result<Option<PullRequestThreads>, ProviderError>,
    },
    ThreadsNetworkFinished {
        id: PullRequestId,
        had_cache: bool,
        result: Result<PullRequestThreads, ProviderError>,
    },
    ThreadsWriteFinished {
        id: PullRequestId,
        result: Result<PullRequestThreads, ProviderError>,
    },
    PipelinesLocalFinished {
        id: PullRequestId,
        result: Result<Option<PipelinesSnapshot>, ProviderError>,
    },
    PipelinesNetworkFinished {
        id: PullRequestId,
        had_cache: bool,
        result: Result<PipelinesSnapshot, ProviderError>,
    },
    RerunPipelineFinished {
        id: PullRequestId,
        result: Result<PipelinesSnapshot, ProviderError>,
    },
}

/// Work the shell must perform against LocalStore / Provider / timers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    QueryPullRequests,
    SyncPullRequests {
        generation: u64,
    },
    QueryInbox,
    SyncInbox {
        generation: u64,
        if_modified_since: Option<String>,
    },
    ArmInboxPollTimer {
        interval: Duration,
    },
    MarkNotificationRead {
        id: NotificationThreadId,
    },
    MarkNotificationDone {
        id: NotificationThreadId,
    },
    FetchDetail {
        id: PullRequestId,
    },
    FetchPatch {
        id: PullRequestId,
    },
    FetchThreads {
        id: PullRequestId,
    },
    /// Network refresh that keeps the cached projection (had_cache path).
    #[allow(dead_code)]
    RefreshThreads {
        id: PullRequestId,
    },
    SaveDiffReviewComment {
        id: PullRequestId,
        anchor: DiffAnchor,
        body: String,
    },
    SubmitPendingReviewWrite {
        id: PullRequestId,
        event: ReviewSubmitEvent,
        summary_body: Option<String>,
    },
    DiscardPendingReviewWrite {
        id: PullRequestId,
    },
    FetchPipelines {
        id: PullRequestId,
    },
    RerunPipeline {
        id: PullRequestId,
        target: RerunTarget,
    },
    SetThreadStatusWrite {
        id: PullRequestId,
        thread_id: String,
        status: ThreadStatus,
    },
    PublishTimelineCommentWrite {
        id: PullRequestId,
        body: String,
    },
    ArmPullRequestsRefreshTimer,
    BootstrapPullRequestsViews,
    FetchViewerLogin,
    PersistViews {
        views: Vec<View>,
        last_used: Option<ViewId>,
        scoped_repository: Option<String>,
    },
    RunMetadataWrite {
        id: PullRequestId,
        write: MetadataWrite,
    },
    PromptRenameView {
        default_name: String,
    },
}

/// Which list the sidebar has selected for the left pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Inbox,
    PullRequests,
}

/// Outcome of one Inbox poll, surfaced back into the kernel as an [`Intent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxSyncOutcome {
    pub poll_interval: Option<Duration>,
    pub last_modified: Option<String>,
    pub not_modified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewTab {
    Overview,
    Pipelines,
    Conversation,
    Changes,
}

/// Right-dock visibility policy for the Metadata panel.
///
/// `Open` is the full panel; `Folded` collapses to a status+merge strip;
/// `Hidden` removes the panel entirely (restored only from Review pane chrome).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataDock {
    Open,
    Folded,
    Hidden,
}

impl MetadataDock {
    /// Auto policy when the active tab changes: open on Overview, folded
    /// elsewhere. A hidden dock stays hidden — hide restores via chrome only.
    fn for_tab(self, tab: ReviewTab) -> Self {
        if self == MetadataDock::Hidden {
            return MetadataDock::Hidden;
        }
        match tab {
            ReviewTab::Overview => MetadataDock::Open,
            _ => MetadataDock::Folded,
        }
    }
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InboxSyncView {
    pub syncing: bool,
    pub last_success: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl InboxSyncView {
    pub fn status_label(&self) -> String {
        if self.syncing {
            return "Syncing inbox…".into();
        }
        if let Some(error) = &self.error {
            return format!("Inbox sync failed: {error}");
        }
        if let Some(at) = self.last_success {
            return format!("Inbox synced {}", format_sync_time(at));
        }
        "Inbox not synced yet".into()
    }

    pub fn empty_placeholder(&self) -> &'static str {
        "You're caught up — no unread notifications."
    }
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
pub struct ReviewCommentView {
    pub id: String,
    pub author_login: SharedString,
    pub body: SharedString,
    pub is_pending: bool,
    pub suggestion: Option<SharedString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadView {
    pub id: String,
    pub anchor: Option<DiffAnchor>,
    pub status: ThreadStatus,
    pub comments: Arc<[ReviewCommentView]>,
}

/// Paint projection. No LocalStore / Provider access from here.
#[derive(Clone)]
pub struct SessionView {
    pub sidebar_tab: SidebarTab,
    pub inbox: LoadState<Arc<[NotificationThread]>>,
    pub inbox_sync: InboxSyncView,
    pub inbox_auth_error: Option<String>,
    pub pull_requests_pane: PullRequestsPaneView,
    pub selected_id: Option<PullRequestId>,
    pub review_tab: ReviewTab,
    pub metadata_dock: MetadataDock,
    pub detail: LoadState<PullRequestDetail>,
    pub pipelines: LoadState<PipelinesSnapshot>,
    pub pipelines_rerun_busy: bool,
    pub overview_markdown: Option<SharedString>,
    pub patch_index: LoadState<()>,
    pub file_paths: Arc<[SharedString]>,
    pub selected_file: Option<SharedString>,
    pub selected_diff_file: Option<DiffFile>,
    pub selected_file_error: Option<ProviderError>,
    pub diff_mode: DiffMode,
    /// Bumps when Diff surface inputs change (mode, selected file, materialization).
    pub diff_revision: u64,
    /// Shared Thread set for Conversation (#80) and Diff surfaces.
    pub shared_threads: Arc<[ThreadView]>,
    pub has_pending_review: bool,
    pub threads_syncing: bool,
    pub metadata_write_pending: bool,
    pub metadata_write_error: Option<String>,
    pub threads: LoadState<()>,
    pub hide_resolved_threads: bool,
    pub conversation_reply_thread: Option<SharedString>,
}

#[derive(Clone, PartialEq, Eq)]
struct OverviewComposeCache {
    pr_id: PullRequestId,
    open_details: Vec<usize>,
    markdown: SharedString,
}

/// Sole external seam for window session policy.
pub struct ReviewSession {
    sidebar_tab: SidebarTab,
    inbox: LoadState<Arc<[NotificationThread]>>,
    inbox_sync: InboxSyncView,
    inbox_auth_error: Option<String>,
    inbox_sync_generation: u64,
    inbox_last_modified: Option<String>,
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
    metadata_dock: MetadataDock,
    detail: LoadState<PullRequestDetail>,
    pipelines: LoadState<PipelinesSnapshot>,
    pipelines_rerun_busy: bool,
    patch_index: LoadState<PatchIndex>,
    file_cache: HashMap<SharedString, DiffFile>,
    file_errors: HashMap<SharedString, ProviderError>,
    selected_file: Option<SharedString>,
    selected_diff_file: Option<DiffFile>,
    diff_mode: DiffMode,
    diff_revision: u64,
    threads: LoadState<PullRequestThreads>,
    threads_syncing: bool,
    hide_resolved_threads: bool,
    conversation_reply_thread: Option<SharedString>,
    overview_segments: Vec<BodySegment>,
    details_open: HashMap<usize, bool>,
    overview_compose_cache: Option<OverviewComposeCache>,
    file_list_paths: Arc<[SharedString]>,
    metadata_write_pending: bool,
    metadata_write_error: Option<String>,
}

impl ReviewSession {
    pub fn new() -> Self {
        Self {
            sidebar_tab: SidebarTab::Inbox,
            inbox: LoadState::Loading,
            inbox_sync: InboxSyncView::default(),
            inbox_auth_error: None,
            inbox_sync_generation: 0,
            inbox_last_modified: None,
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
            metadata_dock: MetadataDock::Open,
            detail: LoadState::Idle,
            pipelines: LoadState::Idle,
            pipelines_rerun_busy: false,
            patch_index: LoadState::Idle,
            file_cache: HashMap::new(),
            file_errors: HashMap::new(),
            selected_file: None,
            selected_diff_file: None,
            diff_mode: DiffMode::Split,
            diff_revision: 0,
            threads: LoadState::Idle,
            threads_syncing: false,
            hide_resolved_threads: false,
            conversation_reply_thread: None,
            overview_segments: Vec::new(),
            details_open: HashMap::new(),
            overview_compose_cache: None,
            file_list_paths: Arc::from([]),
            metadata_write_pending: false,
            metadata_write_error: None,
        }
    }

    pub fn apply(&mut self, intent: Intent) -> Vec<Effect> {
        match intent {
            Intent::Boot => self.boot(),
            Intent::SelectSidebarTab(tab) => {
                self.sidebar_tab = tab;
                Vec::new()
            }
            Intent::RefreshPullRequests => self.begin_pull_requests_sync(),
            Intent::RefreshInbox => self.begin_inbox_sync(),
            Intent::SelectNotification(id) => self.select_notification(id),
            Intent::NotificationDone(id) => vec![Effect::MarkNotificationDone { id }],
            Intent::SelectPullRequest(id) => self.select_pull_request(id),
            Intent::BackToPullRequests => {
                self.clear_selection();
                Vec::new()
            }
            Intent::SetReviewTab(tab) => {
                self.review_tab = tab;
                self.metadata_dock = self.metadata_dock.for_tab(tab);
                Vec::new()
            }
            Intent::ActivatePipelines => {
                self.review_tab = ReviewTab::Pipelines;
                self.metadata_dock = self.metadata_dock.for_tab(ReviewTab::Pipelines);
                Vec::new()
            }
            Intent::RerunPipeline { pipeline_id } => self.begin_rerun_pipeline(pipeline_id),
            Intent::ToggleMetadataDock => {
                self.metadata_dock = match self.metadata_dock {
                    MetadataDock::Open => MetadataDock::Hidden,
                    MetadataDock::Folded | MetadataDock::Hidden => MetadataDock::Open,
                };
                Vec::new()
            }
            Intent::FoldMetadata => {
                self.metadata_dock = MetadataDock::Folded;
                Vec::new()
            }
            Intent::HideMetadata => {
                self.metadata_dock = MetadataDock::Hidden;
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
            Intent::AddDiffReviewComment(range) => self.begin_add_diff_review_comment(range),
            Intent::SubmitPendingReview {
                event,
                summary_body,
            } => self.begin_submit_pending_review(event, summary_body),
            Intent::DiscardPendingReview => self.begin_discard_pending_review(),
            Intent::ToggleHideResolvedThreads => {
                self.hide_resolved_threads = !self.hide_resolved_threads;
                Vec::new()
            }
            Intent::SetConversationReplyThread(thread_id) => {
                self.conversation_reply_thread = thread_id;
                Vec::new()
            }
            Intent::SetThreadStatus { thread_id, status } => {
                self.begin_set_thread_status(thread_id, status)
            }
            Intent::PublishTimelineComment { body } => self.begin_publish_timeline_comment(body),
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
            Intent::ClearListFilters => self.clear_list_filters(),
            Intent::SaveActiveView => self.save_active_view(),
            Intent::SaveViewAsNew { name } => self.save_view_as_new(name),
            Intent::DiscardViewEdits => {
                self.discard_view_edits();
                Vec::new()
            }
            Intent::RenameActiveView { name } => self.rename_active_view(name),
            Intent::RequestRenameActiveView => self.request_rename_active_view(),
            Intent::DeleteActiveView => self.delete_active_view(),
            Intent::ReorderViews { order } => self.reorder_views(order),
            Intent::MoveActiveViewLeft => self.move_active_view(-1),
            Intent::MoveActiveViewRight => self.move_active_view(1),
            Intent::ToggleWorkingFilterAuthor => {
                self.working_filter.i_am_author = !self.working_filter.i_am_author;
                Vec::new()
            }
            Intent::ToggleWorkingFilterReviewer => {
                self.working_filter.i_am_reviewer = !self.working_filter.i_am_reviewer;
                Vec::new()
            }
            Intent::CycleWorkingFilterDrafts => {
                use reviewdesk_core::DraftFilter;
                self.working_filter.drafts = match self.working_filter.drafts {
                    DraftFilter::All => DraftFilter::ReadyOnly,
                    DraftFilter::ReadyOnly => DraftFilter::DraftsOnly,
                    DraftFilter::DraftsOnly => DraftFilter::All,
                };
                Vec::new()
            }
            Intent::CycleWorkingFilterUpdatedWithin => {
                use reviewdesk_core::UpdatedWithin;
                self.working_filter.updated_within = match self.working_filter.updated_within {
                    UpdatedWithin::Any => UpdatedWithin::Last24h,
                    UpdatedWithin::Last24h => UpdatedWithin::Last7d,
                    UpdatedWithin::Last7d => UpdatedWithin::Last30d,
                    UpdatedWithin::Last30d => UpdatedWithin::Any,
                };
                Vec::new()
            }
            Intent::ToggleWorkingFilterReviewState { state } => {
                if let Some(pos) = self
                    .working_filter
                    .review_states
                    .iter()
                    .position(|s| *s == state)
                {
                    self.working_filter.review_states.remove(pos);
                } else {
                    self.working_filter.review_states.push(state);
                }
                Vec::new()
            }
            Intent::ViewsPersistFinished { views } => {
                self.on_views_persist_finished(views);
                Vec::new()
            }
            Intent::InboxQueryFinished { result } => {
                self.on_inbox_query_finished(result);
                Vec::new()
            }
            Intent::InboxSyncFinished { generation, result } => {
                self.on_inbox_sync_finished(generation, result)
            }
            Intent::NotificationReadFinished { id, result } => {
                self.on_notification_read_finished(id, result)
            }
            Intent::NotificationDoneFinished { id, result } => {
                self.on_notification_done_finished(id, result)
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
            Intent::MetadataWrite(write) => self.begin_metadata_write(write),
            Intent::MetadataWriteFinished { id, result } => {
                self.on_metadata_write_finished(id, result);
                Vec::new()
            }
            Intent::ThreadsLocalFinished { id, result } => {
                self.on_threads_local(id, result);
                Vec::new()
            }
            Intent::ThreadsNetworkFinished {
                id,
                had_cache,
                result,
            } => {
                self.on_threads_network(id, had_cache, result);
                Vec::new()
            }
            Intent::ThreadsWriteFinished { id, result } => {
                self.on_threads_write_finished(id, result);
                Vec::new()
            }
            Intent::PipelinesLocalFinished { id, result } => {
                self.on_pipelines_local(id, result);
                Vec::new()
            }
            Intent::PipelinesNetworkFinished {
                id,
                had_cache,
                result,
            } => {
                self.on_pipelines_network(id, had_cache, result);
                Vec::new()
            }
            Intent::RerunPipelineFinished { id, result } => {
                self.on_rerun_pipeline_finished(id, result);
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
        let (shared_threads, has_pending_review) = self.thread_projection();
        let patch_index = match &self.patch_index {
            LoadState::Idle => LoadState::Idle,
            LoadState::Loading => LoadState::Loading,
            LoadState::Ready(_) => LoadState::Ready(()),
            LoadState::Error(error) => LoadState::Error(error.clone()),
        };
        let threads = match &self.threads {
            LoadState::Idle => LoadState::Idle,
            LoadState::Loading => LoadState::Loading,
            LoadState::Ready(_) => LoadState::Ready(()),
            LoadState::Error(error) => LoadState::Error(error.clone()),
        };

        SessionView {
            sidebar_tab: self.sidebar_tab,
            inbox: self.inbox.clone(),
            inbox_sync: self.inbox_sync.clone(),
            inbox_auth_error: self.inbox_auth_error.clone(),
            pull_requests_pane: project_pull_requests_pane(&PullRequestsPaneInputs {
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
            }),
            selected_id: self.selected_id.clone(),
            review_tab: self.review_tab,
            metadata_dock: self.metadata_dock,
            detail: self.detail.clone(),
            pipelines: self.pipelines.clone(),
            pipelines_rerun_busy: self.pipelines_rerun_busy,
            overview_markdown,
            patch_index,
            file_paths: self.file_list_paths.clone(),
            selected_file: self.selected_file.clone(),
            selected_diff_file: self.selected_diff_file.clone(),
            selected_file_error,
            diff_mode: self.diff_mode,
            diff_revision: self.diff_revision,
            shared_threads,
            has_pending_review,
            threads_syncing: self.threads_syncing,
            metadata_write_pending: self.metadata_write_pending,
            metadata_write_error: self.metadata_write_error.clone(),
            threads,
            hide_resolved_threads: self.hide_resolved_threads,
            conversation_reply_thread: self.conversation_reply_thread.clone(),
        }
    }

    fn begin_metadata_write(&mut self, write: MetadataWrite) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        self.metadata_write_pending = true;
        self.metadata_write_error = None;
        vec![Effect::RunMetadataWrite { id, write }]
    }

    fn on_metadata_write_finished(
        &mut self,
        id: PullRequestId,
        result: Result<PullRequestDetail, ProviderError>,
    ) {
        self.metadata_write_pending = false;
        match result {
            Ok(detail) => {
                if self.selected_id.as_ref() == Some(&id) {
                    self.apply_detail(detail);
                }
            }
            Err(error) => {
                self.metadata_write_error = Some(error.to_string());
            }
        }
    }

    fn bump_diff_revision(&mut self) {
        self.diff_revision = self.diff_revision.wrapping_add(1);
    }

    fn boot(&mut self) -> Vec<Effect> {
        self.pull_request_cache = LoadState::Loading;
        self.inbox = LoadState::Loading;
        self.sidebar_tab = SidebarTab::Inbox;
        let mut effects = vec![
            Effect::BootstrapPullRequestsViews,
            Effect::FetchViewerLogin,
            Effect::QueryPullRequests,
            Effect::QueryInbox,
        ];
        effects.extend(self.begin_pull_requests_sync());
        effects.push(Effect::ArmPullRequestsRefreshTimer);
        effects.extend(self.begin_inbox_sync());
        effects
    }

    fn begin_inbox_sync(&mut self) -> Vec<Effect> {
        self.inbox_sync_generation += 1;
        let generation = self.inbox_sync_generation;
        self.inbox_sync.syncing = true;
        self.inbox_sync.error = None;
        vec![Effect::SyncInbox {
            generation,
            if_modified_since: self.inbox_last_modified.clone(),
        }]
    }

    fn on_inbox_query_finished(&mut self, result: Result<Vec<NotificationThread>, ProviderError>) {
        match result {
            Ok(threads) => {
                self.inbox = LoadState::Ready(Arc::from(threads));
            }
            Err(error) => {
                self.inbox = LoadState::Error(error);
            }
        }
    }

    fn on_inbox_sync_finished(
        &mut self,
        generation: u64,
        result: Result<InboxSyncOutcome, ProviderError>,
    ) -> Vec<Effect> {
        if self.inbox_sync_generation != generation {
            return Vec::new();
        }
        self.inbox_sync.syncing = false;
        match result {
            Ok(outcome) => {
                self.inbox_sync.last_success = Some(Utc::now());
                self.inbox_sync.error = None;
                self.inbox_auth_error = None;
                if let Some(last_modified) = outcome.last_modified {
                    self.inbox_last_modified = Some(last_modified);
                }
                let interval = outcome.poll_interval.unwrap_or(DEFAULT_INBOX_POLL);
                vec![Effect::QueryInbox, Effect::ArmInboxPollTimer { interval }]
            }
            Err(ProviderError::Auth) => {
                self.inbox_auth_error = Some(INBOX_AUTH_FIX_COPY.to_string());
                vec![Effect::QueryInbox]
            }
            Err(error) => {
                self.inbox_sync.error = Some(error.to_string());
                vec![Effect::QueryInbox]
            }
        }
    }

    fn on_notification_read_finished(
        &mut self,
        _id: NotificationThreadId,
        result: Result<(), ProviderError>,
    ) -> Vec<Effect> {
        if let Err(error) = result {
            self.inbox_sync.error = Some(error.to_string());
        }
        vec![Effect::QueryInbox]
    }

    fn on_notification_done_finished(
        &mut self,
        _id: NotificationThreadId,
        result: Result<(), ProviderError>,
    ) -> Vec<Effect> {
        if let Err(error) = result {
            self.inbox_sync.error = Some(error.to_string());
        }
        vec![Effect::QueryInbox]
    }

    fn select_notification(&mut self, id: NotificationThreadId) -> Vec<Effect> {
        let Some(thread) = self.find_notification(&id) else {
            return Vec::new();
        };
        let pr_id = thread.pull_request_id.clone();
        let review_tab = review_tab_for_reason(thread.reason);
        let mut effects = Vec::new();
        if thread.unread {
            effects.push(Effect::MarkNotificationRead { id: id.clone() });
        }
        if self.selected_id.as_ref() == Some(&pr_id) {
            self.review_tab = review_tab;
            return effects;
        }
        self.selected_id = Some(pr_id.clone());
        self.review_tab = review_tab;
        self.reset_subordinate_state();
        effects.push(Effect::FetchDetail { id: pr_id.clone() });
        effects.push(Effect::FetchPatch { id: pr_id.clone() });
        effects.push(Effect::FetchPipelines { id: pr_id });
        effects
    }

    fn find_notification(&self, id: &NotificationThreadId) -> Option<NotificationThread> {
        let LoadState::Ready(threads) = &self.inbox else {
            return None;
        };
        threads.iter().find(|t| &t.id == id).cloned()
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
                if let Some(selected) = &self.selected_id
                    && !items.iter().any(|item| &item.id == selected)
                {
                    self.clear_selection();
                }
                self.pull_request_cache = LoadState::Ready(Arc::from(items));
            }
            Err(error) => {
                self.pull_request_cache = LoadState::Error(error);
            }
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
        self.metadata_dock = MetadataDock::Open;
        self.reset_subordinate_state();
        vec![
            Effect::FetchDetail { id: id.clone() },
            Effect::FetchPatch { id: id.clone() },
            Effect::FetchThreads { id: id.clone() },
            Effect::FetchPipelines { id },
        ]
    }

    fn clear_selection(&mut self) {
        self.selected_id = None;
        self.reset_subordinate_state();
    }

    fn reset_subordinate_state(&mut self) {
        self.detail = LoadState::Idle;
        self.pipelines = LoadState::Idle;
        self.pipelines_rerun_busy = false;
        self.patch_index = LoadState::Idle;
        self.file_cache.clear();
        self.file_errors.clear();
        self.selected_file = None;
        self.selected_diff_file = None;
        self.overview_segments.clear();
        self.details_open.clear();
        self.overview_compose_cache = None;
        self.file_list_paths = Arc::from([]);
        self.threads = LoadState::Idle;
        self.threads_syncing = false;
        self.hide_resolved_threads = false;
        self.conversation_reply_thread = None;
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

    fn on_pipelines_local(
        &mut self,
        id: PullRequestId,
        result: Result<Option<PipelinesSnapshot>, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        match result {
            Ok(Some(snapshot)) => {
                self.pipelines = LoadState::Ready(snapshot);
            }
            Ok(None) => {
                self.pipelines = LoadState::Loading;
            }
            Err(_) => {
                self.pipelines = LoadState::Loading;
            }
        }
    }

    fn on_pipelines_network(
        &mut self,
        id: PullRequestId,
        had_cache: bool,
        result: Result<PipelinesSnapshot, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        if had_cache {
            if let Ok(snapshot) = result {
                self.pipelines = LoadState::Ready(snapshot);
            }
            return;
        }
        match result {
            Ok(snapshot) => {
                self.pipelines = LoadState::Ready(snapshot);
            }
            Err(error) => {
                self.pipelines = LoadState::Error(error);
            }
        }
    }

    fn begin_rerun_pipeline(&mut self, pipeline_id: String) -> Vec<Effect> {
        let Some(pr_id) = self.selected_id.clone() else {
            return Vec::new();
        };
        let LoadState::Ready(snapshot) = &self.pipelines else {
            return Vec::new();
        };
        let Some(pipeline) = snapshot.pipelines.iter().find(|p| p.id == pipeline_id) else {
            return Vec::new();
        };
        let RerunAction::Allowed(target) = &pipeline.rerun else {
            return Vec::new();
        };
        self.pipelines_rerun_busy = true;
        vec![Effect::RerunPipeline {
            id: pr_id,
            target: target.clone(),
        }]
    }

    fn on_rerun_pipeline_finished(
        &mut self,
        id: PullRequestId,
        result: Result<PipelinesSnapshot, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        self.pipelines_rerun_busy = false;
        match result {
            Ok(snapshot) => {
                self.pipelines = LoadState::Ready(snapshot);
            }
            Err(error) => {
                self.pipelines = LoadState::Error(error);
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

        let seeded = views.is_empty();
        let views = if seeded { View::seed_set() } else { views };

        // Ready before load_working so seed filters (Needs my review, etc.) apply immediately.
        self.views = LoadState::Ready(Arc::from(views.clone()));
        let launch = resolve_launch_view_id(&views, last_used.as_ref());
        self.activate_view(launch.clone());

        if !seeded {
            return Vec::new();
        }
        vec![Effect::PersistViews {
            views,
            last_used: Some(launch),
            scoped_repository: self.repo_scope.clone(),
        }]
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

    fn clear_list_filters(&mut self) -> Vec<Effect> {
        self.repo_scope = None;
        let all = ViewId::new(reviewdesk_core::VIEW_ID_ALL_PRS);
        if self.active_view_id.as_ref() != Some(&all) {
            self.activate_view(all.clone());
        } else {
            self.load_working_from_active_view();
        }
        vec![Effect::PersistViews {
            views: self.current_views_vec(),
            last_used: Some(all),
            scoped_repository: None,
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

    fn request_rename_active_view(&mut self) -> Vec<Effect> {
        let Some(active) = self.active_view_id.clone() else {
            return Vec::new();
        };
        let LoadState::Ready(views) = &self.views else {
            return Vec::new();
        };
        let Some(view) = views.iter().find(|v| v.id == active) else {
            return Vec::new();
        };
        if !view.can_rename() {
            return Vec::new();
        }
        vec![Effect::PromptRenameView {
            default_name: view.name.clone(),
        }]
    }

    fn move_active_view(&mut self, delta: i32) -> Vec<Effect> {
        let Some(active) = self.active_view_id.clone() else {
            return Vec::new();
        };
        let LoadState::Ready(views) = &self.views else {
            return Vec::new();
        };
        let mut ordered: Vec<ViewId> = views.iter().map(|v| v.id.clone()).collect();
        let Some(index) = ordered.iter().position(|id| *id == active) else {
            return Vec::new();
        };
        let next = index as i32 + delta;
        if next < 0 || next as usize >= ordered.len() {
            return Vec::new();
        }
        ordered.swap(index, next as usize);
        self.reorder_views(ordered)
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

    fn begin_add_diff_review_comment(&mut self, range: DiffLineRange) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        let body = default_review_comment_body(&range);
        let anchor = diff_anchor_from_range(&range);
        self.threads_syncing = true;
        vec![Effect::SaveDiffReviewComment { id, anchor, body }]
    }

    fn begin_submit_pending_review(
        &mut self,
        event: ReviewSubmitEvent,
        summary_body: Option<String>,
    ) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        if !self.has_pending_review() {
            return Vec::new();
        }
        self.threads_syncing = true;
        vec![Effect::SubmitPendingReviewWrite {
            id,
            event,
            summary_body,
        }]
    }

    fn begin_discard_pending_review(&mut self) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        if !self.has_pending_review() {
            return Vec::new();
        }
        self.threads_syncing = true;
        vec![Effect::DiscardPendingReviewWrite { id }]
    }

    fn begin_set_thread_status(&mut self, thread_id: String, status: ThreadStatus) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        if !self.thread_allows_status_change(&thread_id, status) {
            return Vec::new();
        }
        self.threads_syncing = true;
        vec![Effect::SetThreadStatusWrite {
            id,
            thread_id,
            status,
        }]
    }

    fn thread_allows_status_change(&self, thread_id: &str, status: ThreadStatus) -> bool {
        let LoadState::Ready(snapshot) = &self.threads else {
            return false;
        };
        let Some(thread) = snapshot.threads.iter().find(|t| t.id.as_str() == thread_id) else {
            return false;
        };
        if matches!(thread.status, ThreadStatus::Outdated) {
            return false;
        }
        thread.status != status
    }

    fn begin_publish_timeline_comment(&mut self, body: String) -> Vec<Effect> {
        let Some(id) = self.selected_id.clone() else {
            return Vec::new();
        };
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        self.threads_syncing = true;
        vec![Effect::PublishTimelineCommentWrite {
            id,
            body: trimmed.to_string(),
        }]
    }

    fn has_pending_review(&self) -> bool {
        matches!(
            &self.threads,
            LoadState::Ready(snapshot) if snapshot.pending_review.is_some()
        )
    }

    fn apply_threads_snapshot(&mut self, snapshot: PullRequestThreads) {
        self.threads = LoadState::Ready(snapshot);
        self.threads_syncing = false;
    }

    fn on_threads_local(
        &mut self,
        id: PullRequestId,
        result: Result<Option<PullRequestThreads>, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        match result {
            Ok(Some(snapshot)) => self.apply_threads_snapshot(snapshot),
            Ok(None) => {
                self.threads = LoadState::Loading;
            }
            Err(error) => {
                self.threads = LoadState::Error(error);
                self.threads_syncing = false;
            }
        }
    }

    fn on_threads_network(
        &mut self,
        id: PullRequestId,
        had_cache: bool,
        result: Result<PullRequestThreads, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        if had_cache {
            if let Ok(snapshot) = result {
                self.apply_threads_snapshot(snapshot);
            }
            return;
        }
        match result {
            Ok(snapshot) => self.apply_threads_snapshot(snapshot),
            Err(error) => {
                self.threads = LoadState::Error(error);
                self.threads_syncing = false;
            }
        }
    }

    fn on_threads_write_finished(
        &mut self,
        id: PullRequestId,
        result: Result<PullRequestThreads, ProviderError>,
    ) {
        if self.selected_id.as_ref() != Some(&id) {
            return;
        }
        self.threads_syncing = false;
        match result {
            Ok(snapshot) => self.apply_threads_snapshot(snapshot),
            Err(error) => {
                if !matches!(&self.threads, LoadState::Ready(_)) {
                    self.threads = LoadState::Error(error);
                }
            }
        }
    }

    fn thread_projection(&self) -> (Arc<[ThreadView]>, bool) {
        let LoadState::Ready(snapshot) = &self.threads else {
            return (Arc::from([]), false);
        };
        let has_pending = snapshot.pending_review.is_some();
        let views = snapshot
            .threads
            .iter()
            .map(|thread| ThreadView {
                id: thread.id.as_str().to_string(),
                anchor: thread.anchor.clone(),
                status: thread.status,
                comments: thread
                    .comments
                    .iter()
                    .map(|comment| ReviewCommentView {
                        id: comment.id.as_str().to_string(),
                        author_login: SharedString::from(comment.author_login.as_str()),
                        body: SharedString::from(comment.body.as_str()),
                        is_pending: comment.is_pending,
                        suggestion: comment
                            .suggestion
                            .as_ref()
                            .map(|text| SharedString::from(text.as_str())),
                    })
                    .collect::<Arc<_>>(),
            })
            .collect::<Arc<_>>();
        (views, has_pending)
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
            codeowners_rules: &detail.codeowners_rules,
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

fn diff_anchor_from_range(range: &DiffLineRange) -> DiffAnchor {
    let start = range.start().min(range.end());
    let end = range.start().max(range.end());
    let side = match range.end_side() {
        DiffSide::Original => DiffLineSide::Left,
        DiffSide::Modified => DiffLineSide::Right,
    };
    DiffAnchor {
        path: range.path().to_string(),
        side,
        start_line: start as u32,
        end_line: end as u32,
    }
}

fn default_review_comment_body(range: &DiffLineRange) -> String {
    let start = range.start().min(range.end());
    let end = range.start().max(range.end());
    let lines = if start == end {
        format!("line {end}")
    } else {
        format!("lines {start}-{end}")
    };
    format!("Review comment on {lines}.")
}

/// Initial Review pane tab when opening a pull request from an Inbox row.
pub fn review_tab_for_reason(reason: NotificationReason) -> ReviewTab {
    match reason {
        NotificationReason::Mention
        | NotificationReason::TeamMention
        | NotificationReason::Comment => ReviewTab::Conversation,
        NotificationReason::CiActivity => ReviewTab::Pipelines,
        NotificationReason::ReviewRequested
        | NotificationReason::Author
        | NotificationReason::StateChange => ReviewTab::Overview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use reviewdesk_core::{DraftFilter, ReviewState};

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
        let (
            assignees,
            head_ref,
            base_ref,
            review_decision,
            mergeable,
            state,
            auto_merge_enabled,
            codeowners_rules,
            write_capabilities,
        ) = reviewdesk_core::empty_metadata_fields();
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
            assignees,
            head_ref,
            base_ref,
            review_decision,
            mergeable,
            state,
            auto_merge_enabled,
            codeowners_rules,
            write_capabilities,
        }
    }

    fn sample_notification(id: &str, pr: &str, reason: NotificationReason) -> NotificationThread {
        NotificationThread {
            id: NotificationThreadId::new(id).expect("id"),
            pull_request_id: pr_id(pr),
            title: format!("Notify {id}"),
            repo_label: "acme/widgets".into(),
            reason,
            unread: true,
            updated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn boot_emits_query_sync_and_timer() {
        let mut session = ReviewSession::new();
        let effects = session.apply(Intent::Boot);
        assert_eq!(
            effects,
            vec![
                Effect::BootstrapPullRequestsViews,
                Effect::FetchViewerLogin,
                Effect::QueryPullRequests,
                Effect::QueryInbox,
                Effect::SyncPullRequests { generation: 1 },
                Effect::ArmPullRequestsRefreshTimer,
                Effect::SyncInbox {
                    generation: 1,
                    if_modified_since: None,
                },
            ]
        );
        assert!(matches!(
            session.view().pull_requests_pane.cache,
            LoadState::Loading
        ));
        assert!(session.view().pull_requests_pane.sync.syncing);
        assert!(matches!(session.view().inbox, LoadState::Loading));
        assert!(session.view().inbox_sync.syncing);
        assert_eq!(session.view().sidebar_tab, SidebarTab::Inbox);
    }

    #[test]
    fn mention_notification_opens_conversation_tab() {
        let mut session = ReviewSession::new();
        let thread = sample_notification("n-2", "pr-2", NotificationReason::Mention);
        let _ = session.apply(Intent::InboxQueryFinished {
            result: Ok(vec![thread]),
        });
        let _ = session.apply(Intent::SelectNotification(
            NotificationThreadId::new("n-2").expect("id"),
        ));
        assert_eq!(session.view().review_tab, ReviewTab::Conversation);
    }

    #[test]
    fn select_unread_notification_marks_read_and_opens_pull_request() {
        let mut session = ReviewSession::new();
        let thread = sample_notification("n-1", "pr-1", NotificationReason::ReviewRequested);
        let _ = session.apply(Intent::InboxQueryFinished {
            result: Ok(vec![thread]),
        });
        let effects = session.apply(Intent::SelectNotification(
            NotificationThreadId::new("n-1").expect("id"),
        ));
        assert!(effects.contains(&Effect::MarkNotificationRead {
            id: NotificationThreadId::new("n-1").expect("id"),
        }));
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, Effect::FetchDetail { .. }))
        );
        assert_eq!(session.view().selected_id, Some(pr_id("pr-1")));
        assert_eq!(session.view().review_tab, ReviewTab::Overview);
    }

    #[test]
    fn inbox_auth_failure_sets_fix_copy_and_keeps_local_query() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::Boot);
        let _ = session.apply(Intent::InboxQueryFinished {
            result: Ok(vec![sample_notification(
                "n-1",
                "pr-1",
                NotificationReason::Comment,
            )]),
        });
        let effects = session.apply(Intent::InboxSyncFinished {
            generation: 1,
            result: Err(ProviderError::Auth),
        });
        assert_eq!(effects, vec![Effect::QueryInbox]);
        assert_eq!(
            session.view().inbox_auth_error.as_deref(),
            Some(INBOX_AUTH_FIX_COPY)
        );
        assert!(matches!(session.view().inbox, LoadState::Ready(_)));
    }

    #[test]
    fn transient_inbox_sync_failure_surfaces_non_blocking_error() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::Boot);
        let _ = session.apply(Intent::InboxSyncFinished {
            generation: 1,
            result: Err(ProviderError::other("network blip")),
        });
        assert_eq!(
            session.view().inbox_sync.error.as_deref(),
            Some("network blip")
        );
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
                Effect::FetchThreads { id: pr_id("pr-2") },
                Effect::FetchPipelines { id: pr_id("pr-2") },
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
        let pane = session.view().pull_requests_pane;
        assert!(pane.working_filter.i_am_reviewer);
        assert!(!pane.working_filter.i_am_author);
        assert_eq!(pane.working_filter.drafts, DraftFilter::ReadyOnly);
    }

    #[test]
    fn clear_list_filters_resets_to_all_prs_and_clears_scope() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::PullRequestsViewsBootstrapFinished {
            views: Ok(View::seed_set()),
            last_used: Ok(None),
            scoped_repository: Ok(None),
        });
        let _ = session.apply(Intent::SetRepoScope {
            repo: Some("acme/widgets".into()),
        });
        let _ = session.apply(Intent::SelectView(ViewId::new(
            reviewdesk_core::VIEW_ID_NEEDS_MY_REVIEW,
        )));
        let _ = session.apply(Intent::ClearListFilters);
        assert_eq!(
            session.active_view_id.as_ref().map(|id| id.as_str()),
            Some(reviewdesk_core::VIEW_ID_ALL_PRS)
        );
        assert_eq!(session.view().pull_requests_pane.repo_scope, None);
        assert!(session.view().pull_requests_pane.working_filter.is_empty());
    }

    #[test]
    fn repo_scope_keeps_active_view_and_working_filters() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::PullRequestsViewsBootstrapFinished {
            views: Ok(View::seed_set()),
            last_used: Ok(None),
            scoped_repository: Ok(None),
        });
        let before = session.view().pull_requests_pane.working_filter.clone();
        let active = session.active_view_id.clone();
        let _ = session.apply(Intent::SetRepoScope {
            repo: Some("acme/widgets".into()),
        });
        assert_eq!(session.active_view_id, active);
        assert_eq!(session.view().pull_requests_pane.working_filter, before);
        assert_eq!(
            session.view().pull_requests_pane.repo_scope.as_deref(),
            Some("acme/widgets")
        );
        let _ = session.apply(Intent::SetRepoScope { repo: None });
        assert_eq!(session.active_view_id, active);
        assert_eq!(session.view().pull_requests_pane.working_filter, before);
        assert_eq!(session.view().pull_requests_pane.repo_scope, None);
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
    fn diff_review_comments_attach_to_shared_thread_set() {
        use reviewdesk_core::{
            PendingReview, ReviewComment, ReviewCommentId, Thread, ThreadId, ThreadStatus,
        };

        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let snapshot = PullRequestThreads {
            pending_review: Some(PendingReview {
                pull_request_id: pr_id("pr-1"),
                provider_review_id: Some("pending-1".into()),
                summary_body: None,
            }),
            threads: vec![Thread {
                id: ThreadId::new("t-1"),
                anchor: Some(DiffAnchor {
                    path: "new".into(),
                    side: DiffLineSide::Right,
                    start_line: 1,
                    end_line: 1,
                }),
                status: ThreadStatus::Open,
                comments: vec![ReviewComment {
                    id: ReviewCommentId::new("c-1"),
                    author_login: "you".into(),
                    body: "Review comment on line 1.".into(),
                    is_pending: true,
                    suggestion: None,
                }],
            }],
        };
        let _ = session.apply(Intent::ThreadsWriteFinished {
            id: pr_id("pr-1"),
            result: Ok(snapshot),
        });
        let view = session.view();
        assert!(view.has_pending_review);
        assert_eq!(view.shared_threads.len(), 1);
        assert!(view.shared_threads[0].comments[0].is_pending);

        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-2")));
        assert!(!session.view().has_pending_review);
        assert!(session.view().shared_threads.is_empty());
    }

    #[test]
    fn set_thread_status_emits_write_effect_when_allowed() {
        use reviewdesk_core::{ReviewComment, ReviewCommentId, Thread, ThreadId};

        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::ThreadsWriteFinished {
            id: pr_id("pr-1"),
            result: Ok(PullRequestThreads {
                threads: vec![Thread {
                    id: ThreadId::new("t-1"),
                    anchor: Some(DiffAnchor {
                        path: "src/lib.rs".into(),
                        side: DiffLineSide::Right,
                        start_line: 1,
                        end_line: 1,
                    }),
                    status: ThreadStatus::Open,
                    comments: vec![ReviewComment {
                        id: ReviewCommentId::new("c-1"),
                        author_login: "alice".into(),
                        body: "Fix".into(),
                        is_pending: false,
                        suggestion: None,
                    }],
                }],
                pending_review: None,
            }),
        });

        let effects = session.apply(Intent::SetThreadStatus {
            thread_id: "t-1".into(),
            status: ThreadStatus::Resolved,
        });
        assert_eq!(
            effects,
            vec![Effect::SetThreadStatusWrite {
                id: pr_id("pr-1"),
                thread_id: "t-1".into(),
                status: ThreadStatus::Resolved,
            }]
        );
    }

    #[test]
    fn publish_timeline_comment_emits_immediate_write_effect() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::ThreadsWriteFinished {
            id: pr_id("pr-1"),
            result: Ok(PullRequestThreads {
                threads: Vec::new(),
                pending_review: None,
            }),
        });

        let effects = session.apply(Intent::PublishTimelineComment {
            body: "Hello".into(),
        });
        assert_eq!(
            effects,
            vec![Effect::PublishTimelineCommentWrite {
                id: pr_id("pr-1"),
                body: "Hello".into(),
            }]
        );
    }

    #[test]
    fn toggling_diff_mode_does_not_clear_pending_review_drafts() {
        use reviewdesk_core::{PendingReview, PullRequestThreads};

        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::ThreadsWriteFinished {
            id: pr_id("pr-1"),
            result: Ok(PullRequestThreads {
                threads: Vec::new(),
                pending_review: Some(PendingReview {
                    pull_request_id: pr_id("pr-1"),
                    provider_review_id: None,
                    summary_body: None,
                }),
            }),
        });
        assert!(session.view().has_pending_review);

        let _ = session.apply(Intent::ToggleDiffMode);
        assert!(session.view().has_pending_review);
    }

    fn selected_session(id: &str) -> ReviewSession {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id(id)));
        session
    }

    #[test]
    fn selecting_pull_request_opens_metadata_on_overview() {
        let mut session = selected_session("pr-1");
        let view = session.view();
        assert_eq!(view.review_tab, ReviewTab::Overview);
        assert_eq!(view.metadata_dock, MetadataDock::Open);
    }

    #[test]
    fn set_review_tab_switches_tab_without_effects() {
        let cases = [
            ReviewTab::Overview,
            ReviewTab::Pipelines,
            ReviewTab::Conversation,
            ReviewTab::Changes,
        ];
        for tab in cases {
            let mut session = selected_session("pr-1");
            let effects = session.apply(Intent::SetReviewTab(tab));
            assert!(effects.is_empty(), "SetReviewTab should emit no effects");
            assert_eq!(session.view().review_tab, tab);
        }
    }

    #[test]
    fn metadata_folds_when_leaving_overview_and_reopens_on_return() {
        let non_overview = [
            ReviewTab::Pipelines,
            ReviewTab::Conversation,
            ReviewTab::Changes,
        ];
        for tab in non_overview {
            let mut session = selected_session("pr-1");
            let _ = session.apply(Intent::SetReviewTab(tab));
            assert_eq!(
                session.view().metadata_dock,
                MetadataDock::Folded,
                "leaving Overview for {tab:?} should fold Metadata"
            );

            let _ = session.apply(Intent::SetReviewTab(ReviewTab::Overview));
            assert_eq!(
                session.view().metadata_dock,
                MetadataDock::Open,
                "returning to Overview should reopen Metadata"
            );
        }
    }

    #[test]
    fn chrome_toggle_restores_metadata_after_fold() {
        let mut session = selected_session("pr-1");
        let _ = session.apply(Intent::SetReviewTab(ReviewTab::Changes));
        assert_eq!(session.view().metadata_dock, MetadataDock::Folded);

        let _ = session.apply(Intent::ToggleMetadataDock);
        assert_eq!(session.view().metadata_dock, MetadataDock::Open);
    }

    #[test]
    fn chrome_toggle_restores_metadata_after_hide() {
        let mut session = selected_session("pr-1");
        let _ = session.apply(Intent::HideMetadata);
        assert_eq!(session.view().metadata_dock, MetadataDock::Hidden);

        let _ = session.apply(Intent::ToggleMetadataDock);
        assert_eq!(session.view().metadata_dock, MetadataDock::Open);
    }

    #[test]
    fn chrome_toggle_hides_open_metadata() {
        let mut session = selected_session("pr-1");
        assert_eq!(session.view().metadata_dock, MetadataDock::Open);

        let _ = session.apply(Intent::ToggleMetadataDock);
        assert_eq!(session.view().metadata_dock, MetadataDock::Hidden);
    }

    #[test]
    fn fold_keeps_strip_while_hide_removes_it() {
        let mut session = selected_session("pr-1");
        let _ = session.apply(Intent::FoldMetadata);
        assert_eq!(session.view().metadata_dock, MetadataDock::Folded);

        let _ = session.apply(Intent::HideMetadata);
        assert_eq!(session.view().metadata_dock, MetadataDock::Hidden);
    }

    #[test]
    fn hidden_metadata_is_sticky_across_tabs() {
        let mut session = selected_session("pr-1");
        let _ = session.apply(Intent::HideMetadata);

        let _ = session.apply(Intent::SetReviewTab(ReviewTab::Pipelines));
        assert_eq!(session.view().metadata_dock, MetadataDock::Hidden);

        // Returning to Overview must not auto-open a deliberately hidden dock.
        let _ = session.apply(Intent::SetReviewTab(ReviewTab::Overview));
        assert_eq!(session.view().metadata_dock, MetadataDock::Hidden);
    }

    #[test]
    fn back_to_pull_requests_clears_selection() {
        let mut session = selected_session("pr-1");
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(sample_detail("pr-1", "body"))),
        });
        assert!(session.view().selected_id.is_some());

        let effects = session.apply(Intent::BackToPullRequests);
        assert!(effects.is_empty());
        let view = session.view();
        assert!(view.selected_id.is_none());
        assert!(matches!(view.detail, LoadState::Idle));
    }
}
