use clap::{ArgGroup, Parser, Subcommand};
use std::fmt::Write as _;
use std::path::PathBuf;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    ApplicabilityResourceObservationStatus, ApplicabilityResourceStampInput, BranchForkOptions,
    BranchForkResult, BranchHead, BranchId, CanonicalValue, ClaimId, ClaimLifecycleState,
    ClaimMode, ClaimNextOptions, ClaimNextResult, ClaimReleaseOptions, ClaimReleaseResult,
    ClaimTaskOptions, ClaimTaskResult, CommitId, ContextOverview, ContextOverviewOptions, Digest,
    Engine, EntityId, EntityVersionId, HistoryEntry, HistoryQueryOptions, ReplayedState,
    ResourceCreateOptions, ResourceCreateResult, ResourceId, ResourceObservationCreateOptions,
    ResourceObservationCreateResult, ResourceObservationId, Result, RunnableTaskBlockedReason,
    RunnableTaskCandidate, RunnableTaskClaimCoordination, RunnableTasksOptions,
    RunnableTasksProjection, SessionEndOptions, SessionEndResult, SessionId, SessionLifecycleState,
    SessionStartOptions, SessionStartResult, SessionSwitchOptions, SessionSwitchResult,
    StoreInitOptions, TaskCreateCommit, TaskCreateOptions, TaskStatus, TaskTransitionCommit,
    TaskTransitionOptions, VerificationApplicabilityCacheSnapshot,
    VerificationApplicabilityRecordOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationRequirementCreateCommit, VerificationRequirementCreateOptions,
    VerificationResourceBasis, VerificationResult, VerificationTarget, WorkState, WorkVcsError,
    WorkspaceInfo, WorkspaceInitOptions, content_object_digest, parse_canonical_json,
};

#[derive(Debug, Parser)]
#[command(name = "workvcs")]
#[command(about = "WorkVCS v0.1 thin command shell")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long, default_value = "WorkVCS store")]
        display_name: String,
    },
    Doctor {
        #[arg(value_name = "STORE")]
        store: PathBuf,
    },
    #[command(group(
        ArgGroup::new("history-start")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    History {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    ShowAt {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
    },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    Branch {
        #[command(subcommand)]
        command: BranchCommand,
    },
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Ac {
        #[command(subcommand)]
        command: AcceptanceCriterionCommand,
    },
    Vr {
        #[command(subcommand)]
        command: VerificationRequirementCommand,
    },
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Claim {
        #[command(subcommand)]
        command: ClaimCommand,
    },
    Context {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,
    },
    Runnable {
        #[command(subcommand)]
        command: RunnableCommand,
    },
    Verification {
        #[command(subcommand)]
        command: VerificationCommand,
    },
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        display_name: String,

        #[arg(long)]
        initial_branch_name: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum BranchCommand {
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,
    },
    Head {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,
    },
    #[command(group(
        ArgGroup::new("branch-fork-source")
            .required(true)
            .multiple(false)
            .args(["from_branch", "from_commit"])
    ))]
    Fork {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        from_branch: Option<String>,

        #[arg(long)]
        from_commit: Option<String>,

        #[arg(long)]
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        description: String,

        #[arg(long)]
        priority: Option<i64>,
    },
    Transition {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        task: String,

        #[arg(long)]
        task_version: String,

        #[arg(long)]
        status: String,

        #[arg(long)]
        outcome: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum AcceptanceCriterionCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        task: String,

        #[arg(long)]
        task_version: String,

        #[arg(long)]
        local_key: String,

        #[arg(long)]
        statement: String,

        #[arg(long, default_value = "required")]
        classification: String,
    },
    Status {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        criterion: String,
    },
}

#[derive(Debug, Subcommand)]
enum VerificationRequirementCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        criterion: String,

        #[arg(long)]
        criterion_version: String,

        #[arg(long)]
        local_key: String,

        #[arg(long)]
        statement: String,
    },
}

#[derive(Debug, Subcommand)]
enum ResourceCommand {
    Create {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        kind: String,
    },
    #[command(group(
        ArgGroup::new("resource-observation-fingerprint")
            .required(true)
            .multiple(false)
            .args(["fingerprint", "content"])
    ))]
    Observe {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        resource: String,

        #[arg(long)]
        adapter_kind: String,

        #[arg(long)]
        adapter_schema_version: i64,

        #[arg(long)]
        fingerprint: Option<String>,

        #[arg(long)]
        content: Option<String>,

        #[arg(long, default_value = "{}")]
        summary_json: String,
    },
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Start {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        branch: String,

        #[arg(long, default_value = "{}")]
        metadata_json: String,
    },
    Switch {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        workspace: String,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        focus: Option<String>,
    },
    End {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long, default_value = "{}")]
        summary_json: String,
    },
}

#[derive(Debug, Subcommand)]
enum ClaimCommand {
    Next {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,
    },
    Task {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        task: String,
    },
    Release {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,

        #[arg(long)]
        claim: String,
    },
}

#[derive(Debug, Subcommand)]
enum RunnableCommand {
    Tasks {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        session: String,
    },
}

#[derive(Debug, Subcommand)]
enum VerificationCommand {
    #[command(group(
        ArgGroup::new("verification-target")
            .required(true)
            .multiple(false)
            .args(["acceptance_criterion", "verification_requirement"])
    ))]
    Record {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        result: String,

        #[arg(long)]
        method: Option<String>,

        #[arg(long)]
        acceptance_criterion: Option<String>,

        #[arg(long)]
        verification_requirement: Option<String>,

        #[arg(long)]
        resource: Option<String>,

        #[arg(long)]
        adapter_kind: Option<String>,

        #[arg(long)]
        adapter_schema_version: Option<i64>,

        #[arg(long)]
        scope_kind: Option<String>,

        #[arg(long)]
        scope_schema_version: Option<i64>,

        #[arg(long)]
        scope_payload_json: Option<String>,

        #[arg(long)]
        baseline_fingerprint: Option<String>,

        #[arg(long)]
        baseline_observation: Option<String>,
    },
    CacheRecord {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        verification: String,

        #[arg(long, default_value_t = 0)]
        resource_basis_ordinal: i64,

        #[arg(long)]
        adapter_kind: String,

        #[arg(long)]
        adapter_schema_version: i64,

        #[arg(long)]
        scope_schema_version: i64,

        #[arg(long)]
        observation_status: String,

        #[arg(long)]
        observed_fingerprint: Option<String>,

        #[arg(long)]
        observation: Option<String>,

        #[arg(long, default_value = "{}")]
        detail_json: String,
    },
}

