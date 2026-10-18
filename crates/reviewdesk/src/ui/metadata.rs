//! Metadata right-dock panel and its folded status+merge strip.
//!
//! Reads [`SessionView`] only; mutations dispatch [`Intent::MetadataWrite`].

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use reviewdesk_core::{MergeMethod, MergeableState, PullRequestState, ReviewDecision};

use crate::app::ReviewDesk;
use crate::metadata_write::MetadataWrite;
use crate::review_session::{Intent, LoadState, SessionView};

/// Full Metadata dock shown on the right when the dock is `Open`.
pub(crate) fn render_metadata_panel(
    view: &SessionView,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let header = h_flex()
        .w_full()
        .px_3()
        .py(px(4.))
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child("METADATA"),
        )
        .child(
            h_flex()
                .gap_1()
                .child(
                    Button::new("metadata-fold")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Minus)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::FoldMetadata, cx);
                        })),
                )
                .child(
                    Button::new("metadata-hide")
                        .ghost()
                        .xsmall()
                        .icon(IconName::EyeOff)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.dispatch(Intent::HideMetadata, cx);
                        })),
                ),
        );

    v_flex()
        .w(px(320.))
        .min_h_0()
        .border_l_1()
        .border_color(cx.theme().border)
        .child(header)
        .child(div().flex_1().min_h_0().child(metadata_body(view, cx)))
}

fn metadata_body(view: &SessionView, cx: &mut Context<ReviewDesk>) -> impl IntoElement {
    let LoadState::Ready(detail) = &view.detail else {
        return v_flex().p_4().child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Metadata loads with the pull request."),
        );
    };

    let provider_ref = detail.id.as_str().to_string();
    let web_url = detail.web_url.clone();
    let caps = &detail.write_capabilities;
    let pending = view.metadata_write_pending;
    let write_error = view.metadata_write_error.clone();

    let assignees = format_logins(detail.assignees.iter().map(|a| a.login.as_str()));
    let mut panel = v_flex()
        .p_4()
        .gap_3()
        .child(metadata_field(
            "Pull request",
            provider_link(provider_ref, web_url, cx),
            cx,
        ))
        .child(metadata_field(
            "Status",
            status_controls(detail.is_draft, caps.draft_ready, pending, cx),
            cx,
        ))
        .child(metadata_field(
            "Branches",
            branch_row(detail, caps.base_ref, pending, cx),
            cx,
        ))
        .child(metadata_field(
            "Review gate",
            read_only_text(review_decision_label(detail.review_decision)),
            cx,
        ))
        .child(metadata_field(
            "Mergeability",
            read_only_text(mergeable_label(detail.mergeable)),
            cx,
        ))
        .child(metadata_field(
            "Merge",
            merge_row(detail, caps.merge, pending, cx),
            cx,
        ))
        .child(metadata_field(
            "Auto-merge",
            auto_merge_row(detail, caps.auto_merge, pending, cx),
            cx,
        ))
        .child(metadata_field(
            "Assignees",
            assignee_row(&assignees, caps.assignees, pending, cx),
            cx,
        ))
        .child(metadata_field(
            "Reviewers",
            reviewers_row(detail, caps.reviewers, pending, cx),
            cx,
        ))
        .child(metadata_field(
            "Labels",
            labels_row(detail, caps.labels, pending, cx),
            cx,
        ));

    if !detail.codeowners_rules.is_empty() {
        let summary = detail
            .codeowners_rules
            .iter()
            .map(|rule| format!("{} ({})", rule.pattern, rule.owners.join(", ")))
            .collect::<Vec<_>>()
            .join("; ");
        panel = panel.child(metadata_field(
            "CODEOWNERS gate",
            read_only_text(summary),
            cx,
        ));
    }

    if pending {
        panel = panel.child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("Saving…"),
        );
    }
    if let Some(error) = write_error {
        panel = panel.child(
            div()
                .text_xs()
                .text_color(cx.theme().danger)
                .child(format!("Write failed: {error}")),
        );
    }

    panel
}

fn metadata_field(
    label: &'static str,
    value: gpui_kit::AnyElement,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(value)
}

fn read_only_text(text: String) -> gpui_kit::AnyElement {
    div().text_sm().child(text).into_any_element()
}

fn format_logins<'a, I>(logins: I) -> String
where
    I: Iterator<Item = &'a str>,
{
    let collected: Vec<_> = logins.collect();
    if collected.is_empty() {
        "None".into()
    } else {
        collected.join(", ")
    }
}

fn display_ref(value: &str) -> String {
    if value.is_empty() {
        "—".into()
    } else {
        value.to_string()
    }
}

fn review_decision_label(value: ReviewDecision) -> String {
    match value {
        ReviewDecision::Approved => "Approved".into(),
        ReviewDecision::ChangesRequested => "Changes requested".into(),
        ReviewDecision::ReviewRequired => "Review required".into(),
        ReviewDecision::Unknown => "Unknown".into(),
    }
}

