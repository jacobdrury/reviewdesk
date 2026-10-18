use gpui_kit::component::{
    ActiveTheme as _,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement as _, Styled as _, UniformListScrollHandle,
    WeakEntity, div,
};
use gpui_kit::component::diff::DiffState;

use crate::app::ReviewDesk;
use crate::review_session::{Intent, ReviewTab, SessionView};
use crate::ui::{render_files, render_overview};

pub(crate) fn render_review_pane(
    view: &SessionView,
    diff: &Entity<DiffState>,
    files_scroll: &UniformListScrollHandle,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    if view.selected_id.is_none() {
        return v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a pull request from the Inbox."),
            )
            .into_any_element();
    }

    let tab_ix = match view.review_tab {
        ReviewTab::Overview => 0,
        ReviewTab::Files => 1,
    };

    v_flex()
        .size_full()
        .min_h_0()
        .child(
            TabBar::new("review-tabs")
                .w_full()
                .px_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .selected_index(tab_ix)
                .on_click(cx.listener(|this, ix: &usize, _, cx| {
                    let tab = if *ix == 0 {
                        ReviewTab::Overview
                    } else {
                        ReviewTab::Files
                    };
                    this.dispatch(Intent::SetReviewTab(tab), cx);
                }))
                .child(Tab::new().label("Overview"))
                .child(Tab::new().label("Files")),
        )
        .child(div().flex_1().min_h_0().child(match view.review_tab {
            ReviewTab::Overview => render_overview(view, entity, cx).into_any_element(),
            ReviewTab::Files => {
                render_files(view, diff, files_scroll, entity, cx).into_any_element()
            }
        }))
        .into_any_element()
}
