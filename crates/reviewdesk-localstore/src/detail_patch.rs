use reviewdesk_core::{
    InboxLabel, LocalStoreError, Patch, PullRequestDetail, PullRequestId, Reviewer,
};
use serde::{Deserialize, Serialize};
use turso::{Connection, Value};

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

pub async fn put_pull_request_detail(
    conn: &Connection,
    detail: &PullRequestDetail,
) -> Result<(), LocalStoreError> {
    let labels = encode_labels(&detail.labels)?;
    let reviewers = encode_reviewers(&detail.reviewers)?;
    let is_draft = if detail.is_draft { 1i64 } else { 0i64 };

    conn.execute(
        "
INSERT INTO pull_request_details (
    id, title, body, author_login, author_avatar_url, is_draft, labels_json, web_url, reviewers_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
ON CONFLICT(id) DO UPDATE SET
    title = excluded.title,
    body = excluded.body,
    author_login = excluded.author_login,
    author_avatar_url = excluded.author_avatar_url,
    is_draft = excluded.is_draft,
    labels_json = excluded.labels_json,
    web_url = excluded.web_url,
    reviewers_json = excluded.reviewers_json
",
        (
            detail.id.as_str(),
            detail.title.as_str(),
            detail.body.as_str(),
            detail.author_login.as_str(),
            detail.author_avatar_url.as_deref(),
            is_draft,
            labels.as_str(),
            detail.web_url.as_str(),
            reviewers.as_str(),
        ),
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
SELECT
    id, title, body, author_login, author_avatar_url, is_draft, labels_json, web_url, reviewers_json
FROM pull_request_details
WHERE id = ?1
",
            [id.as_str()],
        )
        .await
        .map_err(map_other)?;

    let row = rows.next().await.map_err(map_other)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let pr_id = PullRequestId::new(text_at(&row, 0)?).map_err(|e| LocalStoreError::other(e.to_string()))?;
    let title = text_at(&row, 1)?;
    let body = text_at(&row, 2)?;
    let author_login = text_at(&row, 3)?;
    let author_avatar_url = optional_text_at(&row, 4)?;
    let is_draft = bool_at(&row, 5)?;
    let labels_json = text_at(&row, 6)?;
    let web_url = text_at(&row, 7)?;
    let reviewers_json = text_at(&row, 8)?;

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

fn encode_labels(labels: &[InboxLabel]) -> Result<String, LocalStoreError> {
    let stored = labels
        .iter()
        .map(|label| StoredLabel {
            name: label.name.clone(),
            color: label.color.clone(),
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&stored).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn decode_labels(json: &str) -> Result<Vec<InboxLabel>, LocalStoreError> {
    let stored: Vec<StoredLabel> =
        serde_json::from_str(json).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(stored
        .into_iter()
        .map(|label| InboxLabel {
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
    match row.get_value(index).map_err(|e| LocalStoreError::other(e.to_string()))? {
        Value::Text(text) => Ok(text),
        Value::Null => Err(LocalStoreError::other(format!("unexpected null at column {index}"))),
        _ => Err(LocalStoreError::other(format!("unexpected value at column {index}"))),
    }
}

fn optional_text_at(row: &turso::Row, index: usize) -> Result<Option<String>, LocalStoreError> {
    match row.get_value(index).map_err(|e| LocalStoreError::other(e.to_string()))? {
        Value::Null => Ok(None),
        Value::Text(text) => Ok(Some(text)),
        _ => Err(LocalStoreError::other(format!("unexpected value at column {index}"))),
    }
}

fn bool_at(row: &turso::Row, index: usize) -> Result<bool, LocalStoreError> {
    match row.get_value(index).map_err(|e| LocalStoreError::other(e.to_string()))? {
        Value::Integer(value) => Ok(value != 0),
        _ => Err(LocalStoreError::other(format!("expected integer at column {index}"))),
    }
}
