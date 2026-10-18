//! PROTOTYPE paint: Conversation + Diff Thread variants. Throwaway with the branch.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use gpui_kit::{
    App, Context, ElementId, IntoElement, ParentElement as _, SharedString, Styled as _,
    WeakEntity, div, px,
    prelude::FluentBuilder as _,
};

use crate::app::ReviewDesk;
use crate::threads_prototype::{
    PENDING_REVIEW_SUMMARY, STUB_THREADS, STUB_TIMELINE, StubThread, ThreadFilter, ThreadStatus,
    ThreadVariant, filtered_threads, thread_by_id,
};

pub(crate) fn render_pending_review_strip(
    variant: ThreadVariant,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let compact = matches!(variant, ThreadVariant::C);
    h_flex()
        .w_full()
        .px_3()
        .py(if compact { px(6.) } else { px(8.) })
        .gap_2()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().warning.opacity(0.12))
        .child(
            v_flex()
                .gap_0()
                .min_w_0()
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().warning)
                        .child("Pending review"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .child(PENDING_REVIEW_SUMMARY),
                ),
        )
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child(stub_btn("pr-submit-comment", "Submit comment"))
                .child(stub_btn("pr-approve", "Approve"))
                .child(stub_btn("pr-request", "Request changes"))
                .child(
                    Button::new("pr-discard")
                        .ghost()
                        .xsmall()
                        .label("Discard")
                        .on_click(|_, _, _| {}),
                ),
        )
}

pub(crate) fn render_conversation(
    variant: ThreadVariant,
    filter: ThreadFilter,
    selected_id: &str,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    match variant {
        ThreadVariant::A => render_conversation_inline(cx).into_any_element(),
        ThreadVariant::B => {
            render_conversation_split(selected_id, entity, cx).into_any_element()
        }
        ThreadVariant::C => {
            render_conversation_filtered(filter, selected_id, entity, cx).into_any_element()
        }
    }
}

pub(crate) fn render_changes_with_threads(
    variant: ThreadVariant,
    selected_id: &str,
    files: gpui_kit::AnyElement,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let pending = render_pending_review_strip(variant, cx).into_any_element();

    match variant {
        ThreadVariant::A => v_flex()
            .size_full()
            .min_h_0()
            .child(pending)
            .child(div().flex_1().min_h_0().child(files))
            .into_any_element(),
        ThreadVariant::B => v_flex()
            .size_full()
            .min_h_0()
            .child(pending)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(div().flex_1().min_h_0().min_w_0().child(files))
                    .child(
                        div()
                            .w(px(320.))
                            .h_full()
                            .min_h_0()
                            .border_l_1()
                            .border_color(cx.theme().border)
                            .child(render_thread_detail(
                                selected_id,
                                "Diff rail · same Thread set",
                                true,
                                cx,
                            )),
                    ),
            )
            .into_any_element(),
        ThreadVariant::C => {
            let inspector_open = thread_by_id(selected_id).is_some();
            v_flex()
                .size_full()
                .min_h_0()
                .child(pending)
                .child(div().flex_1().min_h_0().child(files))
                .when(inspector_open, |el| {
                    el.child(
                        div()
                            .h(px(220.))
                            .w_full()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().secondary.opacity(0.35))
                            .child(render_thread_detail(
                                selected_id,
                                "Inspector · Diff marker selected",
                                true,
                                cx,
                            )),
                    )
                })
                .into_any_element()
        }
    }
}

/// Compact Diff annotation body for variants B/C; full card for A.
pub(crate) fn render_diff_thread_annotation(
    variant: ThreadVariant,
    body: &str,
    cx: &App,
) -> impl IntoElement {
    let suggestion = body.contains("```suggestion") || body.starts_with("Suggestion:");
    match variant {
        ThreadVariant::A => v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded_md()
            .bg(cx.theme().secondary)
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Thread · Review comment"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().warning)
                            .child("Pending review"),
                    ),
            )
            .child(div().text_sm().child(body.to_string()))
            .when(suggestion, |el| {
                el.child(
                    div()
                        .mt_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(cx.theme().border)
                        .text_xs()
                        .child("Suggestion stub · compose/display only"),
                )
            })
            .child(
                h_flex()
                    .gap_1()
                    .child(stub_btn("diff-reply", "Reply"))
                    .child(stub_btn("diff-resolve", "Resolve")),
            )
            .into_any_element(),
        ThreadVariant::B | ThreadVariant::C => h_flex()
            .w_full()
            .gap_2()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(cx.theme().secondary.opacity(0.8))
            .items_center()
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().accent_foreground)
                    .child("Thread"),
            )
            .child(
                div()
                    .text_xs()
                    .truncate()
                    .text_color(cx.theme().muted_foreground)
                    .child(body.to_string()),
            )
            .into_any_element(),
    }
}