fn mergeable_label(value: MergeableState) -> String {
    match value {
        MergeableState::Mergeable => "Mergeable".into(),
        MergeableState::Conflicting => "Conflicts".into(),
        MergeableState::Blocked => "Blocked".into(),
        MergeableState::Unknown => "Unknown".into(),
    }
}

fn merge_can_attempt(detail: &reviewdesk_core::PullRequestDetail) -> bool {
    detail.state == PullRequestState::Open
        && !detail.is_draft
        && detail.mergeable == MergeableState::Mergeable
}

fn merge_method_label(method: MergeMethod) -> &'static str {
    match method {
        MergeMethod::Squash => "Squash",
        MergeMethod::Merge => "Merge",
        MergeMethod::Rebase => "Rebase",
    }
}

fn merge_row(
    detail: &reviewdesk_core::PullRequestDetail,
    enabled: bool,
    pending: bool,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    if !enabled {
        return read_only_text("Not supported".into());
    }
    match detail.state {
        PullRequestState::Merged => return read_only_text("Merged".into()),
        PullRequestState::Closed => return read_only_text("Closed".into()),
        PullRequestState::Open => {}
    }
    if detail.is_draft {
        return read_only_text("Draft — mark ready to merge".into());
    }
    if detail.mergeable != MergeableState::Mergeable {
        return read_only_text(format!(
            "Not mergeable ({})",
            mergeable_label(detail.mergeable)
        ));
    }
    let can_merge = merge_can_attempt(detail) && !pending;
    h_flex()
        .gap_1()
        .flex_wrap()
        .children(
            [MergeMethod::Squash, MergeMethod::Merge, MergeMethod::Rebase].map(|method| {
                Button::new(format!("metadata-merge-{method:?}"))
                    .ghost()
                    .xsmall()
                    .disabled(!can_merge)
                    .label(merge_method_label(method))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.dispatch(Intent::MetadataWrite(MetadataWrite::Merge(method)), cx);
                    }))
                    .into_any_element()
            }),
        )
        .into_any_element()
}

fn auto_merge_row(
    detail: &reviewdesk_core::PullRequestDetail,
    enabled: bool,
    pending: bool,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    if !enabled {
        return read_only_text("Not enabled for this repository".into());
    }
    if detail.state != PullRequestState::Open {
        return read_only_text("Unavailable for closed pull requests".into());
    }
    if detail.auto_merge_enabled {
        return h_flex()
            .gap_2()
            .items_center()
            .child(div().text_sm().child("On"))
            .child(
                Button::new("metadata-disable-auto-merge")
                    .ghost()
                    .xsmall()
                    .disabled(pending)
                    .label("Disable")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.dispatch(
                            Intent::MetadataWrite(MetadataWrite::SetAutoMerge {
                                enabled: false,
                                method: MergeMethod::Squash,
                            }),
                            cx,
                        );
                    })),
            )
            .into_any_element();
    }
    h_flex()
        .gap_1()
        .flex_wrap()
        .child(div().text_sm().child("Off"))
        .children(
            [MergeMethod::Squash, MergeMethod::Merge, MergeMethod::Rebase].map(|method| {
                Button::new(format!("metadata-enable-auto-merge-{method:?}"))
                    .ghost()
                    .xsmall()
                    .disabled(pending)
                    .label(format!("Enable ({})", merge_method_label(method)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.dispatch(
                            Intent::MetadataWrite(MetadataWrite::SetAutoMerge {
                                enabled: true,
                                method,
                            }),
                            cx,
                        );
                    }))
                    .into_any_element()
            }),
        )
        .into_any_element()
}

fn status_controls(
    is_draft: bool,
    enabled: bool,
    pending: bool,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    let status = if is_draft { "Draft" } else { "Ready" };
    if !enabled {
        return read_only_text(format!("{status} (read-only)"));
    }
    let target_draft = !is_draft;
    h_flex()
        .gap_2()
        .items_center()
        .child(div().text_sm().child(status))
        .child(
            Button::new("metadata-toggle-draft")
                .ghost()
                .xsmall()
                .disabled(pending)
                .label(if is_draft {
                    "Mark ready"
                } else {
                    "Convert to draft"
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.dispatch(
                        Intent::MetadataWrite(MetadataWrite::SetDraft(target_draft)),
                        cx,
                    );
                })),
        )
        .into_any_element()
}

fn branch_row(
    detail: &reviewdesk_core::PullRequestDetail,
    base_editable: bool,
    pending: bool,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    let head = display_ref(&detail.head_ref);
    let base = display_ref(&detail.base_ref);
    if !base_editable {
        return read_only_text(format!("{head} → {base}"));
    }
    h_flex()
        .gap_2()
        .items_center()
        .child(div().text_sm().child(format!("{head} → {base}")))
        .child(
            Button::new("metadata-set-base-main")
                .ghost()
                .xsmall()
                .disabled(pending)
                .label("Use main")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dispatch(
                        Intent::MetadataWrite(MetadataWrite::SetBaseRef("main".into())),
                        cx,
                    );
                })),
        )
        .into_any_element()
}

