use reviewdesk_core::{DraftFilter, PullRequestListOptions};

/// Build GitHub search `q` string(s) for the Pull requests list.
///
/// GitHub's issue search returns **zero** hits for
/// `review-requested:@me OR author:@me` (with or without parentheses), even
/// though each side alone matches. When authored rows are requested, emit two
/// queries so the Provider can merge.
pub fn pull_request_search_queries(options: &PullRequestListOptions) -> Vec<String> {
    let mut queries = vec![compose("review-requested:@me", options.drafts)];
    if options.include_authored {
        queries.push(compose("author:@me", options.drafts));
    }
    queries
}

/// Single-query form (review-requested only). Prefer [`pull_request_search_queries`]
/// when `include_authored` may be set.
pub fn pull_request_search_query(options: &PullRequestListOptions) -> String {
    pull_request_search_queries(options)
        .into_iter()
        .next()
        .expect("at least review-requested query")
}

fn compose(scope: &str, drafts: DraftFilter) -> String {
    let mut parts: Vec<&str> = vec!["is:open", "is:pr", scope];
    match drafts {
        DraftFilter::ReadyOnly => parts.push("draft:false"),
        DraftFilter::DraftsOnly => parts.push("draft:true"),
        DraftFilter::All => {}
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use reviewdesk_core::{DraftFilter, PullRequestListOptions};

    #[test]
    fn default_options_are_requested_reviewer_ready_only() {
        let q = pull_request_search_queries(&PullRequestListOptions::default());
        assert_eq!(q, vec!["is:open is:pr review-requested:@me draft:false"]);
    }

    #[test]
    fn include_authored_emits_separate_author_query() {
        let options = PullRequestListOptions {
            include_authored: true,
            drafts: DraftFilter::ReadyOnly,
        };
        assert_eq!(
            pull_request_search_queries(&options),
            vec![
                "is:open is:pr review-requested:@me draft:false",
                "is:open is:pr author:@me draft:false",
            ]
        );
    }

    #[test]
    fn drafts_only_uses_draft_true() {
        let options = PullRequestListOptions {
            include_authored: false,
            drafts: DraftFilter::DraftsOnly,
        };
        assert_eq!(
            pull_request_search_queries(&options),
            vec!["is:open is:pr review-requested:@me draft:true"]
        );
    }

    #[test]
    fn all_drafts_omits_draft_qualifier() {
        let options = PullRequestListOptions {
            include_authored: false,
            drafts: DraftFilter::All,
        };
        assert_eq!(
            pull_request_search_queries(&options),
            vec!["is:open is:pr review-requested:@me"]
        );
    }
}
