//! Turso-backed [`LocalStore`](reviewdesk_core::LocalStore) for Review Desk.

mod detail_patch;
mod error;
mod migrate;
mod notification;
mod pull_request_list;
mod store;
mod views;

pub use store::TursoLocalStore;
