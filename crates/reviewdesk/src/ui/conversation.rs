use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{
    App, Context, IntoElement, ParentElement as _, SharedString, Styled as _,
    UniformListScrollHandle, WeakEntity, div, prelude::FluentBuilder as _, px, uniform_list,
};
use reviewdesk_core::ThreadStatus;

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, ReviewCommentView, SessionView, ThreadView};
use crate::ui::pending_review::render_pending_review_chip;

const DEFAULT_TIMELINE_COMMENT: &str = "Timeline comment from Review Desk.";
/// Empty-state composer width — Linear comments center a constrained input, not a
/// full-bleed bottom bar.
const EMPTY_COMPOSER_MAX_WIDTH: f32 = 560.;

pub(crate) fn render_conversation(
    view: &SessionView,
    scroll: &UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    match &view.threads {
        LoadState::Idle | LoadState::Loading => {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading conversation…"),
                )
                .into_any_element();
        }
        LoadState::Error(error) => {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .px_4()
                .child(
                    div()
                        .text_sm()
                        .text_center()
                        .text_color(cx.theme().danger)
                        .child(error.to_string()),
                )
                .into_any_element();
        }
        LoadState::Ready(()) => {}
    }

    let threads = conversation_threads(view.shared_threads.as_ref(), view.hide_resolved_threads);
    let reply_open = view.conversation_reply_thread.clone();
    let row_count = threads.len();
    let header_entity = entity.clone();
    let list_entity = entity.clone();
    let composer_entity = entity;

    // Linear contrast: dimmer chrome canvas → lifted feed surface → thread cards one
    // step above that. Nested replies stay borderless (indent + rail only).
    v_flex()
        .size_full()
        .min_h_0()
        .bg(cx.theme().secondary.opacity(0.22))
        .child(render_conversation_header(view, header_entity, cx))
        .when(row_count == 0, |el| {
            el.child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("No comments yet."),
                    )
                    .child(div().w_full().max_w(px(EMPTY_COMPOSER_MAX_WIDTH)).child(
                        render_timeline_composer(
                            composer_entity.clone(),
                            ComposerPlacement::Centered,
                            cx,
                        ),
                    )),
            )
        })
        .when(row_count > 0, |el| {
            el.child(
                div().flex_1().min_h_0().child(
                    uniform_list("conversation-threads", row_count, move |range, _, cx| {
                        range
                            .map(|ix| {
                                let thread = &threads[ix];
                                let reply = reply_open
                                    .as_ref()
                                    .is_some_and(|id| id.as_ref() == thread.id.as_str());
                                div()
                                    .w_full()
                                    .px_4()
                                    .pt_3()
                                    .when(ix + 1 == threads.len(), |row| row.pb_2())
                                    .child(render_linear_thread(
                                        thread,
                                        reply,
                                        list_entity.clone(),
                                        cx,
                                    ))
                                    .into_any_element()
                            })
                            .collect()
                    })
                    .track_scroll(scroll),
                ),
            )
            .child(render_timeline_composer(
                composer_entity,
                ComposerPlacement::Docked,
                cx,
            ))
        })
        .into_any_element()
}

/// Diff inline card — same Linear comment anatomy as Conversation roots.
pub(crate) fn render_diff_thread_annotation(
    author: &str,
    body: &str,
    is_pending: bool,
    suggestion: Option<&str>,
    cx: &App,
) -> impl IntoElement {
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
                        .child(avatar_chip(author, cx))
                        .child(div().text_xs().font_medium().child(author.to_string()))
                        .when(is_pending, |el| {
                            el.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Draft"),
                            )
                        }),
                )
                .child(render_comment_actions_row("diff-ann", true)),
        )
        .child(div().text_sm().child(body.to_string()))
        .when_some(suggestion, |el, text| el.child(suggestion_block(text, cx)))
        .child(
            h_flex()
                .gap_1()
                .child(ghost_btn("diff-reply", "Reply"))
                .child(ghost_btn("diff-react", "React")),
        )
}

