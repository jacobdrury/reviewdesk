//! PROTOTYPE paint: Pipelines tab variants. Throwaway with the branch.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    scroll::Scrollbar,
    v_flex,
};
use gpui_kit::{
    App, Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle, WeakEntity, div,
    percentage, px, uniform_list, prelude::FluentBuilder as _,
};

use crate::app::ReviewDesk;
use crate::pipelines_prototype::{
    JobState, PipelinesVariant, RollupState, StubJob, StubPipeline, StubPipelinesData, StubScenario,
};

pub(crate) struct PipelinesPaint {
    pub variant: PipelinesVariant,
    pub scenario: StubScenario,
    pub expanded: Vec<bool>,
    pub selected_pipeline: usize,
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
    let (dot, color) = rollup_visual(data.rollup, cx);
    div()
        .id("metadata-check-rollup")
        .cursor_pointer()
        .rounded_md()
        .px_2()
        .py_1()
        .border_1()
        .border_color(cx.theme().border)
        .hover(|s| s.bg(cx.theme().secondary.opacity(0.5)))
        .on_click(cx.listener(|this, _, _, cx| {
            this.open_pipelines_tab(cx);
        }))
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(div().text_sm().text_color(color).child(dot))
                .child(
                    v_flex()
                        .gap_0()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Check rollup"),
                        )
                        .child(div().text_sm().truncate().child(data.rollup_label)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("→ Pipelines"),
                ),
        )
}

