//! PROTOTYPE (throwaway): Pipelines tab layouts for Review Desk.
//!
//! Question: how should Check rollup, newest-first Pipelines with nested Jobs,
//! required-vs-optional urgency, Actions Re-run vs display-only foreign App
//! checks, and empty ≠ green read in the Review pane? Merge/auto-merge stay on
//! Metadata; Metadata only shows a compact Check rollup that activates this tab.
//!
//! Visual language locked to the Linear Issues clone on
//! `prototype/pull-requests-list-nav`: dense single-line rows, nest indent,
//! collapsible group headers, quiet Filter/Display chrome, soft selection wash.
//! Switcher cycles layout variants; `,` / `.` cycle stub scenarios.
//!
//! HITL pick: B (Run history). Left = each Pipeline run (push / MR iteration);
//! detail = A's nested UI grouped by stage (GitLab) or job group (Actions).
//! Metadata: compact "Pipelines" property with status; click opens the tab.
//! A / C remain switchable for comparison only.
//!
//! Run: `cargo run -p reviewdesk`
//! Branch: `prototype/pipelines-tab` (never merge as-is)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipelinesVariant {
    #[default]
    A,
    B,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StubScenario {
    #[default]
    FailingRequired,
    PendingRequired,
    OptionalOnlyFail,
    AllGreen,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollupState {
    Success,
    Pending,
    Failure,
    /// No checks at all — neutral, never painted as success.
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Success,
    Pending,
    Failure,
    /// Stage/job not started (e.g. blocked by an earlier failure).
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineKind {
    WorkflowRun,
    CheckSuite,
    StatusContext,
}

#[derive(Debug, Clone)]
pub struct StubJob {
    pub name: &'static str,
    pub state: JobState,
    pub required: bool,
    pub duration: &'static str,
    /// Actions-backed jobs may offer Re-run; foreign App checks do not.
    pub can_rerun: bool,
    /// GitLab stage name, or Actions grouping label. `None` = ungrouped.
    pub stage: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct StubPipeline {
    /// Product name (workflow / pipeline definition), e.g. "CI".
    pub name: &'static str,
    /// One run id for the left nav: GitLab pipeline iid, Actions run number, SHA.
    pub run_label: &'static str,
    pub kind: PipelineKind,
    pub created: &'static str,
    pub state: JobState,
    pub jobs: &'static [StubJob],
    pub can_rerun: bool,
}

#[derive(Debug, Clone)]
pub struct StubPipelinesData {
    pub rollup: RollupState,
    pub rollup_label: &'static str,
    pub pipelines: &'static [StubPipeline],
    pub empty_copy: Option<&'static str>,
}

impl PipelinesVariant {
    pub fn key(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::A => "Issues list",
            Self::B => "Picked · Run history",
            Self::C => "Group by status",
        }
    }

    pub fn label(self) -> String {
        format!("{} ({})", self.key(), self.name())
    }

    pub fn next(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::C,
            Self::C => Self::A,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::A => Self::C,
            Self::B => Self::A,
            Self::C => Self::B,
        }
    }
}

impl StubScenario {
    pub fn label(self) -> &'static str {
        match self {
            Self::FailingRequired => "Failing required",
            Self::PendingRequired => "Pending required",
            Self::OptionalOnlyFail => "Optional-only fail",
            Self::AllGreen => "All green",
            Self::Empty => "Empty (no checks)",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::FailingRequired => Self::PendingRequired,
            Self::PendingRequired => Self::OptionalOnlyFail,
            Self::OptionalOnlyFail => Self::AllGreen,
            Self::AllGreen => Self::Empty,
            Self::Empty => Self::FailingRequired,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::FailingRequired => Self::Empty,
            Self::PendingRequired => Self::FailingRequired,
            Self::OptionalOnlyFail => Self::PendingRequired,
            Self::AllGreen => Self::OptionalOnlyFail,
            Self::Empty => Self::AllGreen,
        }
    }

    pub fn data(self) -> StubPipelinesData {
        match self {
            Self::FailingRequired => StubPipelinesData {
                rollup: RollupState::Failure,
                rollup_label: "Checks failing · 1 required",
                pipelines: FAILING_REQUIRED,
                empty_copy: None,
            },
            Self::PendingRequired => StubPipelinesData {
                rollup: RollupState::Pending,
                rollup_label: "Checks pending · required still running",
                pipelines: PENDING_REQUIRED,
                empty_copy: None,
            },
            Self::OptionalOnlyFail => StubPipelinesData {
                rollup: RollupState::Success,
                rollup_label: "Required checks passed · 1 optional failed",
                pipelines: OPTIONAL_ONLY_FAIL,
                empty_copy: None,
            },
            Self::AllGreen => StubPipelinesData {
                rollup: RollupState::Success,
                rollup_label: "All checks passed",
                pipelines: ALL_GREEN,
                empty_copy: None,
            },
            Self::Empty => StubPipelinesData {
                rollup: RollupState::Absent,
                rollup_label: "No checks yet",
                pipelines: &[],
                empty_copy: Some(
                    "No Pipelines for this head. Empty is not a pass — nothing reported yet.",
                ),
            },
        }
    }
}