fn conversation_threads(threads: &[ThreadView], hide_resolved: bool) -> Vec<ThreadView> {
    threads
        .iter()
        .filter(|thread| !hide_resolved || !matches!(thread.status, ThreadStatus::Resolved))
        .cloned()
        .collect()
}

fn render_conversation_header(
    view: &SessionView,
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
                .when(view.threads_syncing, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Updating…"),
                    )
                }),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(render_pending_review_chip(view, entity.clone(), cx))
                .child(
                    Button::new("toggle-resolved")
                        .ghost()
                        .xsmall()
                        .label(if view.hide_resolved_threads {
                            "Show resolved"
                        } else {
                            "Hide resolved"
                        })
                        .on_click(move |_, _, cx| {
                            let _ = entity.update(cx, |this, cx| {
                                this.dispatch(Intent::ToggleHideResolvedThreads, cx);
                            });
                        }),
                ),
        )
}

#[derive(Clone, Copy)]
enum ComposerPlacement {
    /// Filled Conversation: full-width bar docked under the thread list.
    Docked,
    /// Empty Conversation: centered, width-constrained (not full-bleed).
    Centered,
}

fn render_timeline_composer(
    entity: WeakEntity<ReviewDesk>,
    placement: ComposerPlacement,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let composer = h_flex()
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
                            h_flex().gap_1().child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("⌘ Enter to comment"),
                            ),
                        )
                        .child(cta_btn(
                            "post-timeline-comment",
                            "Comment",
                            entity.clone(),
                            cx,
                        )),
                ),
        );

    match placement {
        ComposerPlacement::Docked => v_flex()
            .w_full()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .px_4()
            .py_3()
            .gap_2()
            .child(composer)
            .into_any_element(),
        ComposerPlacement::Centered => v_flex()
            .w_full()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .px_4()
            .py_3()
            .gap_2()
            .child(composer)
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Publishes immediately (not Pending review)"),
            )
            .into_any_element(),
    }
}

fn render_linear_thread(
    thread: &ThreadView,
    reply_open: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut App,
) -> impl IntoElement {
    let Some(root) = thread.comments.first() else {
        return div().into_any_element();
    };
    let replies = &thread.comments[1..];
    let resolved = matches!(thread.status, ThreadStatus::Resolved);

    v_flex()
        .w_full()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        // Surface-1 lift on the feed surface (Linear depth = surface ladder + hairline).
        .bg(cx.theme().secondary.opacity(0.72))
        .when(resolved, |el| {
            el.child(
                h_flex().gap_2().items_center().child(
                    div()
                        .px_1()
                        .rounded_sm()
                        .bg(cx.theme().success.opacity(0.2))
                        .text_xs()
                        .text_color(cx.theme().success)
                        .child("Resolved"),
                ),
            )
        })
        .child(render_comment_row(root, thread, true, entity.clone(), cx))
        .when(!replies.is_empty(), |el| {
            el.child(
                v_flex()
                    .w_full()
                    .pl(px(14.))
                    .gap_3()
                    .pt_1()
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .ml(px(11.))
                    .children(replies.iter().map(|comment| {
                        // Linear: nested replies share the parent surface — indent + rail only.
                        render_comment_row(comment, thread, false, entity.clone(), cx)
                            .into_any_element()
                    })),
            )
        })
        .when(reply_open, |el| {
            el.child(
                h_flex()
                    .w_full()
                    .pl(px(14.))
                    .ml(px(11.))
                    .pt_1()
                    .gap_2()
                    .border_l_1()
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
                                    .bg(cx.theme().background.opacity(0.55))
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
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "reply-send-{}",
                                            thread.id
                                        )))
                                        .small()
                                        .label("Reply")
                                        .bg(cx.theme().foreground)
                                        .text_color(cx.theme().background),
                                    ),
                            ),
                    ),
            )
        })
        .into_any_element()
}

