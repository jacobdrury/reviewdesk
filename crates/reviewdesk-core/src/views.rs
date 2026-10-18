//! Local-only Views over the Pull requests list.
//!
//! A [`View`] is a named saved query (filters + optional group-by + sort). Views are
//! product data persisted in LocalStore — never Provider-synced cache. The built-in
//! **All PRs** View cannot be deleted or renamed; seeds and user Views can.
//!
//! Filtering is evaluated client-side against cached [`PullRequestListItem`] rows via
//! [`ViewFilter::matches`], the single place the Views surface decides membership.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::pull_request_list::{
    DraftFilter, PullRequestListItem, PullRequestListSort, ReviewState,
};

/// Stable identity for a View. String-backed; the app assigns ids.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewId(String);

impl ViewId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable ids for the built-in and seed Views.
pub const VIEW_ID_ALL_PRS: &str = "all-prs";
pub const VIEW_ID_NEEDS_MY_REVIEW: &str = "needs-my-review";
pub const VIEW_ID_MINE: &str = "mine";
pub const VIEW_ID_DRAFTS: &str = "drafts";
pub const VIEW_ID_BY_PROJECT: &str = "by-project";

/// Origin of a View. Governs delete/rename affordances and seeding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    /// Built-in **All PRs**: always present, undeletable, unrenamable, reorderable.
    Builtin,
    /// Seeded View (Needs my review / Mine / Drafts / By project); deletable and renamable.
    Seed,
    /// User-created View.
    User,
}

/// Single group-by dimension, plus the nested **By project** mode (`repo → review status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewGroupBy {
    #[default]
    None,
    Repo,
    Author,
    ReviewState,
    Label,
    /// By project: repository first, then review status within each repository.
    RepoThenReviewState,
}

/// Relative-time filter on `updated_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdatedWithin {
    #[default]
    Any,
    Last24h,
    Last7d,
    Last30d,
}

impl UpdatedWithin {
    /// Earliest `updated_at` that still matches, or `None` for [`UpdatedWithin::Any`].
    pub fn earliest(self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let window = match self {
            UpdatedWithin::Any => return None,
            UpdatedWithin::Last24h => Duration::hours(24),
            UpdatedWithin::Last7d => Duration::days(7),
            UpdatedWithin::Last30d => Duration::days(30),
        };
        Some(now - window)
    }
}

/// CI status dimension from the Views grilling (#52).
///
/// Check rollups are not cached yet (Pipelines / #76), so a cached row is effectively
/// `Unknown`. `Any` and `Unknown` match every row; `Passing` / `Failing` / `Pending`
/// cannot be confirmed and match nothing until check data lands — honest, not faked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStatusFilter {
    #[default]
    Any,
    Passing,
    Failing,
    Pending,
    Unknown,
}

/// Filter criteria for a View. Dimensions AND together; multi-value fields OR within.
///
/// `i_am_author` / `i_am_reviewer` bind to the single signed-in identity. In the locally
/// cached set (authored-or-review-requested), "I am reviewer" means a row the viewer did
/// not author.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewFilter {
    pub i_am_author: bool,
    pub i_am_reviewer: bool,
    pub repos: Vec<String>,
    pub authors: Vec<String>,
    pub labels: Vec<String>,
    pub review_states: Vec<ReviewState>,
    pub assignees: Vec<String>,
    pub drafts: DraftFilter,
    pub updated_within: UpdatedWithin,
    pub ci_status: CiStatusFilter,
}

impl Default for ViewFilter {
    fn default() -> Self {
        Self {
            i_am_author: false,
            i_am_reviewer: false,
            repos: Vec::new(),
            authors: Vec::new(),
            labels: Vec::new(),
            review_states: Vec::new(),
            assignees: Vec::new(),
            // All PRs includes drafts and ready by default (#52).
            drafts: DraftFilter::All,
            updated_within: UpdatedWithin::Any,
            ci_status: CiStatusFilter::Any,
        }
    }
}

impl ViewFilter {
    /// True when no dimension narrows the list (the All PRs working query).
    pub fn is_empty(&self) -> bool {
        *self == ViewFilter::default()
    }