fn render_conversation_inline(cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .size_full()
        .min_h_0()
        .child(render_pending_review_strip(ThreadVariant::A, cx))
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .p_3()
                .gap_3()
                .child(section_label("Conversation · chronological", cx))
                .children(
                    STUB_TIMELINE
                        .iter()
                        .map(|item| timeline_card(item.author, item.body, cx).into_any_element()),
                )
                .children(
                    STUB_THREADS
                        .iter()
                        .map(|thread| thread_card(thread, true, true, cx).into_any_element()),
                )
                .child(compose_stub(
                    "Reply or start a Thread from Conversation (attaches to Pending review)",
                    cx,
                )),
        )
}

fn render_conversation_split(
    selected_id: &str,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .size_full()
        .min_h_0()
        .child(render_pending_review_strip(ThreadVariant::B, cx))
        .child(
            h_resizable("conversation-split")
                .child(
                    resizable_panel()
                        .size(px(280.))
                        .size_range(px(200.)..px(400.))
                        .child(
                            v_flex()
                                .size_full()
                                .min_h_0()
                                .border_r_1()
                                .border_color(cx.theme().border)
                                .child(
                                    div()
                                        .px_3()
                                        .py_2()
                                        .border_b_1()
                                        .border_color(cx.theme().border)
                                        .child(section_label("Threads", cx)),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_h_0()
                                        .children(STUB_THREADS.iter().map(|thread| {
                                            thread_summary_row(
                                                thread,
                                                thread.id == selected_id,
                                                entity.clone(),
                                                cx,
                                            )
                                            .into_any_element()
                                        })),
                                ),
                        ),
                )
                .child(
                    resizable_panel().child(render_thread_detail(
                        selected_id,
                        "Conversation detail",
                        true,
                        cx,
                    )),
                ),
        )
}

fn render_conversation_filtered(
    filter: ThreadFilter,
    selected_id: &str,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let threads = filtered_threads(filter);
    v_flex()
        .size_full()
        .min_h_0()
        .child(render_pending_review_strip(ThreadVariant::C, cx))
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py_2()
                .gap_1()
                .border_b_1()
                .border_color(cx.theme().border)
                .children(ThreadFilter::all().iter().copied().map(|f| {
                    let selected = f == filter;
                    let owner = entity.clone();
                    Button::new(SharedString::from(format!("thread-filter-{}", f.label())))
                        .when(selected, |b| b.primary())
                        .when(!selected, |b| b.ghost())
                        .xsmall()
                        .label(f.label())
                        .on_click(move |_, _, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                this.set_thread_filter(f, cx);
                            });
                        })
                })),
        )
        .child(
            v_flex().flex_1().min_h_0().children(threads.into_iter().map(|thread| {
                thread_summary_row(thread, thread.id == selected_id, entity.clone(), cx)
                    .into_any_element()
            })),
        )
        .child(
            div()
                .h(px(200.))
                .w_full()
                .border_t_1()
                .border_color(cx.theme().border)
                .child(render_thread_detail(
                    selected_id,
                    "Inspector · Conversation",
                    true,
                    cx,
                )),
        )
}

fn render_thread_detail(
    selected_id: &str,
    caption: &str,
    show_compose: bool,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let Some(thread) = thread_by_id(selected_id) else {
        return v_flex()
            .size_full()
            .p_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a Thread"),
            )
            .into_any_element();
    };

    v_flex()
        .size_full()
        .min_h_0()
        .child(
            div()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(section_label(caption, cx)),
        )
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .p_3()
                .child(thread_card(thread, true, false, cx)),
        )
        .when(show_compose, |el| {
            el.child(
                div()
                    .p_3()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(compose_stub(
                        "Reply (draft on Pending review · stub)",
                        cx,
                    )),
            )
        })
        .into_any_element()
}

