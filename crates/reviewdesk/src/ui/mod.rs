//! Private paint modules. Not a behavioral seam — they read [`SessionView`] only.

mod check_rollup;
mod files;
mod metadata;
mod overview;
mod pipelines_tab;
mod pull_requests;
mod review_pane;

pub(crate) use files::render_files;
pub(crate) use metadata::{render_metadata_panel, render_metadata_strip};
pub(crate) use overview::render_overview;
pub(crate) use pull_requests::render_pull_requests;
pub(crate) use review_pane::render_review_pane;
