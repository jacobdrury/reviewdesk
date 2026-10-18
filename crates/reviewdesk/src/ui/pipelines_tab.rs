//! Pipelines tab: Check rollup summary and newest-first Pipeline list with Jobs.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::Scrollbar,
    v_flex,
};
use gpui_kit::{
    App, Context, IntoElement, ParentElement as _, SharedString, Styled as _,
    UniformListScrollHandle, WeakEntity, div, px, uniform_list,
};

use reviewdesk_core::{CheckRollupState, JobState, Pipeline, PipelinesSnapshot, RerunAction};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView};

const LIST_ROW_PL: f32 = 16.;
const LIST_NEST_STEP: f32 = 20.;

#[derive(Clone)]
enum FlatRow {
    PipelineHeader {
        pipeline: Pipeline,
    },
    Job {
        name: SharedString,
        state: JobState,
        required: bool,
        depth: u8,
    },
}

pub(crate) fn render_pipelines_tab(
    view: &SessionView,
    scroll: &UniformListScrollHandle,
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
            render_pipelines_ready(view, snapshot, scroll, entity, cx).into_any_element()
        }
    }
}

fn render_pipelines_ready(
    view: &SessionView,
    snapshot: &PipelinesSnapshot,
    scroll: &UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let rows = build_rows(snapshot);
    let row_count = rows.len();
    let rows_for_list = rows.clone();
    let rollup = &snapshot.rollup;
    let scroll = scroll.clone();
    let rerun_busy = view.pipelines_rerun_busy;

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_rollup_banner(rollup, cx))
        .child(if snapshot.pipelines.is_empty() {
            empty_state(cx).into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("pipelines-list", row_count, move |range, _, cx| {
                        range
                            .map(|ix| match &rows_for_list[ix] {
                                FlatRow::PipelineHeader { pipeline } => {
                                    pipeline_header(pipeline, rerun_busy, entity.clone(), cx)
                                        .into_any_element()
                                }
                                FlatRow::Job {
                                    name,
                                    state,
                                    required,
                                    depth,
                                } => job_row(name.clone(), *state, *required, *depth, cx)
                                    .into_any_element(),
                            })
                            .collect()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll))
                .into_any_element()
        })
}

fn render_rollup_banner(
    rollup: &reviewdesk_core::CheckRollup,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let (icon, color) = rollup_visuals(rollup.state, cx);
    h_flex()
        .w_full()
        .px_3()
        .py_2()
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(Icon::new(icon).small().text_color(color))
        .child(
            v_flex()
                .gap_0()
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child("Check rollup"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(color)
                        .child(rollup.label.clone()),
                ),
        )
}

fn empty_state(cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .p_4()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("No Pipelines for this head. Empty is not a pass — nothing reported yet."),
        )
}

fn build_rows(snapshot: &PipelinesSnapshot) -> Vec<FlatRow> {
    let mut rows = Vec::new();
    for pipeline in &snapshot.pipelines {
        rows.push(FlatRow::PipelineHeader {
            pipeline: pipeline.clone(),
        });
        for job in &pipeline.jobs {
            rows.push(FlatRow::Job {
                name: job.name.clone().into(),
                state: job.state,
                required: job.required,
                depth: 1,
            });
        }
    }
    rows
}

fn pipeline_header(
    pipeline: &Pipeline,
    rerun_busy: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut App,
) -> impl IntoElement {
    let (icon, color) = job_visuals(aggregate_job_state(&pipeline.jobs), cx);
    let kind_label = match pipeline.kind {
        reviewdesk_core::PipelineKind::WorkflowRun => "workflow run",
        reviewdesk_core::PipelineKind::CheckSuite => "check suite",
        reviewdesk_core::PipelineKind::StatusContext => "status",
    };
    let can_rerun = matches!(pipeline.rerun, RerunAction::Allowed(_));
    let pipeline_id = pipeline.id.clone();

    h_flex()
        .w_full()
        .pl(px(LIST_ROW_PL))
        .pr_2()
        .py(px(6.))
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border.opacity(0.5))
        .child(Icon::new(icon).xsmall().text_color(color))
        .child(
            v_flex().flex_1().min_w_0().child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .truncate()
                            .child(pipeline.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} · {}", pipeline.run_label, kind_label)),
                    ),
            ),
        )
        .child({
            let mut btn = Button::new(format!("rerun-{}", pipeline.id))
                .ghost()
                .xsmall()
                .label("Re-run");
            if !can_rerun || rerun_busy {
                btn = btn.disabled(true);
            } else {
                btn = btn.on_click(move |_, _, cx| {
                    let pipeline_id = pipeline_id.clone();
                    let _ = entity.update(cx, |this, cx| {
                        this.dispatch(Intent::RerunPipeline { pipeline_id }, cx);
                    });
                });
            }
            btn
        })
}

fn job_row(
    name: SharedString,
    state: JobState,
    required: bool,
    depth: u8,
    cx: &mut App,
) -> impl IntoElement {
    let (icon, color) = job_visuals(state, cx);
    let req = if required { "required" } else { "optional" };
    h_flex()
        .w_full()
        .pl(px(LIST_ROW_PL + LIST_NEST_STEP * f32::from(depth)))
        .pr_2()
        .py(px(4.))
        .gap_2()
        .items_center()
        .child(Icon::new(icon).xsmall().text_color(color))
        .child(div().text_sm().truncate().child(name))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{} · {}", state_label(state), req)),
        )
}

fn aggregate_job_state(jobs: &[reviewdesk_core::Job]) -> JobState {
    reviewdesk_core::aggregate_pipeline_state(jobs)
}

fn rollup_visuals(
    state: CheckRollupState,
    cx: &mut Context<ReviewDesk>,
) -> (IconName, gpui_kit::Hsla) {
    match state {
        CheckRollupState::Success => (IconName::CircleCheck, cx.theme().success),
        CheckRollupState::Pending => (IconName::LoaderCircle, cx.theme().warning),
        CheckRollupState::Failure | CheckRollupState::Error => {
            (IconName::CircleX, cx.theme().danger)
        }
        CheckRollupState::Absent => (IconName::Square, cx.theme().muted_foreground),
    }
}

fn job_visuals(state: JobState, cx: &mut App) -> (IconName, gpui_kit::Hsla) {
    match state {
        JobState::Success => (IconName::CircleCheck, cx.theme().success),
        JobState::Pending => (IconName::LoaderCircle, cx.theme().warning),
        JobState::Failure | JobState::Error => (IconName::CircleX, cx.theme().danger),
        JobState::Skipped => (IconName::Square, cx.theme().muted_foreground),
    }
}

fn state_label(state: JobState) -> &'static str {
    match state {
        JobState::Success => "passed",
        JobState::Pending => "pending",
        JobState::Failure => "failed",
        JobState::Skipped => "skipped",
        JobState::Error => "error",
    }
}
