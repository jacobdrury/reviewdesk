//! GitHub.com Provider for Review Desk.

mod client;
mod error_map;
mod id;
mod map;
mod notification;
mod pipelines_map;
mod query;
mod threads_map;

pub use client::GitHubProvider;
pub use id::{GitHubIdError, parse_pull_request_id, pull_request_id};
pub use query::{pull_request_search_queries, pull_request_search_query};
