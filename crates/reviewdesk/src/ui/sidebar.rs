use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, h_flex, v_flex,
};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, WeakEntity, div, prelude::FluentBuilder as _, px,
};

use crate::app::ReviewDesk;
use crate::review_session::{Intent, SessionView, SidebarTab};

const SIDEBAR_WIDTH: f32 = 244.;

pub(crate) fn render_sidebar(
    view: &SessionView,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let pane = &view.pull_requests_pane;
    let global_prs = pane.repo_scope.is_none();
    let repos = pane.repositories.clone();

    let mut side = v_flex()
        .w(px(SIDEBAR_WIDTH))
        .flex_shrink_0()
        .h_full()
        .min_h_0()
        .px_2()
        .py_2()
        .gap_0()
        .border_r_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().sidebar)
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .px_2()
                .py(px(6.))
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .truncate()
                        .child("Review Desk"),
                ),
        )
        .child(div().h(px(6.)))
        .child(side_nav_item(
            SideNavItemProps {
                id: SharedString::from("nav-inbox"),
                icon: IconName::Inbox,
                label: "Inbox",
                selected: view.sidebar_tab == SidebarTab::Inbox,
                badge: None,
                target: NavTarget::Inbox,
                entity: entity.clone(),
            },
            cx,
        ))
        .child(side_nav_item(
            SideNavItemProps {
                id: SharedString::from("nav-prs"),
                icon: IconName::Github,
                label: "Pull requests",
                selected: view.sidebar_tab == SidebarTab::PullRequests && global_prs,
                badge: None,
                target: NavTarget::AllRepos,
                entity: entity.clone(),
            },
            cx,
        ))
        .child(div().h(px(10.)))
        .child(side_section("Your repositories", cx));

    for repo in repos.iter() {
        let repo = repo.clone();
        let selected =
            view.sidebar_tab == SidebarTab::PullRequests && pane.repo_scope.as_ref() == Some(&repo);
        let label = short_repo(&repo);
        side = side.child(side_nav_item(
            SideNavItemProps {
                id: SharedString::from(format!("repo-{repo}")),
                icon: IconName::Github,
                label: &label,
                selected,
                badge: None,
                target: NavTarget::Repo(repo),
                entity: entity.clone(),
            },
            cx,
        ));
    }

    side
}

fn side_section(label: &str, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    div()
        .px_2()
        .py(px(4.))
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(label.to_string())
}

#[derive(Clone)]
enum NavTarget {
    Inbox,
    AllRepos,
    Repo(String),
}

struct SideNavItemProps<'a> {
    id: SharedString,
    icon: IconName,
    label: &'a str,
    selected: bool,
    badge: Option<usize>,
    target: NavTarget,
    entity: WeakEntity<ReviewDesk>,
}

fn side_nav_item(props: SideNavItemProps<'_>, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let SideNavItemProps {
        id,
        icon,
        label,
        selected,
        badge,
        target,
        entity,
    } = props;
    let label = label.to_string();
    let selected_bg = cx.theme().secondary.opacity(0.55);
    let hover_bg = cx.theme().secondary.opacity(0.35);
    let label_color = cx.theme().sidebar_foreground;
    let muted = cx.theme().muted_foreground;

    div()
        .id(id)
        .w_full()
        .px_2()
        .py(px(5.))
        .rounded_md()
        .cursor_pointer()
        .when(selected, |el| el.bg(selected_bg))
        .when(!selected, |el| el.hover(move |style| style.bg(hover_bg)))
        .on_click({
            let target = target.clone();
            move |_, _, cx| {
                let _ = entity.update(cx, |this, cx| match &target {
                    NavTarget::Inbox => {
                        this.dispatch(Intent::SelectSidebarTab(SidebarTab::Inbox), cx);
                    }
                    NavTarget::AllRepos => {
                        this.dispatch(Intent::SelectSidebarTab(SidebarTab::PullRequests), cx);
                        this.dispatch(Intent::SetRepoScope { repo: None }, cx);
                    }
                    NavTarget::Repo(repo) => {
                        this.dispatch(Intent::SelectSidebarTab(SidebarTab::PullRequests), cx);
                        this.dispatch(
                            Intent::SetRepoScope {
                                repo: Some(repo.clone()),
                            },
                            cx,
                        );
                    }
                });
            }
        })
        .child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(Icon::new(icon).xsmall().text_color(if selected {
                    label_color
                } else {
                    muted
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .truncate()
                        .text_color(label_color)
                        .child(label),
                )
                .when_some(badge.filter(|n| *n > 0), |el, count| {
                    el.child(div().text_xs().text_color(muted).child(count.to_string()))
                }),
        )
}

fn short_repo(full: &str) -> String {
    full.rsplit_once('/')
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| full.to_string())
}