    /// Does `item` satisfy this filter, given the signed-in identity and clock?
    pub fn matches(
        &self,
        item: &PullRequestListItem,
        viewer_login: Option<&str>,
        now: DateTime<Utc>,
    ) -> bool {
        if self.i_am_author && !self.authored_by_viewer(item, viewer_login) {
            return false;
        }
        if self.i_am_reviewer && !self.reviewer_is_viewer(item, viewer_login) {
            return false;
        }
        if !self.draft_allows(item) {
            return false;
        }
        if !self.repos.is_empty() && !self.repos.iter().any(|repo| repo == &item.repo_label) {
            return false;
        }
        if !self.authors.is_empty()
            && !self.authors.iter().any(|login| login == &item.author_login)
        {
            return false;
        }
        if !self.labels.is_empty() && !self.label_matches(item) {
            return false;
        }
        if !self.review_states.is_empty() && !self.review_states.contains(&item.review_state) {
            return false;
        }
        if let Some(earliest) = self.updated_within.earliest(now)
            && item.updated_at < earliest
        {
            return false;
        }
        // Check rollups are not cached yet: only Any / Unknown can be honoured.
        if matches!(
            self.ci_status,
            CiStatusFilter::Passing | CiStatusFilter::Failing | CiStatusFilter::Pending
        ) {
            return false;
        }
        // Assignee data is not cached yet (#53); a set filter cannot be confirmed.
        if !self.assignees.is_empty() {
            return false;
        }
        true
    }

    fn authored_by_viewer(&self, item: &PullRequestListItem, viewer_login: Option<&str>) -> bool {
        viewer_login.is_some_and(|login| item.author_login == login)
    }

    fn reviewer_is_viewer(&self, item: &PullRequestListItem, viewer_login: Option<&str>) -> bool {
        viewer_login.is_some_and(|login| item.author_login != login)
    }

    fn draft_allows(&self, item: &PullRequestListItem) -> bool {
        match self.drafts {
            DraftFilter::ReadyOnly => !item.is_draft,
            DraftFilter::DraftsOnly => item.is_draft,
            DraftFilter::All => true,
        }
    }

    fn label_matches(&self, item: &PullRequestListItem) -> bool {
        self.labels
            .iter()
            .any(|wanted| item.labels.iter().any(|label| &label.name == wanted))
    }
}

/// A named, local-only saved query over Pull requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub id: ViewId,
    pub name: String,
    pub kind: ViewKind,
    pub position: i64,
    pub filter: ViewFilter,
    pub group_by: ViewGroupBy,
    pub sort: PullRequestListSort,
}

impl View {
    /// Can the user rename this View? (All PRs is unrenamable.)
    pub fn can_rename(&self) -> bool {
        self.kind != ViewKind::Builtin
    }

    /// Can the user delete this View? (All PRs is undeletable.)
    pub fn can_delete(&self) -> bool {
        self.kind != ViewKind::Builtin
    }

    /// Can the user Save edits into this View in place? (All PRs only accepts Save as new.)
    pub fn can_save_in_place(&self) -> bool {
        self.kind != ViewKind::Builtin
    }

