//! GitHub.com Provider for Review Desk.

mod client;
mod error_map;
mod id;
mod map;
mod query;

pub use client::GitHubProvider;
pub use id::{GitHubIdError, parse_pull_request_id, pull_request_id};
pub use query::inbox_search_query;
