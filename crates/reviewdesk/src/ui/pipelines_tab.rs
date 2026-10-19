//! Pipelines tab (baked layout B): Run history nav + stage-grouped job detail.
//!
//! Visual language from `prototype/pipelines-tab` variant B — Linear Issues chrome,
//! dual-pane Project detail, collapsible stage groups. Real LocalStore snapshot data.

use std::collections::HashSet;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    resizable::{h_resizable, resizable_panel},
    scroll::Scrollbar,
    v_flex,
};
use gpui_kit::{
    App, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle, WeakEntity, div,
    prelude::FluentBuilder as _, px, uniform_list,
};

use reviewdesk_core::{
    CheckRollup, CheckRollupState, Job, JobState, Pipeline, PipelineKind, PipelinesSnapshot,
    RerunAction, aggregate_pipeline_state,
};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView};

/// Matches Linear Issues list nest padding from the PR-list / Pipelines prototype.
const LIST_ROW_BASE_PL: f32 = 16.;
const LIST_NEST_STEP: f32 = 20.;

fn list_row_pl(depth: u8) -> f32 {
    LIST_ROW_BASE_PL + LIST_NEST_STEP * f32::from(depth)
}

/// GPUI chrome for the Pipelines tab (scroll + selection + disclosure).
pub(crate) struct PipelinesChrome<'a> {
    pub list_scroll: &'a UniformListScrollHandle,
    pub detail_scroll: &'a UniformListScrollHandle,
    pub selected_pipeline: usize,
    pub collapsed_groups: &'a HashSet<String>,
}

pub(crate) fn render_pipelines_tab(
    view: &SessionView,
    chrome: PipelinesChrome<'_>,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    match &view.pipelines {
        LoadState::Idle | LoadState::Loading => v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Loading Pipelines…"),
            )
            .into_any_element(),
        LoadState::Error(error) => v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(error.to_string()),
            )
            .into_any_element(),
        LoadState::Ready(snapshot) => {
            render_run_history(view, snapshot, chrome, entity, cx).into_any_element()
        }
    }
}

fn render_run_history(
    view: &SessionView,
    snapshot: &PipelinesSnapshot,
    chrome: PipelinesChrome<'_>,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let list_scroll = chrome.list_scroll.clone();
    let detail_scroll = chrome.detail_scroll.clone();
    let collapsed = chrome.collapsed_groups.clone();
    let selected = chrome
        .selected_pipeline
        .min(snapshot.pipelines.len().saturating_sub(1));
    let pipelines = snapshot.pipelines.clone();
    let rerun_busy = view.pipelines_rerun_busy;
    let empty = pipelines.is_empty();

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_linear_toolbar(&snapshot.rollup, cx))
        .child(if empty {
            empty_state(
                "No Pipelines for this head. Empty is not a pass — nothing reported yet.",
                cx,
            )
            .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .child(
                    h_resizable("pipelines-split")
                        .child(
                            resizable_panel()
                                .size(px(240.))
                                .size_range(px(200.)..px(340.))
                                .child(render_pipeline_nav(
                                    pipelines.clone(),
                                    selected,
                                    list_scroll,
                                    entity.clone(),
                                    cx,
                                )),
                        )
                        .child(resizable_panel().size_range(px(280.)..px(2000.)).child(
                            render_run_detail(
                                pipelines.get(selected).cloned(),
                                selected,
                                collapsed,
                                detail_scroll,
                                rerun_busy,
                                entity,
                                cx,
                            ),
                        )),
                )
                .into_any_element()
        })
}

/// Quiet Filter + Display strip + Check rollup chip (Linear top chrome).
fn render_linear_toolbar(rollup: &CheckRollup, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let color = rollup_color(rollup.state, cx);
    let icon = rollup_icon(rollup.state);
    h_flex()
        .w_full()
        .px_4()
        .py_2()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .items_center()
                .gap_1()
                .px_2()
                .py(px(4.))
                .rounded_md()
                .hover(|s| s.bg(cx.theme().secondary.opacity(0.5)))
                .child(
                    Icon::new(IconName::Menu)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Filter"),
                ),
        )
        .child(filter_chip("Rollup", "is", &rollup.label, icon, color, cx))
        .when(rollup.state != CheckRollupState::Absent, |el| {
            el.child(filter_chip(
                "Required",
                "drives",
                "urgency",
                IconName::Asterisk,
                cx.theme().muted_foreground,
                cx,
            ))
        })
        .child(div().flex_1())
        .child(
            h_flex()
                .items_center()
                .gap_1()
                .px_2()
                .py(px(4.))
                .rounded_md()
                .hover(|s| s.bg(cx.theme().secondary.opacity(0.5)))
                .child(
                    Icon::new(IconName::Settings2)
                        .xsmall()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Display"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("· run history"),
                ),
        )
}

