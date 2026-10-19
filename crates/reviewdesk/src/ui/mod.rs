//! Private paint modules. Not a behavioral seam — they read [`SessionView`] only.

mod check_rollup;
mod conversation;
mod files;
mod inbox;
mod metadata;
mod overview;
mod pending_review;
mod pipelines_tab;
mod pull_requests;
mod review_pane;
mod sidebar;

pub(crate) use conversation::{render_conversation, render_diff_thread_annotation};
pub(crate) use files::render_files;
pub(crate) use inbox::render_inbox;
pub(crate) use metadata::{render_metadata_panel, render_metadata_strip};
pub(crate) use overview::render_overview;
pub(crate) use pull_requests::{PullRequestsListChrome, render_pull_requests};
pub(crate) use review_pane::render_review_pane;
pub(crate) use sidebar::render_sidebar;
