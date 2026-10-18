//! PROTOTYPE (throwaway): Pipelines tab layouts for Review Desk.
//!
//! Question: how should Check rollup, newest-first Pipelines with nested Jobs,
//! required-vs-optional urgency, Actions Re-run vs display-only foreign App
//! checks, and empty ≠ green read in the Review pane? Merge/auto-merge stay on
//! Metadata; Metadata only shows a compact Check rollup that activates this tab.
//!
//! Host shell is the picked primary chrome (Linear sidebar + right Metadata dock
//! + focus Review). Switcher cycles Pipelines layout variants; `,` / `.` cycle
//! stub scenarios.
//!
//! Three variants:
//! - A (Nested list): rollup + expandable Pipeline → Job rows
//! - B (Split panes): Pipeline list | Job detail (Files-like dual scroll)
//! - C (Attention-first): needs-attention band, then denser Pipeline timeline
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
}

#[derive(Debug, Clone)]
pub struct StubPipeline {
    pub name: &'static str,
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
            Self::A => "Nested list",
            Self::B => "Split panes",
            Self::C => "Attention-first",
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
            Self::Neutral => "—",
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

// --- stub fixtures (newest Pipeline first) ---

static FAILING_REQUIRED: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
        kind: PipelineKind::WorkflowRun,
        created: "2m ago",
        state: JobState::Failure,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "test",
                state: JobState::Failure,
                required: true,
                duration: "1m 12s",
                can_rerun: true,
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "24s",
                can_rerun: true,
            },
            StubJob {
                name: "build",
                state: JobState::Success,
                required: false,
                duration: "48s",
                can_rerun: true,
            },
        ],
    },
    StubPipeline {
        name: "Codecov",
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
        }],
    },
    StubPipeline {
        name: "continuous-integration/travis-ci",
        kind: PipelineKind::StatusContext,
        created: "4m ago",
        state: JobState::Success,
        can_rerun: false,
        jobs: &[StubJob {
            name: "continuous-integration/travis-ci",
            state: JobState::Success,
            required: false,
            duration: "—",
            can_rerun: false,
        }],
    },
];

static PENDING_REQUIRED: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
        kind: PipelineKind::WorkflowRun,
        created: "30s ago",
        state: JobState::Pending,
        can_rerun: true,
        jobs: &[
            StubJob {
                name: "test",
                state: JobState::Pending,
                required: true,
                duration: "…",
                can_rerun: true,
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "22s",
                can_rerun: true,
            },
        ],
    },
    StubPipeline {
        name: "deploy-preview",
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
        }],
    },
];

static OPTIONAL_ONLY_FAIL: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
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
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "19s",
                can_rerun: true,
            },
        ],
    },
    StubPipeline {
        name: "CodeQL",
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
        }],
    },
];

static ALL_GREEN: &[StubPipeline] = &[
    StubPipeline {
        name: "CI",
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
            },
            StubJob {
                name: "lint",
                state: JobState::Success,
                required: true,
                duration: "21s",
                can_rerun: true,
            },
        ],
    },
    StubPipeline {
        name: "Codecov",
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
        }],
    },
];
