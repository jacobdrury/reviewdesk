//! Private paint modules. Not a behavioral seam — they read [`SessionView`] only.

mod files;
mod inbox;
mod overview;
mod pipelines_tab;
mod review_pane;
mod shell_prototype;

pub(crate) use files::render_files;
#[allow(unused_imports)] // kept for non-prototype shell restore
pub(crate) use inbox::render_inbox;
pub(crate) use overview::render_overview;
pub(crate) use pipelines_tab::{PipelinesPaint, render_compact_check_rollup, render_pipelines_tab};
#[allow(unused_imports)] // kept for non-prototype shell restore
pub(crate) use review_pane::render_review_pane;
pub(crate) use shell_prototype::{PrototypeChrome, render_prototype_shell, sync_session_tab};
