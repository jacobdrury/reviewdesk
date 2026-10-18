//! Turso-backed [`LocalStore`](reviewdesk_core::LocalStore) for Review Desk.

mod detail_patch;
mod error;
mod migrate;
mod pipelines;
mod pull_request_list;
mod store;

pub use store::TursoLocalStore;
