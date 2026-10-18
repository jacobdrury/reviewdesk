use reviewdesk_core::{DraftFilter, InboxOptions};

/// Build the GitHub search `q` string for Inbox listing.
pub fn inbox_search_query(options: &InboxOptions) -> String {
    let mut parts: Vec<&str> = vec!["is:open", "is:pr"];

    let scope = if options.include_authored {
        "(review-requested:@me OR author:@me)"
    } else {
        "review-requested:@me"
    };
    parts.push(scope);

    match options.drafts {
        DraftFilter::ReadyOnly => parts.push("draft:false"),
        DraftFilter::DraftsOnly => parts.push("draft:true"),
        DraftFilter::All => {}
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use reviewdesk_core::{DraftFilter, InboxOptions};

    #[test]
    fn default_options_are_requested_reviewer_ready_only() {
        let q = inbox_search_query(&InboxOptions::default());
        assert_eq!(q, "is:open is:pr review-requested:@me draft:false");
    }

    #[test]
    fn include_authored_adds_or_author() {
        let options = InboxOptions {
            include_authored: true,
            drafts: DraftFilter::ReadyOnly,
        };
        let q = inbox_search_query(&options);
        assert_eq!(
            q,
            "is:open is:pr (review-requested:@me OR author:@me) draft:false"
        );
    }

    #[test]
    fn drafts_only_uses_draft_true() {
        let options = InboxOptions {
            include_authored: false,
            drafts: DraftFilter::DraftsOnly,
        };
        assert_eq!(
            inbox_search_query(&options),
            "is:open is:pr review-requested:@me draft:true"
        );
    }

    #[test]
    fn all_drafts_omits_draft_qualifier() {
        let options = InboxOptions {
            include_authored: false,
            drafts: DraftFilter::All,
        };
        assert_eq!(
            inbox_search_query(&options),
            "is:open is:pr review-requested:@me"
        );
    }
}
