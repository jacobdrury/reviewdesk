use std::collections::HashMap;

use chrono::{DateTime, Utc};
use reviewdesk_core::{
    DraftFilter, InboxItem, InboxLabel, InboxQuery, InboxSortField, LocalStoreError, ReviewState,
    SortDirection,
};
use turso::Connection;

use crate::error::map_other;

pub async fn sync_inbox(conn: &Connection, items: &[InboxItem]) -> Result<(), LocalStoreError> {
    for item in items {
        upsert_item(conn, item).await?;
    }
    reconcile(conn, items).await?;
    Ok(())
}

async fn upsert_item(conn: &Connection, item: &InboxItem) -> Result<(), LocalStoreError> {
    let review_state = encode_review_state(item.review_state);
    let updated_at = item.updated_at.to_rfc3339();
    let is_draft = if item.is_draft { 1i64 } else { 0 };

    conn.execute(
        "
INSERT INTO reviewdesk_inbox_items (
    id, title, web_url, updated_at, author_login, author_avatar_url, repo_label, review_state, is_draft
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
ON CONFLICT(id) DO UPDATE SET
    title = excluded.title,
    web_url = excluded.web_url,
    updated_at = excluded.updated_at,
    author_login = excluded.author_login,
    author_avatar_url = excluded.author_avatar_url,
    repo_label = excluded.repo_label,
    review_state = excluded.review_state,
    is_draft = excluded.is_draft
",
        (
            item.id.as_str(),
            item.title.as_str(),
            item.web_url.as_str(),
            updated_at.as_str(),
            item.author_login.as_str(),
            item.author_avatar_url.as_deref(),
            item.repo_label.as_str(),
            review_state.as_str(),
            is_draft,
        ),
    )
    .await
    .map_err(map_other)?;

    conn.execute(
        "DELETE FROM reviewdesk_inbox_item_labels WHERE inbox_item_id = ?1",
        [item.id.as_str()],
    )
    .await
    .map_err(map_other)?;

    for label in &item.labels {
        conn.execute(
            "
INSERT INTO reviewdesk_inbox_item_labels (inbox_item_id, name, color)
VALUES (?1, ?2, ?3)
",
            (item.id.as_str(), label.name.as_str(), label.color.as_str()),
        )
        .await
        .map_err(map_other)?;
    }

    Ok(())
}

async fn reconcile(conn: &Connection, items: &[InboxItem]) -> Result<(), LocalStoreError> {
    if items.is_empty() {
        conn.execute("DELETE FROM reviewdesk_inbox_items", ())
            .await
            .map_err(map_other)?;
        return Ok(());
    }

    let mut stmt = conn
        .prepare("DELETE FROM reviewdesk_inbox_items WHERE id = ?1")
        .await
        .map_err(map_other)?;

    let keep: std::collections::HashSet<&str> = items.iter().map(|i| i.id.as_str()).collect();
    let mut rows = conn
        .query("SELECT id FROM reviewdesk_inbox_items", ())
        .await
        .map_err(map_other)?;

    let mut stale = Vec::new();
    while let Some(row) = rows.next().await.map_err(map_other)? {
        let id = row
            .get_value(0)
            .map_err(|e| LocalStoreError::other(e.to_string()))?;
        let id = value_to_string(&id)?;
        if !keep.contains(id.as_str()) {
            stale.push(id);
        }
    }

    for id in stale {
        stmt.execute([id.as_str()]).await.map_err(map_other)?;
    }

    Ok(())
}

pub async fn query_inbox(
    conn: &Connection,
    query: &InboxQuery,
) -> Result<Vec<InboxItem>, LocalStoreError> {
    let (sql, params) = build_list_sql(query)?;
    let mut rows = conn.query(&sql, params).await.map_err(map_other)?;

    let mut items = Vec::new();
    let mut ids = Vec::new();
    while let Some(row) = rows.next().await.map_err(map_other)? {
        let (item, id) = row_to_inbox_item(&row)?;
        items.push(item);
        ids.push(id);
    }

    if ids.is_empty() {
        return Ok(items);
    }

    let labels = load_labels(conn, &ids).await?;
    for item in &mut items {
        item.labels = labels.get(item.id.as_str()).cloned().unwrap_or_default();
    }

    Ok(items)
}

fn build_list_sql(query: &InboxQuery) -> Result<(String, Vec<turso::Value>), LocalStoreError> {
    let mut clauses = Vec::new();
    let mut params: Vec<turso::Value> = Vec::new();

    match query.filter.options.drafts {
        DraftFilter::ReadyOnly => clauses.push("is_draft = 0".to_string()),
        DraftFilter::DraftsOnly => clauses.push("is_draft = 1".to_string()),
        DraftFilter::All => {}
    }

    if let Some(repo) = &query.filter.repo_label {
        params.push(turso::Value::Text(repo.clone()));
        clauses.push(format!("repo_label = ?{}", params.len()));
    }
    if let Some(author) = &query.filter.author_login {
        params.push(turso::Value::Text(author.clone()));
        clauses.push(format!("author_login = ?{}", params.len()));
    }
    if let Some(state) = query.filter.review_state {
        params.push(turso::Value::Text(encode_review_state(state)));
        clauses.push(format!("review_state = ?{}", params.len()));
    }
    if let Some(label) = &query.filter.has_label {
        params.push(turso::Value::Text(label.clone()));
        clauses.push(format!(
            "EXISTS (SELECT 1 FROM reviewdesk_inbox_item_labels l WHERE l.inbox_item_id = reviewdesk_inbox_items.id AND l.name = ?{})",
            params.len()
        ));
    }

    let order = match (query.sort.field, query.sort.direction) {
        (InboxSortField::UpdatedAt, SortDirection::Desc) => "updated_at DESC",
        (InboxSortField::UpdatedAt, SortDirection::Asc) => "updated_at ASC",
    };

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };

    let sql = format!(
        "SELECT id, title, web_url, updated_at, author_login, author_avatar_url, repo_label, review_state, is_draft FROM reviewdesk_inbox_items{where_sql} ORDER BY {order}"
    );

    Ok((sql, params))
}

