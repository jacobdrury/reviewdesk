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
    InboxItem, Patch, ProviderError, PullRequestDetail, PullRequestId,
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
    RefreshInbox,
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
    InboxQueryFinished {
        result: Result<Vec<InboxItem>, ProviderError>,
    },
    InboxSyncFinished {
        generation: u64,
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
}

/// Work the shell must perform against LocalStore / Provider / timers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    QueryInbox,
    SyncInbox {
        generation: u64,
    },
    FetchDetail {
        id: PullRequestId,
    },
    FetchPatch {
        id: PullRequestId,
    },
    ArmInboxRefreshTimer,
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
pub struct InboxSyncView {
    pub syncing: bool,
    pub last_success: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl InboxSyncView {
    pub fn status_label(&self) -> String {
        if self.syncing {
            return "Syncing Inbox…".into();
        }
        if let Some(error) = &self.error {
            return format!("Sync failed: {error}");
        }
        if let Some(at) = self.last_success {
            return format!("Inbox synced {}", format_sync_time(at));
        }
        "Inbox not synced yet".into()
    }

    pub fn empty_placeholder(&self) -> &'static str {
        if self.syncing {
            "Syncing pull requests…"
        } else {
            "No pull requests waiting for your review."
        }
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
    pub inbox: LoadState<Arc<[InboxItem]>>,
    pub inbox_sync: InboxSyncView,
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
    inbox: LoadState<Arc<[InboxItem]>>,
    inbox_sync: InboxSyncView,
    inbox_sync_generation: u64,
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
            inbox: LoadState::Loading,
            inbox_sync: InboxSyncView::default(),
            inbox_sync_generation: 0,
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
            Intent::RefreshInbox => self.begin_inbox_sync(),
            Intent::SelectPullRequest(id) => self.select_pull_request(id),
            Intent::SetReviewTab(tab) => {
                self.review_tab = tab;
                Vec::new()
            }
            Intent::ToggleDetails {
                id,
                currently_open,
            } => {
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
            Intent::InboxQueryFinished { result } => {
                self.on_inbox_query_finished(result);
                Vec::new()
            }
            Intent::InboxSyncFinished { generation, result } => {
                self.on_inbox_sync_finished(generation, result)
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
        let overview_markdown =
            ready_detail.map(|detail| self.cached_overview_markdown(&detail));
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

        SessionView {
            inbox: self.inbox.clone(),
            inbox_sync: self.inbox_sync.clone(),
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
        self.inbox = LoadState::Loading;
        let mut effects = self.begin_inbox_sync();
        effects.insert(0, Effect::QueryInbox);
        effects.push(Effect::ArmInboxRefreshTimer);
        effects
    }

    fn begin_inbox_sync(&mut self) -> Vec<Effect> {
        self.inbox_sync_generation += 1;
        let generation = self.inbox_sync_generation;
        self.inbox_sync.syncing = true;
        self.inbox_sync.error = None;
        vec![Effect::SyncInbox { generation }]
    }

    fn on_inbox_query_finished(&mut self, result: Result<Vec<InboxItem>, ProviderError>) {
        match result {
            Ok(items) => {
                if let Some(selected) = &self.selected_id
                    && !items.iter().any(|item| &item.id == selected)
                {
                    self.clear_selection();
                }
                self.inbox = LoadState::Ready(Arc::from(items));
            }
            Err(error) => {
                self.inbox = LoadState::Error(error);
            }
        }
    }

    fn on_inbox_sync_finished(
        &mut self,
        generation: u64,
        result: Result<(), ProviderError>,
    ) -> Vec<Effect> {
        if self.inbox_sync_generation != generation {
            return Vec::new();
        }
        self.inbox_sync.syncing = false;
        match result {
            Ok(()) => {
                self.inbox_sync.last_success = Some(Utc::now());
                self.inbox_sync.error = None;
            }
            Err(error) => {
                self.inbox_sync.error = Some(error.to_string());
            }
        }
        vec![Effect::QueryInbox]
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

    fn on_patch_local(
        &mut self,
        id: PullRequestId,
        result: Result<Option<Patch>, ProviderError>,
    ) {
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

    fn sample_item(id: &str) -> InboxItem {
        InboxItem {
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
                Effect::QueryInbox,
                Effect::SyncInbox { generation: 1 },
                Effect::ArmInboxRefreshTimer,
            ]
        );
        assert!(matches!(session.view().inbox, LoadState::Loading));
        assert!(session.view().inbox_sync.syncing);
    }

    #[test]
    fn selection_emits_parallel_fetches_and_resets_review_state() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::Boot);
        let _ = session.apply(Intent::InboxQueryFinished {
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
    fn missing_selection_from_inbox_reload_clears_review_pane() {
        let mut session = ReviewSession::new();
        let _ = session.apply(Intent::SelectPullRequest(pr_id("pr-1")));
        let _ = session.apply(Intent::DetailLocalFinished {
            id: pr_id("pr-1"),
            result: Ok(Some(sample_detail("pr-1", "body"))),
        });
        let _ = session.apply(Intent::InboxQueryFinished {
            result: Ok(vec![sample_item("pr-other")]),
        });
        let view = session.view();
        assert!(view.selected_id.is_none());
        assert!(matches!(view.detail, LoadState::Idle));
        assert!(matches!(view.patch_index, LoadState::Idle));
    }

    #[test]
    fn stale_inbox_sync_finish_is_ignored() {
        let mut session = ReviewSession::new();
        let first = session.apply(Intent::RefreshInbox);
        assert_eq!(first, vec![Effect::SyncInbox { generation: 1 }]);
        let second = session.apply(Intent::RefreshInbox);
        assert_eq!(second, vec![Effect::SyncInbox { generation: 2 }]);

        let stale = session.apply(Intent::InboxSyncFinished {
            generation: 1,
            result: Ok(()),
        });
        assert!(stale.is_empty());
        assert!(session.view().inbox_sync.syncing);

        let fresh = session.apply(Intent::InboxSyncFinished {
            generation: 2,
            result: Ok(()),
        });
        assert_eq!(fresh, vec![Effect::QueryInbox]);
        assert!(!session.view().inbox_sync.syncing);
        assert!(session.view().inbox_sync.last_success.is_some());
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
