# Review Desk

Desktop client for managing and doing code reviews across providers, starting with GitHub.

## Language

**Review Desk**:
The product: a desktop app for reviewing code changes from a Provider.
_Avoid_: Diff Desk, the app, the client

**Provider**:
A host that exposes pull requests and their patches. The first provider is github.com.
_Avoid_: forge, backend, integration, SCM, GitLab (as the default; GitLab is a later provider)

**Inbox**:
The notifications home: Provider events the reviewer should triage (review requests, comments, mentions, CI and merge state changes on watched pull requests).
_Avoid_: queue, todo list, feed, PR list (that surface is Pull requests)

**Pull requests**:
The list surface of pull requests, with optional Views for filter, group, and sort. Distinct from Inbox.
_Avoid_: Inbox (for this surface), queue, browse

**View**:
A named, local-only saved query over Pull requests: filters, optional group-by, and sort. Unsaved edits are a working query until saved or discarded. One built-in View, **All PRs**, cannot be deleted or renamed; other Views may be seeds or user-created.
_Avoid_: filter preset, saved search, smart list, section

**Pull request**:
A proposed change set on a Provider that can appear in Pull requests (and in Inbox as notification subjects) and open in the Review pane.
_Avoid_: merge request (except when speaking about GitLab), MR, PR as a type name in domain prose (abbreviation in UI copy is fine)

**Patch**:
The textual diff of a pull request, loaded for display on the Diff surface.
_Avoid_: diff (as the data), changeset file list alone

**Review pane**:
The surface for a selected pull request: Overview, peer tabs Pipelines / Conversation / Changes, and the Metadata panel.
_Avoid_: detail pane, main pane, content area

**Overview**:
The Review pane surface for the pull request's title, description, author and web URL chrome, and the CODEOWNERS rule breakdown after the description. Not the Metadata panel or Diff.
_Avoid_: details tab, PR header, sidebar

**Metadata panel**:
A foldable, hideable Review pane panel (open by default on Overview) for pull-request status and mutations: labels, assignees, reviewers, draft↔ready, branches, review-gate summary, merge and auto-merge, plus a compact Check rollup that activates Pipelines. Folds to a status strip; hide restores only from Review pane chrome.
_Avoid_: sidebar, details pane, PR header, properties panel

**Pipelines**:
The Review pane tab that shows the Check rollup and a newest-first list of Pipelines for the pull request head. Not a workflow-file browser, in-app job-log viewer, or merge box.
_Avoid_: Checks tab, CI tab, Actions tab, status checks panel

**Pipeline**:
One CI execution for the pull request head, shaped like a GitLab pipeline: a GitHub Actions workflow run when available; otherwise a third-party check suite; otherwise a lone legacy status context. Sorted newest-first by Provider creation time. Nested units are Jobs. Stages are out of scope for v1.
_Avoid_: workflow run (as the product name), check suite (as the product name), build, CI run

**Job**:
A unit of work inside a Pipeline: an Actions job / check run, or the single context of a legacy status Pipeline. May offer Re-run only when the Provider can perform it for this identity (typically Actions); foreign App checks are display-only.
_Avoid_: step, check (as the product name for the nested row), task

**Check rollup**:
The combined status-check state for the pull request head (success, pending, failure, or error). Required checks drive urgency: required failure → failure; required pending → pending even if optional Jobs are green; optional-only failures stay visible but do not flip the rollup to failure when all required contexts succeed. Shown as compact chrome when Pipelines is not focused (activates the tab) and as the summary at the top of Pipelines when it is. Empty (no checks) is neutral/absent, not success.
_Avoid_: CI badge, combined status (as the product name), build status

**Diff surface**:
The UI that shows a Patch and accepts Annotations. Lives under the Changes tab of the Review pane. Read-only for provider writeback in the first slice.
_Avoid_: editor, diff view (as the domain name)

**Annotation**:
A review remark anchored to a location on the Diff surface. In the first slice it exists only in memory.
_Avoid_: comment (when meaning the in-app stub), suggestion, note
