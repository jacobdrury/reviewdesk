//! Client-side View application over cached Pull requests rows: filter, sort, group, empty states.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use crate::pull_request_list::{
    PullRequestListItem, PullRequestListSort, PullRequestListSortField, ReviewState, SortDirection,
};
use crate::views::{
    VIEW_ID_ALL_PRS, VIEW_ID_NEEDS_MY_REVIEW, View, ViewFilter, ViewGroupBy, ViewId,
};

/// Why the Pull requests list pane is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestsEmptyKind {
    /// LocalStore has no pull request rows (sync has not populated yet or truly empty cache).
    EmptyStore,
    /// Cache has rows, but the active View + repo scope match none.
    ZeroMatch,
}

/// One row in the virtualized Pull requests list (group header or PR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullRequestListRow {
    Group {
        key: String,
        label: String,
        count: usize,
        depth: u8,
    },
    Item {
        item: PullRequestListItem,
        depth: u8,
    },
}

/// Resolved launch View: last-used when still present, else Needs my review, else All PRs.
pub fn resolve_launch_view_id(views: &[View], last_used: Option<&ViewId>) -> ViewId {
    if let Some(id) = last_used
        && views.iter().any(|v| &v.id == id)
    {
        return id.clone();
    }
    if views
        .iter()
        .any(|v| v.id.as_str() == VIEW_ID_NEEDS_MY_REVIEW)
    {
        return ViewId::new(VIEW_ID_NEEDS_MY_REVIEW);
    }
    ViewId::new(VIEW_ID_ALL_PRS)
}

pub fn review_state_label(state: ReviewState, is_draft: bool) -> &'static str {
    if is_draft {
        return "Draft";
    }
    match state {
        ReviewState::Pending => "Review required",
        ReviewState::Approved => "Approved",
        ReviewState::ChangesRequested => "Changes requested",
        ReviewState::Commented => "Commented",
        ReviewState::Dismissed => "Dismissed",
        ReviewState::Unknown => "Unknown",
    }
}

pub fn group_by_display_label(group_by: ViewGroupBy) -> &'static str {
    match group_by {
        ViewGroupBy::None => "None",
        ViewGroupBy::Repo => "repo",
        ViewGroupBy::Author => "author",
        ViewGroupBy::ReviewState => "review state",
        ViewGroupBy::Label => "label",
        ViewGroupBy::RepoThenReviewState => "project → status",
    }
}

/// Filter, optional repo scope, sort, and build grouped list rows.
pub fn build_pull_request_list_rows(
    all_items: &[PullRequestListItem],
    filter: &ViewFilter,
    group_by: ViewGroupBy,
    sort: &PullRequestListSort,
    viewer_login: Option<&str>,
    now: DateTime<Utc>,
    repo_scope: Option<&str>,
) -> (Vec<PullRequestListRow>, Option<PullRequestsEmptyKind>) {
    if all_items.is_empty() {
        return (Vec::new(), Some(PullRequestsEmptyKind::EmptyStore));
    }

    let mut matched: Vec<PullRequestListItem> = all_items
        .iter()
        .filter(|item| filter.matches(item, viewer_login, now))
        .filter(|item| repo_scope.is_none_or(|repo| item.repo_label == repo))
        .cloned()
        .collect();

    sort_pull_requests(&mut matched, sort);

    if matched.is_empty() {
        return (Vec::new(), Some(PullRequestsEmptyKind::ZeroMatch));
    }

    let rows = group_pull_requests(&matched, group_by);
    (rows, None)
}

pub fn visible_list_rows(
    rows: Vec<PullRequestListRow>,
    collapsed: &HashSet<String>,
) -> Vec<PullRequestListRow> {
    let mut out = Vec::with_capacity(rows.len());
    let mut skip_until_depth: Option<u8> = None;

    for row in rows {
        match row {
            PullRequestListRow::Group {
                key,
                label,
                count,
                depth,
            } => {
                if let Some(skip_depth) = skip_until_depth {
                    if depth > skip_depth {
                        continue;
                    }
                    skip_until_depth = None;
                }
                let collapse_this = collapsed.contains(&key);
                if collapse_this {
                    skip_until_depth = Some(depth);
                }
                out.push(PullRequestListRow::Group {
                    key,
                    label,
                    count,
                    depth,
                });
            }
            PullRequestListRow::Item { item, depth } => {
                if skip_until_depth.is_some() {
                    continue;
                }
                out.push(PullRequestListRow::Item { item, depth });
            }
        }
    }
    out
}

pub fn distinct_repo_labels(items: &[PullRequestListItem]) -> Vec<String> {
    let mut repos: Vec<String> = items.iter().map(|item| item.repo_label.clone()).collect();
    repos.sort();
    repos.dedup();
    repos
}

