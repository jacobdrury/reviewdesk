use reviewdesk_core::LocalStoreError;
use turso::Connection;

use crate::error::{map_migration, map_other};

/// Ordered, idempotent schema steps. Each version runs at most once per database.
const MIGRATIONS: &[(u32, &str)] = &[
    (
        1,
        "
CREATE TABLE IF NOT EXISTS reviewdesk_schema_migrations (
    version INTEGER NOT NULL PRIMARY KEY
);
CREATE TABLE IF NOT EXISTS reviewdesk_inbox_items (
    id TEXT NOT NULL PRIMARY KEY
);
",
    ),
    (
        2,
        "
CREATE TABLE IF NOT EXISTS pull_request_details (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    author_login TEXT NOT NULL,
    author_avatar_url TEXT,
    is_draft INTEGER NOT NULL,
    labels_json TEXT NOT NULL,
    web_url TEXT NOT NULL,
    reviewers_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS patches (
    pull_request_id TEXT NOT NULL PRIMARY KEY,
    patch_text TEXT NOT NULL
);
",
    ),
    (
        3,
        "
DROP TABLE IF EXISTS reviewdesk_inbox_item_labels;
DROP TABLE IF EXISTS reviewdesk_inbox_items;
CREATE TABLE reviewdesk_inbox_items (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    web_url TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author_login TEXT NOT NULL,
    author_avatar_url TEXT,
    repo_label TEXT NOT NULL,
    review_state TEXT NOT NULL,
    is_draft INTEGER NOT NULL DEFAULT 0 CHECK (is_draft IN (0, 1))
);
CREATE TABLE reviewdesk_inbox_item_labels (
    inbox_item_id TEXT NOT NULL REFERENCES reviewdesk_inbox_items(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    PRIMARY KEY (inbox_item_id, name)
);
CREATE INDEX idx_inbox_items_updated_at ON reviewdesk_inbox_items(updated_at);
CREATE INDEX idx_inbox_items_repo_label ON reviewdesk_inbox_items(repo_label);
CREATE INDEX idx_inbox_items_author_login ON reviewdesk_inbox_items(author_login);
CREATE INDEX idx_inbox_items_review_state ON reviewdesk_inbox_items(review_state);
CREATE INDEX idx_inbox_items_is_draft ON reviewdesk_inbox_items(is_draft);
CREATE INDEX idx_inbox_item_labels_name ON reviewdesk_inbox_item_labels(name);
",
    ),
    (
        4,
        "
drop table if exists reviewdesk_inbox_item_labels;
drop table if exists reviewdesk_inbox_items;
create table reviewdesk_pull_request_list_items (
    id text not null primary key,
    title text not null,
    web_url text not null,
    updated_at text not null,
    author_login text not null,
    author_avatar_url text,
    repo_label text not null,
    review_state text not null,
    is_draft integer not null default 0 check (is_draft in (0, 1))
);
create table reviewdesk_pull_request_list_item_labels (
    pull_request_list_item_id text not null references reviewdesk_pull_request_list_items(id) on delete cascade,
    name text not null,
    color text not null,
    primary key (pull_request_list_item_id, name)
);
create index idx_pull_request_list_items_updated_at on reviewdesk_pull_request_list_items(updated_at);
create index idx_pull_request_list_items_repo_label on reviewdesk_pull_request_list_items(repo_label);
create index idx_pull_request_list_items_author_login on reviewdesk_pull_request_list_items(author_login);
create index idx_pull_request_list_items_review_state on reviewdesk_pull_request_list_items(review_state);
create index idx_pull_request_list_items_is_draft on reviewdesk_pull_request_list_items(is_draft);
create index idx_pull_request_list_item_labels_name on reviewdesk_pull_request_list_item_labels(name);
",
    ),
    (
        5,
        "
create table reviewdesk_notification_threads (
    id text not null primary key,
    pull_request_id text not null,
    title text not null,
    repo_label text not null,
    reason text not null,
    unread integer not null default 1 check (unread in (0, 1)),
    updated_at text not null
);
create index idx_notification_threads_updated_at on reviewdesk_notification_threads(updated_at);
create index idx_notification_threads_unread on reviewdesk_notification_threads(unread);
",
    ),
    (
        6,
        "
create table if not exists pull_request_threads (
    pull_request_id text not null primary key,
    threads_json text not null
);
",
    ),
    (
        7,
        "
alter table pull_request_details add column assignees_json text not null default '[]';
alter table pull_request_details add column head_ref text not null default '';
alter table pull_request_details add column base_ref text not null default '';
alter table pull_request_details add column review_decision text not null default 'unknown';
alter table pull_request_details add column mergeable text not null default 'unknown';
alter table pull_request_details add column codeowners_rules_json text not null default '[]';
alter table pull_request_details add column write_capabilities_json text not null default '{}';
",
    ),
    (
        8,
        "
alter table pull_request_details add column state text not null default 'open';
alter table pull_request_details add column auto_merge_enabled integer not null default 0 check (auto_merge_enabled in (0, 1));
",
    ),
];

pub async fn prefer_wal(conn: &Connection) -> Result<(), LocalStoreError> {
    let mut rows = conn
        .query("PRAGMA journal_mode=WAL;", ())
        .await
        .map_err(map_other)?;
    while rows.next().await.map_err(map_other)?.is_some() {}
    Ok(())
}

pub async fn run(conn: &Connection) -> Result<(), LocalStoreError> {
    let applied = applied_version(conn).await?;
    let latest = MIGRATIONS.last().map(|(v, _)| *v).unwrap_or(0);

    if applied > latest {
        return Err(LocalStoreError::migration(format!(
            "database schema version {applied} is newer than supported {latest}"
        )));
    }

    let mut current = applied;
    for &(version, sql) in MIGRATIONS {
        if version <= current {
            continue;
        }
        if version != current + 1 {
            return Err(LocalStoreError::migration(format!(
                "missing migration step after version {current}"
            )));
        }

        conn.execute_batch(sql).await.map_err(map_migration)?;
        conn.execute(
            "INSERT INTO reviewdesk_schema_migrations (version) VALUES (?1)",
            [i64::from(version)],
        )
        .await
        .map_err(map_migration)?;
        current = version;
    }

    Ok(())
}

pub async fn applied_version(conn: &Connection) -> Result<u32, LocalStoreError> {
    let mut stmt = match conn
        .prepare("SELECT COALESCE(MAX(version), 0) FROM reviewdesk_schema_migrations")
        .await
    {
        Ok(stmt) => stmt,
        Err(_) => return Ok(0),
    };

    let mut rows = stmt.query(()).await.map_err(map_other)?;
    let row = rows
        .next()
        .await
        .map_err(map_other)?
        .ok_or_else(|| LocalStoreError::other("schema version query returned no row"))?;

    let value = row
        .get_value(0)
        .map_err(|e| LocalStoreError::other(e.to_string()))?;
    match value {
        turso::Value::Integer(v) => {
            if v < 0 {
                return Err(LocalStoreError::other("negative schema version"));
            }
            Ok(v as u32)
        }
        _ => Err(LocalStoreError::other("unexpected schema version type")),
    }
}
