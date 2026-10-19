use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
};
use gpui_kit::{Context, IntoElement, ParentElement as _, Styled as _, WeakEntity, div};
use reviewdesk_core::ReviewSubmitEvent;

use crate::app::ReviewDesk;
use crate::review_session::{Intent, SessionView, ThreadView};

pub(crate) fn pending_draft_count(threads: &[ThreadView]) -> usize {
    threads
        .iter()
        .flat_map(|thread| thread.comments.iter())
        .filter(|comment| comment.is_pending)
        .count()
}

pub(crate) fn render_pending_review_chip(
    view: &SessionView,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    if !view.has_pending_review {
        return div().into_any_element();
    }
    let drafts = pending_draft_count(view.shared_threads.as_ref());
    h_flex()
        .gap_2()
        .items_center()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary.opacity(0.45))
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child(format!("Pending review · {drafts} drafts")),
        )
        .child(
            Button::new("pending-submit-chip")
                .small()
                .label("Submit…")
                .bg(cx.theme().foreground)
                .text_color(cx.theme().background)
                .on_click({
                    let owner = entity.clone();
                    move |_, _, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.dispatch(
                                Intent::SubmitPendingReview {
                                    event: ReviewSubmitEvent::Comment,
                                    summary_body: None,
                                },
                                cx,
                            );
                        });
                    }
                }),
        )
        .child(
            Button::new("pending-discard-chip")
                .ghost()
                .xsmall()
                .label("Discard")
                .on_click({
                    let owner = entity.clone();
                    move |_, _, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.dispatch(Intent::DiscardPendingReview, cx);
                        });
                    }
                }),
        )
        .into_any_element()
}

pub(crate) fn render_pending_review_bar(
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .px_3()
        .py_2()
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary)
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child("Pending review"),
        )
        .child(
            Button::new("pending-submit-comment")
                .outline()
                .xsmall()
                .label("Submit comment")
                .on_click({
                    let owner = entity.clone();
                    move |_, _, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.dispatch(
                                Intent::SubmitPendingReview {
                                    event: ReviewSubmitEvent::Comment,
                                    summary_body: None,
                                },
                                cx,
                            );
                        });
                    }
                }),
        )
        .child(
            Button::new("pending-approve")
                .outline()
                .xsmall()
                .label("Approve")
                .on_click({
                    let owner = entity.clone();
                    move |_, _, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.dispatch(
                                Intent::SubmitPendingReview {
                                    event: ReviewSubmitEvent::Approve,
                                    summary_body: None,
                                },
                                cx,
                            );
                        });
                    }
                }),
        )
        .child(
            Button::new("pending-request-changes")
                .outline()
                .xsmall()
                .label("Request changes")
                .on_click({
                    let owner = entity.clone();
                    move |_, _, cx| {
                        let _ = owner.update(cx, |this, cx| {
                            this.dispatch(
                                Intent::SubmitPendingReview {
                                    event: ReviewSubmitEvent::RequestChanges,
                                    summary_body: None,
                                },
                                cx,
                            );
                        });
                    }
                }),
        )
        .child(
            Button::new("pending-discard")
                .ghost()
                .xsmall()
                .label("Discard")
                .on_click(move |_, _, cx| {
                    let _ = entity.update(cx, |this, cx| {
                        this.dispatch(Intent::DiscardPendingReview, cx);
                    });
                }),
        )
}
