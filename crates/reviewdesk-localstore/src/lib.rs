//! Turso-backed [`LocalStore`](reviewdesk_core::LocalStore) for Review Desk.

mod detail_patch;
mod threads;
mod error;
mod migrate;
mod pull_request_list;
mod store;

pub use store::TursoLocalStore;
