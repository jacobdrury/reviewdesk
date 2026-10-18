//! PROTOTYPE paint: Linear comments clone for Conversation + Diff Threads.

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{
    App, Context, IntoElement, ParentElement as _, SharedString, Styled as _, WeakEntity, div, px,
    prelude::FluentBuilder as _,
};

use crate::app::ReviewDesk;
use crate::threads_prototype::{
    CodeLine, PENDING_DRAFT_COUNT, StubComment, StubThread, ThreadStatus, conversation_threads,
    thread_by_id,
};

pub(crate) fn render_conversation(
    hide_resolved: bool,
    reply_thread_id: Option<&'static str>,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let threads = conversation_threads(hide_resolved);
    v_flex()
        .size_full()
        .min_h_0()
        .child(render_conversation_header(hide_resolved, entity.clone(), cx))
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .px_4()
                .pt_3()
                .pb_2()
                .gap_3()
                .children(threads.into_iter().map(|thread| {
                    render_linear_thread(
                        thread,
                        reply_thread_id == Some(thread.id),
                        true,
                        entity.clone(),
                        cx,
                    )
                    .into_any_element()
                })),
        )
        .child(render_leave_a_comment_composer(cx))
}

pub(crate) fn render_changes_with_threads(
    reply_thread_id: Option<&'static str>,
    files: gpui_kit::AnyElement,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .size_full()
        .min_h_0()
        .child(render_pending_drafts_chip(cx))
        .child(div().flex_1().min_h_0().child(files))
        .when_some(reply_thread_id.and_then(thread_by_id), |el, thread| {
            el.child(
                div()
                    .w_full()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .px_3()
                    .py_2()
                    .child(render_linear_thread(thread, true, false, entity, cx)),
            )
        })
}

/// Diff inline card — same Linear comment anatomy as Conversation roots.
pub(crate) fn render_diff_thread_annotation(body: &str, cx: &App) -> impl IntoElement {
    let suggestion = body.contains("```suggestion") || body.contains("Pending review");
    v_flex()
        .w_full()
        .gap_1()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(avatar_chip("you", cx))
                        .child(div().text_xs().font_medium().child("you"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Draft"),
                        ),
                )
                .child(render_comment_actions_row("diff-ann", true)),
        )
        .child(div().text_sm().child(body.to_string()))
        .when(suggestion, |el| {
            el.child(suggestion_block("replacement lines…", cx))
        })
        .child(
            h_flex()
                .gap_1()
                .child(ghost_btn("diff-reply", "Reply"))
                .child(ghost_btn("diff-react", "React")),
        )
}

fn render_conversation_header(
    hide_resolved: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
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
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child("Conversation"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Linear comments baseline"),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(render_pending_drafts_chip(cx))
                .child(
                    Button::new("toggle-resolved")
                        .ghost()
                        .xsmall()
                        .label(if hide_resolved {
                            "Show resolved"
                        } else {
                            "Hide resolved"
                        })
                        .on_click(move |_, _, cx| {
                            let _ = entity.update(cx, |this, cx| {
                                this.toggle_hide_resolved(cx);
                            });
                        }),
                ),
        )
}

fn render_pending_drafts_chip(cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    h_flex()
        .gap_2()
        .items_center()
        .px_2()
        .py(px(4.))
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.45))
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child(format!(
                    "Pending review · {PENDING_DRAFT_COUNT} drafts"
                )),
        )
        .child(cta_btn("pr-submit", "Submit…", cx))
        .child(text_action_btn("pr-discard", "Discard", cx.theme().muted_foreground))
}

fn render_leave_a_comment_composer(cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    v_flex()
        .w_full()
        .border_t_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .px_4()
        .py_3()
        .gap_2()
        .child(
            h_flex()
                .w_full()
                .gap_2()
                .items_start()
                .child(avatar_chip("you", cx))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_2()
                        .child(
                            div()
                                .w_full()
                                .min_h(px(64.))
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .border_1()
                                .border_color(cx.theme().border)
                                .bg(cx.theme().secondary.opacity(0.25))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Leave a comment…"),
                                ),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .items_center()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(ghost_btn("attach", "📎"))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("⌘ Enter to comment"),
                                        ),
                                )
                                .child(cta_btn("post-comment", "Comment", cx)),
                        ),
                ),
        )
}

fn render_linear_thread(
    thread: &StubThread,
    reply_open: bool,
    show_resolve_controls: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let root = &thread.comments[0];
    let replies = &thread.comments[1..];
    let resolved = matches!(thread.status, ThreadStatus::Resolved);
    let showcase = thread
        .comments
        .iter()
        .find(|c| c.showcase_resolve)
        .map(|c| c.body);

    v_flex()
        .w_full()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.4))
        .when(resolved, |el| {
            el.child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .px_1()
                            .rounded_sm()
                            .bg(cx.theme().success.opacity(0.2))
                            .text_xs()
                            .text_color(cx.theme().success)
                            .child("Resolved"),
                    )
                    .when_some(showcase, |row, body| {
                        row.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .truncate()
                                .child(format!("Showcase: {body}")),
                        )
                    }),
            )
        })
        .child(render_comment_row(
            root,
            thread,
            true,
            show_resolve_controls,
            entity.clone(),
            cx,
        ))
        .when(!replies.is_empty(), |el| {
            el.child(
                v_flex()
                    .w_full()
                    .pl(px(12.))
                    .gap_2()
                    .border_l_2()
                    .border_color(cx.theme().border)
                    .ml(px(10.))
                    .children(replies.iter().map(|comment| {
                        render_reply_card(comment, thread, show_resolve_controls, entity.clone(), cx)
                            .into_any_element()
                    })),
            )
        })
        .when(reply_open, |el| {
            el.child(
                h_flex()
                    .w_full()
                    .pl(px(12.))
                    .ml(px(10.))
                    .gap_2()
                    .border_l_2()
                    .border_color(cx.theme().border)
                    .child(avatar_chip("you", cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .w_full()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .bg(cx.theme().background)
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Reply to thread… (draft → Pending review)"),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("⌘ Enter"),
                                    )
                                    .child(cta_btn(
                                        format!("reply-send-{}", thread.id),
                                        "Reply",
                                        cx,
                                    )),
                            ),
                    ),
            )
        })
}

