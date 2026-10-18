use gpui_kit::component::diff::DiffState;
use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement as _, Styled as _, UniformListScrollHandle,
    WeakEntity, div,
};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, MetadataDock, ReviewTab, SessionView};
use crate::ui::{
    pipelines_tab::render_pipelines_tab, render_files, render_metadata_panel,
    render_metadata_strip, render_overview,
};

const TABS: [(ReviewTab, &str); 4] = [
    (ReviewTab::Overview, "Overview"),
    (ReviewTab::Pipelines, "Pipelines"),
    (ReviewTab::Conversation, "Conversation"),
    (ReviewTab::Changes, "Changes"),
];

pub(crate) fn render_review_pane(
    view: &SessionView,
    diff: &Entity<DiffState>,
    files_scroll: &UniformListScrollHandle,
    pipelines_scroll: &UniformListScrollHandle,
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
                    .child("Select a pull request from Pull requests."),
            )
            .into_any_element();
    }

    let tab_ix = TABS
        .iter()
        .position(|(tab, _)| *tab == view.review_tab)
        .unwrap_or(0);

    let dock = view.metadata_dock;
    let toggle_icon = match dock {
        MetadataDock::Open => IconName::PanelRightClose,
        MetadataDock::Folded | MetadataDock::Hidden => IconName::PanelRightOpen,
    };

    let mut tab_bar = TabBar::new("review-tabs")
        .w_full()
        .selected_index(tab_ix)
        .on_click(cx.listener(|this, ix: &usize, _, cx| {
            let tab = TABS
                .get(*ix)
                .map(|(tab, _)| *tab)
                .unwrap_or(ReviewTab::Overview);
            this.dispatch(Intent::SetReviewTab(tab), cx);
        }));
    for (_, label) in TABS {
        tab_bar = tab_bar.child(Tab::new().label(label));
    }

    let content = match view.review_tab {
        ReviewTab::Overview => render_overview(view, entity.clone(), cx).into_any_element(),
        ReviewTab::Pipelines => render_pipelines_tab(view, pipelines_scroll, entity.clone(), cx),
        ReviewTab::Conversation => render_empty_tab(
            "Conversation",
            "Threads and timeline comments arrive in a later ticket.",
            cx,
        ),
        ReviewTab::Changes => {
            render_files(view, diff, files_scroll, entity.clone(), cx).into_any_element()
        }
    };

    let mut body = h_flex()
        .flex_1()
        .min_h_0()
        .w_full()
        .child(div().flex_1().min_h_0().min_w_0().child(content));
    if dock == MetadataDock::Open {
        body = body.child(render_metadata_panel(view, cx));
    }

    let mut pane = v_flex()
        .size_full()
        .min_h_0()
        .child(render_review_chrome(tab_bar, toggle_icon, cx))
        .child(body);
    if dock == MetadataDock::Folded {
        pane = pane.child(render_metadata_strip(view, cx));
    }

    pane.into_any_element()
}

fn render_review_chrome(
    tab_bar: TabBar,
    toggle_icon: IconName,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .px_2()
        .gap_2()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            Button::new("review-back")
                .ghost()
                .small()
                .icon(IconName::ArrowLeft)
                .label("Pull requests")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dispatch(Intent::BackToPullRequests, cx);
                })),
        )
        .child(div().flex_1().min_w_0().child(tab_bar))
        .child(
            Button::new("metadata-chrome-toggle")
                .ghost()
                .small()
                .icon(toggle_icon)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dispatch(Intent::ToggleMetadataDock, cx);
                })),
        )
}

fn render_empty_tab(
    title: &'static str,
    message: &'static str,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    v_flex()
        .size_full()
        .min_h_0()
        .items_center()
        .justify_center()
        .gap_1()
        .child(div().text_sm().font_medium().child(title))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(message),
        )
        .into_any_element()
}
