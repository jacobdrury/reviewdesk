use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};
use gpui_kit::{
    Context, IntoElement, ParentElement as _, Styled as _, UniformListScrollHandle, WeakEntity, px,
};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, SessionView, SidebarTab};

use super::inbox::render_inbox;
use super::pull_requests::render_pull_requests;

pub(crate) fn render_sidebar(
    view: &SessionView,
    inbox_scroll: &UniformListScrollHandle,
    pull_requests_scroll: &UniformListScrollHandle,
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
                .px_2()
                .py(px(4.))
                .gap_1()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(sidebar_tab_button(
                    "sidebar-inbox",
                    "Inbox",
                    view.sidebar_tab == SidebarTab::Inbox,
                    SidebarTab::Inbox,
                    cx,
                ))
                .child(sidebar_tab_button(
                    "sidebar-pull-requests",
                    "Pull requests",
                    view.sidebar_tab == SidebarTab::PullRequests,
                    SidebarTab::PullRequests,
                    cx,
                )),
        )
        .child(match view.sidebar_tab {
            SidebarTab::Inbox => {
                render_inbox(view, inbox_scroll, entity.clone(), cx).into_any_element()
            }
            SidebarTab::PullRequests => {
                render_pull_requests(view, pull_requests_scroll, entity, cx).into_any_element()
            }
        })
}

fn sidebar_tab_button(
    id: &'static str,
    label: &'static str,
    selected: bool,
    tab: SidebarTab,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let mut button = Button::new(id).small().label(label);
    if selected {
        button = button.outline();
    } else {
        button = button.ghost();
    }
    button.on_click(cx.listener(move |this, _, _, cx| {
        this.dispatch(Intent::SelectSidebarTab(tab), cx);
    }))
}
