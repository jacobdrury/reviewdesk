//! Turso-backed [`LocalStore`](reviewdesk_core::LocalStore) for Review Desk.

mod detail_patch;
mod error;
mod inbox;
mod migrate;
mod store;

pub use store::TursoLocalStore;