fn main() {
    match run(Cli::parse()) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<String> {
    match cli.command {
        Command::Init {
            store,
            display_name,
        } => {
            let engine = Engine::init(store, StoreInitOptions::new(display_name)?)?;
            let info = engine.store_info()?;
            Ok(format!(
                "initialized store_id={} schema_version={}\n",
                info.store_id, info.manifest.schema_version
            ))
        }
        Command::Doctor { store } => {
            let engine = Engine::open(store)?;
            let info = engine.store_info()?;
            let integrity = engine.validate_integrity()?;
            Ok(format!(
                "ok store_id={} schema_version={} canonical_json_profile={} checked_branches={} checked_commits={}\n",
                info.store_id,
                info.manifest.schema_version,
                info.manifest.canonical_json_profile,
                integrity.checked_branches,
                integrity.checked_commits
            ))
        }
        Command::History {
            store,
            branch,
            commit,
            limit,
        } => {
            let engine = Engine::open(store)?;
            let mut options = match (branch, commit) {
                (Some(branch_id), None) => {
                    HistoryQueryOptions::from_branch(BranchId::parse_canonical(&branch_id)?)
                }
                (None, Some(commit_id)) => {
                    HistoryQueryOptions::from_commit(CommitId::parse_canonical(&commit_id)?)
                }
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "history requires exactly one of --branch or --commit".to_owned(),
                    ));
                }
            };
            if let Some(limit) = limit {
                options = options.with_limit(limit)?;
            }
            let history = engine.history(options)?;
            let mut output = format!(
                "start_commit_id={}\nentries={}\n",
                history.start_commit_id,
                history.entries.len()
            );
            for entry in &history.entries {
                render_history_entry(&mut output, entry);
            }
            Ok(output)
        }
        Command::ShowAt { store, commit } => {
            let engine = Engine::open(store)?;
            let state = engine.show_at(CommitId::parse_canonical(&commit)?)?;
            Ok(render_replayed_state(&state))
        }
        Command::Workspace {
            command:
                WorkspaceCommand::Create {
                    store,
                    display_name,
                    initial_branch_name,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = WorkspaceInitOptions::new(display_name)?;
            if let Some(initial_branch_name) = initial_branch_name {
                options = options.with_initial_branch_name(initial_branch_name)?;
            }
            let workspace = engine.create_workspace(options)?;
            Ok(render_workspace_info(&workspace))
        }
        Command::Branch {
            command: BranchCommand::List { store, workspace },
        } => {
            let engine = Engine::open(store)?;
            let branches =
                engine.list_branches(workvcs_core::WorkspaceId::parse_canonical(&workspace)?)?;
            Ok(render_branch_list(&branches))
        }
        Command::Branch {
            command: BranchCommand::Head { store, branch },
        } => {
            let engine = Engine::open(store)?;
            let head = engine.branch_head(BranchId::parse_canonical(&branch)?)?;
            Ok(render_branch_head(&head))
        }
        Command::Branch {
            command:
                BranchCommand::Fork {
                    store,
                    from_branch,
                    from_commit,
                    name,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let options = match (from_branch, from_commit) {
                (Some(branch), None) => {
                    BranchForkOptions::from_branch(BranchId::parse_canonical(&branch)?, name)?
                }
                (None, Some(commit)) => {
                    BranchForkOptions::from_commit(CommitId::parse_canonical(&commit)?, name)?
                }
                _ => {
                    return Err(WorkVcsError::WorkspaceInvalid(
                        "branch fork requires exactly one source".to_owned(),
                    ));
                }
            };
            let forked = engine.fork_branch(options)?;
            Ok(render_branch_fork(&forked))
        }
        Command::Task {
            command:
                TaskCommand::Create {
                    store,
                    branch,
                    head,
                    description,
                    priority,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = TaskCreateOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                description,
            )?;
            if let Some(priority) = priority {
                options = options.with_priority(priority)?;
            }
            let task = engine.create_task(options)?;
            Ok(render_task_create(&task))
        }
        Command::Task {
            command:
                TaskCommand::Transition {
                    store,
                    branch,
                    head,
                    task,
                    task_version,
                    status,
                    outcome,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = TaskTransitionOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                EntityId::parse_canonical(&task)?,
                EntityVersionId::parse_canonical(&task_version)?,
                parse_task_status(&status)?,
            )?;
            if let Some(outcome) = outcome {
                options = options.with_outcome(outcome)?;
            }
            let transition = engine.transition_task(options)?;
            Ok(render_task_transition(&transition))
        }
        Command::Ac {
            command:
                AcceptanceCriterionCommand::Create {
                    store,
                    branch,
                    head,
                    task,
                    task_version,
                    local_key,
                    statement,
                    classification,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let criterion =
                engine.create_acceptance_criterion(AcceptanceCriterionCreateOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&task)?,
                    EntityVersionId::parse_canonical(&task_version)?,
                    local_key,
                    statement,
                    parse_acceptance_criterion_classification(&classification)?,
                )?)?;
            Ok(render_acceptance_criterion_create(&criterion))
        }
        Command::Ac {
            command:
                AcceptanceCriterionCommand::Status {
                    store,
                    branch,
                    criterion,
                },
        } => {
            let engine = Engine::open(store)?;
            let status = engine.acceptance_criterion_effective_status_for_branch(
                BranchId::parse_canonical(&branch)?,
                EntityId::parse_canonical(&criterion)?,
            )?;
            Ok(render_acceptance_criterion_status(status))
        }
        Command::Vr {
            command:
                VerificationRequirementCommand::Create {
                    store,
                    branch,
                    head,
                    criterion,
                    criterion_version,
                    local_key,
                    statement,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let requirement = engine.create_verification_requirement(
                VerificationRequirementCreateOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&criterion)?,
                    EntityVersionId::parse_canonical(&criterion_version)?,
                    local_key,
                    statement,
                )?,
            )?;
            Ok(render_verification_requirement_create(&requirement))
        }
        Command::Resource {
            command: ResourceCommand::Create { store, kind },
        } => {
            let mut engine = Engine::open(store)?;
            let resource = engine.create_resource(ResourceCreateOptions::new(kind)?)?;
            Ok(render_resource_create(&resource))
        }
        Command::Resource {
            command:
                ResourceCommand::Observe {
                    store,
                    resource,
                    adapter_kind,
                    adapter_schema_version,
                    fingerprint,
                    content,
                    summary_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let fingerprint = fingerprint_from_cli(fingerprint, content)?;
            let observation =
                engine.record_resource_observation(ResourceObservationCreateOptions::new(
                    ResourceId::parse_canonical(&resource)?,
                    adapter_kind,
                    adapter_schema_version,
                    fingerprint,
                    parse_cli_object("resource observation summary", &summary_json)?,
                )?)?;
            Ok(render_resource_observation_create(&observation))
        }
        Command::Verification {
            command:
                VerificationCommand::Record {
                    store,
                    branch,
                    head,
                    result,
                    method,
                    acceptance_criterion,
                    verification_requirement,
                    resource,
                    adapter_kind,
                    adapter_schema_version,
                    scope_kind,
                    scope_schema_version,
                    scope_payload_json,
                    baseline_fingerprint,
                    baseline_observation,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let target = match (acceptance_criterion, verification_requirement) {
                (Some(acceptance_criterion), None) => VerificationTarget::AcceptanceCriterion(
                    EntityId::parse_canonical(&acceptance_criterion)?,
                ),
                (None, Some(verification_requirement)) => {
                    VerificationTarget::VerificationRequirement(EntityId::parse_canonical(
                        &verification_requirement,
                    )?)
                }
                _ => {
                    return Err(WorkVcsError::TaskInvalid(
                        "verification record requires exactly one target".to_owned(),
                    ));
                }
            };
            let mut options = VerificationCreateOptions::new(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                target,
                parse_verification_result(&result)?,
            )?;
            if let Some(method) = method {
                options = options.with_method(method_value(&method))?;
            }
            if resource.is_some()
                || adapter_kind.is_some()
                || adapter_schema_version.is_some()
                || scope_kind.is_some()
                || scope_schema_version.is_some()
                || scope_payload_json.is_some()
                || baseline_fingerprint.is_some()
                || baseline_observation.is_some()
            {
                options = options.with_resource_basis(vec![resource_basis_from_cli(
                    ResourceBasisArgs {
                        resource,
                        adapter_kind,
                        adapter_schema_version,
                        scope_kind,
                        scope_schema_version,
                        scope_payload_json,
                        baseline_fingerprint,
                        baseline_observation,
                    },
                )?])?;
            }
            let verification = engine.create_verification(options)?;
            Ok(render_verification_create(&verification))
        }
        Command::Verification {
            command:
                VerificationCommand::CacheRecord {
                    store,
                    branch,
                    head,
                    verification,
                    resource_basis_ordinal,
                    adapter_kind,
                    adapter_schema_version,
                    scope_schema_version,
                    observation_status,
                    observed_fingerprint,
                    observation,
                    detail_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let stamp = applicability_stamp_from_cli(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
                &observation_status,
                observed_fingerprint,
                observation,
            )?;
            let snapshot = engine.record_verification_applicability(
                VerificationApplicabilityRecordOptions::new(
                    BranchId::parse_canonical(&branch)?,
                    EntityId::parse_canonical(&verification)?,
                    CommitId::parse_canonical(&head)?,
                )?
                .with_resource_stamps(vec![stamp])?
                .with_detail(parse_cli_object(
                    "verification applicability detail",
                    &detail_json,
                )?)?,
            )?;
            Ok(render_verification_applicability_cache(&snapshot))
        }
        Command::Session {
            command:
                SessionCommand::Start {
                    store,
                    workspace,
                    branch,
                    metadata_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let session = engine.start_session(
                SessionStartOptions::new(
                    workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                    BranchId::parse_canonical(&branch)?,
                )?
                .with_metadata(parse_cli_object("session metadata", &metadata_json)?)?,
            )?;
            Ok(render_session_start(&session))
        }
        Command::Session {
            command:
                SessionCommand::Switch {
                    store,
                    session,
                    workspace,
                    branch,
                    focus,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = SessionSwitchOptions::new(
                SessionId::parse_canonical(&session)?,
                workvcs_core::WorkspaceId::parse_canonical(&workspace)?,
                BranchId::parse_canonical(&branch)?,
            );
            if let Some(focus) = focus {
                options = options.with_focus(EntityId::parse_canonical(&focus)?);
            }
            let switched = engine.switch_session(options)?;
            Ok(render_session_switch(&switched))
        }
        Command::Session {
            command:
                SessionCommand::End {
                    store,
                    session,
                    summary_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let ended = engine.end_session(
                SessionEndOptions::new(SessionId::parse_canonical(&session)?)?
                    .with_summary(parse_cli_object("session summary", &summary_json)?)?,
            )?;
            Ok(render_session_end(&ended))
        }
        Command::Claim {
            command: ClaimCommand::Next { store, session },
        } => {
            let mut engine = Engine::open(store)?;
            let claimed = engine
                .claim_next_task(ClaimNextOptions::new(SessionId::parse_canonical(&session)?))?;
            Ok(render_claim_next(&claimed))
        }
        Command::Claim {
            command:
                ClaimCommand::Task {
                    store,
                    session,
                    task,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let claim = engine.claim_task(ClaimTaskOptions::new(
                SessionId::parse_canonical(&session)?,
                EntityId::parse_canonical(&task)?,
            ))?;
            Ok(render_claim_task(&claim))
        }
        Command::Claim {
            command:
                ClaimCommand::Release {
                    store,
                    session,
                    claim,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let released = engine.release_claim(ClaimReleaseOptions::new(
                SessionId::parse_canonical(&session)?,
                ClaimId::parse_canonical(&claim)?,
            ))?;
            Ok(render_claim_release(&released))
        }
        Command::Context { store, session } => {
            let engine = Engine::open(store)?;
            let context = engine.context_overview(ContextOverviewOptions::new(
                SessionId::parse_canonical(&session)?,
            ))?;
            Ok(render_context_overview(&context))
        }
        Command::Runnable {
            command: RunnableCommand::Tasks { store, session },
        } => {
            let engine = Engine::open(store)?;
            let projection = engine.runnable_tasks(RunnableTasksOptions::new(
                SessionId::parse_canonical(&session)?,
            ))?;
            Ok(render_runnable_tasks(&projection))
        }
    }
}

fn parse_task_status(value: &str) -> Result<TaskStatus> {
    match value {
        "pending" => Ok(TaskStatus::Pending),
        "in_progress" => Ok(TaskStatus::InProgress),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        "failed" => Ok(TaskStatus::Failed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "task status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_acceptance_criterion_classification(
    value: &str,
) -> Result<AcceptanceCriterionClassification> {
    match value {
        "required" => Ok(AcceptanceCriterionClassification::Required),
        "optional" => Ok(AcceptanceCriterionClassification::Optional),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "acceptance criterion classification {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_verification_result(value: &str) -> Result<VerificationResult> {
    match value {
        "passed" => Ok(VerificationResult::Passed),
        "failed" => Ok(VerificationResult::Failed),
        "inconclusive" => Ok(VerificationResult::Inconclusive),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "verification result {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_observation_status(value: &str) -> Result<ApplicabilityResourceObservationStatus> {
    match value {
        "observed" => Ok(ApplicabilityResourceObservationStatus::Observed),
        "unavailable" => Ok(ApplicabilityResourceObservationStatus::Unavailable),
        "error" => Ok(ApplicabilityResourceObservationStatus::Error),
        other => Err(WorkVcsError::TaskInvalid(format!(
            "resource observation status {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_cli_object(label: &str, json: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(json.as_bytes())?;
    if matches!(value, CanonicalValue::Object(_)) {
        Ok(value)
    } else {
        Err(WorkVcsError::TaskInvalid(format!(
            "{label} must be an object"
        )))
    }
}

fn fingerprint_from_cli(fingerprint: Option<String>, content: Option<String>) -> Result<Digest> {
    match (fingerprint, content) {
        (Some(fingerprint), None) => Digest::from_hex(&fingerprint),
        (None, Some(content)) => Ok(content_object_digest(content.as_bytes())),
        _ => Err(WorkVcsError::TaskInvalid(
            "expected exactly one fingerprint source".to_owned(),
        )),
    }
}

fn required_arg<T>(label: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| WorkVcsError::TaskInvalid(format!("{label} is required")))
}

struct ResourceBasisArgs {
    resource: Option<String>,
    adapter_kind: Option<String>,
    adapter_schema_version: Option<i64>,
    scope_kind: Option<String>,
    scope_schema_version: Option<i64>,
    scope_payload_json: Option<String>,
    baseline_fingerprint: Option<String>,
    baseline_observation: Option<String>,
}

fn resource_basis_from_cli(args: ResourceBasisArgs) -> Result<VerificationResourceBasis> {
    let mut basis = VerificationResourceBasis::new(
        ResourceId::parse_canonical(&required_arg("--resource", args.resource)?)?,
        required_arg("--adapter-kind", args.adapter_kind)?,
        required_arg("--adapter-schema-version", args.adapter_schema_version)?,
        required_arg("--scope-kind", args.scope_kind)?,
        required_arg("--scope-schema-version", args.scope_schema_version)?,
        parse_cli_object(
            "resource basis scope payload",
            &required_arg("--scope-payload-json", args.scope_payload_json)?,
        )?,
        Digest::from_hex(&required_arg(
            "--baseline-fingerprint",
            args.baseline_fingerprint,
        )?)?,
    )?;
    if let Some(baseline_observation) = args.baseline_observation {
        basis = basis.with_baseline_observation_id(ResourceObservationId::parse_canonical(
            &baseline_observation,
        )?)?;
    }
    Ok(basis)
}

fn applicability_stamp_from_cli(
    resource_basis_ordinal: i64,
    adapter_kind: String,
    adapter_schema_version: i64,
    scope_schema_version: i64,
    observation_status: &str,
    observed_fingerprint: Option<String>,
    observation: Option<String>,
) -> Result<ApplicabilityResourceStampInput> {
    let mut stamp = match parse_observation_status(observation_status)? {
        ApplicabilityResourceObservationStatus::Observed => {
            ApplicabilityResourceStampInput::observed(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
                Digest::from_hex(&required_arg(
                    "--observed-fingerprint",
                    observed_fingerprint,
                )?)?,
            )?
        }
        ApplicabilityResourceObservationStatus::Unavailable => {
            if observed_fingerprint.is_some() || observation.is_some() {
                return Err(WorkVcsError::TaskInvalid(
                    "unavailable resource stamp must not include observed data".to_owned(),
                ));
            }
            ApplicabilityResourceStampInput::unavailable(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
            )?
        }
        ApplicabilityResourceObservationStatus::Error => {
            if observed_fingerprint.is_some() || observation.is_some() {
                return Err(WorkVcsError::TaskInvalid(
                    "error resource stamp must not include observed data".to_owned(),
                ));
            }
            ApplicabilityResourceStampInput::error(
                resource_basis_ordinal,
                adapter_kind,
                adapter_schema_version,
                scope_schema_version,
            )?
        }
    };
    if let Some(observation) = observation {
        stamp = stamp.with_observation_id(ResourceObservationId::parse_canonical(&observation)?)?;
    }
    Ok(stamp)
}

fn method_value(name: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "kind".to_owned(),
            CanonicalValue::String("manual".to_owned()),
        ),
        ("name".to_owned(), CanonicalValue::String(name.to_owned())),
    ])
    .expect("method object")
}

fn render_workspace_info(workspace: &WorkspaceInfo) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nbranch_name={}\ngenesis_commit_id={}\ngenesis_changeset_id={}\nstate_digest={}\n",
        workspace.workspace_id,
        workspace.initial_branch_id,
        workspace.initial_branch_name,
        workspace.genesis_commit_id,
        workspace.genesis_changeset_id,
        workspace.state_digest
    )
}

fn render_branch_head(head: &BranchHead) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nbranch_name={}\nhead_commit_id={}\nlifecycle_state={}\nstate_digest={}\n",
        head.workspace_id,
        head.branch_id,
        head.name,
        head.head_commit_id,
        head.lifecycle_state,
        head.state_digest
    )
}

fn render_branch_list(branches: &[BranchHead]) -> String {
    let workspace_id = branches
        .first()
        .map(|branch| branch.workspace_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let mut output = format!("workspace_id={workspace_id}\nbranches={}\n", branches.len());
    for (index, branch) in branches.iter().enumerate() {
        let _ = writeln!(output, "branch.{index}.branch_id={}", branch.branch_id);
        let _ = writeln!(output, "branch.{index}.branch_name={}", branch.name);
        let _ = writeln!(
            output,
            "branch.{index}.head_commit_id={}",
            branch.head_commit_id
        );
        let _ = writeln!(
            output,
            "branch.{index}.lifecycle_state={}",
            branch.lifecycle_state
        );
        let _ = writeln!(
            output,
            "branch.{index}.state_digest={}",
            branch.state_digest
        );
    }
    output
}

fn render_branch_fork(branch: &BranchForkResult) -> String {
    let source_branch_id = branch
        .source_branch_id
        .map(|branch_id| branch_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "workspace_id={}\nbranch_id={}\nbranch_name={}\nsource_branch_id={}\nhead_commit_id={}\nstate_digest={}\nevent_id={}\ncreated_at_us={}\n",
        branch.workspace_id,
        branch.branch_id,
        branch.name,
        source_branch_id,
        branch.head_commit_id,
        branch.state_digest,
        branch.event_id,
        branch.created_at_us
    )
}

fn render_task_create(task: &TaskCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\ntask_entity_id={}\ntask_entity_version_id={}\ntask_state_digest={}\nwork_state_digest={}\nstatus={}\n",
        task.workspace_id,
        task.branch_id,
        task.previous_head_commit_id,
        task.commit_id,
        task.changeset_id,
        task.task_entity_id,
        task.task_entity_version_id,
        task.task_state_digest,
        task.work_state_digest,
        task.state.status
    )
}

fn render_task_transition(transition: &TaskTransitionCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\ntask_entity_id={}\nprevious_task_entity_version_id={}\ntask_entity_version_id={}\ntask_state_digest={}\nwork_state_digest={}\nstatus={}\n",
        transition.workspace_id,
        transition.branch_id,
        transition.previous_head_commit_id,
        transition.commit_id,
        transition.changeset_id,
        transition.task_entity_id,
        transition.previous_task_entity_version_id,
        transition.task_entity_version_id,
        transition.task_state_digest,
        transition.work_state_digest,
        transition.state.status
    )
}

fn render_acceptance_criterion_create(criterion: &AcceptanceCriterionCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\ntask_entity_id={}\ntask_entity_version_id={}\nacceptance_criterion_entity_id={}\nacceptance_criterion_entity_version_id={}\nacceptance_criterion_state_digest={}\nwork_state_digest={}\nlocal_key={}\nclassification={}\n",
        criterion.workspace_id,
        criterion.branch_id,
        criterion.previous_head_commit_id,
        criterion.commit_id,
        criterion.changeset_id,
        criterion.task_entity_id,
        criterion.task_entity_version_id,
        criterion.acceptance_criterion_entity_id,
        criterion.acceptance_criterion_entity_version_id,
        criterion.acceptance_criterion_state_digest,
        criterion.work_state_digest,
        criterion.local_key,
        criterion.state.classification
    )
}

fn render_acceptance_criterion_status(status: AcceptanceCriterionEffectiveStatus) -> String {
    format!("status={status}\n")
}

fn render_verification_requirement_create(
    requirement: &VerificationRequirementCreateCommit,
) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\nacceptance_criterion_entity_id={}\nacceptance_criterion_entity_version_id={}\nverification_requirement_entity_id={}\nverification_requirement_entity_version_id={}\nverification_requirement_state_digest={}\nwork_state_digest={}\nlocal_key={}\n",
        requirement.workspace_id,
        requirement.branch_id,
        requirement.previous_head_commit_id,
        requirement.commit_id,
        requirement.changeset_id,
        requirement.acceptance_criterion_entity_id,
        requirement.acceptance_criterion_entity_version_id,
        requirement.verification_requirement_entity_id,
        requirement.verification_requirement_entity_version_id,
        requirement.verification_requirement_state_digest,
        requirement.work_state_digest,
        requirement.local_key
    )
}

fn render_verification_create(verification: &VerificationCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\nverification_entity_id={}\nverification_entity_version_id={}\nverification_state_digest={}\nverifies_relation_id={}\nverifies_relation_version_id={}\nwork_state_digest={}\nresult={}\n",
        verification.workspace_id,
        verification.branch_id,
        verification.previous_head_commit_id,
        verification.commit_id,
        verification.changeset_id,
        verification.verification_entity_id,
        verification.verification_entity_version_id,
        verification.verification_state_digest,
        verification.verifies_relation_id,
        verification.verifies_relation_version_id,
        verification.work_state_digest,
        verification.state.result
    )
}

fn render_resource_create(resource: &ResourceCreateResult) -> String {
    format!(
        "resource_id={}\nresource_kind={}\ncreated_at_us={}\n",
        resource.resource_id, resource.resource_kind, resource.created_at_us
    )
}

fn render_resource_observation_create(observation: &ResourceObservationCreateResult) -> String {
    format!(
        "observation_id={}\nresource_id={}\nadapter_kind={}\nadapter_schema_version={}\nfingerprint={}\ncaptured_at_us={}\n",
        observation.observation_id,
        observation.resource_id,
        observation.state.adapter_kind,
        observation.state.adapter_schema_version,
        observation.state.fingerprint,
        observation.captured_at_us
    )
}

fn render_verification_applicability_cache(
    snapshot: &VerificationApplicabilityCacheSnapshot,
) -> String {
    format!(
        "branch_id={}\nverification_entity_id={}\nevaluated_commit_id={}\napplicability={}\nreason_code={}\nevaluated_at_us={}\nresource_stamps={}\n",
        snapshot.branch_id,
        snapshot.verification_entity_id,
        snapshot.evaluated_commit_id,
        snapshot.applicability,
        snapshot.reason_code,
        snapshot.evaluated_at_us,
        snapshot.resource_stamps.len()
    )
}

fn render_session_start(session: &SessionStartResult) -> String {
    format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nstarted_at_us={}\nlifecycle_state={}\n",
        session.session_id,
        session.workspace_id,
        session.branch_id,
        session.started_at_us,
        session_lifecycle_state(session.state.lifecycle_state)
    )
}