fn filter_chip(
    key: &str,
    op: &str,
    value: &str,
    icon: IconName,
    icon_color: gpui_kit::Hsla,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_1()
        .px_2()
        .py(px(3.))
        .rounded_md()
        .bg(cx.theme().secondary.opacity(0.55))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(key.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(op.to_string()),
        )
        .child(Icon::new(icon).xsmall().text_color(icon_color))
        .child(div().text_xs().child(value.to_string()))
}

fn render_pipeline_nav(
    pipelines: Vec<Pipeline>,
    selected: usize,
    scroll: UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let count = pipelines.len();
    v_flex()
        .size_full()
        .min_h_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .px_3()
                .py(px(8.))
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child("Pipelines"),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("pipelines-b-nav", count, move |range, _, cx| {
                        range
                            .map(|ix| {
                                let pipeline = &pipelines[ix];
                                let state = aggregate_pipeline_state(&pipeline.jobs);
                                let is_sel = ix == selected;
                                let owner = entity.clone();
                                let kind = kind_label(pipeline.kind);
                                let created = relative_time(pipeline.created_at);
                                div()
                                    .id(SharedString::from(format!("nav-pl-{ix}")))
                                    .w_full()
                                    .px_3()
                                    .py(px(7.))
                                    .border_b_1()
                                    .border_color(cx.theme().border.opacity(0.55))
                                    .cursor_pointer()
                                    .when(is_sel, |el| el.bg(cx.theme().secondary.opacity(0.4)))
                                    .when(!is_sel, |el| {
                                        el.hover(|s| s.bg(cx.theme().secondary.opacity(0.25)))
                                    })
                                    .on_click(move |_, _, cx| {
                                        let _ = owner.update(cx, |this, cx| {
                                            this.select_pipeline(ix, cx);
                                        });
                                    })
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                Icon::new(job_icon(state))
                                                    .xsmall()
                                                    .text_color(job_color(state, cx)),
                                            )
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .gap_0()
                                                    .child(div().text_sm().truncate().child(
                                                        format!(
                                                            "{} {}",
                                                            pipeline.name, pipeline.run_label
                                                        ),
                                                    ))
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .truncate()
                                                            .child(format!("{kind} · {created}")),
                                                    ),
                                            ),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll)),
        )
}

/// Detail = Issues-list UI scoped to one run, grouped by stage (default: `jobs`).
fn render_run_detail(
    pipeline: Option<Pipeline>,
    pipeline_index: usize,
    collapsed: HashSet<String>,
    scroll: UniformListScrollHandle,
    rerun_busy: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let Some(pipeline) = pipeline else {
        return empty_state("Select a Pipeline run.", cx).into_any_element();
    };
    let name = pipeline.name.clone();
    let run_label = pipeline.run_label.clone();
    let kind = kind_label(pipeline.kind);
    let state = aggregate_pipeline_state(&pipeline.jobs);
    let can_rerun = matches!(pipeline.rerun, RerunAction::Allowed(_));
    let pipeline_id = pipeline.id.clone();
    let rows = build_stage_rows(pipeline_index, &pipeline, &collapsed);
    let row_count = rows.len();
    let rows_for_list = rows;

    v_flex()
        .size_full()
        .min_h_0()
        .child(
            h_flex()
                .w_full()
                .px_4()
                .py(px(8.))
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .min_w_0()
                        .child(
                            Icon::new(job_icon(state))
                                .xsmall()
                                .text_color(job_color(state, cx)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .truncate()
                                .child(format!("{name} {run_label}")),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{kind} · stages → jobs")),
                        ),
                )
                .child(if can_rerun {
                    let mut btn = Button::new("rerun-pipeline")
                        .ghost()
                        .xsmall()
                        .label("Re-run");
                    if rerun_busy {
                        btn = btn.disabled(true);
                    } else {
                        let owner = entity.clone();
                        btn = btn.on_click(move |_, _, cx| {
                            let pipeline_id = pipeline_id.clone();
                            let _ = owner.update(cx, |this, cx| {
                                this.dispatch(Intent::RerunPipeline { pipeline_id }, cx);
                            });
                        });
                    }
                    btn.into_any_element()
                } else {
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("display only")
                        .into_any_element()
                }),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("pipelines-b-detail", row_count, move |range, _, cx| {
                        range
                            .map(|ix| match &rows_for_list[ix] {
                                FlatRow::Group {
                                    key,
                                    label,
                                    count,
                                    state,
                                    depth,
                                } => group_header_row(
                                    key,
                                    label,
                                    *count,
                                    *state,
                                    *depth,
                                    collapsed.contains(key),
                                    entity.clone(),
                                    cx,
                                )
                                .into_any_element(),
                                FlatRow::Job { job, depth } => {
                                    job_issue_row(job, *depth, cx).into_any_element()
                                }
                            })
                            .collect()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll)),
        )
        .into_any_element()
}