fn sort_pull_requests(items: &mut [PullRequestListItem], sort: &PullRequestListSort) {
    match sort.field {
        PullRequestListSortField::UpdatedAt => {
            items.sort_by(|a, b| {
                let ord = a.updated_at.cmp(&b.updated_at);
                match sort.direction {
                    SortDirection::Asc => ord,
                    SortDirection::Desc => ord.reverse(),
                }
            });
        }
    }
}

fn group_pull_requests(
    items: &[PullRequestListItem],
    group_by: ViewGroupBy,
) -> Vec<PullRequestListRow> {
    match group_by {
        ViewGroupBy::None => items
            .iter()
            .cloned()
            .map(|item| PullRequestListRow::Item { item, depth: 0 })
            .collect(),
        ViewGroupBy::RepoThenReviewState => build_nested_repo_status_rows(items),
        ViewGroupBy::Repo => build_flat_group_rows(items, |item| item.repo_label.clone()),
        ViewGroupBy::Author => build_flat_group_rows(items, |item| item.author_login.clone()),
        ViewGroupBy::ReviewState => build_flat_group_rows(items, |item| {
            review_state_label(item.review_state, item.is_draft).to_string()
        }),
        ViewGroupBy::Label => build_label_group_rows(items),
    }
}

fn build_flat_group_rows(
    items: &[PullRequestListItem],
    key_for: impl Fn(&PullRequestListItem) -> String,
) -> Vec<PullRequestListRow> {
    let mut keys: Vec<String> = Vec::new();
    for item in items {
        let key = key_for(item);
        if !keys.iter().any(|k| k == &key) {
            keys.push(key);
        }
    }

    let mut rows = Vec::new();
    for key in keys {
        let group_items: Vec<PullRequestListItem> = items
            .iter()
            .filter(|item| key_for(item) == key)
            .cloned()
            .collect();
        rows.push(PullRequestListRow::Group {
            key: format!("group:{key}"),
            label: key,
            count: group_items.len(),
            depth: 0,
        });
        for item in group_items {
            rows.push(PullRequestListRow::Item { item, depth: 1 });
        }
    }
    rows
}

fn build_label_group_rows(items: &[PullRequestListItem]) -> Vec<PullRequestListRow> {
    let mut labels: Vec<String> = Vec::new();
    for item in items {
        for label in &item.labels {
            if !labels.iter().any(|l| l == &label.name) {
                labels.push(label.name.clone());
            }
        }
    }
    labels.sort();

    let mut rows = Vec::new();
    for label in labels {
        let group_items: Vec<PullRequestListItem> = items
            .iter()
            .filter(|item| item.labels.iter().any(|l| l.name == label))
            .cloned()
            .collect();
        rows.push(PullRequestListRow::Group {
            key: format!("label:{label}"),
            label: label.clone(),
            count: group_items.len(),
            depth: 0,
        });
        for item in group_items {
            rows.push(PullRequestListRow::Item { item, depth: 1 });
        }
    }
    rows
}