fn render_session_switch(session: &SessionSwitchResult) -> String {
    let focus_entity_id = session
        .state
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "session_id={}\nprevious_workspace_id={}\nprevious_branch_id={}\nworkspace_id={}\nbranch_id={}\nreleased_claims={}\nfocus_entity_id={}\noccurred_at_us={}\nlifecycle_state={}\n",
        session.session_id,
        session.previous_workspace_id,
        session.previous_branch_id,
        session.active_workspace_id,
        session.active_branch_id,
        session.released_claims,
        focus_entity_id,
        session.occurred_at_us,
        session_lifecycle_state(session.state.lifecycle_state)
    )
}

fn render_session_end(session: &SessionEndResult) -> String {
    format!(
        "session_id={}\nsession_diff_id={}\nended_at_us={}\nlifecycle_state={}\n",
        session.session_id,
        session.session_diff_id,
        session.ended_at_us,
        session_lifecycle_state(session.state.lifecycle_state)
    )
}

fn render_claim_task(claim: &ClaimTaskResult) -> String {
    format!(
        "claim_id={}\nsession_id={}\nworkspace_id={}\nbranch_id={}\ntask_entity_id={}\nmode={}\nclaimed_at_us={}\nlifecycle_state={}\n",
        claim.claim_id,
        claim.session_id,
        claim.workspace_id,
        claim.branch_id,
        claim.task_entity_id,
        claim_mode(claim.mode),
        claim.claimed_at_us,
        claim_lifecycle_state(claim.state.lifecycle_state)
    )
}

