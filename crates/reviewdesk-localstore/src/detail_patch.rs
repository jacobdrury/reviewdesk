use reviewdesk_core::{
    Assignee, CodeownersRule, LocalStoreError, MergeableState, MetadataWriteCapabilities, Patch,
    PullRequestDetail, PullRequestId, PullRequestLabel, PullRequestState, ReviewDecision, Reviewer,
};
use serde::{Deserialize, Serialize};
use turso::{Connection, Value, params};

use crate::error::map_other;

#[derive(Serialize, Deserialize)]
struct StoredLabel {
    name: String,
    color: String,
}

#[derive(Serialize, Deserialize)]
struct StoredReviewer {
    login: String,
    avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredAssignee {
    login: String,
    avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredCodeownersRule {
    pattern: String,
    owners: Vec<String>,
    matched_paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct StoredWriteCapabilities {
    #[serde(default)]
    labels: bool,
    #[serde(default)]
    assignees: bool,
    #[serde(default)]
    reviewers: bool,
    #[serde(default)]
    draft_ready: bool,
    #[serde(default)]
    base_ref: bool,
    #[serde(default)]
    merge: bool,
    #[serde(default)]
    auto_merge: bool,
}

pub async fn put_pull_request_detail(
    conn: &Connection,
    detail: &PullRequestDetail,
) -> Result<(), LocalStoreError> {
    let labels = encode_labels(&detail.labels)?;
    let reviewers = encode_reviewers(&detail.reviewers)?;
    let assignees = encode_assignees(&detail.assignees)?;
    let codeowners = encode_codeowners(&detail.codeowners_rules)?;
    let capabilities = encode_capabilities(&detail.write_capabilities)?;
    let is_draft = if detail.is_draft { 1i64 } else { 0i64 };

    conn.execute(
        "
insert into pull_request_details (
    id, title, body, author_login, author_avatar_url, is_draft, labels_json, web_url, reviewers_json,
    assignees_json, head_ref, base_ref, review_decision, mergeable, state, auto_merge_enabled, codeowners_rules_json, write_capabilities_json
) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
on conflict(id) do update set
    title = excluded.title,
    body = excluded.body,
    author_login = excluded.author_login,
    author_avatar_url = excluded.author_avatar_url,
    is_draft = excluded.is_draft,
    labels_json = excluded.labels_json,
    web_url = excluded.web_url,
    reviewers_json = excluded.reviewers_json,
    assignees_json = excluded.assignees_json,
    head_ref = excluded.head_ref,
    base_ref = excluded.base_ref,
    review_decision = excluded.review_decision,
    mergeable = excluded.mergeable,
    state = excluded.state,
    auto_merge_enabled = excluded.auto_merge_enabled,
    codeowners_rules_json = excluded.codeowners_rules_json,
    write_capabilities_json = excluded.write_capabilities_json
",
        params![
            detail.id.as_str(),
            detail.title.as_str(),
            detail.body.as_str(),
            detail.author_login.as_str(),
            detail.author_avatar_url.as_deref(),
            is_draft,
            labels.as_str(),
            detail.web_url.as_str(),
            reviewers.as_str(),
            assignees.as_str(),
            detail.head_ref.as_str(),
            detail.base_ref.as_str(),
            review_decision_to_str(detail.review_decision),
            mergeable_to_str(detail.mergeable),
            state_to_str(detail.state),
            i64::from(detail.auto_merge_enabled),
            codeowners.as_str(),
            capabilities.as_str(),
        ],
    )
    .await
    .map_err(map_other)?;

    Ok(())
}

pub async fn get_pull_request_detail(
    conn: &Connection,
    id: &PullRequestId,
) -> Result<Option<PullRequestDetail>, LocalStoreError> {
    let mut rows = conn
        .query(
            "
select
    id, title, body, author_login, author_avatar_url, is_draft, labels_json, web_url, reviewers_json,
    assignees_json, head_ref, base_ref, review_decision, mergeable, state, auto_merge_enabled, codeowners_rules_json, write_capabilities_json
from pull_request_details
where id = ?1
",
            [id.as_str()],
        )
        .await
        .map_err(map_other)?;

    let row = rows.next().await.map_err(map_other)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let pr_id =
        PullRequestId::new(text_at(&row, 0)?).map_err(|e| LocalStoreError::other(e.to_string()))?;
    let title = text_at(&row, 1)?;
    let body = text_at(&row, 2)?;
    let author_login = text_at(&row, 3)?;
    let author_avatar_url = optional_text_at(&row, 4)?;
    let is_draft = bool_at(&row, 5)?;
    let labels_json = text_at(&row, 6)?;
    let web_url = text_at(&row, 7)?;
    let reviewers_json = text_at(&row, 8)?;
    let assignees_json = optional_text_at(&row, 9)?.unwrap_or_else(|| "[]".into());
    let head_ref = optional_text_at(&row, 10)?.unwrap_or_default();
    let base_ref = optional_text_at(&row, 11)?.unwrap_or_default();
    let review_decision = optional_text_at(&row, 12)?
        .map(|s| review_decision_from_str(&s))
        .unwrap_or(ReviewDecision::Unknown);
    let mergeable = optional_text_at(&row, 13)?
        .map(|s| mergeable_from_str(&s))
        .unwrap_or(MergeableState::Unknown);
    let state = optional_text_at(&row, 14)?
        .map(|s| state_from_str(&s))
        .unwrap_or(PullRequestState::Open);
    let auto_merge_enabled = bool_at(&row, 15).unwrap_or(false);
    let codeowners_json = optional_text_at(&row, 16)?.unwrap_or_else(|| "[]".into());
    let capabilities_json = optional_text_at(&row, 17)?.unwrap_or_else(|| "{}".into());

    Ok(Some(PullRequestDetail {
        id: pr_id,
        title,
        body,
        author_login,
        author_avatar_url,
        is_draft,
        labels: decode_labels(&labels_json)?,
        web_url,
        reviewers: decode_reviewers(&reviewers_json)?,
        assignees: decode_assignees(&assignees_json)?,
        head_ref,
        base_ref,
        review_decision,
        mergeable,
        state,
        auto_merge_enabled,
        codeowners_rules: decode_codeowners(&codeowners_json)?,
        write_capabilities: decode_capabilities(&capabilities_json)?,
    }))
}

pub async fn put_patch(
    conn: &Connection,
    id: &PullRequestId,
    patch: &Patch,
) -> Result<(), LocalStoreError> {
    conn.execute(
        "
INSERT INTO patches (pull_request_id, patch_text) VALUES (?1, ?2)
ON CONFLICT(pull_request_id) DO UPDATE SET patch_text = excluded.patch_text
",
        (id.as_str(), patch.as_str()),
    )
    .await
    .map_err(map_other)?;

    Ok(())
}

pub async fn get_patch(
    conn: &Connection,
    id: &PullRequestId,
) -> Result<Option<Patch>, LocalStoreError> {
    let mut rows = conn
        .query(
            "SELECT patch_text FROM patches WHERE pull_request_id = ?1",
            [id.as_str()],
        )
        .await
        .map_err(map_other)?;

    let row = rows.next().await.map_err(map_other)?;
    let Some(row) = row else {
        return Ok(None);
    };

    Ok(Some(Patch::new(text_at(&row, 0)?)))
}

fn encode_labels(labels: &[PullRequestLabel]) -> Result<String, LocalStoreError> {
    let stored = labels
        .iter()
        .map(|label| StoredLabel {
            name: label.name.clone(),
            color: label.color.clone(),
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&stored).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn decode_labels(json: &str) -> Result<Vec<PullRequestLabel>, LocalStoreError> {
    let trimmed = json.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(Vec::new());
    }
    let stored: Vec<StoredLabel> =
        serde_json::from_str(trimmed).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(stored
        .into_iter()
        .map(|label| PullRequestLabel {
            name: label.name,
            color: label.color,
        })
        .collect())
}

fn encode_reviewers(reviewers: &[Reviewer]) -> Result<String, LocalStoreError> {
    let stored = reviewers
        .iter()
        .map(|reviewer| StoredReviewer {
            login: reviewer.login.clone(),
            avatar_url: reviewer.avatar_url.clone(),
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&stored).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn encode_assignees(assignees: &[Assignee]) -> Result<String, LocalStoreError> {
    let stored = assignees
        .iter()
        .map(|assignee| StoredAssignee {
            login: assignee.login.clone(),
            avatar_url: assignee.avatar_url.clone(),
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&stored).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn decode_assignees(json: &str) -> Result<Vec<Assignee>, LocalStoreError> {
    let stored: Vec<StoredAssignee> =
        serde_json::from_str(json).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(stored
        .into_iter()
        .map(|assignee| Assignee {
            login: assignee.login,
            avatar_url: assignee.avatar_url,
        })
        .collect())
}

fn encode_codeowners(rules: &[CodeownersRule]) -> Result<String, LocalStoreError> {
    let stored = rules
        .iter()
        .map(|rule| StoredCodeownersRule {
            pattern: rule.pattern.clone(),
            owners: rule.owners.clone(),
            matched_paths: rule.matched_paths.clone(),
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&stored).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn decode_codeowners(json: &str) -> Result<Vec<CodeownersRule>, LocalStoreError> {
    let stored: Vec<StoredCodeownersRule> =
        serde_json::from_str(json).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(stored
        .into_iter()
        .map(|rule| CodeownersRule {
            pattern: rule.pattern,
            owners: rule.owners,
            matched_paths: rule.matched_paths,
        })
        .collect())
}

fn encode_capabilities(caps: &MetadataWriteCapabilities) -> Result<String, LocalStoreError> {
    let stored = StoredWriteCapabilities {
        labels: caps.labels,
        assignees: caps.assignees,
        reviewers: caps.reviewers,
        draft_ready: caps.draft_ready,
        base_ref: caps.base_ref,
        merge: caps.merge,
        auto_merge: caps.auto_merge,
    };
    serde_json::to_string(&stored).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn decode_capabilities(json: &str) -> Result<MetadataWriteCapabilities, LocalStoreError> {
    let trimmed = json.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(MetadataWriteCapabilities::default());
    }
    let stored: StoredWriteCapabilities =
        serde_json::from_str(trimmed).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(MetadataWriteCapabilities {
        labels: stored.labels,
        assignees: stored.assignees,
        reviewers: stored.reviewers,
        draft_ready: stored.draft_ready,
        base_ref: stored.base_ref,
        merge: stored.merge,
        auto_merge: stored.auto_merge,
    })
}

fn review_decision_to_str(value: ReviewDecision) -> &'static str {
    match value {
        ReviewDecision::Approved => "approved",
        ReviewDecision::ChangesRequested => "changes_requested",
        ReviewDecision::ReviewRequired => "review_required",
        ReviewDecision::Unknown => "unknown",
    }
}

fn review_decision_from_str(value: &str) -> ReviewDecision {
    match value {
        "approved" => ReviewDecision::Approved,
        "changes_requested" => ReviewDecision::ChangesRequested,
        "review_required" => ReviewDecision::ReviewRequired,
        _ => ReviewDecision::Unknown,
    }
}

fn mergeable_to_str(value: MergeableState) -> &'static str {
    match value {
        MergeableState::Mergeable => "mergeable",
        MergeableState::Conflicting => "conflicting",
        MergeableState::Blocked => "blocked",
        MergeableState::Unknown => "unknown",
    }
}

fn mergeable_from_str(value: &str) -> MergeableState {
    match value {
        "mergeable" => MergeableState::Mergeable,
        "conflicting" => MergeableState::Conflicting,
        "blocked" => MergeableState::Blocked,
        _ => MergeableState::Unknown,
    }
}

fn state_to_str(value: PullRequestState) -> &'static str {
    match value {
        PullRequestState::Open => "open",
        PullRequestState::Merged => "merged",
        PullRequestState::Closed => "closed",
    }
}

fn state_from_str(value: &str) -> PullRequestState {
    match value {
        "merged" => PullRequestState::Merged,
        "closed" => PullRequestState::Closed,
        _ => PullRequestState::Open,
    }
}

fn decode_reviewers(json: &str) -> Result<Vec<Reviewer>, LocalStoreError> {
    let stored: Vec<StoredReviewer> =
        serde_json::from_str(json).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(stored
        .into_iter()
        .map(|reviewer| Reviewer {
            login: reviewer.login,
            avatar_url: reviewer.avatar_url,
        })
        .collect())
}

fn text_at(row: &turso::Row, index: usize) -> Result<String, LocalStoreError> {
    match row
        .get_value(index)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        Value::Text(text) => Ok(text),
        Value::Null => Err(LocalStoreError::other(format!(
            "unexpected null at column {index}"
        ))),
        _ => Err(LocalStoreError::other(format!(
            "unexpected value at column {index}"
        ))),
    }
}

fn optional_text_at(row: &turso::Row, index: usize) -> Result<Option<String>, LocalStoreError> {
    match row
        .get_value(index)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        Value::Null => Ok(None),
        Value::Text(text) => Ok(Some(text)),
        _ => Err(LocalStoreError::other(format!(
            "unexpected value at column {index}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_capabilities_accepts_empty_object() {
        let caps = decode_capabilities("{}").unwrap();
        assert_eq!(caps, MetadataWriteCapabilities::default());
    }

    #[test]
    fn decode_labels_accepts_empty_array() {
        assert!(decode_labels("[]").unwrap().is_empty());
    }
}

fn bool_at(row: &turso::Row, index: usize) -> Result<bool, LocalStoreError> {
    match row
        .get_value(index)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        Value::Integer(value) => Ok(value != 0),
        _ => Err(LocalStoreError::other(format!(
            "expected integer at column {index}"
        ))),
    }
}