fn group_header_row(
    key: &str,
    label: &str,
    count: usize,
    state: Option<JobState>,
    depth: u8,
    is_collapsed: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &App,
) -> impl IntoElement {
    let chevron = if is_collapsed {
        IconName::ChevronRight
    } else {
        IconName::ChevronDown
    };
    let group_key = key.to_string();
    let pt = if depth == 0 { 14. } else { 8. };

    let mut header = h_flex().w_full().items_center().gap_2().child(
        Icon::new(chevron)
            .xsmall()
            .text_color(cx.theme().muted_foreground),
    );
    if let Some(state) = state {
        header = header.child(
            Icon::new(job_icon(state))
                .xsmall()
                .text_color(job_color(state, cx)),
        );
    } else {
        header = header.child(
            Icon::new(IconName::Folder)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        );
    }

    div()
        .id(SharedString::from(format!("group-{key}")))
        .w_full()
        .pl(px(list_row_pl(depth)))
        .pr_4()
        .pt(px(pt))
        .pb(px(6.))
        .cursor_pointer()
        .rounded_md()
        .hover(|s| s.bg(cx.theme().secondary.opacity(0.3)))
        .on_click(move |_, _, cx| {
            let key = group_key.clone();
            let _ = entity.update(cx, |this, cx| {
                this.toggle_pipelines_collapsed_group(key, cx);
            });
        })
        .child(
            header
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(div().text_sm().font_medium().child(label.to_string()))
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(count.to_string()),
                        ),
                )
                .child(div().flex_1()),
        )
}

