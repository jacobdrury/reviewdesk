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
        .border_r_1()
        .border_color(cx.theme().border)
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
                        .child("Loading Inbox…"),
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
                let selected = view.selected_id.clone();
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        uniform_list("inbox-list", items.len(), move |range, _, cx| {
                            range
                                .map(|ix| {
                                    let item = &items[ix];
                                    let id = item.id.clone();
                                    let selected = selected.as_ref() == Some(&item.id);
                                    let owner = entity.clone();
                                    ListItem::new(SharedString::from(item.id.as_str()))
                                        .selected(selected)
                                        .px_3()
                                        .py(px(4.))
                                        .child(
                                            v_flex()
                                                .w_full()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_medium()
                                                        .truncate()
                                                        .child(item.title.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .truncate()
                                                        .child(format!(
                                                            "{} · {}",
                                                            item.repo_label, item.author_login
                                                        )),
                                                ),
                                        )
                                        .on_click(move |_, _, cx| {
                                            let id = id.clone();
                                            let _ = owner.update(cx, |this, cx| {
                                                this.dispatch(Intent::SelectPullRequest(id), cx);
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