async fn load_labels(
    conn: &Connection,
    ids: &[String],
) -> Result<HashMap<String, Vec<InboxLabel>>, LocalStoreError> {
    let mut map: HashMap<String, Vec<InboxLabel>> = HashMap::new();
    for id in ids {
        let mut rows = conn
            .query(
                "SELECT name, color FROM reviewdesk_inbox_item_labels WHERE inbox_item_id = ?1 ORDER BY name",
                [id.as_str()],
            )
            .await
            .map_err(map_other)?;
        while let Some(row) = rows.next().await.map_err(map_other)? {
            let name = value_to_string(
                &row.get_value(0)
                    .map_err(|e| LocalStoreError::other(e.to_string()))?,
            )?;
            let color = value_to_string(
                &row.get_value(1)
                    .map_err(|e| LocalStoreError::other(e.to_string()))?,
            )?;
            map.entry(id.clone())
                .or_default()
                .push(InboxLabel { name, color });
        }
    }
    Ok(map)
}

fn row_to_inbox_item(row: &turso::Row) -> Result<(InboxItem, String), LocalStoreError> {
    use reviewdesk_core::PullRequestId;

    let id = value_to_string(
        &row.get_value(0)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let title = value_to_string(
        &row.get_value(1)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let web_url = value_to_string(
        &row.get_value(2)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let updated_raw = value_to_string(
        &row.get_value(3)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let author_login = value_to_string(
        &row.get_value(4)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let author_avatar_url = optional_string(
        row.get_value(5)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let repo_label = value_to_string(
        &row.get_value(6)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let review_state = decode_review_state(&value_to_string(
        &row.get_value(7)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?)?;
    let is_draft = match row
        .get_value(8)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        turso::Value::Integer(v) => v != 0,
        _ => false,
    };

    let updated_at = DateTime::parse_from_rfc3339(&updated_raw)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
        .with_timezone(&Utc);

    let pr_id =
        PullRequestId::new(id.clone()).map_err(|e| LocalStoreError::other(e.to_string()))?;

    let item = InboxItem {
        id: pr_id,
        title,
        web_url,
        updated_at,
        author_login,
        author_avatar_url,
        repo_label,
        labels: Vec::new(),
        review_state,
        is_draft,
    };

    Ok((item, id))
}

fn value_to_string(value: &turso::Value) -> Result<String, LocalStoreError> {
    match value {
        turso::Value::Text(s) => Ok(s.clone()),
        turso::Value::Integer(i) => Ok(i.to_string()),
        turso::Value::Null => Ok(String::new()),
        _ => Err(LocalStoreError::other("unexpected column type")),
    }
}

fn optional_string(value: turso::Value) -> Result<Option<String>, LocalStoreError> {
    match value {
        turso::Value::Null => Ok(None),
        other => Ok(Some(value_to_string(&other)?)),
    }
}

fn encode_review_state(state: ReviewState) -> String {
    match state {
        ReviewState::Pending => "pending",
        ReviewState::Approved => "approved",
        ReviewState::ChangesRequested => "changes_requested",
        ReviewState::Commented => "commented",
        ReviewState::Dismissed => "dismissed",
        ReviewState::Unknown => "unknown",
    }
    .to_string()
}

fn decode_review_state(raw: &str) -> Result<ReviewState, LocalStoreError> {
    Ok(match raw {
        "pending" => ReviewState::Pending,
        "approved" => ReviewState::Approved,
        "changes_requested" => ReviewState::ChangesRequested,
        "commented" => ReviewState::Commented,
        "dismissed" => ReviewState::Dismissed,
        "unknown" => ReviewState::Unknown,
        _ => ReviewState::Unknown,
    })
}