fn render_comment_row(
    comment: &ReviewCommentView,
    thread: &ThreadView,
    is_root: bool,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut App,
) -> impl IntoElement {
    let thread_id = thread.id.clone();
    let can_resolve =
        is_root && !matches!(thread.status, ThreadStatus::Outdated) && thread.anchor.is_some();
    let resolve_label = if matches!(thread.status, ThreadStatus::Resolved) {
        "Unresolve"
    } else {
        "Resolve"
    };
    let next_status = if matches!(thread.status, ThreadStatus::Resolved) {
        ThreadStatus::Open
    } else {
        ThreadStatus::Resolved
    };

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
                        .child(avatar_chip(comment.author_login.as_ref(), cx))
                        .child(
                            div()
                                .text_sm()
                                .font_medium()
                                .child(comment.author_login.to_string()),
                        )
                        .when(comment.is_pending, |el| {
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
                        .when(
                            matches!(thread.status, ThreadStatus::Outdated) && is_root,
                            |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Outdated"),
                                )
                            },
                        ),
                )
                .child(
                    h_flex()
                        .gap_0()
                        .items_center()
                        .opacity(0.7)
                        .when(can_resolve, |el| {
                            let owner = entity.clone();
                            let tid = thread_id.clone();
                            el.child(
                                Button::new(SharedString::from(format!("resolve-{thread_id}")))
                                    .ghost()
                                    .xsmall()
                                    .label(resolve_label)
                                    .on_click(move |_, _, cx| {
                                        let _ = owner.update(cx, |this, cx| {
                                            this.dispatch(
                                                Intent::SetThreadStatus {
                                                    thread_id: tid.clone(),
                                                    status: next_status,
                                                },
                                                cx,
                                            );
                                        });
                                    }),
                            )
                        })
                        .child(ghost_btn(format!("more-{thread_id}"), "···")),
                ),
        )
        .when(is_root && thread.anchor.is_some(), |el| {
            el.child(div().pl(px(28.)).child(render_anchor_header(thread, cx)))
        })
        .child(div().pl(px(28.)).text_sm().child(comment.body.to_string()))
        .when_some(comment.suggestion.as_ref(), |el, suggestion| {
            el.child(
                div()
                    .pl(px(28.))
                    .child(suggestion_block(suggestion.as_ref(), cx)),
            )
        })
        .child(
            h_flex()
                .pl(px(28.))
                .gap_1()
                .items_center()
                .child({
                    let owner = entity.clone();
                    let tid = thread_id.clone();
                    Button::new(SharedString::from(format!("reply-{thread_id}")))
                        .ghost()
                        .xsmall()
                        .label("Reply")
                        .on_click(move |_, _, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                this.dispatch(
                                    Intent::SetConversationReplyThread(Some(tid.clone().into())),
                                    cx,
                                );
                            });
                        })
                })
                .child(ghost_btn(format!("react-{thread_id}"), "React")),
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

fn render_anchor_header(thread: &ThreadView, cx: &mut App) -> impl IntoElement {
    let Some(anchor) = &thread.anchor else {
        return div().into_any_element();
    };
    div()
        .text_xs()
        .font_medium()
        .text_color(cx.theme().link)
        .child(format!(
            "{} · L{}–L{}",
            anchor.path, anchor.start_line, anchor.end_line
        ))
        .into_any_element()
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
                .child("Suggestion"),
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
fn cta_btn(
    id: impl Into<SharedString>,
    label: &str,
    entity: WeakEntity<ReviewDesk>,
    cx: &App,
) -> impl IntoElement {
    Button::new(id.into())
        .small()
        .label(label)
        .bg(cx.theme().foreground)
        .text_color(cx.theme().background)
        .on_click(move |_, _, cx| {
            let _ = entity.update(cx, |this, cx| {
                this.dispatch(
                    Intent::PublishTimelineComment {
                        body: DEFAULT_TIMELINE_COMMENT.to_string(),
                    },
                    cx,
                );
            });
        })
}