fn render_reply_card(
    comment: &StubComment,
    thread: &StubThread,
    show_resolve_controls: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(render_comment_row(
            comment,
            thread,
            false,
            show_resolve_controls,
            entity,
            cx,
        ))
}

fn render_comment_row(
    comment: &StubComment,
    thread: &StubThread,
    is_root: bool,
    show_resolve_controls: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let thread_id = thread.id;
    v_flex()
        .w_full()
        .gap_1()
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(avatar_chip(comment.author, cx))
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .child(comment.author.to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(comment.relative_time.to_string()),
                        )
                        .when(comment.pending, |el| {
                            el.child(
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .bg(cx.theme().secondary)
                                    .text_xs()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child("Draft"),
                            )
                        })
                        .when(matches!(thread.status, ThreadStatus::Outdated) && is_root, |el| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Outdated"),
                            )
                        }),
                )
                .child(render_comment_actions_row(
                    thread_id,
                    show_resolve_controls && is_root,
                )),
        )
        .when(is_root && thread.path.is_some(), |el| {
            el.child(
                div().pl(px(28.)).child(render_anchor_header(
                    thread.path.unwrap_or(""),
                    thread.line_label.unwrap_or(""),
                    thread.code_context,
                    cx,
                )),
            )
        })
        .child(
            div()
                .pl(px(28.))
                .text_sm()
                .child(comment.body.to_string()),
        )
        .when_some(comment.suggestion, |el, text| {
            el.child(div().pl(px(28.)).child(suggestion_block(text, cx)))
        })
        .child(
            h_flex()
                .pl(px(28.))
                .gap_1()
                .items_center()
                .child({
                    let owner = entity.clone();
                    Button::new(SharedString::from(format!("reply-{thread_id}")))
                        .ghost()
                        .xsmall()
                        .label("Reply")
                        .on_click(move |_, _, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                this.set_reply_thread(thread_id, cx);
                            });
                        })
                })
                .child(ghost_btn(format!("react-{thread_id}"), "React"))
                .when(comment.showcase_resolve, |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().success)
                            .child("Resolved from this reply"),
                    )
                }),
        )
}

fn render_comment_actions_row(id: &str, show_resolve: bool) -> impl IntoElement {
    h_flex()
        .gap_0()
        .items_center()
        .opacity(0.7)
        .when(show_resolve, |el| {
            el.child(ghost_btn(format!("resolve-{id}"), "Resolve"))
        })
        .child(ghost_btn(format!("more-{id}"), "···"))
}

fn render_anchor_header(
    path: &str,
    line_label: &str,
    code_context: Option<&'static [CodeLine]>,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .mb_1()
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().link)
                .child(format!("{path} · {line_label}")),
        )
        .when_some(code_context, |el, lines| {
            el.child(render_code_context(lines, cx))
        })
}

fn render_code_context(lines: &'static [CodeLine], cx: &App) -> impl IntoElement {
    v_flex()
        .w_full()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .overflow_x_hidden()
        .children(lines.iter().map(|line| {
            let bg = if line.highlighted {
                cx.theme().accent.opacity(0.18)
            } else {
                gpui_kit::hsla(0., 0., 0., 0.)
            };
            h_flex()
                .w_full()
                .gap_2()
                .px_2()
                .py(px(1.))
                .bg(bg)
                .child(
                    div()
                        .w(px(36.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(line.number.to_string()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_xs()
                        .text_color(if line.highlighted {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(if line.text.is_empty() {
                            " ".to_string()
                        } else {
                            line.text.to_string()
                        }),
                )
        }))
}

fn suggestion_block(text: &str, cx: &App) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .mt_1()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.45))
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child("Suggestion · compose/display only"),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(text.to_string()),
        )
}

fn avatar_chip(author: &str, cx: &App) -> impl IntoElement {
    let initial = author
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('?');
    div()
        .size(px(22.))
        .rounded(px(11.))
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().secondary)
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(initial.to_string()),
        )
}

fn ghost_btn(id: impl Into<SharedString>, label: &str) -> Button {
    Button::new(id.into())
        .ghost()
        .xsmall()
        .label(label)
        .on_click(|_, _, _| {})
}

/// High-contrast CTA — avoids `.primary()`, which reads as light-on-light in Zed themes.
fn cta_btn(id: impl Into<SharedString>, label: &str, cx: &App) -> impl IntoElement {
    Button::new(id.into())
        .small()
        .label(label)
        .bg(cx.theme().foreground)
        .text_color(cx.theme().background)
        .on_click(|_, _, _| {})
}

fn text_action_btn(
    id: impl Into<SharedString>,
    label: &str,
    color: gpui_kit::Hsla,
) -> impl IntoElement {
    Button::new(id.into())
        .ghost()
        .xsmall()
        .label(label)
        .text_color(color)
        .on_click(|_, _, _| {})
}