    /// Built-in **All PRs** plus the seed Views, in launch order.
    ///
    /// Seeds return only on a fresh LocalStore; the app seeds once.
    pub fn seed_set() -> Vec<View> {
        vec![
            View {
                id: ViewId::new(VIEW_ID_ALL_PRS),
                name: "All PRs".to_string(),
                kind: ViewKind::Builtin,
                position: 0,
                filter: ViewFilter::default(),
                group_by: ViewGroupBy::None,
                sort: PullRequestListSort::default(),
            },
            View {
                id: ViewId::new(VIEW_ID_NEEDS_MY_REVIEW),
                name: "Needs my review".to_string(),
                kind: ViewKind::Seed,
                position: 1,
                filter: ViewFilter {
                    i_am_reviewer: true,
                    drafts: DraftFilter::ReadyOnly,
                    ..ViewFilter::default()
                },
                group_by: ViewGroupBy::ReviewState,
                sort: PullRequestListSort::default(),
            },
            View {
                id: ViewId::new(VIEW_ID_MINE),
                name: "Mine".to_string(),
                kind: ViewKind::Seed,
                position: 2,
                filter: ViewFilter {
                    i_am_author: true,
                    ..ViewFilter::default()
                },
                group_by: ViewGroupBy::Repo,
                sort: PullRequestListSort::default(),
            },
            View {
                id: ViewId::new(VIEW_ID_DRAFTS),
                name: "Drafts".to_string(),
                kind: ViewKind::Seed,
                position: 3,
                filter: ViewFilter {
                    i_am_author: true,
                    drafts: DraftFilter::DraftsOnly,
                    ..ViewFilter::default()
                },
                group_by: ViewGroupBy::Repo,
                sort: PullRequestListSort::default(),
            },
            View {
                id: ViewId::new(VIEW_ID_BY_PROJECT),
                name: "By project".to_string(),
                kind: ViewKind::Seed,
                position: 4,
                filter: ViewFilter::default(),
                group_by: ViewGroupBy::RepoThenReviewState,
                sort: PullRequestListSort::default(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::PullRequestId;
    use crate::pull_request_list::PullRequestLabel;
    use chrono::TimeZone;

    fn item(id: &str, author: &str, draft: bool) -> PullRequestListItem {
        PullRequestListItem {
            id: PullRequestId::new(id).unwrap(),
            title: format!("Title {id}"),
            web_url: format!("https://example.test/{id}"),
            updated_at: Utc.with_ymd_and_hms(2026, 1, 10, 0, 0, 0).unwrap(),
            author_login: author.to_string(),
            author_avatar_url: None,
            repo_label: "acme/widgets".to_string(),
            labels: vec![PullRequestLabel {
                name: "bug".to_string(),
                color: "ff0000".to_string(),
            }],
            review_state: ReviewState::Pending,
            is_draft: draft,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap()
    }

    #[test]
    fn empty_filter_matches_any_row() {
        let filter = ViewFilter::default();
        assert!(filter.is_empty());
        assert!(filter.matches(&item("acme/widgets#1", "alice", false), Some("me"), now()));
        assert!(filter.matches(&item("acme/widgets#2", "me", true), Some("me"), now()));
    }

    #[test]
    fn i_am_author_binds_to_signed_in_identity() {
        let filter = ViewFilter {
            i_am_author: true,
            ..ViewFilter::default()
        };
        assert!(filter.matches(&item("acme/widgets#1", "me", false), Some("me"), now()));
        assert!(!filter.matches(&item("acme/widgets#2", "alice", false), Some("me"), now()));
        // Unknown identity cannot confirm authorship.
        assert!(!filter.matches(&item("acme/widgets#1", "me", false), None, now()));
    }

    #[test]
    fn i_am_reviewer_means_not_authored_by_me() {
        let filter = ViewFilter {
            i_am_reviewer: true,
            drafts: DraftFilter::ReadyOnly,
            ..ViewFilter::default()
        };
        assert!(filter.matches(&item("acme/widgets#1", "alice", false), Some("me"), now()));
        // Authored by me: not a review request.
        assert!(!filter.matches(&item("acme/widgets#2", "me", false), Some("me"), now()));
        // Ready-only excludes drafts.
        assert!(!filter.matches(&item("acme/widgets#3", "alice", true), Some("me"), now()));
    }

    #[test]
    fn multi_value_dimensions_or_within_and_and_across() {
        let filter = ViewFilter {
            repos: vec!["acme/widgets".to_string(), "acme/kit".to_string()],
            authors: vec!["alice".to_string()],
            ..ViewFilter::default()
        };
        assert!(filter.matches(&item("acme/widgets#1", "alice", false), Some("me"), now()));
        // Right repo, wrong author: AND across dimensions fails.
        assert!(!filter.matches(&item("acme/widgets#2", "bob", false), Some("me"), now()));
    }

    #[test]
    fn updated_within_filters_by_clock() {
        let mut stale = item("acme/widgets#1", "alice", false);
        stale.updated_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let filter = ViewFilter {
            updated_within: UpdatedWithin::Last7d,
            ..ViewFilter::default()
        };
        assert!(!filter.matches(&stale, Some("me"), now()));
        let fresh = item("acme/widgets#2", "alice", false);
        assert!(filter.matches(&fresh, Some("me"), now()));
    }

    #[test]
    fn seed_set_has_builtin_all_prs_and_four_seeds() {
        let views = View::seed_set();
        assert_eq!(views.len(), 5);
        let all = &views[0];
        assert_eq!(all.id.as_str(), VIEW_ID_ALL_PRS);
        assert_eq!(all.kind, ViewKind::Builtin);
        assert!(!all.can_delete());
        assert!(!all.can_rename());
        assert!(!all.can_save_in_place());
        assert!(all.filter.is_empty());

        let by_project = views
            .iter()
            .find(|v| v.id.as_str() == VIEW_ID_BY_PROJECT)
            .unwrap();
        assert_eq!(by_project.group_by, ViewGroupBy::RepoThenReviewState);
        assert!(by_project.can_delete());
    }

    #[test]
    fn view_filter_round_trips_through_json() {
        let filter = ViewFilter {
            i_am_reviewer: true,
            repos: vec!["acme/widgets".to_string()],
            review_states: vec![ReviewState::Approved, ReviewState::ChangesRequested],
            drafts: DraftFilter::ReadyOnly,
            updated_within: UpdatedWithin::Last30d,
            ..ViewFilter::default()
        };
        let json = serde_json::to_string(&filter).unwrap();
        let back: ViewFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(filter, back);
    }
}
