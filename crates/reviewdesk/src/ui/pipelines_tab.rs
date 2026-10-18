//! PROTOTYPE paint: Linear Issues-style Pipelines tab variants.
//! Visual language borrowed from `prototype/pull-requests-list-nav`.

use std::collections::HashSet;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    resizable::{h_resizable, resizable_panel},
    scroll::Scrollbar,
    v_flex,
};
use gpui_kit::{
    App, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle, WeakEntity, div,
    px, uniform_list, prelude::FluentBuilder as _,
};

use crate::app::ReviewDesk;
use crate::pipelines_prototype::{
    JobState, PipelinesVariant, RollupState, StubJob, StubPipeline, StubPipelinesData, StubScenario,
};

/// Matches Linear Issues list nest padding from the PR-list prototype.
const LIST_ROW_BASE_PL: f32 = 16.;
const LIST_NEST_STEP: f32 = 20.;

fn list_row_pl(depth: u8) -> f32 {
    LIST_ROW_BASE_PL + LIST_NEST_STEP * f32::from(depth)
}

pub(crate) struct PipelinesPaint {
    pub variant: PipelinesVariant,
    pub scenario: StubScenario,
    pub selected_pipeline: usize,
    pub collapsed_groups: HashSet<String>,
    pub list_scroll: UniformListScrollHandle,
    pub detail_scroll: UniformListScrollHandle,
    pub entity: WeakEntity<ReviewDesk>,
}

pub(crate) fn render_pipelines_tab(
    paint: PipelinesPaint,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let data = paint.scenario.data();
    match paint.variant {
        PipelinesVariant::A => render_variant_a(paint, data, cx).into_any_element(),
        PipelinesVariant::B => render_variant_b(paint, data, cx).into_any_element(),
        PipelinesVariant::C => render_variant_c(paint, data, cx).into_any_element(),
    }
}

pub(crate) fn render_compact_check_rollup(
    scenario: StubScenario,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let data = scenario.data();
    let color = rollup_color(data.rollup, cx);
    div()
        .id("metadata-check-rollup")
        .cursor_pointer()
        .rounded_md()
        .px_2()
        .py(px(5.))
        .hover(|s| s.bg(cx.theme().secondary.opacity(0.45)))
        .on_click(cx.listener(|this, _, _, cx| {
            this.open_pipelines_tab(cx);
        }))
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(rollup_icon(data.rollup))
                        .xsmall()
                        .text_color(color),
                )
                .child(
                    v_flex()
                        .gap_0()
                        .min_w_0()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Check rollup"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .truncate()
                                .child(data.rollup_label),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Pipelines"),
                ),
        )
}

// --- A: Issues list (Pipeline groups → Job rows) ---

fn render_variant_a(
    paint: PipelinesPaint,
    data: StubPipelinesData,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let scroll = paint.list_scroll.clone();
    let entity = paint.entity.clone();
    let collapsed = paint.collapsed_groups.clone();
    let rows = build_issues_rows(data.pipelines, &collapsed);
    let row_count = rows.len();

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_linear_toolbar(&data, "pipeline", cx))
        .child(match data.empty_copy {
            Some(copy) => empty_state(copy, cx).into_any_element(),
            None => div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("proto-pipelines-a", row_count, move |range, _, cx| {
                        range
                            .map(|ix| match &rows[ix] {
                                FlatRow::Group {
                                    key,
                                    label,
                                    count,
                                    state,
                                    depth,
                                    can_rerun,
                                } => group_header_row(
                                    key,
                                    label,
                                    *count,
                                    *state,
                                    *depth,
                                    *can_rerun,
                                    collapsed.contains(key),
                                    entity.clone(),
                                    cx,
                                )
                                .into_any_element(),
                                FlatRow::Job {
                                    pipeline_name,
                                    pipeline_index,
                                    job,
                                    depth,
                                } => job_issue_row(
                                    *pipeline_index,
                                    pipeline_name,
                                    job,
                                    *depth,
                                    false,
                                    cx,
                                )
                                .into_any_element(),
                            })
                            .collect()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll))
                .into_any_element(),
        })
}

// --- B: Project detail (Pipeline nav | Jobs list) ---