fn build_nested_repo_status_rows(items: &[PullRequestListItem]) -> Vec<PullRequestListRow> {
    let mut repos: Vec<String> = Vec::new();
    for item in items {
        if !repos.iter().any(|r| r == &item.repo_label) {
            repos.push(item.repo_label.clone());
        }
    }

    let mut rows = Vec::new();
    for repo in repos {
        let repo_items: Vec<PullRequestListItem> = items
            .iter()
            .filter(|item| item.repo_label == repo)
            .cloned()
            .collect();
        rows.push(PullRequestListRow::Group {
            key: format!("repo:{repo}"),
            label: repo.clone(),
            count: repo_items.len(),
            depth: 0,
        });

        let mut states: Vec<String> = Vec::new();
        for item in &repo_items {
            let state = review_state_label(item.review_state, item.is_draft).to_string();
            if !states.iter().any(|s| s == &state) {
                states.push(state);
            }
        }
        for state in states {
            let state_items: Vec<PullRequestListItem> = repo_items
                .iter()
                .filter(|item| review_state_label(item.review_state, item.is_draft) == state)
                .cloned()
                .collect();
            rows.push(PullRequestListRow::Group {
                key: format!("repo:{repo}/status:{state}"),
                label: state.clone(),
                count: state_items.len(),
                depth: 1,
            });
            for item in state_items {
                rows.push(PullRequestListRow::Item { item, depth: 2 });
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::PullRequestId;
    use crate::pull_request_list::DraftFilter;
    use crate::pull_request_list::PullRequestLabel;
    use crate::views::ViewFilter;
    use chrono::TimeZone;

    fn item(
        id: &str,
        repo: &str,
        author: &str,
        draft: bool,
        state: ReviewState,
    ) -> PullRequestListItem {
        PullRequestListItem {
            id: PullRequestId::new(id).unwrap(),
            title: format!("Title {id}"),
            web_url: format!("https://example.test/{id}"),
            updated_at: Utc.with_ymd_and_hms(2026, 1, 10, 0, 0, 0).unwrap(),
            author_login: author.to_string(),
            author_avatar_url: None,
            repo_label: repo.to_string(),
            labels: vec![PullRequestLabel {
                name: "bug".to_string(),
                color: "ff0000".to_string(),
            }],
            review_state: state,
            is_draft: draft,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap()
    }

    #[test]
    fn launch_prefers_last_used_then_needs_review_then_all() {
        let views = View::seed_set();
        let last = ViewId::new(VIEW_ID_ALL_PRS);
        assert_eq!(
            resolve_launch_view_id(&views, Some(&last)),
            ViewId::new(VIEW_ID_ALL_PRS)
        );
        assert_eq!(
            resolve_launch_view_id(&views, Some(&ViewId::new("missing"))),
            ViewId::new(VIEW_ID_NEEDS_MY_REVIEW)
        );
        let only_all = vec![View::seed_set()[0].clone()];
        assert_eq!(
            resolve_launch_view_id(&only_all, None),
            ViewId::new(VIEW_ID_ALL_PRS)
        );
    }

    #[test]
    fn empty_store_when_cache_empty() {
        let filter = ViewFilter::default();
        let (rows, empty) = build_pull_request_list_rows(
            &[],
            &filter,
            ViewGroupBy::None,
            &PullRequestListSort::default(),
            Some("me"),
            now(),
            None,
        );
        assert!(rows.is_empty());
        assert_eq!(empty, Some(PullRequestsEmptyKind::EmptyStore));
    }

    #[test]
    fn zero_match_when_filter_excludes_everything() {
        let items = vec![item("a#1", "acme/a", "alice", false, ReviewState::Pending)];
        let filter = ViewFilter {
            i_am_author: true,
            ..ViewFilter::default()
        };
        let (rows, empty) = build_pull_request_list_rows(
            &items,
            &filter,
            ViewGroupBy::None,
            &PullRequestListSort::default(),
            Some("me"),
            now(),
            None,
        );
        assert!(rows.is_empty());
        assert_eq!(empty, Some(PullRequestsEmptyKind::ZeroMatch));
    }

    #[test]
    fn repo_scope_narrows_before_view_filter() {
        let items = vec![
            item("a#1", "acme/a", "me", false, ReviewState::Pending),
            item("b#1", "acme/b", "me", false, ReviewState::Pending),
        ];
        let filter = ViewFilter::default();
        let (rows, empty) = build_pull_request_list_rows(
            &items,
            &filter,
            ViewGroupBy::None,
            &PullRequestListSort::default(),
            Some("me"),
            now(),
            Some("acme/b"),
        );
        assert_eq!(empty, None);
        assert_eq!(rows.len(), 1);
        assert!(
            matches!(&rows[0], PullRequestListRow::Item { item, .. } if item.id.as_str() == "b#1")
        );
    }

    #[test]
    fn nested_by_project_groups_repo_then_status() {
        let items = vec![
            item("a#1", "acme/a", "alice", false, ReviewState::Pending),
            item("a#2", "acme/a", "bob", false, ReviewState::Approved),
        ];
        let (rows, _) = build_pull_request_list_rows(
            &items,
            &ViewFilter::default(),
            ViewGroupBy::RepoThenReviewState,
            &PullRequestListSort::default(),
            Some("me"),
            now(),
            None,
        );
        assert!(matches!(
            &rows[0],
            PullRequestListRow::Group { depth: 0, .. }
        ));
        assert!(matches!(
            &rows[1],
            PullRequestListRow::Group { depth: 1, .. }
        ));
        assert!(matches!(
            &rows[2],
            PullRequestListRow::Item { depth: 2, .. }
        ));
    }

    #[test]
    fn collapsed_groups_hide_children() {
        let items = vec![item("a#1", "acme/a", "alice", false, ReviewState::Pending)];
        let (rows, _) = build_pull_request_list_rows(
            &items,
            &ViewFilter::default(),
            ViewGroupBy::Repo,
            &PullRequestListSort::default(),
            Some("me"),
            now(),
            None,
        );
        let mut collapsed = HashSet::new();
        collapsed.insert("group:acme/a".to_string());
        let visible = visible_list_rows(rows, &collapsed);
        assert_eq!(visible.len(), 1);
        assert!(matches!(visible[0], PullRequestListRow::Group { .. }));
    }
}