impl PipelineKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::WorkflowRun => "workflow run",
            Self::CheckSuite => "check suite",
            Self::StatusContext => "status",
        }
    }
}

impl JobState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Success => "passed",
            Self::Pending => "pending",
            Self::Failure => "failed",
            Self::Skipped => "skipped",
        }
    }
}

impl RollupState {
    pub fn short(self) -> &'static str {
        match self {
            Self::Success => "passed",
            Self::Pending => "pending",
            Self::Failure => "failing",
            Self::Absent => "none",
        }
    }
}

// --- stub fixtures (newest Pipeline first = newest run / push) ---
//
// FAILING_REQUIRED models the common case: one CI definition, three runs as
// the MR/PR iterates (GitLab pipeline iids / Actions run numbers), plus a
// foreign App suite. Jobs carry `stage` so B's detail can nest like GitLab
// stages (or Actions job groups).

static FAILING_REQUIRED: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
        run_label: "#1842",
        kind: PipelineKind::WorkflowRun,
        created: "2m ago",
        state: JobState::Failure,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "compile",
                state: JobState::Success,
                required: true,
                duration: "48s",
                can_rerun: true,
                stage: Some("build"),
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "24s",
                can_rerun: true,
                stage: Some("test"),
            },
            StubJob {
                name: "unit",
                state: JobState::Failure,
                required: true,
                duration: "1m 12s",
                can_rerun: true,
                stage: Some("test"),
            },
            StubJob {
                name: "pages",
                state: JobState::Skipped,
                required: false,
                duration: "—",
                can_rerun: false,
                stage: Some("deploy"),
            },
        ],
    },
    StubPipeline {
        name: "CI",
        run_label: "#1841",
        kind: PipelineKind::WorkflowRun,
        created: "45m ago",
        state: JobState::Success,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "compile",
                state: JobState::Success,
                required: true,
                duration: "44s",
                can_rerun: true,
                stage: Some("build"),
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "21s",
                can_rerun: true,
                stage: Some("test"),
            },
            StubJob {
                name: "unit",
                state: JobState::Success,
                required: true,
                duration: "58s",
                can_rerun: true,
                stage: Some("test"),
            },
            StubJob {
                name: "pages",
                state: JobState::Success,
                required: false,
                duration: "33s",
                can_rerun: true,
                stage: Some("deploy"),
            },
        ],
    },
    StubPipeline {
        name: "CI",
        run_label: "#1840",
        kind: PipelineKind::WorkflowRun,
        created: "2h ago",
        state: JobState::Failure,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "compile",
                state: JobState::Failure,
                required: true,
                duration: "12s",
                can_rerun: true,
                stage: Some("build"),
            },
            StubJob {
                name: "lint",
                state: JobState::Skipped,
                required: true,
                duration: "—",
                can_rerun: false,
                stage: Some("test"),
            },
        ],
    },
    StubPipeline {
        name: "Codecov",
        run_label: "suite",
        kind: PipelineKind::CheckSuite,
        created: "3m ago",
        state: JobState::Success,
        can_rerun: false,
        jobs: &[StubJob {
            name: "patch",
            state: JobState::Success,
            required: false,
            duration: "12s",
            can_rerun: false,
            stage: None,
        }],
    },
];