fn render_variant_b(
    paint: PipelinesPaint,
    data: StubPipelinesData,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let list_scroll = paint.list_scroll.clone();
    let detail_scroll = paint.detail_scroll.clone();
    let selected = paint
        .selected_pipeline
        .min(data.pipelines.len().saturating_sub(1));
    let pipelines = data.pipelines;
    let entity = paint.entity.clone();

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_linear_toolbar(&data, "run history", cx))
        .child(match data.empty_copy {
            Some(copy) => empty_state(copy, cx).into_any_element(),
            None => div()
                .flex_1()
                .min_h_0()
                .child(
                    h_resizable("proto-pipelines-split")
                        .child(
                            resizable_panel()
                                .size(px(240.))
                                .size_range(px(200.)..px(340.))
                                .child(render_pipeline_nav(
                                    pipelines,
                                    selected,
                                    list_scroll,
                                    entity.clone(),
                                    cx,
                                )),
                        )
                        .child(
                            resizable_panel().size_range(px(280.)..px(2000.)).child(
                                render_run_detail(
                                    pipelines.get(selected),
                                    selected,
                                    paint.collapsed_groups.clone(),
                                    detail_scroll,
                                    entity,
                                    cx,
                                ),
                            ),
                        ),
                )
                .into_any_element(),
        })
}

// --- C: Group by status ---

fn render_variant_c(
    paint: PipelinesPaint,
    data: StubPipelinesData,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let scroll = paint.list_scroll.clone();
    let entity = paint.entity.clone();
    let collapsed = paint.collapsed_groups.clone();
    let rows = build_status_rows(data.pipelines, &collapsed);
    let row_count = rows.len();

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_linear_toolbar(&data, "status", cx))
        .child(match data.empty_copy {
            Some(copy) => empty_state(copy, cx).into_any_element(),
            None => div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("proto-pipelines-c", row_count, move |range, _, cx| {
                        range
                            .map(|ix| match &rows[ix] {
                                FlatRow::Group {
                                    key,
                                    label,
                                    count,
                                    state,
                                    depth,
                                    can_rerun,
                                } => group_header_row(
                                    key,
                                    label,
                                    *count,
                                    *state,
                                    *depth,
                                    *can_rerun,
                                    collapsed.contains(key),
                                    entity.clone(),
                                    cx,
                                )
                                .into_any_element(),
                                FlatRow::Job {
                                    pipeline_name,
                                    pipeline_index,
                                    job,
                                    depth,
                                } => job_issue_row(
                                    *pipeline_index,
                                    pipeline_name,
                                    job,
                                    *depth,
                                    true,
                                    cx,
                                )
                                .into_any_element(),
                            })
                            .collect()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll))
                .into_any_element(),
        })
}