fn render_claim_next(result: &ClaimNextResult) -> String {
    let mut output = format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ninspected_candidates={}\nselected={}\n",
        result.session_id,
        result.workspace_id,
        result.branch_id,
        result.head_commit_id,
        result.inspected_candidates,
        result.selected.is_some()
    );
    if let Some(claim) = &result.selected {
        let _ = writeln!(output, "claim_id={}", claim.claim_id);
        let _ = writeln!(output, "task_entity_id={}", claim.task_entity_id);
        let _ = writeln!(output, "mode={}", claim_mode(claim.mode));
        let _ = writeln!(output, "claimed_at_us={}", claim.claimed_at_us);
        let _ = writeln!(
            output,
            "lifecycle_state={}",
            claim_lifecycle_state(claim.state.lifecycle_state)
        );
    }
    output
}

fn render_claim_release(claim: &ClaimReleaseResult) -> String {
    format!(
        "claim_id={}\nsession_id={}\nreleased_at_us={}\nlifecycle_state={}\n",
        claim.claim_id,
        claim.session_id,
        claim.released_at_us,
        claim_lifecycle_state(claim.state.lifecycle_state)
    )
}

fn render_context_overview(context: &ContextOverview) -> String {
    let session = &context.session;
    let focus_entity_id = session
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let last_activity_at_us = session
        .last_activity_at_us
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let runnable_ready = context
        .runnable_tasks
        .candidates
        .iter()
        .filter(|candidate| candidate.runnable)
        .count();
    let mut output = format!(
        "session_id={}\nlifecycle_state={}\nworkspace_id={}\nbranch_id={}\nbranch_name={}\nhead_commit_id={}\nstate_digest={}\nstarted_at_us={}\nlast_activity_at_us={}\nfocus_entity_id={}\nfocus_path_entries={}\ncontext_workspaces={}\nrunnable_candidates={}\nrunnable_ready={}\n",
        session.session_id,
        session_lifecycle_state(session.lifecycle_state),
        context.branch.workspace_id,
        context.branch.branch_id,
        context.branch.name,
        context.branch.head_commit_id,
        context.branch.state_digest,
        session.started_at_us,
        last_activity_at_us,
        focus_entity_id,
        session
            .focus
            .as_ref()
            .map(|focus| focus.path.len())
            .unwrap_or(0),
        session.context_workspaces.len(),
        context.runnable_tasks.candidates.len(),
        runnable_ready
    );
    for (index, workspace_id) in session.context_workspaces.iter().enumerate() {
        let _ = writeln!(
            output,
            "context_workspace.{index}.workspace_id={workspace_id}"
        );
    }
    for (index, candidate) in context.runnable_tasks.candidates.iter().enumerate() {
        render_runnable_candidate(&mut output, index, candidate);
    }
    output
}