fn render_variant_a(
    paint: PipelinesPaint,
    data: StubPipelinesData,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let scroll = paint.list_scroll.clone();
    let pipelines = data.pipelines;
    let expanded = paint.expanded;
    let entity = paint.entity.clone();
    let row_count = visible_row_count_a(pipelines, &expanded);

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_rollup_banner(&data, "A · Nested list", cx))
        .child(match data.empty_copy {
            Some(copy) => empty_state(copy, cx).into_any_element(),
            None => div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("proto-pipelines-a", row_count, {
                        let expanded = expanded.clone();
                        move |range, _, cx| {
                            range
                                .filter_map(|flat_ix| {
                                    let row = row_at_a(pipelines, &expanded, flat_ix)?;
                                    Some(match row {
                                        FlatRow::Pipeline { index, pipeline } => {
                                            let open =
                                                expanded.get(index).copied().unwrap_or(false);
                                            pipeline_row_a(
                                                index,
                                                pipeline,
                                                open,
                                                entity.clone(),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        FlatRow::Job {
                                            pipeline_index,
                                            job,
                                        } => job_row(pipeline_index, job, cx).into_any_element(),
                                    })
                                })
                                .collect()
                        }
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll))
                .into_any_element(),
        })
}

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
        .child(render_rollup_banner(&data, "B · Split panes", cx))
        .child(match data.empty_copy {
            Some(copy) => empty_state(copy, cx).into_any_element(),
            None => div()
                .flex_1()
                .min_h_0()
                .child(
                    h_resizable("proto-pipelines-split")
                        .child(
                            resizable_panel()
                                .size(px(280.))
                                .size_range(px(200.)..px(420.))
                                .child(
                                    v_flex()
                                        .size_full()
                                        .min_h_0()
                                        .border_r_1()
                                        .border_color(cx.theme().border)
                                        .child(
                                            div()
                                                .px_3()
                                                .py(px(4.))
                                                .border_b_1()
                                                .border_color(cx.theme().border)
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_medium()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child("PIPELINES"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_h_0()
                                                .relative()
                                                .child(
                                                    uniform_list(
                                                        "proto-pipelines-b-list",
                                                        pipelines.len(),
                                                        {
                                                            let entity = entity.clone();
                                                            move |range, _, cx| {
                                                                range
                                                                    .map(|ix| {
                                                                        let pipeline =
                                                                            &pipelines[ix];
                                                                        let is_sel = ix == selected;
                                                                        let owner = entity.clone();
                                                                        ListItem::new(
                                                                            SharedString::from(
                                                                                format!(
                                                                                    "pl-b-{ix}"
                                                                                ),
                                                                            ),
                                                                        )
                                                                        .selected(is_sel)
                                                                        .px_3()
                                                                        .py(px(6.))
                                                                        .child(
                                                                            v_flex()
                                                                                .w_full()
                                                                                .gap_1()
                                                                                .child(
                                                                                    h_flex()
                                                                                        .w_full()
                                                                                        .items_center()
                                                                                        .gap_2()
                                                                                        .child(
                                                                                            state_dot(
                                                                                                pipeline
                                                                                                    .state,
                                                                                                cx,
                                                                                            ),
                                                                                        )
                                                                                        .child(
                                                                                            div()
                                                                                                .flex_1()
                                                                                                .min_w_0()
                                                                                                .text_sm()
                                                                                                .font_medium()
                                                                                                .truncate()
                                                                                                .child(
                                                                                                    pipeline
                                                                                                        .name,
                                                                                                ),
                                                                                        ),
                                                                                )
                                                                                .child(
                                                                                    div()
                                                                                        .text_xs()
                                                                                        .text_color(
                                                                                            cx.theme()
                                                                                                .muted_foreground,
                                                                                        )
                                                                                        .truncate()
                                                                                        .child(
                                                                                            format!(
                                                                                                "{} · {} · {}",
                                                                                                pipeline
                                                                                                    .kind
                                                                                                    .label(),
                                                                                                pipeline
                                                                                                    .state
                                                                                                    .label(),
                                                                                                pipeline
                                                                                                    .created
                                                                                            ),
                                                                                        ),
                                                                                ),
                                                                        )
                                                                        .on_click(
                                                                            move |_, _, cx| {
                                                                                let _ = owner
                                                                                    .update(
                                                                                        cx,
                                                                                        |this,
                                                                                         cx| {
                                                                                            this.select_pipeline(
                                                                                            ix, cx,
                                                                                        );
                                                                                        },
                                                                                    );
                                                                            },
                                                                        )
                                                                        .into_any_element()
                                                                    })
                                                                    .collect()
                                                            }
                                                        },
                                                    )
                                                    .size_full()
                                                    .track_scroll(&list_scroll),
                                                )
                                                .child(Scrollbar::vertical(&list_scroll)),
                                        ),
                                ),
                        )
                        .child(
                            resizable_panel().size_range(px(280.)..px(2000.)).child(
                                render_pipeline_detail(
                                    pipelines.get(selected),
                                    detail_scroll,
                                    cx,
                                ),
                            ),
                        ),
                )
                .into_any_element(),
        })
}

fn render_variant_c(
    paint: PipelinesPaint,
    data: StubPipelinesData,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let scroll = paint.list_scroll.clone();
    let attention = attention_jobs(data.pipelines);
    let pipelines = data.pipelines;
    let attention_len = attention.len();
    let total = if data.empty_copy.is_some() {
        0
    } else {
        attention_len + 1 + pipelines.len()
    };

    v_flex()
        .size_full()
        .min_h_0()
        .child(render_rollup_banner(&data, "C · Attention-first", cx))
        .child(match data.empty_copy {
            Some(copy) => empty_state(copy, cx).into_any_element(),
            None => div()
                .flex_1()
                .min_h_0()
                .relative()
                .child(
                    uniform_list("proto-pipelines-c", total, move |range, _, cx| {
                        range
                            .map(|ix| {
                                if ix < attention_len {
                                    let (pipeline_name, job) = attention[ix];
                                    return attention_row(ix, pipeline_name, job, cx)
                                        .into_any_element();
                                }
                                if ix == attention_len {
                                    let label = if attention_len == 0 {
                                        "All Pipelines · newest first"
                                    } else {
                                        "All Pipelines · newest first (below attention)"
                                    };
                                    return section_label(label, cx).into_any_element();
                                }
                                let p_ix = ix - attention_len - 1;
                                timeline_pipeline_row(p_ix, &pipelines[p_ix], cx).into_any_element()
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

fn render_rollup_banner(
    data: &StubPipelinesData,
    variant_hint: &str,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let (dot, color) = rollup_visual(data.rollup, cx);
    h_flex()
        .w_full()
        .px_3()
        .py(px(8.))
        .items_center()
        .justify_between()
        .gap_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(rollup_bg(data.rollup, cx))
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(div().text_sm().text_color(color).child(dot))
                .child(
                    v_flex()
                        .min_w_0()
                        .gap_0()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("Check rollup · {variant_hint}")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .truncate()
                                .child(data.rollup_label),
                        ),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("state · {}", data.rollup.short())),
        )
}

fn render_pipeline_detail(
    pipeline: Option<&StubPipeline>,
    scroll: UniformListScrollHandle,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let Some(pipeline) = pipeline else {
        return empty_state("Select a Pipeline.", cx).into_any_element();
    };
    let jobs = pipeline.jobs;
    let can_rerun = pipeline.can_rerun;
    let name = pipeline.name;
    let kind = pipeline.kind.label();
    let state = pipeline.state.label();
    let created = pipeline.created;

    v_flex()
        .size_full()
        .min_h_0()
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py(px(6.))
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    v_flex()
                        .min_w_0()
                        .gap_0()
                        .child(div().text_sm().font_medium().truncate().child(name))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{kind} · {state} · {created}")),
                        ),
                )
                .child(if can_rerun {
                    Button::new("rerun-pipeline")
                        .outline()
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
                    uniform_list("proto-pipelines-b-jobs", jobs.len(), move |range, _, cx| {
                        range
                            .map(|ix| job_row(0, &jobs[ix], cx).into_any_element())
                            .collect()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll)),
        )
        .into_any_element()
}

fn pipeline_row_a(
    index: usize,
    pipeline: &StubPipeline,
    open: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &App,
) -> impl IntoElement {
    let can_rerun = pipeline.can_rerun;
    h_flex()
        .id(SharedString::from(format!("pl-a-{index}")))
        .w_full()
        .px_3()
        .py(px(6.))
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .cursor_pointer()
        .hover(|s| s.bg(cx.theme().secondary.opacity(0.35)))
        .on_click(move |_, _, cx| {
            let _ = entity.update(cx, |this, cx| {
                this.toggle_pipeline_expanded(index, cx);
            });
        })
        .child(
            Icon::new(IconName::ChevronRight)
                .xsmall()
                .text_color(cx.theme().muted_foreground)
                .rotate(percentage(if open { 0.25 } else { 0. })),
        )
        .child(state_dot(pipeline.state, cx))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0()
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .truncate()
                        .child(pipeline.name),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .child(format!(
                            "{} · {} jobs · {}",
                            pipeline.kind.label(),
                            pipeline.jobs.len(),
                            pipeline.created
                        )),
                ),
        )
        .child(if can_rerun {
            Button::new(SharedString::from(format!("rerun-a-{index}")))
                .ghost()
                .xsmall()
                .label("Re-run")
                .on_click(|_, _, _| {})
                .into_any_element()
        } else {
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("App")
                .into_any_element()
        })
}

fn job_row(pipeline_index: usize, job: &StubJob, cx: &App) -> impl IntoElement {
    let emphasis = job.required && matches!(job.state, JobState::Failure | JobState::Pending);
    h_flex()
        .id(SharedString::from(format!(
            "job-{}-{}",
            pipeline_index, job.name
        )))
        .w_full()
        .px_3()
        .pl_8()
        .py(px(4.))
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border.opacity(0.6))
        .when(emphasis, |el| el.bg(cx.theme().danger.opacity(0.08)))
        .child(state_dot(job.state, cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .truncate()
                .when(emphasis, |el| el.font_medium())
                .child(job.name),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(if job.required { "required" } else { "optional" }),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(job.duration),
        )
        .child(if job.can_rerun {
            Button::new(SharedString::from(format!(
                "rerun-job-{}-{}",
                pipeline_index, job.name
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
        })
}

fn attention_row(
    ix: usize,
    pipeline_name: &str,
    job: &StubJob,
    cx: &App,
) -> impl IntoElement {
    h_flex()
        .id(SharedString::from(format!("attn-{ix}")))
        .w_full()
        .px_3()
        .py(px(6.))
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().danger.opacity(0.1))
        .child(state_dot(job.state, cx))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0()
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .truncate()
                        .child(format!("{} · {}", pipeline_name, job.name)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!(
                            "{} · {}",
                            if job.required {
                                "required"
                            } else {
                                "optional"
                            },
                            job.state.label()
                        )),
                ),
        )
        .child(if job.can_rerun {
            Button::new(SharedString::from(format!("rerun-attn-{ix}")))
                .outline()
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
        })
}

fn timeline_pipeline_row(index: usize, pipeline: &StubPipeline, cx: &App) -> impl IntoElement {
    let required_fail = pipeline
        .jobs
        .iter()
        .any(|j| j.required && j.state == JobState::Failure);
    let required_pending = pipeline
        .jobs
        .iter()
        .any(|j| j.required && j.state == JobState::Pending);
    let urgency = if required_fail {
        "required failing"
    } else if required_pending {
        "required pending"
    } else {
        "ok / optional noise"
    };

    h_flex()
        .id(SharedString::from(format!("tl-{index}")))
        .w_full()
        .px_3()
        .py(px(5.))
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .w(px(8.))
                .h(px(8.))
                .rounded_full()
                .bg(state_color(pipeline.state, cx)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .truncate()
                .child(format!(
                    "{} · {} · {}",
                    pipeline.name,
                    pipeline.kind.label(),
                    pipeline.created
                )),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(urgency),
        )
}

fn section_label(text: &str, cx: &App) -> impl IntoElement {
    div()
        .w_full()
        .px_3()
        .py(px(6.))
        .bg(cx.theme().secondary.opacity(0.4))
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(text.to_string()),
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

fn attention_jobs(pipelines: &[StubPipeline]) -> Vec<(&'static str, &'static StubJob)> {
    let mut out = Vec::new();
    for pipeline in pipelines {
        for job in pipeline.jobs {
            let required_bad =
                job.required && matches!(job.state, JobState::Failure | JobState::Pending);
            let optional_fail = !job.required && job.state == JobState::Failure;
            if !required_bad && !optional_fail {
                continue;
            }
            out.push((pipeline.name, job));
        }
    }
    out.sort_by_key(|(_, job)| {
        let required_rank = if job.required { 0 } else { 1 };
        let state_rank = match job.state {
            JobState::Failure => 0,
            JobState::Pending => 1,
            _ => 2,
        };
        (required_rank, state_rank)
    });
    out
}

enum FlatRow {
    Pipeline {
        index: usize,
        pipeline: &'static StubPipeline,
    },
    Job {
        pipeline_index: usize,
        job: &'static StubJob,
    },
}

fn visible_row_count_a(pipelines: &[StubPipeline], expanded: &[bool]) -> usize {
    let mut n = 0;
    for (ix, pipeline) in pipelines.iter().enumerate() {
        n += 1;
        if expanded.get(ix).copied().unwrap_or(false) {
            n += pipeline.jobs.len();
        }
    }
    n
}

fn row_at_a(
    pipelines: &'static [StubPipeline],
    expanded: &[bool],
    flat_ix: usize,
) -> Option<FlatRow> {
    let mut cursor = 0;
    for (index, pipeline) in pipelines.iter().enumerate() {
        if cursor == flat_ix {
            return Some(FlatRow::Pipeline { index, pipeline });
        }
        cursor += 1;
        if !expanded.get(index).copied().unwrap_or(false) {
            continue;
        }
        for job in pipeline.jobs {
            if cursor == flat_ix {
                return Some(FlatRow::Job {
                    pipeline_index: index,
                    job,
                });
            }
            cursor += 1;
        }
    }
    None
}

fn state_dot(state: JobState, cx: &App) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(state_color(state, cx))
        .child(match state {
            JobState::Success => "●",
            JobState::Pending => "◐",
            JobState::Failure => "✕",
        })
}

fn state_color(state: JobState, cx: &App) -> gpui_kit::Hsla {
    match state {
        JobState::Success => cx.theme().success,
        JobState::Pending => cx.theme().warning,
        JobState::Failure => cx.theme().danger,
    }
}

fn rollup_visual(
    state: RollupState,
    cx: &mut Context<ReviewDesk>,
) -> (&'static str, gpui_kit::Hsla) {
    match state {
        RollupState::Success => ("●", cx.theme().success),
        RollupState::Pending => ("◐", cx.theme().warning),
        RollupState::Failure => ("✕", cx.theme().danger),
        RollupState::Absent => ("○", cx.theme().muted_foreground),
    }
}

fn rollup_bg(state: RollupState, cx: &mut Context<ReviewDesk>) -> gpui_kit::Hsla {
    match state {
        RollupState::Failure => cx.theme().danger.opacity(0.1),
        RollupState::Pending => cx.theme().warning.opacity(0.1),
        RollupState::Success => cx.theme().success.opacity(0.08),
        RollupState::Absent => cx.theme().secondary.opacity(0.35),
    }
}
