use reviewdesk_core::LocalStoreError;
use turso::Error as TursoError;

pub fn map_open(path: &std::path::Path, err: TursoError) -> LocalStoreError {
    LocalStoreError::open(path.display().to_string(), err.to_string())
}

pub fn map_migration(err: TursoError) -> LocalStoreError {
    LocalStoreError::migration(err.to_string())
}

pub fn map_other(err: TursoError) -> LocalStoreError {
    LocalStoreError::other(err.to_string())
}