/// Dense Linear Issues row for a Job.
fn job_issue_row(job: &Job, depth: u8, cx: &App) -> impl IntoElement {
    let emphasis = job.required && matches!(job.state, JobState::Failure | JobState::Pending);
    let can_rerun = matches!(job.rerun, RerunAction::Allowed(_));
    let mut row = h_flex()
        .w_full()
        .items_center()
        .gap_2()
        .child(
            h_flex()
                .flex_none()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_none()
                        .w(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(job_icon(job.state))
                                .xsmall()
                                .text_color(job_color(job.state, cx)),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .w(px(52.))
                        .text_xs()
                        .text_color(if job.required {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(if job.required { "req" } else { "opt" }),
                )
                .child(
                    div()
                        .flex_none()
                        .w(px(40.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("—"),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .when(emphasis, |el| el.font_medium())
                .truncate()
                .child(job.name.clone()),
        );

    if job.state == JobState::Failure && job.required {
        row = row.child(
            h_flex()
                .items_center()
                .justify_center()
                .size(px(16.))
                .rounded_full()
                .bg(cx.theme().danger.opacity(0.85))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger_foreground)
                        .child("!"),
                ),
        );
    }

    row = row.child(if can_rerun {
        // Job-level Actions re-run is gated in the mapper today; keep chrome for Allowed.
        Button::new(SharedString::from(format!("rerun-job-{}", job.id)))
            .ghost()
            .xsmall()
            .label("Re-run")
            .disabled(true)
            .into_any_element()
    } else {
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child("—")
            .into_any_element()
    });

    div()
        .id(SharedString::from(format!("job-{}", job.id)))
        .w_full()
        .pl(px(list_row_pl(depth)))
        .pr_4()
        .py(px(7.))
        .border_b_1()
        .border_color(cx.theme().border.opacity(0.55))
        .when(emphasis, |el| el.bg(cx.theme().danger.opacity(0.06)))
        .hover(|s| s.bg(cx.theme().secondary.opacity(0.25)))
        .child(row)
}

fn empty_state(copy: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .px_6()
        .gap_2()
        .child(Icon::new(IconName::Square).text_color(cx.theme().muted_foreground))
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child("No checks"),
        )
        .child(
            div()
                .text_sm()
                .text_center()
                .text_color(cx.theme().muted_foreground)
                .child(copy.to_string()),
        )
}

#[derive(Clone)]
enum FlatRow {
    Group {
        key: String,
        label: String,
        count: usize,
        state: Option<JobState>,
        depth: u8,
    },
    Job {
        job: Job,
        depth: u8,
    },
}

/// Stage groups inside one run. v1 Jobs have no stage field — one `jobs` group.
fn build_stage_rows(
    pipeline_index: usize,
    pipeline: &Pipeline,
    collapsed: &HashSet<String>,
) -> Vec<FlatRow> {
    let stage = "jobs";
    let key = format!("stage:{pipeline_index}:{stage}");
    let stage_state = aggregate_stage_state(&pipeline.jobs);
    let mut rows = vec![FlatRow::Group {
        key: key.clone(),
        label: stage.to_string(),
        count: pipeline.jobs.len(),
        state: Some(stage_state),
        depth: 0,
    }];
    if collapsed.contains(&key) {
        return rows;
    }
    for job in &pipeline.jobs {
        rows.push(FlatRow::Job {
            job: job.clone(),
            depth: 1,
        });
    }
    rows
}

fn aggregate_stage_state(jobs: &[Job]) -> JobState {
    if jobs
        .iter()
        .any(|j| j.state == JobState::Failure || j.state == JobState::Error)
    {
        return JobState::Failure;
    }
    if jobs.iter().any(|j| j.state == JobState::Pending) {
        return JobState::Pending;
    }
    if !jobs.is_empty() && jobs.iter().all(|j| j.state == JobState::Skipped) {
        return JobState::Skipped;
    }
    JobState::Success
}

fn kind_label(kind: PipelineKind) -> &'static str {
    match kind {
        PipelineKind::WorkflowRun => "workflow run",
        PipelineKind::CheckSuite => "check suite",
        PipelineKind::StatusContext => "status",
    }
}

fn job_icon(state: JobState) -> IconName {
    match state {
        JobState::Success => IconName::CircleCheck,
        JobState::Pending => IconName::LoaderCircle,
        JobState::Failure | JobState::Error => IconName::CircleX,
        JobState::Skipped => IconName::Square,
    }
}

fn job_color(state: JobState, cx: &App) -> gpui_kit::Hsla {
    match state {
        JobState::Success => cx.theme().success,
        JobState::Pending => cx.theme().warning,
        JobState::Failure | JobState::Error => cx.theme().danger,
        JobState::Skipped => cx.theme().muted_foreground,
    }
}

fn rollup_icon(state: CheckRollupState) -> IconName {
    match state {
        CheckRollupState::Success => IconName::CircleCheck,
        CheckRollupState::Pending => IconName::LoaderCircle,
        CheckRollupState::Failure | CheckRollupState::Error => IconName::CircleX,
        CheckRollupState::Absent => IconName::Square,
    }
}

fn rollup_color(state: CheckRollupState, cx: &mut Context<ReviewDesk>) -> gpui_kit::Hsla {
    match state {
        CheckRollupState::Success => cx.theme().success,
        CheckRollupState::Pending => cx.theme().warning,
        CheckRollupState::Failure | CheckRollupState::Error => cx.theme().danger,
        CheckRollupState::Absent => cx.theme().muted_foreground,
    }
}

fn relative_time(at: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let Ok(delta) = now.signed_duration_since(at).to_std() else {
        return "—".into();
    };
    let secs = delta.as_secs();
    if secs < 60 {
        return "now".into();
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 48 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    if days < 14 {
        return format!("{days}d");
    }
    format!("{}w", days / 7)
}
