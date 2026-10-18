use reviewdesk_core::LocalStoreError;
use turso::Connection;

use crate::error::{map_migration, map_other};

/// Ordered, idempotent schema steps. Each version runs at most once per database.
const MIGRATIONS: &[(u32, &str)] = &[(
    1,
    "
CREATE TABLE IF NOT EXISTS reviewdesk_schema_migrations (
    version INTEGER NOT NULL PRIMARY KEY
);
CREATE TABLE IF NOT EXISTS reviewdesk_inbox_items (
    id TEXT NOT NULL PRIMARY KEY
);
",
)];

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

    for &(version, sql) in MIGRATIONS {
        if version <= applied {
            continue;
        }
        if version != applied + 1 {
            return Err(LocalStoreError::migration(format!(
                "missing migration step after version {applied}"
            )));
        }

        conn.execute_batch(sql).await.map_err(map_migration)?;
        conn.execute(
            "INSERT INTO reviewdesk_schema_migrations (version) VALUES (?1)",
            [i64::from(version)],
        )
        .await
        .map_err(map_migration)?;
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

    let value = row.get_value(0).map_err(|e| LocalStoreError::other(e.to_string()))?;
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
