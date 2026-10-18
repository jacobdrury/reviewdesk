//! Private paint modules. Not a behavioral seam — they read [`SessionView`] only.

mod files;
mod inbox;
mod overview;
mod pull_requests;
mod review_pane;
mod sidebar;

pub(crate) use files::render_files;
pub(crate) use overview::render_overview;
pub(crate) use review_pane::render_review_pane;
pub(crate) use sidebar::render_sidebar;