/// Quiet Filter + Display strip + Check rollup chip (Linear top chrome).
fn render_linear_toolbar(
    data: &StubPipelinesData,
    group_by: &str,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let color = rollup_color(data.rollup, cx);
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
        .child(filter_chip("Rollup", "is", data.rollup_label, rollup_icon(data.rollup), color, cx))
        .when(data.rollup != RollupState::Absent, |el| {
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
                        .child(format!("· {group_by}")),
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
    pipelines: &'static [StubPipeline],
    selected: usize,
    scroll: UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
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
                    uniform_list("proto-pipelines-b-nav", pipelines.len(), move |range, _, cx| {
                        range
                            .map(|ix| {
                                let pipeline = &pipelines[ix];
                                let is_sel = ix == selected;
                                let owner = entity.clone();
                                div()
                                    .id(SharedString::from(format!("nav-pl-{ix}")))
                                    .w_full()
                                    .px_3()
                                    .py(px(7.))
                                    .border_b_1()
                                    .border_color(cx.theme().border.opacity(0.55))
                                    .cursor_pointer()
                                    .when(is_sel, |el| {
                                        el.bg(cx.theme().secondary.opacity(0.4))
                                    })
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
                                                Icon::new(job_icon(pipeline.state))
                                                    .xsmall()
                                                    .text_color(job_color(pipeline.state, cx)),
                                            )
                                            .child(
                                                v_flex()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .gap_0()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .truncate()
                                                            .child(format!(
                                                                "{} {}",
                                                                pipeline.name, pipeline.run_label
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(
                                                                cx.theme().muted_foreground,
                                                            )
                                                            .truncate()
                                                            .child(format!(
                                                                "{} · {}",
                                                                pipeline.kind.label(),
                                                                pipeline.created
                                                            )),
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

/// B detail = A's Issues-list UI, scoped to one run, grouped by stage.
fn render_run_detail(
    pipeline: Option<&'static StubPipeline>,
    pipeline_index: usize,
    collapsed: HashSet<String>,
    scroll: UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let Some(pipeline) = pipeline else {
        return empty_state("Select a Pipeline run.", cx).into_any_element();
    };
    let name = pipeline.name;
    let run_label = pipeline.run_label;
    let kind = pipeline.kind.label();
    let can_rerun = pipeline.can_rerun;
    let rows = build_stage_rows(pipeline_index, pipeline, &collapsed);
    let row_count = rows.len();

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
                            Icon::new(job_icon(pipeline.state))
                                .xsmall()
                                .text_color(job_color(pipeline.state, cx)),
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
                    Button::new("rerun-pipeline")
                        .ghost()
                        .xsmall()
                        .label("Re-run")
                        .on_click(|_, _, _| {})
                        .into_any_element()
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
                    uniform_list("proto-pipelines-b-detail", row_count, move |range, _, cx| {
                        range
                            .map(|ix| match &rows[ix] {
                                FlatRow::Group {
                                    key,
                                    label,
                                    count,
                                    state,
                                    depth,
                                    can_rerun,
                                } => group_header_row(
                                    key,
                                    label,
                                    *count,
                                    *state,
                                    *depth,
                                    *can_rerun,
                                    collapsed.contains(key),
                                    entity.clone(),
                                    cx,
                                )
                                .into_any_element(),
                                FlatRow::Job {
                                    pipeline_name,
                                    pipeline_index,
                                    job,
                                    depth,
                                } => job_issue_row(
                                    *pipeline_index,
                                    pipeline_name,
                                    job,
                                    *depth,
                                    false,
                                    cx,
                                )
                                .into_any_element(),
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
    can_rerun: bool,
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
                this.toggle_collapsed_group(key, cx);
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
                .child(div().flex_1())
                .child(if can_rerun {
                    Button::new(SharedString::from(format!("rerun-group-{key}")))
                        .ghost()
                        .xsmall()
                        .label("Re-run")
                        .on_click(|_, _, _| {})
                        .into_any_element()
                } else {
                    div().into_any_element()
                }),
        )
}

/// Dense Linear Issues row for a Job.
fn job_issue_row(
    pipeline_index: usize,
    pipeline_name: &str,
    job: &StubJob,
    depth: u8,
    show_pipeline_pill: bool,
    cx: &App,
) -> impl IntoElement {
    let emphasis = job.required && matches!(job.state, JobState::Failure | JobState::Pending);
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
                        .child(job.duration),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .when(emphasis, |el| el.font_medium())
                .truncate()
                .child(job.name),
        );

    if show_pipeline_pill {
        row = row.child(pipeline_pill(pipeline_name, cx));
    }

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

    row = row.child(if job.can_rerun {
        Button::new(SharedString::from(format!(
            "rerun-job-{pipeline_index}-{}",
            job.name
        )))
        .ghost()
        .xsmall()
        .label("Re-run")
        .on_click(|_, _, _| {})
        .into_any_element()
    } else {
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child("—")
            .into_any_element()
    });

    div()
        .id(SharedString::from(format!(
            "job-{pipeline_index}-{}",
            job.name
        )))
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

fn pipeline_pill(name: &str, cx: &App) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_1()
        .px_1()
        .py(px(2.))
        .rounded_md()
        .bg(cx.theme().secondary.opacity(0.65))
        .flex_shrink_0()
        .child(
            Icon::new(IconName::LoaderCircle)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            div()
                .max_w(px(120.))
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .truncate()
                .child(name.to_string()),
        )
}

fn empty_state(copy: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .px_6()
        .gap_2()
        .child(
            Icon::new(IconName::Square)
                .text_color(cx.theme().muted_foreground),
        )
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

enum FlatRow {
    Group {
        key: String,
        label: String,
        count: usize,
        state: Option<JobState>,
        depth: u8,
        can_rerun: bool,
    },
    Job {
        pipeline_name: &'static str,
        pipeline_index: usize,
        job: &'static StubJob,
        depth: u8,
    },
}

fn build_issues_rows(
    pipelines: &'static [StubPipeline],
    collapsed: &HashSet<String>,
) -> Vec<FlatRow> {
    let mut rows = Vec::new();
    for (index, pipeline) in pipelines.iter().enumerate() {
        let key = format!("pipeline:{index}");
        rows.push(FlatRow::Group {
            key: key.clone(),
            label: pipeline.name.to_string(),
            count: pipeline.jobs.len(),
            state: Some(pipeline.state),
            depth: 0,
            can_rerun: pipeline.can_rerun,
        });
        if collapsed.contains(&key) {
            continue;
        }
        for job in pipeline.jobs {
            rows.push(FlatRow::Job {
                pipeline_name: pipeline.name,
                pipeline_index: index,
                job,
                depth: 1,
            });
        }
    }
    rows
}

fn build_status_rows(
    pipelines: &'static [StubPipeline],
    collapsed: &HashSet<String>,
) -> Vec<FlatRow> {
    let mut failed = Vec::new();
    let mut pending = Vec::new();
    let mut passed = Vec::new();
    let mut skipped = Vec::new();
    for (index, pipeline) in pipelines.iter().enumerate() {
        for job in pipeline.jobs {
            let entry = (pipeline.name, index, job);
            match job.state {
                JobState::Failure => failed.push(entry),
                JobState::Pending => pending.push(entry),
                JobState::Success => passed.push(entry),
                JobState::Skipped => skipped.push(entry),
            }
        }
    }
    let sort_jobs = |items: &mut Vec<(&'static str, usize, &'static StubJob)>| {
        items.sort_by_key(|(_, _, job)| if job.required { 0 } else { 1 });
    };
    sort_jobs(&mut failed);
    sort_jobs(&mut pending);
    sort_jobs(&mut passed);
    sort_jobs(&mut skipped);

    let mut rows = Vec::new();
    for (key, label, state, items) in [
        ("status:failed", "Failed", JobState::Failure, failed),
        ("status:pending", "Pending", JobState::Pending, pending),
        ("status:passed", "Passed", JobState::Success, passed),
        ("status:skipped", "Skipped", JobState::Skipped, skipped),
    ] {
        if items.is_empty() {
            continue;
        }
        rows.push(FlatRow::Group {
            key: key.to_string(),
            label: label.to_string(),
            count: items.len(),
            state: Some(state),
            depth: 0,
            can_rerun: false,
        });
        if collapsed.contains(key) {
            continue;
        }
        for (pipeline_name, pipeline_index, job) in items {
            rows.push(FlatRow::Job {
                pipeline_name,
                pipeline_index,
                job,
                depth: 1,
            });
        }
    }
    rows
}

/// Stage groups inside one run (GitLab stages / Actions job groups).
fn build_stage_rows(
    pipeline_index: usize,
    pipeline: &'static StubPipeline,
    collapsed: &HashSet<String>,
) -> Vec<FlatRow> {
    let mut order: Vec<&'static str> = Vec::new();
    for job in pipeline.jobs {
        let stage = job.stage.unwrap_or("jobs");
        if !order.contains(&stage) {
            order.push(stage);
        }
    }

    let mut rows = Vec::new();
    for stage in order {
        let jobs: Vec<&'static StubJob> = pipeline
            .jobs
            .iter()
            .filter(|j| j.stage.unwrap_or("jobs") == stage)
            .collect();
        let key = format!("stage:{pipeline_index}:{stage}");
        let stage_state = aggregate_stage_state(&jobs);
        rows.push(FlatRow::Group {
            key: key.clone(),
            label: stage.to_string(),
            count: jobs.len(),
            state: Some(stage_state),
            depth: 0,
            can_rerun: false,
        });
        if collapsed.contains(&key) {
            continue;
        }
        for job in jobs {
            rows.push(FlatRow::Job {
                pipeline_name: pipeline.name,
                pipeline_index,
                job,
                depth: 1,
            });
        }
    }
    rows
}

fn aggregate_stage_state(jobs: &[&StubJob]) -> JobState {
    if jobs.iter().any(|j| j.state == JobState::Failure) {
        return JobState::Failure;
    }
    if jobs.iter().any(|j| j.state == JobState::Pending) {
        return JobState::Pending;
    }
    if jobs.iter().all(|j| j.state == JobState::Skipped) {
        return JobState::Skipped;
    }
    JobState::Success
}

fn job_icon(state: JobState) -> IconName {
    match state {
        JobState::Success => IconName::CircleCheck,
        JobState::Pending => IconName::LoaderCircle,
        JobState::Failure => IconName::CircleX,
        JobState::Skipped => IconName::Square,
    }
}

fn job_color(state: JobState, cx: &App) -> gpui_kit::Hsla {
    match state {
        JobState::Success => cx.theme().success,
        JobState::Pending => cx.theme().warning,
        JobState::Failure => cx.theme().danger,
        JobState::Skipped => cx.theme().muted_foreground,
    }
}

fn rollup_icon(state: RollupState) -> IconName {
    match state {
        RollupState::Success => IconName::CircleCheck,
        RollupState::Pending => IconName::LoaderCircle,
        RollupState::Failure => IconName::CircleX,
        RollupState::Absent => IconName::Square,
    }
}

fn rollup_color(state: RollupState, cx: &mut Context<ReviewDesk>) -> gpui_kit::Hsla {
    match state {
        RollupState::Success => cx.theme().success,
        RollupState::Pending => cx.theme().warning,
        RollupState::Failure => cx.theme().danger,
        RollupState::Absent => cx.theme().muted_foreground,
    }
}
