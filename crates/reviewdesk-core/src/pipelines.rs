//! Check rollup and Pipelines domain types for the pull request head.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Combined status-check state for the pull request head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckRollupState {
    /// No checks reported — neutral, not success.
    Absent,
    Success,
    Pending,
    Failure,
    Error,
}

/// How a Pipeline is shaped on the Provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineKind {
    WorkflowRun,
    CheckSuite,
    StatusContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    Success,
    Pending,
    Failure,
    Skipped,
    Error,
}

/// Provider target for an Actions-backed re-run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RerunTarget {
    WorkflowRun { run_id: u64 },
    WorkflowJob { job_id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RerunAction {
    Allowed(RerunTarget),
    Gated { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub state: JobState,
    pub required: bool,
    pub rerun: RerunAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub run_label: String,
    pub kind: PipelineKind,
    pub created_at: DateTime<Utc>,
    pub jobs: Vec<Job>,
    pub rerun: RerunAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRollup {
    pub state: CheckRollupState,
    /// Human-readable summary for chrome (e.g. "Checks failing · 1 required").
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelinesSnapshot {
    pub rollup: CheckRollup,
    pub pipelines: Vec<Pipeline>,
}

impl PipelinesSnapshot {
    /// Newest-first by [`Pipeline::created_at`].
    pub fn sorted_newest_first(mut self) -> Self {
        self.pipelines
            .sort_by_key(|a| std::cmp::Reverse(a.created_at));
        self
    }
}

/// Required-driven rollup: empty is absent; optional-only failures do not flip
/// failure when all required jobs succeed.
pub fn compute_check_rollup(jobs: &[&Job]) -> CheckRollup {
    if jobs.is_empty() {
        return CheckRollup {
            state: CheckRollupState::Absent,
            label: "No checks yet".into(),
        };
    }

    let required: Vec<_> = jobs.iter().copied().filter(|j| j.required).collect();
    let scope = if required.is_empty() { jobs } else { &required };

    let any_required = !required.is_empty();
    let mut required_failed = 0usize;
    let mut required_pending = 0usize;
    let mut optional_failed = 0usize;
    let mut any_error = false;

    for job in jobs {
        match job.state {
            JobState::Error => any_error = true,
            JobState::Failure if job.required => required_failed += 1,
            JobState::Failure => optional_failed += 1,
            JobState::Pending if job.required => required_pending += 1,
            _ => {}
        }
    }

    if any_error {
        return CheckRollup {
            state: CheckRollupState::Error,
            label: "Checks errored".into(),
        };
    }

    if required_failed > 0 {
        let label = if any_required {
            format!("Checks failing · {required_failed} required")
        } else {
            "Checks failing".into()
        };
        return CheckRollup {
            state: CheckRollupState::Failure,
            label,
        };
    }

    if required_pending > 0 {
        return CheckRollup {
            state: CheckRollupState::Pending,
            label: "Checks pending · required still running".into(),
        };
    }

    let all_success = scope.iter().all(|j| j.state == JobState::Success);
    if all_success {
        let label = if optional_failed > 0 {
            format!("Required checks passed · {optional_failed} optional failed")
        } else {
            "All checks passed".into()
        };
        return CheckRollup {
            state: CheckRollupState::Success,
            label,
        };
    }

    if scope.iter().any(|j| j.state == JobState::Pending) {
        return CheckRollup {
            state: CheckRollupState::Pending,
            label: "Checks pending".into(),
        };
    }

    CheckRollup {
        state: CheckRollupState::Success,
        label: "Required checks passed".into(),
    }
}

pub fn aggregate_pipeline_state(jobs: &[Job]) -> JobState {
    if jobs.is_empty() {
        return JobState::Pending;
    }
    if jobs.iter().any(|j| j.state == JobState::Error) {
        return JobState::Error;
    }
    if jobs
        .iter()
        .any(|j| j.required && j.state == JobState::Failure)
    {
        return JobState::Failure;
    }
    if jobs.iter().any(|j| j.state == JobState::Failure) {
        return JobState::Failure;
    }
    if jobs.iter().any(|j| j.state == JobState::Pending) {
        return JobState::Pending;
    }
    if jobs
        .iter()
        .all(|j| j.state == JobState::Success || j.state == JobState::Skipped)
    {
        return JobState::Success;
    }
    JobState::Pending
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(name: &str, state: JobState, required: bool) -> Job {
        Job {
            id: name.into(),
            name: name.into(),
            state,
            required,
            rerun: RerunAction::Gated {
                reason: "not actions".into(),
            },
        }
    }

    #[test]
    fn empty_checks_are_absent_not_success() {
        let rollup = compute_check_rollup(&[]);
        assert_eq!(rollup.state, CheckRollupState::Absent);
    }

    #[test]
    fn required_failure_drives_rollup_failure() {
        let jobs = [
            job("ci", JobState::Failure, true),
            job("lint", JobState::Success, false),
        ];
        let refs: Vec<_> = jobs.iter().collect();
        let rollup = compute_check_rollup(&refs);
        assert_eq!(rollup.state, CheckRollupState::Failure);
    }

    #[test]
    fn required_pending_beats_optional_success() {
        let jobs = [
            job("ci", JobState::Pending, true),
            job("lint", JobState::Success, false),
        ];
        let refs: Vec<_> = jobs.iter().collect();
        let rollup = compute_check_rollup(&refs);
        assert_eq!(rollup.state, CheckRollupState::Pending);
    }

    #[test]
    fn optional_only_failure_stays_success_when_required_pass() {
        let jobs = [
            job("ci", JobState::Success, true),
            job("optional", JobState::Failure, false),
        ];
        let refs: Vec<_> = jobs.iter().collect();
        let rollup = compute_check_rollup(&refs);
        assert_eq!(rollup.state, CheckRollupState::Success);
        assert!(rollup.label.contains("optional"));
    }
}