fn render_runnable_tasks(projection: &RunnableTasksProjection) -> String {
    let mut output = format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ncandidates={}\n",
        projection.session_id,
        projection.workspace_id,
        projection.branch_id,
        projection.head_commit_id,
        projection.candidates.len()
    );
    for (index, candidate) in projection.candidates.iter().enumerate() {
        render_runnable_candidate(&mut output, index, candidate);
    }
    output
}

fn render_runnable_candidate(output: &mut String, index: usize, candidate: &RunnableTaskCandidate) {
    let blocked_reasons = candidate
        .blocked_reasons
        .iter()
        .map(|reason| runnable_blocked_reason(*reason))
        .collect::<Vec<_>>()
        .join(",");
    let unsatisfied_dependencies = candidate
        .unsatisfied_dependency_entity_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let _ = writeln!(
        output,
        "candidate.{index}.task_entity_id={}",
        candidate.task.task_entity_id
    );
    let _ = writeln!(
        output,
        "candidate.{index}.task_entity_version_id={}",
        candidate.task.task_entity_version_id
    );
    let _ = writeln!(
        output,
        "candidate.{index}.status={}",
        candidate.task.state.status
    );
    let _ = writeln!(
        output,
        "candidate.{index}.priority={}",
        candidate.task.state.priority
    );
    let _ = writeln!(output, "candidate.{index}.runnable={}", candidate.runnable);
    let _ = writeln!(
        output,
        "candidate.{index}.lifecycle_eligible={}",
        candidate.lifecycle_eligible
    );
    let _ = writeln!(
        output,
        "candidate.{index}.dependency_ready={}",
        candidate.dependency_ready
    );
    let _ = writeln!(
        output,
        "candidate.{index}.claim={}",
        runnable_claim_coordination(&candidate.claim_coordination)
    );
    let _ = writeln!(
        output,
        "candidate.{index}.unsatisfied_dependencies={unsatisfied_dependencies}"
    );
    let _ = writeln!(
        output,
        "candidate.{index}.blocked_reasons={blocked_reasons}"
    );
}

