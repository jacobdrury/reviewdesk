# Review Desk

Desktop client for managing and doing code reviews across providers, starting with GitHub.

## Language

**Review Desk**:
The product: a desktop app for reviewing code changes from a Provider.
_Avoid_: Diff Desk, the app, the client

**Provider**:
A host that exposes pull requests, patches, notifications, and review writeback. The first provider is github.com.
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
The right side of the first-slice shell. It holds Overview and Files for the selected pull request.
_Avoid_: detail pane, main pane, content area

**Overview**:
The Review pane tab that shows pull-request metadata for reading (title, description, reviewers, and related fields). Not the Diff.
_Avoid_: details tab, PR header, metadata panel

**Diff surface**:
The UI that shows a Patch and accepts Annotations. Lives under the Files tab of the Review pane. Read-only for provider writeback in the first slice.
_Avoid_: editor, diff view (as the domain name)

**Annotation**:
A review remark anchored to a location on the Diff surface. In the first slice it exists only in memory.
_Avoid_: comment (when meaning the in-app stub), suggestion, note