static PENDING_REQUIRED: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
        run_label: "#1843",
        kind: PipelineKind::WorkflowRun,
        created: "30s ago",
        state: JobState::Pending,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "compile",
                state: JobState::Success,
                required: true,
                duration: "40s",
                can_rerun: true,
                stage: Some("build"),
            },
            StubJob {
                name: "unit",
                state: JobState::Pending,
                required: true,
                duration: "…",
                can_rerun: true,
                stage: Some("test"),
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "22s",
                can_rerun: true,
                stage: Some("test"),
            },
        ],
    },
    StubPipeline {
        name: "CI",
        run_label: "#1842",
        kind: PipelineKind::WorkflowRun,
        created: "20m ago",
        state: JobState::Success,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "compile",
                state: JobState::Success,
                required: true,
                duration: "41s",
                can_rerun: true,
                stage: Some("build"),
            },
            StubJob {
                name: "unit",
                state: JobState::Success,
                required: true,
                duration: "55s",
                can_rerun: true,
                stage: Some("test"),
            },
        ],
    },
    StubPipeline {
        name: "deploy-preview",
        run_label: "#902",
        kind: PipelineKind::WorkflowRun,
        created: "1m ago",
        state: JobState::Success,
        can_rerun: true,
        jobs: &[StubJob {
            name: "vercel",
            state: JobState::Success,
            required: false,
            duration: "41s",
            can_rerun: true,
            stage: None,
        }],
    },
];

static OPTIONAL_ONLY_FAIL: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
        run_label: "#1901",
        kind: PipelineKind::WorkflowRun,
        created: "5m ago",
        state: JobState::Success,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "test",
                state: JobState::Success,
                required: true,
                duration: "58s",
                can_rerun: true,
                stage: Some("checks"),
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "19s",
                can_rerun: true,
                stage: Some("checks"),
            },
        ],
    },
    StubPipeline {
        name: "CodeQL",
        run_label: "suite",
        kind: PipelineKind::CheckSuite,
        created: "6m ago",
        state: JobState::Failure,
        can_rerun: false,
        jobs: &[StubJob {
            name: "Analyze",
            state: JobState::Failure,
            required: false,
            duration: "2m 01s",
            can_rerun: false,
            stage: None,
        }],
    },
];

static ALL_GREEN: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
        run_label: "#2004",
        kind: PipelineKind::WorkflowRun,
        created: "8m ago",
        state: JobState::Success,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "test",
                state: JobState::Success,
                required: true,
                duration: "1m 04s",
                can_rerun: true,
                stage: Some("test"),
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "21s",
                can_rerun: true,
                stage: Some("test"),
            },
        ],
    },
    StubPipeline {
        name: "CI",
        run_label: "#2003",
        kind: PipelineKind::WorkflowRun,
        created: "1h ago",
        state: JobState::Success,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "test",
                state: JobState::Success,
                required: true,
                duration: "59s",
                can_rerun: true,
                stage: Some("test"),
            },
        ],
    },
    StubPipeline {
        name: "Codecov",
        run_label: "suite",
        kind: PipelineKind::CheckSuite,
        created: "9m ago",
        state: JobState::Success,
        can_rerun: false,
        jobs: &[StubJob {
            name: "patch",
            state: JobState::Success,
            required: false,
            duration: "9s",
            can_rerun: false,
            stage: None,
        }],
    },
];