fn session_lifecycle_state(state: SessionLifecycleState) -> &'static str {
    match state {
        SessionLifecycleState::Active => "active",
        SessionLifecycleState::Ended => "ended",
    }
}

fn claim_lifecycle_state(state: ClaimLifecycleState) -> &'static str {
    match state {
        ClaimLifecycleState::Active => "active",
        ClaimLifecycleState::Released => "released",
    }
}

fn claim_mode(mode: ClaimMode) -> &'static str {
    match mode {
        ClaimMode::Exclusive => "exclusive",
    }
}

fn runnable_claim_coordination(claim: &RunnableTaskClaimCoordination) -> String {
    match claim {
        RunnableTaskClaimCoordination::Unclaimed => "unclaimed".to_owned(),
        RunnableTaskClaimCoordination::ClaimedBySession { claim_id } => {
            format!("claimed_by_session:{claim_id}")
        }
        RunnableTaskClaimCoordination::ClaimedByOtherSession {
            claim_id,
            session_id,
        } => format!("claimed_by_other_session:{claim_id}:{session_id}"),
    }
}

fn runnable_blocked_reason(reason: RunnableTaskBlockedReason) -> &'static str {
    match reason {
        RunnableTaskBlockedReason::LifecycleIneligible => "lifecycle_ineligible",
        RunnableTaskBlockedReason::DependencyBlocked => "dependency_blocked",
        RunnableTaskBlockedReason::ClaimBlocked => "claim_blocked",
    }
}

fn render_history_entry(output: &mut String, entry: &HistoryEntry) {
    let parent = entry
        .parent_commit_id
        .map(|commit_id| commit_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let _ = writeln!(
        output,
        "commit={} kind={} operation={} schema={} parent={} state_digest={}",
        entry.commit_id,
        entry.commit_kind,
        entry.operation_type,
        entry.operation_schema_version,
        parent,
        entry.state_digest
    );
}

fn render_replayed_state(state: &ReplayedState) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nstate_digest={}\n",
        state.workspace_id, state.commit_id, state.state_digest
    );
    render_work_state(&mut output, &state.state);
    output
}