fn reviewers_row(
    detail: &reviewdesk_core::PullRequestDetail,
    enabled: bool,
    pending: bool,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    let text = format_logins(detail.reviewers.iter().map(|r| r.login.as_str()));
    if !enabled {
        return read_only_text(format!("{text} (not supported)"));
    }
    let first = detail
        .reviewers
        .first()
        .map(|r| r.login.clone())
        .unwrap_or_else(|| "hubot".into());
    h_flex()
        .gap_2()
        .items_center()
        .child(div().text_sm().child(text))
        .child(
            Button::new("metadata-request-reviewer")
                .ghost()
                .xsmall()
                .disabled(pending)
                .label("Request")
                .on_click({
                    let login = first.clone();
                    cx.listener(move |this, _, _, cx| {
                        this.dispatch(
                            Intent::MetadataWrite(MetadataWrite::RequestReviewer(login.clone())),
                            cx,
                        );
                    })
                }),
        )
        .child(
            Button::new("metadata-remove-reviewer")
                .ghost()
                .xsmall()
                .disabled(pending || detail.reviewers.is_empty())
                .label("Remove")
                .on_click({
                    let login = first.clone();
                    cx.listener(move |this, _, _, cx| {
                        this.dispatch(
                            Intent::MetadataWrite(MetadataWrite::RemoveReviewer(login.clone())),
                            cx,
                        );
                    })
                }),
        )
        .child(
            Button::new("metadata-rerequest-reviewer")
                .ghost()
                .xsmall()
                .disabled(pending || detail.reviewers.is_empty())
                .label("Re-request")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.dispatch(
                        Intent::MetadataWrite(MetadataWrite::RerequestReviewer(first.clone())),
                        cx,
                    );
                })),
        )
        .into_any_element()
}

fn assignee_row(
    assignees: &str,
    enabled: bool,
    pending: bool,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    if !enabled {
        return read_only_text(format!("{assignees} (not supported)"));
    }
    h_flex()
        .gap_2()
        .items_center()
        .child(div().text_sm().child(assignees.to_string()))
        .child(
            Button::new("metadata-clear-assignees")
                .ghost()
                .xsmall()
                .disabled(pending)
                .label("Clear")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dispatch(
                        Intent::MetadataWrite(MetadataWrite::SetAssignees(Vec::new())),
                        cx,
                    );
                })),
        )
        .into_any_element()
}

fn labels_row(
    detail: &reviewdesk_core::PullRequestDetail,
    enabled: bool,
    pending: bool,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    let labels = format_logins(detail.labels.iter().map(|l| l.name.as_str()));
    if !enabled {
        return read_only_text(format!("{labels} (not supported)"));
    }
    let mut next_labels: Vec<String> = detail.labels.iter().map(|l| l.name.clone()).collect();
    if !next_labels.iter().any(|l| l == "bug") {
        next_labels.push("bug".into());
    }
    let write_labels = next_labels.clone();
    h_flex()
        .gap_2()
        .items_center()
        .child(div().text_sm().child(labels))
        .child(
            Button::new("metadata-add-bug-label")
                .ghost()
                .xsmall()
                .disabled(pending)
                .label("+ bug")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.dispatch(
                        Intent::MetadataWrite(MetadataWrite::SetLabels(write_labels.clone())),
                        cx,
                    );
                })),
        )
        .into_any_element()
}

/// Provider link rendered as `owner/repo#n`, opening the Provider web URL.
fn provider_link(
    provider_ref: String,
    web_url: String,
    cx: &mut Context<ReviewDesk>,
) -> gpui_kit::AnyElement {
    h_flex()
        .id("metadata-provider-link")
        .items_center()
        .gap_1()
        .cursor_pointer()
        .rounded_sm()
        .text_color(cx.theme().link)
        .hover(|style| style.text_color(cx.theme().accent))
        .on_click(move |_, _, cx| {
            cx.open_url(&web_url);
        })
        .child(div().text_sm().font_medium().child(provider_ref))
        .child(
            Icon::new(IconName::ExternalLink)
                .xsmall()
                .text_color(cx.theme().muted_foreground),
        )
        .into_any_element()
}

/// Folded footer strip: keeps status + merge visible after a fold.
pub(crate) fn render_metadata_strip(
    view: &SessionView,
    cx: &mut Context<ReviewDesk>,
) -> impl IntoElement {
    let (status, review_gate, mergeable) = match &view.detail {
        LoadState::Ready(detail) => (
            if detail.is_draft { "Draft" } else { "Ready" },
            review_decision_label(detail.review_decision),
            mergeable_label(detail.mergeable),
        ),
        _ => ("…", "…".into(), "…".into()),
    };

    h_flex()
        .w_full()
        .px_3()
        .py(px(4.))
        .items_center()
        .justify_between()
        .border_t_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .gap_3()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child("METADATA"),
                )
                .child(div().text_xs().child(status))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(review_gate),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(mergeable),
                ),
        )
        .child(
            Button::new("metadata-strip-expand")
                .ghost()
                .xsmall()
                .icon(IconName::PanelRightOpen)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.dispatch(Intent::ToggleMetadataDock, cx);
                })),
        )
}
