use reviewdesk_core::{LocalStoreError, PullRequestId, PullRequestThreads};
use turso::Connection;

use crate::error::map_other;

pub async fn put_pull_request_threads(
    conn: &Connection,
    id: &PullRequestId,
    snapshot: &PullRequestThreads,
) -> Result<(), LocalStoreError> {
    let payload =
        serde_json::to_string(snapshot).map_err(|e| LocalStoreError::other(e.to_string()))?;

    conn.execute(
        "
insert into pull_request_threads (pull_request_id, threads_json)
values (?1, ?2)
on conflict(pull_request_id) do update set threads_json = excluded.threads_json
",
        (id.as_str(), payload.as_str()),
    )
    .await
    .map_err(map_other)?;

    Ok(())
}

pub async fn get_pull_request_threads(
    conn: &Connection,
    id: &PullRequestId,
) -> Result<Option<PullRequestThreads>, LocalStoreError> {
    let mut rows = conn
        .query(
            "select threads_json from pull_request_threads where pull_request_id = ?1",
            [id.as_str()],
        )
        .await
        .map_err(map_other)?;

    let row = rows.next().await.map_err(map_other)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let json = match row
        .get_value(0)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        turso::Value::Text(text) => text,
        _ => return Err(LocalStoreError::other("unexpected threads_json type")),
    };

    let snapshot: PullRequestThreads =
        serde_json::from_str(&json).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(Some(snapshot))
}
