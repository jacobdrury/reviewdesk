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
The list of open pull requests the reviewer should act on. For the first slice: pull requests on github.com where the user is a requested reviewer (and optionally the author).
_Avoid_: queue, todo list, feed, notification center

**Pull request**:
A proposed change set on a provider that can be listed in the Inbox and opened for review.
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