fn render_work_state(output: &mut String, state: &WorkState) {
    let _ = writeln!(output, "entities={}", state.entities().len());
    for (entity_id, entity_version_id) in state.entities() {
        let _ = writeln!(output, "entity={} version={}", entity_id, entity_version_id);
    }
    let _ = writeln!(output, "relations={}", state.relations().len());
    for (relation_id, relation_version_id) in state.relations() {
        let _ = writeln!(
            output,
            "relation={} version={}",
            relation_id, relation_version_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_exposes_thin_command_shells() {
        let command = Cli::command();
        let names = command
            .get_subcommands()
            .map(|command| command.get_name())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "init",
                "doctor",
                "history",
                "show-at",
                "workspace",
                "branch",
                "task",
                "ac",
                "vr",
                "resource",
                "session",
                "claim",
                "context",
                "runnable",
                "verification"
            ]
        );
    }

    #[test]
    fn history_requires_one_start_selector() {
        let missing = Cli::try_parse_from(["workvcs", "history", "store.sqlite"]);
        assert!(missing.is_err());

        let branch = BranchId::new_v7().to_string();
        let commit = CommitId::new_v7().to_string();
        let both = Cli::try_parse_from([
            "workvcs",
            "history",
            "store.sqlite",
            "--branch",
            &branch,
            "--commit",
            &commit,
        ]);
        assert!(both.is_err());
    }

    #[test]
    fn init_and_doctor_use_engine_store_boundary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");

        let init = run(Cli::try_parse_from([
            "workvcs",
            "init",
            path.to_str().expect("path text"),
            "--display-name",
            "cli-store",
        ])
        .expect("parse init"))
        .expect("run init");
        assert!(init.starts_with("initialized store_id="));
        assert!(init.contains("schema_version=1"));

        let doctor =
            run(
                Cli::try_parse_from(["workvcs", "doctor", path.to_str().expect("path text")])
                    .expect("parse doctor"),
            )
            .expect("run doctor");
        assert!(doctor.starts_with("ok store_id="));
        assert!(doctor.contains("canonical_json_profile=workvcs-jcs-v1"));
        assert!(doctor.contains("checked_branches=0"));
        assert!(doctor.contains("checked_commits=0"));
    }

    #[test]
    fn cli_runs_branch_head_and_fork_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let source_branch = value(&workspace, "branch_id");
        let mut source_head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &source_head,
            "--description",
            "Create fork source state",
        ])
        .expect("parse task"))
        .expect("create task");
        source_head = value(&task, "commit_id");

        let source = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "head",
            store,
            "--branch",
            &source_branch,
        ])
        .expect("parse source head"))
        .expect("source head");
        assert_eq!(value(&source, "head_commit_id"), source_head);

        let fork = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "fork",
            store,
            "--from-branch",
            &source_branch,
            "--name",
            "experiment",
        ])
        .expect("parse fork"))
        .expect("fork branch");
        let fork_branch = value(&fork, "branch_id");
        assert_eq!(value(&fork, "source_branch_id"), source_branch);
        assert_eq!(value(&fork, "head_commit_id"), source_head);

        let branches = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "list",
            store,
            "--workspace",
            &value(&workspace, "workspace_id"),
        ])
        .expect("parse branch list"))
        .expect("list branches");
        assert!(branches.contains("branches=2"));
        assert!(branches.contains("branch.0.branch_name=experiment"));

        let later_source = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &source_head,
            "--description",
            "Advance source after fork",
        ])
        .expect("parse later task"))
        .expect("advance source branch");
        assert_ne!(value(&later_source, "commit_id"), source_head);

        let fork_head = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "head",
            store,
            "--branch",
            &fork_branch,
        ])
        .expect("parse fork head"))
        .expect("fork head");
        assert_eq!(value(&fork_head, "branch_name"), "experiment");
        assert_eq!(value(&fork_head, "head_commit_id"), source_head);

        let fork_history = run(Cli::try_parse_from([
            "workvcs",
            "history",
            store,
            "--branch",
            &fork_branch,
            "--limit",
            "1",
        ])
        .expect("parse fork history"))
        .expect("fork history");
        assert!(fork_history.contains(&format!("start_commit_id={source_head}")));
        assert!(fork_history.contains("entries=1"));
    }

    #[test]
    fn cli_runs_session_switch_to_forked_branch_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let source_branch = value(&workspace, "branch_id");
        let source_head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &source_branch,
            "--head",
            &source_head,
            "--description",
            "Switch session to fork",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");
        let task_head = value(&task, "commit_id");

        let fork = run(Cli::try_parse_from([
            "workvcs",
            "branch",
            "fork",
            store,
            "--from-branch",
            &source_branch,
            "--name",
            "runtime",
        ])
        .expect("parse fork"))
        .expect("fork branch");
        let fork_branch = value(&fork, "branch_id");
        assert_eq!(value(&fork, "head_commit_id"), task_head);

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &source_branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let claim = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse claim"))
        .expect("claim source task");
        assert!(claim.contains("lifecycle_state=active"));

        let switched = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "switch",
            store,
            "--session",
            &session_id,
            "--workspace",
            &workspace_id,
            "--branch",
            &fork_branch,
            "--focus",
            &task_id,
        ])
        .expect("parse switch"))
        .expect("switch session");
        assert_eq!(value(&switched, "previous_branch_id"), source_branch);
        assert_eq!(value(&switched, "branch_id"), fork_branch);
        assert_eq!(value(&switched, "released_claims"), "1");
        assert_eq!(value(&switched, "focus_entity_id"), task_id);
        assert!(switched.contains("lifecycle_state=active"));

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable on fork");
        assert!(runnable.contains("candidates=1"));
        assert!(runnable.contains(&format!("candidate.0.task_entity_id={task_id}")));
        assert!(runnable.contains("candidate.0.claim=unclaimed"));
    }

    #[test]
    fn cli_runs_claim_next_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let mut head = value(&workspace, "genesis_commit_id");

        let first = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "First CLI next task",
        ])
        .expect("parse first task"))
        .expect("create first task");
        head = value(&first, "commit_id");
        let second = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Second CLI next task",
        ])
        .expect("parse second task"))
        .expect("create second task");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");

        let claimed =
            run(
                Cli::try_parse_from(["workvcs", "claim", "next", store, "--session", &session_id])
                    .expect("parse claim next"),
            )
            .expect("claim next");
        assert_eq!(value(&claimed, "selected"), "true");
        assert!(claimed.contains("claim_id="));
        assert!(claimed.contains("lifecycle_state=active"));
        let selected_task = value(&claimed, "task_entity_id");
        assert!(
            selected_task == value(&first, "task_entity_id")
                || selected_task == value(&second, "task_entity_id")
        );

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable after claim next");
        assert!(runnable.contains(&format!("candidate.0.task_entity_id={selected_task}")));
        assert!(runnable.contains("candidate.0.claim=claimed_by_session:"));
    }

    #[test]
    fn cli_runs_minimal_semantic_workflow_through_engine() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");

        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let mut head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Ship CLI workflow",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");
        head = value(&task, "commit_id");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "The CLI can run the minimal semantic workflow.",
        ])
        .expect("parse ac"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");
        let current_task_version = value(&criterion, "task_entity_version_id");
        head = value(&criterion, "commit_id");

        let status = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse status"))
        .expect("initial status");
        assert_eq!(status, "status=unverified\n");

        let verification = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "record",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--acceptance-criterion",
            &criterion_id,
            "--result",
            "passed",
            "--method",
            "manual-review",
        ])
        .expect("parse verification"))
        .expect("record verification");
        head = value(&verification, "commit_id");

        let verified = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse verified"))
        .expect("verified status");
        assert_eq!(verified, "status=verified\n");

        let transition = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "transition",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &current_task_version,
            "--status",
            "done",
            "--outcome",
            "completed",
        ])
        .expect("parse transition"))
        .expect("transition task");
        assert!(transition.contains("status=done"));
    }

    #[test]
    fn cli_runs_resource_backed_verification_and_cache_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let branch = value(&workspace, "branch_id");
        let mut head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Ship resource-backed CLI workflow",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");
        head = value(&task, "commit_id");

        let criterion = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &value(&task, "task_entity_version_id"),
            "--local-key",
            "AC-1",
            "--statement",
            "The resource-backed CLI verification is applicable.",
        ])
        .expect("parse ac"))
        .expect("create ac");
        let criterion_id = value(&criterion, "acceptance_criterion_entity_id");
        let current_task_version = value(&criterion, "task_entity_version_id");
        head = value(&criterion, "commit_id");

        let resource =
            run(
                Cli::try_parse_from(["workvcs", "resource", "create", store, "--kind", "git"])
                    .expect("parse resource"),
            )
            .expect("create resource");
        let resource_id = value(&resource, "resource_id");
        let observation = run(Cli::try_parse_from([
            "workvcs",
            "resource",
            "observe",
            store,
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--content",
            "baseline bytes",
        ])
        .expect("parse observe"))
        .expect("record observation");
        let observation_id = value(&observation, "observation_id");
        let fingerprint = value(&observation, "fingerprint");

        let verification = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "record",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--acceptance-criterion",
            &criterion_id,
            "--result",
            "passed",
            "--method",
            "manual-review",
            "--resource",
            &resource_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--scope-kind",
            "path",
            "--scope-schema-version",
            "1",
            "--scope-payload-json",
            "{\"path\":\"src/lib.rs\"}",
            "--baseline-fingerprint",
            &fingerprint,
            "--baseline-observation",
            &observation_id,
        ])
        .expect("parse verification"))
        .expect("record verification");
        let verification_id = value(&verification, "verification_entity_id");
        head = value(&verification, "commit_id");

        let stale = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse stale status"))
        .expect("stale status");
        assert_eq!(stale, "status=stale\n");

        let cache = run(Cli::try_parse_from([
            "workvcs",
            "verification",
            "cache-record",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--verification",
            &verification_id,
            "--adapter-kind",
            "git",
            "--adapter-schema-version",
            "1",
            "--scope-schema-version",
            "1",
            "--observation-status",
            "observed",
            "--observed-fingerprint",
            &fingerprint,
            "--observation",
            &observation_id,
        ])
        .expect("parse cache"))
        .expect("record cache");
        assert!(cache.contains("applicability=applicable"));
        assert!(cache.contains("reason_code=all_basis_applicable"));

        let verified = run(Cli::try_parse_from([
            "workvcs",
            "ac",
            "status",
            store,
            "--branch",
            &branch,
            "--criterion",
            &criterion_id,
        ])
        .expect("parse verified"))
        .expect("verified status");
        assert_eq!(verified, "status=verified\n");

        let transition = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "transition",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--task",
            &task_id,
            "--task-version",
            &current_task_version,
            "--status",
            "done",
            "--outcome",
            "completed",
        ])
        .expect("parse transition"))
        .expect("transition task");
        assert!(transition.contains("status=done"));
    }

    #[test]
    fn cli_runs_runtime_runnable_and_claim_workflow() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");
        let store = path.to_str().expect("path text");

        run(
            Cli::try_parse_from(["workvcs", "init", store, "--display-name", "cli-store"])
                .expect("parse init"),
        )
        .expect("run init");
        let workspace = run(Cli::try_parse_from([
            "workvcs",
            "workspace",
            "create",
            store,
            "--display-name",
            "workspace",
        ])
        .expect("parse workspace"))
        .expect("create workspace");
        let workspace_id = value(&workspace, "workspace_id");
        let branch = value(&workspace, "branch_id");
        let head = value(&workspace, "genesis_commit_id");

        let task = run(Cli::try_parse_from([
            "workvcs",
            "task",
            "create",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--description",
            "Claim runnable task",
        ])
        .expect("parse task"))
        .expect("create task");
        let task_id = value(&task, "task_entity_id");

        let session = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "start",
            store,
            "--workspace",
            &workspace_id,
            "--branch",
            &branch,
        ])
        .expect("parse session start"))
        .expect("start session");
        let session_id = value(&session, "session_id");
        assert!(session.contains("lifecycle_state=active"));

        let context =
            run(
                Cli::try_parse_from(["workvcs", "context", store, "--session", &session_id])
                    .expect("parse context"),
            )
            .expect("context overview");
        assert!(context.contains("lifecycle_state=active"));
        assert!(context.contains(&format!("workspace_id={workspace_id}")));
        assert!(context.contains(&format!("branch_id={branch}")));
        assert!(context.contains("context_workspaces=1"));
        assert!(context.contains("runnable_candidates=1"));
        assert!(context.contains(&format!("candidate.0.task_entity_id={task_id}")));

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable tasks");
        assert!(runnable.contains("candidates=1"));
        assert!(runnable.contains(&format!("candidate.0.task_entity_id={task_id}")));
        assert!(runnable.contains("candidate.0.runnable=true"));
        assert!(runnable.contains("candidate.0.claim=unclaimed"));

        let claim = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "task",
            store,
            "--session",
            &session_id,
            "--task",
            &task_id,
        ])
        .expect("parse claim"))
        .expect("claim task");
        let claim_id = value(&claim, "claim_id");
        assert!(claim.contains("mode=exclusive"));
        assert!(claim.contains("lifecycle_state=active"));

        let claimed_runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse claimed runnable"))
        .expect("claimed runnable tasks");
        assert!(
            claimed_runnable.contains(&format!("candidate.0.claim=claimed_by_session:{claim_id}"))
        );

        let release = run(Cli::try_parse_from([
            "workvcs",
            "claim",
            "release",
            store,
            "--session",
            &session_id,
            "--claim",
            &claim_id,
        ])
        .expect("parse release"))
        .expect("release claim");
        assert!(release.contains("lifecycle_state=released"));

        let ended = run(Cli::try_parse_from([
            "workvcs",
            "session",
            "end",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse session end"))
        .expect("end session");
        assert!(ended.contains("lifecycle_state=ended"));
    }

    fn value(output: &str, key: &str) -> String {
        output
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("missing {key} in output:\n{output}"))
            .to_owned()
    }
}
