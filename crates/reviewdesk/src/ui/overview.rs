use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    spinner::Spinner,
    text::{MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView, markdown_ast},
    v_flex,
};
use gpui_kit::{
    App, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, WeakEntity, Window, div, percentage,
};

use crate::app::ReviewDesk;
use crate::overview_body::{DetailsFence, parse_details_fence};
use crate::review_session::{Intent, LoadState, SessionView};

pub(crate) fn render_overview(
    view: &SessionView,
    entity: WeakEntity<ReviewDesk>,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    if let Some(markdown) = &view.overview_markdown {
        let owner = entity.clone();
        return div().size_full().min_h_0().p_4().child(
            TextView::markdown("pr-overview", markdown.clone())
                .scrollable(true)
                .text_sm()
                .size_full()
                .plugin(DetailsTogglePlugin { owner }),
        )
        .into_any_element();
    }

    match &view.detail {
        LoadState::Idle | LoadState::Loading => v_flex()
            .size_full()
            .min_h_0()
            .items_center()
            .justify_center()
            .gap_2()
            .child(Spinner::new().small())
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Loading overview…"),
            )
            .into_any_element(),
        LoadState::Error(error) => v_flex()
            .size_full()
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
                    .child(error.to_string()),
            )
            .child(
                Button::new("overview-retry")
                    .outline()
                    .small()
                    .label("Retry")
                    .on_click({
                        let owner = entity.clone();
                        move |_, _, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                this.dispatch(Intent::RetryDetail, cx);
                            });
                        }
                    }),
            )
            .into_any_element(),
        LoadState::Ready(_) => {
            // Markdown should already be projected when detail is Ready.
            v_flex()
                .size_full()
                .min_h_0()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading overview…"),
                )
                .into_any_element()
        }
    }
}

struct DetailsTogglePlugin {
    owner: WeakEntity<ReviewDesk>,
}

impl MarkdownPlugin for DetailsTogglePlugin {
    fn is_block(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "reviewdesk-details"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        _: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        let markdown_ast::Node::Code(code) = node else {
            return None;
        };
        let fence = parse_details_fence(code.lang.as_deref(), code.meta.as_deref(), &code.value)?;
        Some(
            MarkdownNode::new("reviewdesk-details", fence.clone())
                .text(fence.summary.clone())
                .markdown(fence.summary),
        )
    }

    fn render(&self, node: &MarkdownNode, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let fence = node
            .data::<DetailsFence>()
            .cloned()
            .expect("reviewdesk-details node data");
        let owner = self.owner.clone();
        let id = fence.id;
        let open = fence.open;
        div()
            .id(("pr-details-toggle", id))
            .h_flex()
            .w_full()
            .items_center()
            .gap_1()
            .py_1()
            .cursor_pointer()
            .rounded_sm()
            .hover(|style| style.bg(cx.theme().secondary_hover))
            .on_click(move |_, _, cx| {
                let _ = owner.update(cx, |this, cx| {
                    this.dispatch(
                        Intent::ToggleDetails {
                            id,
                            currently_open: open,
                        },
                        cx,
                    );
                });
            })
            .child(
                Icon::new(IconName::ChevronRight)
                    .xsmall()
                    .text_color(cx.theme().muted_foreground)
                    .rotate(percentage(if open { 0.25 } else { 0. })),
            )
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .min_w_0()
                    .child(fence.summary.clone()),
            )
    }
}
