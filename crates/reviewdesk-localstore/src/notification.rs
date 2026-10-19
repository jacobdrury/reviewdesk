use chrono::{DateTime, Utc};
use reviewdesk_core::{
    LocalStoreError, NotificationQuery, NotificationReason, NotificationThread,
    NotificationThreadId, PullRequestId,
};
use turso::Connection;

use crate::error::map_other;

pub async fn sync_notifications(
    conn: &Connection,
    threads: &[NotificationThread],
) -> Result<(), LocalStoreError> {
    for thread in threads {
        upsert_thread(conn, thread).await?;
    }
    reconcile(conn, threads).await?;
    Ok(())
}

async fn upsert_thread(
    conn: &Connection,
    thread: &NotificationThread,
) -> Result<(), LocalStoreError> {
    let unread = if thread.unread { 1i64 } else { 0 };
    let updated_at = thread.updated_at.to_rfc3339();

    conn.execute(
        "
insert into reviewdesk_notification_threads (
    id, pull_request_id, title, repo_label, reason, unread, updated_at
) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)
on conflict(id) do update set
    pull_request_id = excluded.pull_request_id,
    title = excluded.title,
    repo_label = excluded.repo_label,
    reason = excluded.reason,
    unread = excluded.unread,
    updated_at = excluded.updated_at
",
        (
            thread.id.as_str(),
            thread.pull_request_id.as_str(),
            thread.title.as_str(),
            thread.repo_label.as_str(),
            thread.reason.as_str(),
            unread,
            updated_at.as_str(),
        ),
    )
    .await
    .map_err(map_other)?;

    Ok(())
}

async fn reconcile(
    conn: &Connection,
    threads: &[NotificationThread],
) -> Result<(), LocalStoreError> {
    if threads.is_empty() {
        conn.execute("delete from reviewdesk_notification_threads", ())
            .await
            .map_err(map_other)?;
        return Ok(());
    }

    let mut stmt = conn
        .prepare("delete from reviewdesk_notification_threads where id = ?1")
        .await
        .map_err(map_other)?;

    let keep: std::collections::HashSet<&str> = threads.iter().map(|t| t.id.as_str()).collect();
    let mut rows = conn
        .query("select id from reviewdesk_notification_threads", ())
        .await
        .map_err(map_other)?;

    let mut stale = Vec::new();
    while let Some(row) = rows.next().await.map_err(map_other)? {
        let id = value_to_string(
            &row.get_value(0)
                .map_err(|e| LocalStoreError::other(e.to_string()))?,
        )?;
        if !keep.contains(id.as_str()) {
            stale.push(id);
        }
    }

    for id in stale {
        stmt.execute([id.as_str()]).await.map_err(map_other)?;
    }

    Ok(())
}

pub async fn query_notifications(
    conn: &Connection,
    query: &NotificationQuery,
) -> Result<Vec<NotificationThread>, LocalStoreError> {
    let sql = if query.unread_only {
        "select id, pull_request_id, title, repo_label, reason, unread, updated_at \
         from reviewdesk_notification_threads where unread = 1 order by updated_at desc"
    } else {
        "select id, pull_request_id, title, repo_label, reason, unread, updated_at \
         from reviewdesk_notification_threads order by updated_at desc"
    };

    let mut rows = conn.query(sql, ()).await.map_err(map_other)?;
    let mut threads = Vec::new();
    while let Some(row) = rows.next().await.map_err(map_other)? {
        threads.push(row_to_thread(&row)?);
    }
    Ok(threads)
}

pub async fn set_notification_read(
    conn: &Connection,
    id: &NotificationThreadId,
) -> Result<(), LocalStoreError> {
    conn.execute(
        "update reviewdesk_notification_threads set unread = 0 where id = ?1",
        [id.as_str()],
    )
    .await
    .map_err(map_other)?;
    Ok(())
}

pub async fn remove_notification(
    conn: &Connection,
    id: &NotificationThreadId,
) -> Result<(), LocalStoreError> {
    conn.execute(
        "delete from reviewdesk_notification_threads where id = ?1",
        [id.as_str()],
    )
    .await
    .map_err(map_other)?;
    Ok(())
}

fn row_to_thread(row: &turso::Row) -> Result<NotificationThread, LocalStoreError> {
    let id = value_to_string(
        &row.get_value(0)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let pull_request_id = value_to_string(
        &row.get_value(1)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let title = value_to_string(
        &row.get_value(2)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let repo_label = value_to_string(
        &row.get_value(3)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let reason_raw = value_to_string(
        &row.get_value(4)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;
    let unread = match row
        .get_value(5)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        turso::Value::Integer(v) => v != 0,
        _ => true,
    };
    let updated_raw = value_to_string(
        &row.get_value(6)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
    )?;

    let reason = NotificationReason::parse(&reason_raw).ok_or_else(|| {
        LocalStoreError::other(format!("unknown stored notification reason: {reason_raw}"))
    })?;
    let updated_at = DateTime::parse_from_rfc3339(&updated_raw)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
        .with_timezone(&Utc);

    Ok(NotificationThread {
        id: NotificationThreadId::new(id).map_err(|e| LocalStoreError::other(e.to_string()))?,
        pull_request_id: PullRequestId::new(pull_request_id)
            .map_err(|e| LocalStoreError::other(e.to_string()))?,
        title,
        repo_label,
        reason,
        unread,
        updated_at,
    })
}

fn value_to_string(value: &turso::Value) -> Result<String, LocalStoreError> {
    match value {
        turso::Value::Text(s) => Ok(s.clone()),
        turso::Value::Integer(i) => Ok(i.to_string()),
        turso::Value::Null => Ok(String::new()),
        _ => Err(LocalStoreError::other("unexpected column type")),
    }
}
