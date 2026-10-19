use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    list::ListItem,
    scroll::Scrollbar,
    spinner::Spinner,
    v_flex,
};
use gpui_kit::{
    Context, IntoElement, ParentElement as _, SharedString, Styled as _, UniformListScrollHandle,
    WeakEntity, div, px, uniform_list,
};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, LoadState, SessionView};

pub(crate) fn render_inbox(
    view: &SessionView,
    inbox_scroll: &UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .size_full()
        .min_h_0()
        .child(
            gpui_kit::component::h_flex()
                .w_full()
                .px_3()
                .py(px(4.))
                .justify_between()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .font_medium()
                        .child("INBOX"),
                )
                .child(
                    Button::new("inbox-refresh")
                        .ghost()
                        .xsmall()
                        .icon(IconName::RefreshCw)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::RefreshInbox, cx);
                        })),
                ),
        )
        .child(
            div()
                .px_3()
                .py(px(2.))
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .truncate()
                        .child(view.inbox_sync.status_label()),
                ),
        )
        .child(if let Some(message) = &view.inbox_auth_error {
            div()
                .px_3()
                .py(px(2.))
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(message.clone()),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .child(match &view.inbox {
            LoadState::Idle | LoadState::Loading => v_flex()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .gap_2()
                .child(Spinner::new().small())
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading inbox…"),
                )
                .into_any_element(),
            LoadState::Error(error) => {
                let message = error.to_string();
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .px_4()
                    .child(
                        div()
                            .text_sm()
                            .text_center()
                            .text_color(cx.theme().danger)
                            .child(message),
                    )
                    .child(
                        Button::new("inbox-retry")
                            .outline()
                            .small()
                            .label("Retry")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dispatch(Intent::RefreshInbox, cx);
                            })),
                    )
                    .into_any_element()
            }
            LoadState::Ready(items) if items.is_empty() => v_flex()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .px_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .text_center()
                        .child(view.inbox_sync.empty_placeholder()),
                )
                .into_any_element(),
            LoadState::Ready(items) => {
                let items = items.clone();
                let selected_id = view.selected_id.clone();
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        uniform_list("inbox-list", items.len(), move |range, _, cx| {
                            range
                                .map(|ix| {
                                    let thread = &items[ix];
                                    let notification_id = thread.id.clone();
                                    let selected =
                                        selected_id.as_ref() == Some(&thread.pull_request_id);
                                    let owner_for_done = entity.clone();
                                    let done_id = notification_id.clone();
                                    let owner_for_open = entity.clone();
                                    ListItem::new(SharedString::from(thread.id.as_str()))
                                        .selected(selected)
                                        .px_3()
                                        .py(px(4.))
                                        .child(
                                            gpui_kit::component::h_flex()
                                                .w_full()
                                                .gap_2()
                                                .items_start()
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .gap_1()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .font_medium()
                                                                .truncate()
                                                                .child(thread.title.clone()),
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
                                                                    thread.repo_label,
                                                                    thread.reason.label()
                                                                )),
                                                        ),
                                                )
                                                .child(
                                                    Button::new(SharedString::from(format!(
                                                        "inbox-done-{}",
                                                        thread.id.as_str()
                                                    )))
                                                    .ghost()
                                                    .xsmall()
                                                    .label("Done")
                                                    .on_click(move |_, _, cx| {
                                                        let id = done_id.clone();
                                                        let _ = owner_for_done.update(
                                                            cx,
                                                            |this, cx| {
                                                                this.dispatch(
                                                                    Intent::NotificationDone(id),
                                                                    cx,
                                                                );
                                                            },
                                                        );
                                                    }),
                                                ),
                                        )
                                        .on_click(move |_, _, cx| {
                                            let id = notification_id.clone();
                                            let _ = owner_for_open.update(cx, |this, cx| {
                                                this.dispatch(Intent::SelectNotification(id), cx);
                                            });
                                        })
                                        .into_any_element()
                                })
                                .collect()
                        })
                        .size_full()
                        .track_scroll(inbox_scroll),
                    )
                    .child(Scrollbar::vertical(inbox_scroll))
                    .into_any_element()
            }
        })
}
