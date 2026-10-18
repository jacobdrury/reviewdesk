use reviewdesk_core::{LocalStoreError, PipelinesSnapshot, PullRequestId};
use turso::Connection;

use crate::error::map_other;

pub async fn put_pipelines(
    conn: &Connection,
    id: &PullRequestId,
    snapshot: &PipelinesSnapshot,
) -> Result<(), LocalStoreError> {
    let json =
        serde_json::to_string(snapshot).map_err(|e| LocalStoreError::other(e.to_string()))?;
    conn.execute(
        "
insert into pull_request_pipelines (pull_request_id, snapshot_json)
values (?1, ?2)
on conflict(pull_request_id) do update set snapshot_json = excluded.snapshot_json
",
        (id.as_str(), json.as_str()),
    )
    .await
    .map_err(map_other)?;
    Ok(())
}

pub async fn get_pipelines(
    conn: &Connection,
    id: &PullRequestId,
) -> Result<Option<PipelinesSnapshot>, LocalStoreError> {
    let mut rows = conn
        .query(
            "select snapshot_json from pull_request_pipelines where pull_request_id = ?1",
            [id.as_str()],
        )
        .await
        .map_err(map_other)?;

    let row = rows.next().await.map_err(map_other)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let json: String = row
        .get(0)
        .map_err(|e| LocalStoreError::other(e.to_string()))?;
    let snapshot: PipelinesSnapshot =
        serde_json::from_str(&json).map_err(|e| LocalStoreError::other(e.to_string()))?;
    Ok(Some(snapshot))
}