fn thread_summary_row(
    thread: &StubThread,
    selected: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let id = thread.id;
    let status = status_label(thread.status);
    let pending = if thread.pending_draft {
        " · pending"
    } else {
        ""
    };
    ListItem::new(format!("thread-sum-{id}"))
        .selected(selected)
        .px_3()
        .py_2()
        .child(
            v_flex()
                .gap_0()
                .w_full()
                .min_w_0()
                .child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .truncate()
                                .child(thread.path.to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{status}{pending}")),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .child(thread.line_label.to_string()),
                )
                .child(
                    div().text_xs().truncate().child(
                        thread
                            .comments
                            .first()
                            .map(|c| c.body)
                            .unwrap_or("")
                            .to_string(),
                    ),
                ),
        )
        .on_click(move |_, _, cx| {
            let _ = entity.update(cx, |this, cx| {
                this.select_thread(id, cx);
            });
        })
}

fn thread_card(
    thread: &StubThread,
    expanded: bool,
    show_header_actions: bool,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let status = status_label(thread.status);
    let resolve_label = if matches!(thread.status, ThreadStatus::Resolved) {
        "Unresolve"
    } else {
        "Resolve"
    };
    v_flex()
        .w_full()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.4))
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .gap_2()
                .child(
                    v_flex()
                        .gap_0()
                        .min_w_0()
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .truncate()
                                .child(thread.path.to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{} · {}", thread.line_label, status)),
                        ),
                )
                .when(show_header_actions, |el| {
                    el.child(
                        h_flex()
                            .gap_1()
                            .child(stub_btn(
                                format!("resolve-{}", thread.id),
                                resolve_label,
                            ))
                            .when(thread.pending_draft, |row| {
                                row.child(
                                    div()
                                        .px_1()
                                        .rounded_sm()
                                        .bg(cx.theme().warning.opacity(0.25))
                                        .text_xs()
                                        .text_color(cx.theme().warning)
                                        .child("Pending"),
                                )
                            }),
                    )
                }),
        )
        .when(expanded, |el| {
            el.children(thread.comments.iter().map(|comment| {
                comment_block(
                    comment.author,
                    comment.body,
                    comment.pending,
                    comment.suggestion,
                    cx,
                )
                .into_any_element()
            }))
        })
}

fn comment_block(
    author: &str,
    body: &str,
    pending: bool,
    suggestion: Option<&str>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .p_2()
        .rounded_md()
        .bg(cx.theme().background)
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(div().text_xs().font_medium().child(author.to_string()))
                .when(pending, |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().warning)
                            .child("on Pending review"),
                    )
                })
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Review comment"),
                ),
        )
        .child(div().text_sm().child(body.to_string()))
        .when_some(suggestion, |el, text| {
            el.child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .mt_1()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().accent.opacity(0.08))
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .child("Suggestion · compose/display only"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(text.to_string()),
                    ),
            )
        })
}

fn timeline_card(author: &str, body: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .gap_2()
                .child(div().text_xs().font_medium().child(author.to_string()))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("timeline comment"),
                ),
        )
        .child(div().text_sm().child(body.to_string()))
}

fn compose_stub(placeholder: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_2()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.25))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(placeholder.to_string()),
        )
        .child(
            h_flex()
                .gap_1()
                .child(stub_btn("compose-reply", "Save draft"))
                .child(stub_btn("compose-suggestion", "Add Suggestion")),
        )
}

fn section_label(text: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    div()
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(text.to_string())
}

fn status_label(status: ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Open => "Open",
        ThreadStatus::Resolved => "Resolved",
        ThreadStatus::Outdated => "Outdated",
    }
}

fn stub_btn(id: impl Into<ElementId>, label: &str) -> Button {
    Button::new(id)
        .outline()
        .xsmall()
        .label(label)
        .on_click(|_, _, _| {})
}
