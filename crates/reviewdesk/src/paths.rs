//! Per-user application data paths for the local database.

use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "reviewdesk";
const DB_FILE_NAME: &str = "local.db";

/// Override for tests and debugging (`REVIEWDESK_DATA_DIR`).
pub const DATA_DIR_ENV: &str = "REVIEWDESK_DATA_DIR";

/// Resolves the Review Desk application data directory (OS conventions).
pub fn app_data_dir() -> Result<PathBuf, String> {
    if let Ok(override_dir) = std::env::var(DATA_DIR_ENV) {
        if override_dir.is_empty() {
            return Err(format!("{DATA_DIR_ENV} must not be empty"));
        }
        return Ok(PathBuf::from(override_dir));
    }

    let base = directories::ProjectDirs::from("", "", APP_DIR_NAME)
        .ok_or_else(|| "could not resolve application data directory".to_string())?;
    Ok(base.data_dir().to_path_buf())
}

/// Path to the embedded Turso database file under the app data directory.
pub fn local_store_db_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join(DB_FILE_NAME))
}

/// Ensures the parent directory for `db_path` exists.
pub fn ensure_db_parent(db_path: &Path) -> Result<(), String> {
    let parent = db_path
        .parent()
        .ok_or_else(|| "database path has no parent directory".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create data directory {}: {e}", parent.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_store_db_path_lives_under_app_data_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let data = dir.path().join("data");
        // SAFETY: test runs serially; only this thread touches the override env var.
        unsafe {
            std::env::set_var(DATA_DIR_ENV, data.to_str().expect("utf8"));
        }
        let db = local_store_db_path().expect("path");
        assert_eq!(db, data.join(DB_FILE_NAME));
        unsafe {
            std::env::remove_var(DATA_DIR_ENV);
        }
    }

    #[test]
    fn empty_override_is_rejected() {
        unsafe {
            std::env::set_var(DATA_DIR_ENV, "");
        }
        assert!(app_data_dir().is_err());
        unsafe {
            std::env::remove_var(DATA_DIR_ENV);
        }
    }
}
