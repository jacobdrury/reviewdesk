//! Metadata panel fields, review-gate readouts, and write capability flags.

/// Someone assigned to the pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignee {
    pub login: String,
    pub avatar_url: Option<String>,
}

/// Provider `reviewDecision` (read-only in Metadata).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    ReviewRequired,
    #[default]
    Unknown,
}

/// Whether the pull request can merge (read-only in Metadata).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeableState {
    Mergeable,
    Conflicting,
    Blocked,
    #[default]
    Unknown,
}

/// One CODEOWNERS rule with paths on this pull request that matched it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeownersRule {
    pub pattern: String,
    pub owners: Vec<String>,
    pub matched_paths: Vec<String>,
}

/// Which Metadata manage writes the active Provider identity supports for a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MetadataWriteCapabilities {
    pub labels: bool,
    pub assignees: bool,
    pub reviewers: bool,
    pub draft_ready: bool,
    pub base_ref: bool,
}
