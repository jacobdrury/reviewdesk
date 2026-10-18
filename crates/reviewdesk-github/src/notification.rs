use chrono::{DateTime, Utc};
use reviewdesk_core::{NotificationReason, NotificationThread, NotificationThreadId};
use serde::Deserialize;

use crate::id::pull_request_id;

/// REST `Thread` payload from `GET /notifications`.
#[derive(Debug, Deserialize)]
pub struct NotificationThreadPayload {
    pub id: String,
    pub reason: String,
    #[serde(default = "default_unread")]
    pub unread: bool,
    pub updated_at: DateTime<Utc>,
    pub subject: SubjectPayload,
    pub repository: RepositoryPayload,
}

fn default_unread() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SubjectPayload {
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryPayload {
    pub full_name: String,
}

/// Maps a REST notification thread to a domain [`NotificationThread`].
///
/// Returns `None` when the subject is not a pull request, the reason is not on
/// the Inbox allowlist, or the subject URL cannot be resolved to a pull request id.
pub fn map_notification_thread(payload: &NotificationThreadPayload) -> Option<NotificationThread> {
    if payload.subject.kind != "PullRequest" {
        return None;
    }
    let reason = NotificationReason::parse(&payload.reason)?;
    let pull_request_id = payload
        .subject
        .url
        .as_deref()
        .and_then(pull_request_id_from_subject_url)?;
    let id = NotificationThreadId::new(payload.id.clone()).ok()?;

    Some(NotificationThread {
        id,
        pull_request_id,
        title: payload.subject.title.clone(),
        repo_label: payload.repository.full_name.clone(),
        reason,
        unread: payload.unread,
        updated_at: payload.updated_at,
    })
}

/// Parses `https://api.github.com/repos/{owner}/{repo}/pulls/{number}` into a
/// pull-request id. Returns `None` for any other subject URL shape.
fn pull_request_id_from_subject_url(url: &str) -> Option<reviewdesk_core::PullRequestId> {
    let after_repos = url.split("/repos/").nth(1)?;
    let mut segments = after_repos.split('/');
    let owner = segments.next()?;
    let repo = segments.next()?;
    let kind = segments.next()?;
    if kind != "pulls" {
        return None;
    }
    let number: u64 = segments.next()?.parse().ok()?;
    pull_request_id(owner, repo, number).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTIFICATIONS_FIXTURE: &str = r#"[
      {
        "id": "1001",
        "reason": "review_requested",
        "unread": true,
        "updated_at": "2026-10-17T12:00:00Z",
        "subject": {
          "title": "Add widgets",
          "type": "PullRequest",
          "url": "https://api.github.com/repos/acme/widgets/pulls/12"
        },
        "repository": { "full_name": "acme/widgets" }
      },
      {
        "id": "1002",
        "reason": "subscribed",
        "unread": true,
        "updated_at": "2026-10-17T13:00:00Z",
        "subject": {
          "title": "Noisy watched PR",
          "type": "PullRequest",
          "url": "https://api.github.com/repos/acme/widgets/pulls/13"
        },
        "repository": { "full_name": "acme/widgets" }
      },
      {
        "id": "1003",
        "reason": "mention",
        "unread": false,
        "updated_at": "2026-10-17T14:00:00Z",
        "subject": {
          "title": "An issue, not a PR",
          "type": "Issue",
          "url": "https://api.github.com/repos/acme/widgets/issues/14"
        },
        "repository": { "full_name": "acme/widgets" }
      }
    ]"#;

    #[test]
    fn maps_pull_request_subject_on_allowlist() {
        let payloads: Vec<NotificationThreadPayload> =
            serde_json::from_str(NOTIFICATIONS_FIXTURE).unwrap();
        let mapped: Vec<_> = payloads
            .iter()
            .filter_map(map_notification_thread)
            .collect();

        assert_eq!(mapped.len(), 1);
        let thread = &mapped[0];
        assert_eq!(thread.id.as_str(), "1001");
        assert_eq!(thread.pull_request_id.as_str(), "acme/widgets#12");
        assert_eq!(thread.title, "Add widgets");
        assert_eq!(thread.repo_label, "acme/widgets");
        assert_eq!(thread.reason, NotificationReason::ReviewRequested);
        assert!(thread.unread);
    }

    #[test]
    fn drops_non_allowlisted_reason() {
        let payloads: Vec<NotificationThreadPayload> =
            serde_json::from_str(NOTIFICATIONS_FIXTURE).unwrap();
        assert!(
            !payloads
                .iter()
                .filter_map(map_notification_thread)
                .any(|thread| thread.id.as_str() == "1002")
        );
    }

    #[test]
    fn drops_non_pull_request_subject() {
        let payloads: Vec<NotificationThreadPayload> =
            serde_json::from_str(NOTIFICATIONS_FIXTURE).unwrap();
        assert!(
            !payloads
                .iter()
                .filter_map(map_notification_thread)
                .any(|thread| thread.id.as_str() == "1003")
        );
    }

    #[test]
    fn subject_url_without_pulls_is_rejected() {
        assert!(
            pull_request_id_from_subject_url("https://api.github.com/repos/acme/widgets/issues/14")
                .is_none()
        );
    }
}
