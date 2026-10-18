use reviewdesk_core::{
    LocalStoreError, PullRequestListSort, View, ViewFilter, ViewGroupBy, ViewId, ViewKind,
};
use turso::{Connection, Value};

use crate::error::map_other;

const LAST_USED_VIEW_KEY: &str = "last_used_view";

pub async fn list_views(conn: &Connection) -> Result<Vec<View>, LocalStoreError> {
    let mut rows = conn
        .query(
            "select id, name, kind, position, filter_json, group_by, sort_json
from reviewdesk_views
order by position asc, id asc",
            (),
        )
        .await
        .map_err(map_other)?;

    let mut views = Vec::new();
    while let Some(row) = rows.next().await.map_err(map_other)? {
        views.push(row_to_view(&row)?);
    }
    Ok(views)
}

pub async fn save_view(conn: &Connection, view: &View) -> Result<(), LocalStoreError> {
    let kind = encode_json(&view.kind)?;
    let group_by = encode_json(&view.group_by)?;
    let filter_json = encode_json(&view.filter)?;
    let sort_json = encode_json(&view.sort)?;

    conn.execute(
        "
insert into reviewdesk_views (id, name, kind, position, filter_json, group_by, sort_json)
values (?1, ?2, ?3, ?4, ?5, ?6, ?7)
on conflict(id) do update set
    name = excluded.name,
    kind = excluded.kind,
    position = excluded.position,
    filter_json = excluded.filter_json,
    group_by = excluded.group_by,
    sort_json = excluded.sort_json
",
        (
            view.id.as_str(),
            view.name.as_str(),
            kind.as_str(),
            view.position,
            filter_json.as_str(),
            group_by.as_str(),
            sort_json.as_str(),
        ),
    )
    .await
    .map_err(map_other)?;

    Ok(())
}

pub async fn delete_view(conn: &Connection, id: &ViewId) -> Result<(), LocalStoreError> {
    conn.execute(
        "delete from reviewdesk_views where id = ?1",
        [id.as_str()],
    )
    .await
    .map_err(map_other)?;
    Ok(())
}

pub async fn reorder_views(conn: &Connection, order: &[ViewId]) -> Result<(), LocalStoreError> {
    for (position, id) in order.iter().enumerate() {
        conn.execute(
            "update reviewdesk_views set position = ?1 where id = ?2",
            (position as i64, id.as_str()),
        )
        .await
        .map_err(map_other)?;
    }
    Ok(())
}

pub async fn get_last_used_view(conn: &Connection) -> Result<Option<ViewId>, LocalStoreError> {
    let mut rows = conn
        .query(
            "select value from reviewdesk_preferences where key = ?1",
            [LAST_USED_VIEW_KEY],
        )
        .await
        .map_err(map_other)?;

    let Some(row) = rows.next().await.map_err(map_other)? else {
        return Ok(None);
    };
    Ok(Some(ViewId::new(text_at(&row, 0)?)))
}

pub async fn set_last_used_view(conn: &Connection, id: &ViewId) -> Result<(), LocalStoreError> {
    conn.execute(
        "
insert into reviewdesk_preferences (key, value) values (?1, ?2)
on conflict(key) do update set value = excluded.value
",
        (LAST_USED_VIEW_KEY, id.as_str()),
    )
    .await
    .map_err(map_other)?;
    Ok(())
}

fn row_to_view(row: &turso::Row) -> Result<View, LocalStoreError> {
    let id = ViewId::new(text_at(row, 0)?);
    let name = text_at(row, 1)?;
    let kind: ViewKind = decode_json(&text_at(row, 2)?)?;
    let position = int_at(row, 3)?;
    let filter: ViewFilter = decode_json(&text_at(row, 4)?)?;
    let group_by: ViewGroupBy = decode_json(&text_at(row, 5)?)?;
    let sort: PullRequestListSort = decode_json(&text_at(row, 6)?)?;

    Ok(View {
        id,
        name,
        kind,
        position,
        filter,
        group_by,
        sort,
    })
}

fn encode_json<T: serde::Serialize>(value: &T) -> Result<String, LocalStoreError> {
    serde_json::to_string(value).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn decode_json<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, LocalStoreError> {
    serde_json::from_str(json).map_err(|e| LocalStoreError::other(e.to_string()))
}

fn text_at(row: &turso::Row, index: usize) -> Result<String, LocalStoreError> {
    match row
        .get_value(index)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        Value::Text(text) => Ok(text),
        _ => Err(LocalStoreError::other(format!(
            "expected text at column {index}"
        ))),
    }
}

fn int_at(row: &turso::Row, index: usize) -> Result<i64, LocalStoreError> {
    match row
        .get_value(index)
        .map_err(|e| LocalStoreError::other(e.to_string()))?
    {
        Value::Integer(value) => Ok(value),
        _ => Err(LocalStoreError::other(format!(
            "expected integer at column {index}"
        ))),
    }
}
