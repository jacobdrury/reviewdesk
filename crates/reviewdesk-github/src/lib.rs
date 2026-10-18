//! GitHub.com Provider for Review Desk.

mod client;
mod error_map;
mod map;
mod query;

pub use client::GitHubProvider;
pub use query::inbox_search_query;
