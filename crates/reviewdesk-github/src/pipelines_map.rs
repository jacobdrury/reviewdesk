use std::collections::HashMap;

use chrono::{DateTime, Utc};
use reviewdesk_core::{
    Job, JobState, Pipeline, PipelineKind, PipelinesSnapshot, ProviderError, RerunAction,
    RerunTarget, compute_check_rollup,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelinesData {
    pub repository: Option<PipelinesRepository>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelinesRepository {
    pub pull_request: Option<PipelinesPullRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelinesPullRequest {
    pub status_check_rollup: Option<StatusCheckRollup>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCheckRollup {
    pub contexts: ContextConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextConnection {
    pub nodes: Vec<Option<StatusContextNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
pub enum StatusContextNode {
    CheckRun(CheckRunNode),
    StatusContext(LegacyStatusNode),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CheckRunNode {
    pub database_id: Option<u64>,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>, // reserved for duration display
    pub is_required: Option<bool>,
    pub check_suite: Option<CheckSuiteNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckSuiteNode {
    pub id: String,
    pub app: Option<AppNode>,
    pub workflow_run: Option<WorkflowRunNode>,
}

#[derive(Debug, Deserialize)]
pub struct AppNode {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunNode {
    pub database_id: u64,
    pub run_number: u64,
    pub created_at: DateTime<Utc>,
    pub workflow: Option<WorkflowNode>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowNode {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyStatusNode {
    pub id: String,
    pub context: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub is_required: Option<bool>,
}

struct PipelineBuilder {
    id: String,
    name: String,
    run_label: String,
    kind: PipelineKind,
    created_at: DateTime<Utc>,
    workflow_run_id: Option<u64>,
    jobs: Vec<Job>,
}

pub fn map_pipelines(
    data: PipelinesData,
    pull_number: u64,
) -> Result<PipelinesSnapshot, ProviderError> {
    let pr = data
        .repository
        .and_then(|r| r.pull_request)
        .ok_or(ProviderError::NotFound)?;

    let nodes = pr
        .status_check_rollup
        .map(|r| r.contexts.nodes)
        .unwrap_or_default();

    let mut builders: HashMap<String, PipelineBuilder> = HashMap::new();
    let mut all_jobs: Vec<Job> = Vec::new();

    for node in nodes.into_iter().flatten() {
        match node {
            StatusContextNode::CheckRun(run) => {
                let job = map_check_run_job(&run, pull_number);
                all_jobs.push(job.clone());

                let key = pipeline_key_for_check_run(&run);
                let builder = builders
                    .entry(key.clone())
                    .or_insert_with(|| pipeline_shell_from_check_run(&run, &key));
                builder.jobs.push(job);
            }
            StatusContextNode::StatusContext(status) => {
                let job = map_legacy_job(&status, pull_number);
                all_jobs.push(job.clone());
                let key = format!("status:{}", status.context);
                builders.insert(
                    key.clone(),
                    PipelineBuilder {
                        id: key.clone(),
                        name: status.context.clone(),
                        run_label: status.context.clone(),
                        kind: PipelineKind::StatusContext,
                        created_at: status.created_at,
                        workflow_run_id: None,
                        jobs: vec![job],
                    },
                );
            }
        }
    }

    let job_refs: Vec<&Job> = all_jobs.iter().collect();
    let rollup = compute_check_rollup(&job_refs);

    let mut pipelines: Vec<Pipeline> = builders
        .into_values()
        .map(|b| {
            let rerun = match b.workflow_run_id {
                Some(run_id) => RerunAction::Allowed(RerunTarget::WorkflowRun { run_id }),
                None => RerunAction::Gated {
                    reason: "Re-run is only available for GitHub Actions workflow runs".into(),
                },
            };
            Pipeline {
                id: b.id,
                name: b.name,
                run_label: b.run_label,
                kind: b.kind,
                created_at: b.created_at,
                jobs: b.jobs,
                rerun,
            }
        })
        .collect();

    pipelines.sort_by_key(|a| std::cmp::Reverse(a.created_at));

    Ok(PipelinesSnapshot { rollup, pipelines })
}

fn pipeline_key_for_check_run(run: &CheckRunNode) -> String {
    if let Some(suite) = &run.check_suite {
        if let Some(wr) = &suite.workflow_run {
            return format!("workflow:{}", wr.database_id);
        }
        return format!("suite:{}", suite.id);
    }
    format!(
        "check:{}",
        run.database_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| run.name.clone())
    )
}

fn pipeline_shell_from_check_run(run: &CheckRunNode, key: &str) -> PipelineBuilder {
    if let Some(suite) = &run.check_suite {
        if let Some(wr) = &suite.workflow_run {
            let workflow_name = wr
                .workflow
                .as_ref()
                .map(|w| w.name.clone())
                .unwrap_or_else(|| "Workflow".into());
            return PipelineBuilder {
                id: key.to_string(),
                name: workflow_name,
                run_label: format!("#{}", wr.run_number),
                kind: PipelineKind::WorkflowRun,
                created_at: wr.created_at,
                workflow_run_id: Some(wr.database_id),
                jobs: Vec::new(),
            };
        }
        let app_name = suite
            .app
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Check suite".into());
        return PipelineBuilder {
            id: key.to_string(),
            name: app_name,
            run_label: "suite".into(),
            kind: PipelineKind::CheckSuite,
            created_at: run.started_at.unwrap_or_else(Utc::now),
            workflow_run_id: None,
            jobs: Vec::new(),
        };
    }

    PipelineBuilder {
        id: key.to_string(),
        name: run.name.clone(),
        run_label: run.name.clone(),
        kind: PipelineKind::CheckSuite,
        created_at: run.started_at.unwrap_or_else(Utc::now),
        workflow_run_id: None,
        jobs: Vec::new(),
    }
}

fn map_check_run_job(run: &CheckRunNode, pull_number: u64) -> Job {
    let required = run.is_required.unwrap_or(false);
    let state = check_run_state(&run.status, run.conclusion.as_deref());
    let id = run
        .database_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| run.name.clone());

    let rerun = if let Some(suite) = &run.check_suite {
        if suite.workflow_run.is_some() {
            RerunAction::Gated {
                reason: "Re-run workflow from the pipeline header".into(),
            }
        } else {
            RerunAction::Gated {
                reason: "App checks can only be re-requested by the owning app".into(),
            }
        }
    } else {
        RerunAction::Gated {
            reason: "Re-run not available for this check".into(),
        }
    };

    let _ = pull_number;

    Job {
        id,
        name: run.name.clone(),
        state,
        required,
        rerun,
    }
}

fn map_legacy_job(status: &LegacyStatusNode, pull_number: u64) -> Job {
    let required = status.is_required.unwrap_or(false);
    let state = legacy_status_state(&status.state);
    let _ = pull_number;
    Job {
        id: status.id.clone(),
        name: status.context.clone(),
        state,
        required,
        rerun: RerunAction::Gated {
            reason: "Legacy status contexts cannot be re-run from Review Desk".into(),
        },
    }
}

fn check_run_state(status: &str, conclusion: Option<&str>) -> JobState {
    match status {
        "QUEUED" | "IN_PROGRESS" | "PENDING" | "WAITING" => JobState::Pending,
        "COMPLETED" => match conclusion {
            Some("SUCCESS") | Some("NEUTRAL") | Some("SKIPPED") => JobState::Success,
            Some("FAILURE") => JobState::Failure,
            Some("CANCELLED") => JobState::Skipped,
            Some("TIMED_OUT") | Some("ACTION_REQUIRED") | Some("STALE") => JobState::Error,
            _ => JobState::Pending,
        },
        _ => JobState::Pending,
    }
}

fn legacy_status_state(state: &str) -> JobState {
    match state {
        "SUCCESS" => JobState::Success,
        "PENDING" | "EXPECTED" => JobState::Pending,
        "FAILURE" => JobState::Failure,
        "ERROR" => JobState::Error,
        _ => JobState::Pending,
    }
}
