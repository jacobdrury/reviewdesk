//! Private paint modules. Not a behavioral seam — they read [`SessionView`] only.

mod files;
mod overview;
mod pull_requests;
mod review_pane;

pub(crate) use files::render_files;
pub(crate) use overview::render_overview;
pub(crate) use pull_requests::render_pull_requests;
pub(crate) use review_pane::render_review_pane;
