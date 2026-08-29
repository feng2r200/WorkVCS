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
    Engine, EntityId, EntityVersionId, EvidenceId, HistoryEntry, HistoryQueryOptions,
    NextWorkOptions, NextWorkResult, RecordCreateCommit, RecordCreateOptions, RecordKind,
    RecordListOptions, RecordListResult, RecordRelationCreateCommit, RecordRelationCreateOptions,
    RecordRelationListOptions, RecordRelationListResult, RecordRelationType, RecordSnapshot,
    RecordStatus, RecordTransitionCommit, RecordTransitionOptions, ReplayedState,
    ResolvedWhyQuerySubject, ResourceCreateOptions, ResourceCreateResult, ResourceId,
    ResourceObservationCreateOptions, ResourceObservationCreateResult, ResourceObservationId,
    Result, RunnableTaskBlockedReason, RunnableTaskCandidate, RunnableTaskClaimCoordination,
    RunnableTasksOptions, RunnableTasksProjection, SessionEndOptions, SessionEndResult, SessionId,
    SessionLifecycleState, SessionStartOptions, SessionStartResult, SessionSwitchOptions,
    SessionSwitchResult, StoreInitOptions, TaskCreateCommit, TaskCreateOptions, TaskStatus,
    TaskTransitionCommit, TaskTransitionOptions, VerificationApplicabilityCacheSnapshot,
    VerificationApplicabilityRecordOptions, VerificationCreateCommit, VerificationCreateOptions,
    VerificationRequirementCreateCommit, VerificationRequirementCreateOptions,
    VerificationResourceBasis, VerificationResult, VerificationTarget, WhyDeferredRelationFamily,
    WhyEntityKind, WhyQueryOptions, WhyQueryResult, WhyQueryTarget, WhyRelationDirection,
    WhyRelationEndpoint, WhyRelationKind, WorkState, WorkVcsError, WorkspaceInfo,
    WorkspaceInitOptions, canonical_bytes, content_object_digest, parse_canonical_json,
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
    #[command(group(
        ArgGroup::new("why-target")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    #[command(group(
        ArgGroup::new("why-subject")
            .required(true)
            .multiple(false)
            .args(["entity", "evidence"])
    ))]
    Why {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        entity: Option<String>,

        #[arg(long)]
        evidence: Option<String>,
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
    Record {
        #[command(subcommand)]
        command: RecordCommand,
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
    Next {
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
enum RecordCommand {
    Show {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,

        #[arg(long)]
        record: String,
    },
    List {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,

        #[arg(long)]
        kind: Option<String>,
    },
    LinkInvalidates {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        source_record: String,

        #[arg(long)]
        target_record: String,

        #[arg(long)]
        rationale: String,
    },
    RelationList {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,

        #[arg(long = "type")]
        relation_type: Option<String>,

        #[arg(long)]
        source_record: Option<String>,

        #[arg(long)]
        target_record: Option<String>,
    },
    Assumption {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Attempt {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    AttemptStatus {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        record: String,

        #[arg(long)]
        record_version: String,

        #[arg(long)]
        status: String,

        #[arg(long)]
        rationale: String,
    },
    AssumptionStatus {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        record: String,

        #[arg(long)]
        record_version: String,

        #[arg(long)]
        status: String,

        #[arg(long)]
        rationale: String,
    },
    Decision {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Finding {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Handoff {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Question {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
    },
    Risk {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        head: String,

        #[arg(long)]
        statement: String,

        #[arg(long)]
        scope_json: Option<String>,
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
        Command::Why {
            store,
            branch,
            commit,
            entity,
            evidence,
        } => {
            let engine = Engine::open(store)?;
            let target = match (branch, commit) {
                (Some(branch_id), None) => {
                    WhyQueryTarget::branch_head(BranchId::parse_canonical(&branch_id)?)
                }
                (None, Some(commit_id)) => {
                    WhyQueryTarget::commit(CommitId::parse_canonical(&commit_id)?)
                }
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "why requires exactly one of --branch or --commit".to_owned(),
                    ));
                }
            };
            let options = match (entity, evidence) {
                (Some(entity_id), None) => {
                    WhyQueryOptions::for_entity(target, EntityId::parse_canonical(&entity_id)?)
                }
                (None, Some(evidence_id)) => WhyQueryOptions::for_evidence(
                    target,
                    EvidenceId::parse_canonical(&evidence_id)?,
                ),
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "why requires exactly one of --entity or --evidence".to_owned(),
                    ));
                }
            };
            Ok(render_why(&engine.why(options)?))
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
        Command::Record {
            command:
                RecordCommand::Show {
                    store,
                    commit,
                    record,
                },
        } => {
            let engine = Engine::open(store)?;
            render_record_show(&engine.record_at(
                CommitId::parse_canonical(&commit)?,
                EntityId::parse_canonical(&record)?,
            )?)
        }
        Command::Record {
            command:
                RecordCommand::List {
                    store,
                    commit,
                    kind,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = RecordListOptions::new(CommitId::parse_canonical(&commit)?);
            if let Some(kind) = kind {
                options = options.with_kind(parse_record_kind(&kind)?);
            }
            Ok(render_record_list(&engine.records_at(options)?))
        }
        Command::Record {
            command:
                RecordCommand::LinkInvalidates {
                    store,
                    branch,
                    head,
                    source_record,
                    target_record,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let relation =
                engine.create_record_relation(RecordRelationCreateOptions::invalidates(
                    BranchId::parse_canonical(&branch)?,
                    CommitId::parse_canonical(&head)?,
                    EntityId::parse_canonical(&source_record)?,
                    EntityId::parse_canonical(&target_record)?,
                    rationale,
                )?)?;
            Ok(render_record_relation_create(&relation))
        }
        Command::Record {
            command:
                RecordCommand::RelationList {
                    store,
                    commit,
                    relation_type,
                    source_record,
                    target_record,
                },
        } => {
            let engine = Engine::open(store)?;
            let mut options = RecordRelationListOptions::new(CommitId::parse_canonical(&commit)?);
            if let Some(relation_type) = relation_type {
                options = options.with_relation_type(parse_record_relation_type(&relation_type)?);
            }
            if let Some(source_record) = source_record {
                options = options.with_source_record(EntityId::parse_canonical(&source_record)?);
            }
            if let Some(target_record) = target_record {
                options = options.with_target_record(EntityId::parse_canonical(&target_record)?);
            }
            Ok(render_record_relation_list(
                &engine.record_relations_at(options)?,
            ))
        }
        Command::Record {
            command:
                RecordCommand::Assumption {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::assumption(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Attempt {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::attempt(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::AttemptStatus {
                    store,
                    branch,
                    head,
                    record,
                    record_version,
                    status,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let branch_id = BranchId::parse_canonical(&branch)?;
            let head_id = CommitId::parse_canonical(&head)?;
            let record_id = EntityId::parse_canonical(&record)?;
            let record_version_id = EntityVersionId::parse_canonical(&record_version)?;
            let options = match parse_attempt_record_status(&status)? {
                RecordStatus::Succeeded => RecordTransitionOptions::complete_attempt_succeeded(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                RecordStatus::Failed => RecordTransitionOptions::complete_attempt_failed(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                RecordStatus::Inconclusive => {
                    RecordTransitionOptions::complete_attempt_inconclusive(
                        branch_id,
                        head_id,
                        record_id,
                        record_version_id,
                        rationale,
                    )?
                }
                _ => {
                    return Err(WorkVcsError::RecordInvalid(format!(
                        "attempt status {status:?} is not a transition target"
                    )));
                }
            };
            let record = engine.transition_record(options)?;
            Ok(render_record_transition(&record))
        }
        Command::Record {
            command:
                RecordCommand::AssumptionStatus {
                    store,
                    branch,
                    head,
                    record,
                    record_version,
                    status,
                    rationale,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let branch_id = BranchId::parse_canonical(&branch)?;
            let head_id = CommitId::parse_canonical(&head)?;
            let record_id = EntityId::parse_canonical(&record)?;
            let record_version_id = EntityVersionId::parse_canonical(&record_version)?;
            let options = match parse_assumption_record_status(&status)? {
                RecordStatus::Validated => RecordTransitionOptions::validate_assumption(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                RecordStatus::Invalidated => RecordTransitionOptions::invalidate_assumption(
                    branch_id,
                    head_id,
                    record_id,
                    record_version_id,
                    rationale,
                )?,
                _ => {
                    return Err(WorkVcsError::RecordInvalid(format!(
                        "assumption status {status:?} is not a transition target"
                    )));
                }
            };
            let record = engine.transition_record(options)?;
            Ok(render_record_transition(&record))
        }
        Command::Record {
            command:
                RecordCommand::Finding {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::finding(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Handoff {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::handoff(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Decision {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::decision(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Question {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::question(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
        }
        Command::Record {
            command:
                RecordCommand::Risk {
                    store,
                    branch,
                    head,
                    statement,
                    scope_json,
                },
        } => {
            let mut engine = Engine::open(store)?;
            let mut options = RecordCreateOptions::risk(
                BranchId::parse_canonical(&branch)?,
                CommitId::parse_canonical(&head)?,
                statement,
            )?;
            if let Some(scope_json) = scope_json {
                options = options.with_scope(parse_cli_object("record scope", &scope_json)?)?;
            }
            let record = engine.create_record(options)?;
            Ok(render_record_create(&record))
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
        Command::Next { store, session } => {
            let mut engine = Engine::open(store)?;
            let next =
                engine.next_work(NextWorkOptions::new(SessionId::parse_canonical(&session)?))?;
            Ok(render_next_work(&next))
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

fn parse_assumption_record_status(value: &str) -> Result<RecordStatus> {
    match value {
        "validated" => Ok(RecordStatus::Validated),
        "invalidated" => Ok(RecordStatus::Invalidated),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "assumption status {other:?} is not in the CLI transition vocabulary"
        ))),
    }
}

fn parse_attempt_record_status(value: &str) -> Result<RecordStatus> {
    match value {
        "succeeded" => Ok(RecordStatus::Succeeded),
        "failed" => Ok(RecordStatus::Failed),
        "inconclusive" => Ok(RecordStatus::Inconclusive),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "attempt status {other:?} is not in the CLI transition vocabulary"
        ))),
    }
}

fn parse_record_kind(value: &str) -> Result<RecordKind> {
    match value {
        "assumption" => Ok(RecordKind::Assumption),
        "attempt" => Ok(RecordKind::Attempt),
        "decision" => Ok(RecordKind::Decision),
        "finding" => Ok(RecordKind::Finding),
        "handoff" => Ok(RecordKind::Handoff),
        "question" => Ok(RecordKind::Question),
        "risk" => Ok(RecordKind::Risk),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record kind {other:?} is not in the CLI vocabulary"
        ))),
    }
}

fn parse_record_relation_type(value: &str) -> Result<RecordRelationType> {
    match value {
        "invalidates" => Ok(RecordRelationType::Invalidates),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "record relation type {other:?} is not in the CLI vocabulary"
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

fn render_record_create(record: &RecordCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrecord_entity_id={}\nrecord_entity_version_id={}\nrecord_state_digest={}\nwork_state_digest={}\nrecord_kind={}\nrecord_status={}\n",
        record.workspace_id,
        record.branch_id,
        record.previous_head_commit_id,
        record.commit_id,
        record.changeset_id,
        record.operation_id,
        record.record_entity_id,
        record.record_entity_version_id,
        record.record_state_digest,
        record.work_state_digest,
        record.state.kind,
        record.state.status
    )
}

fn render_record_transition(record: &RecordTransitionCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrecord_entity_id={}\nprevious_record_entity_version_id={}\nrecord_entity_version_id={}\nrecord_state_digest={}\nwork_state_digest={}\nrecord_kind={}\nprevious_record_status={}\nrecord_status={}\n",
        record.workspace_id,
        record.branch_id,
        record.previous_head_commit_id,
        record.commit_id,
        record.changeset_id,
        record.operation_id,
        record.record_entity_id,
        record.previous_record_entity_version_id,
        record.record_entity_version_id,
        record.record_state_digest,
        record.work_state_digest,
        record.state.kind,
        record.previous_state.status,
        record.state.status
    )
}

fn render_record_show(record: &RecordSnapshot) -> Result<String> {
    let statement_json = serde_json::to_string(&record.state.statement).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("record statement encode failed: {error}"))
    })?;
    let scope_json = String::from_utf8(canonical_bytes(&record.state.scope)?).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("record scope encode produced non-UTF-8: {error}"))
    })?;
    Ok(format!(
        "workspace_id={}\ncommit_id={}\nrecord_entity_id={}\nrecord_entity_version_id={}\nrecord_state_digest={}\nrecord_kind={}\nrecord_status={}\nrecord_statement_json={}\nrecord_scope_json={}\n",
        record.workspace_id,
        record.commit_id,
        record.record_entity_id,
        record.record_entity_version_id,
        record.state_digest,
        record.state.kind,
        record.state.status,
        statement_json,
        scope_json
    ))
}

fn render_record_list(result: &RecordListResult) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nrecords={}\n",
        result.workspace_id,
        result.commit_id,
        result.records.len()
    );
    for (index, record) in result.records.iter().enumerate() {
        writeln!(
            output,
            "record.{index}.record_entity_id={}",
            record.record_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "record.{index}.record_entity_version_id={}",
            record.record_entity_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "record.{index}.record_state_digest={}",
            record.state_digest
        )
        .expect("write to String");
        writeln!(output, "record.{index}.record_kind={}", record.state.kind)
            .expect("write to String");
        writeln!(
            output,
            "record.{index}.record_status={}",
            record.state.status
        )
        .expect("write to String");
    }
    output
}

fn render_record_relation_create(relation: &RecordRelationCreateCommit) -> String {
    format!(
        "workspace_id={}\nbranch_id={}\nprevious_head_commit_id={}\ncommit_id={}\nchangeset_id={}\noperation_id={}\nrelation_id={}\nrelation_version_id={}\nrelation_type={}\nsource_record_entity_id={}\ntarget_record_entity_id={}\nrelation_state_digest={}\nwork_state_digest={}\n",
        relation.workspace_id,
        relation.branch_id,
        relation.previous_head_commit_id,
        relation.commit_id,
        relation.changeset_id,
        relation.operation_id,
        relation.relation_id,
        relation.relation_version_id,
        relation.relation_type,
        relation.source_record_entity_id,
        relation.target_record_entity_id,
        relation.relation_state_digest,
        relation.work_state_digest
    )
}

fn render_record_relation_list(result: &RecordRelationListResult) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nrelations={}\n",
        result.workspace_id,
        result.commit_id,
        result.relations.len()
    );
    for (index, relation) in result.relations.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.relation_id={}",
            relation.relation_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            relation.relation_version_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_type={}",
            relation.relation_type
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.source_record_entity_id={}",
            relation.source_record_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.target_record_entity_id={}",
            relation.target_record_entity_id
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            relation.state_digest
        )
        .expect("write to String");
    }
    output
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

fn render_next_work(result: &NextWorkResult) -> String {
    let selected = result.claim_next.selected.is_some();
    let focus_entity_id = result
        .context
        .session
        .focus
        .as_ref()
        .map(|focus| focus.focus_entity_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let runnable_ready = result
        .context
        .runnable_tasks
        .candidates
        .iter()
        .filter(|candidate| candidate.runnable)
        .count();
    let mut output = format!(
        "session_id={}\nworkspace_id={}\nbranch_id={}\nhead_commit_id={}\ninspected_candidates={}\nselected={}\ncontext_focus_entity_id={}\ncontext_workspaces={}\ncontext_runnable_candidates={}\ncontext_runnable_ready={}\n",
        result.claim_next.session_id,
        result.claim_next.workspace_id,
        result.claim_next.branch_id,
        result.claim_next.head_commit_id,
        result.claim_next.inspected_candidates,
        selected,
        focus_entity_id,
        result.context.session.context_workspaces.len(),
        result.context.runnable_tasks.candidates.len(),
        runnable_ready
    );
    if let Some(claim) = &result.claim_next.selected {
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

fn render_why(result: &WhyQueryResult) -> String {
    let mut output = String::new();
    match result.target.target {
        WhyQueryTarget::BranchHead(branch_id) => {
            writeln!(output, "target_kind=branch_head").expect("write to String");
            writeln!(output, "target_branch_id={branch_id}").expect("write to String");
        }
        WhyQueryTarget::Commit(commit_id) => {
            writeln!(output, "target_kind=commit").expect("write to String");
            writeln!(output, "target_commit_id={commit_id}").expect("write to String");
        }
    }
    writeln!(output, "workspace_id={}", result.target.workspace_id).expect("write to String");
    writeln!(output, "commit_id={}", result.target.commit_id).expect("write to String");
    writeln!(output, "state_digest={}", result.target.state_digest).expect("write to String");
    match result.subject {
        ResolvedWhyQuerySubject::Entity {
            entity_id,
            entity_version_id,
        } => {
            writeln!(output, "subject_kind=entity").expect("write to String");
            writeln!(output, "subject_entity_id={entity_id}").expect("write to String");
            writeln!(output, "subject_entity_version_id={entity_version_id}")
                .expect("write to String");
        }
        ResolvedWhyQuerySubject::Evidence { evidence_id } => {
            writeln!(output, "subject_kind=evidence").expect("write to String");
            writeln!(output, "subject_evidence_id={evidence_id}").expect("write to String");
        }
    }
    writeln!(output, "relation_edges={}", result.relation_edges.len()).expect("write to String");
    for (index, edge) in result.relation_edges.iter().enumerate() {
        writeln!(
            output,
            "relation.{index}.relation_kind={}",
            why_relation_kind(edge.relation_kind)
        )
        .expect("write to String");
        writeln!(
            output,
            "relation.{index}.direction={}",
            why_relation_direction(edge.direction)
        )
        .expect("write to String");
        writeln!(output, "relation.{index}.relation_id={}", edge.relation_id)
            .expect("write to String");
        writeln!(
            output,
            "relation.{index}.relation_version_id={}",
            edge.relation_version_id
        )
        .expect("write to String");
        render_why_endpoint(&mut output, index, "source", edge.source);
        render_why_endpoint(&mut output, index, "target", edge.target);
        writeln!(
            output,
            "relation.{index}.relation_state_digest={}",
            edge.state_digest
        )
        .expect("write to String");
    }
    writeln!(
        output,
        "deferred_relation_families={}",
        result.deferred_relation_families.len()
    )
    .expect("write to String");
    for (index, family) in result.deferred_relation_families.iter().enumerate() {
        writeln!(
            output,
            "deferred_relation_family.{index}={}",
            why_deferred_relation_family(*family)
        )
        .expect("write to String");
    }
    output
}

fn render_why_endpoint(
    output: &mut String,
    index: usize,
    side: &str,
    endpoint: WhyRelationEndpoint,
) {
    match endpoint {
        WhyRelationEndpoint::Entity {
            entity_kind,
            entity_id,
        } => {
            writeln!(output, "relation.{index}.{side}_kind=entity").expect("write to String");
            writeln!(
                output,
                "relation.{index}.{side}_entity_kind={}",
                why_entity_kind(entity_kind)
            )
            .expect("write to String");
            writeln!(output, "relation.{index}.{side}_entity_id={entity_id}")
                .expect("write to String");
        }
        WhyRelationEndpoint::Evidence { evidence_id } => {
            writeln!(output, "relation.{index}.{side}_kind=evidence").expect("write to String");
            writeln!(output, "relation.{index}.{side}_evidence_id={evidence_id}")
                .expect("write to String");
        }
    }
}

fn why_relation_kind(kind: WhyRelationKind) -> &'static str {
    match kind {
        WhyRelationKind::PrimaryContainment => "primary_containment",
        WhyRelationKind::StructuralReference => "structural_reference",
        WhyRelationKind::Verifies => "verifies",
        WhyRelationKind::EvidencedBy => "evidenced_by",
        WhyRelationKind::RecordInvalidates => "record_invalidates",
    }
}

fn why_relation_direction(direction: WhyRelationDirection) -> &'static str {
    match direction {
        WhyRelationDirection::Incoming => "incoming",
        WhyRelationDirection::Outgoing => "outgoing",
    }
}

fn why_entity_kind(kind: WhyEntityKind) -> &'static str {
    match kind {
        WhyEntityKind::Goal => "goal",
        WhyEntityKind::Plan => "plan",
        WhyEntityKind::Task => "task",
        WhyEntityKind::AcceptanceCriterion => "acceptance_criterion",
        WhyEntityKind::VerificationRequirement => "verification_requirement",
        WhyEntityKind::Verification => "verification",
        WhyEntityKind::Record => "record",
    }
}

fn why_deferred_relation_family(family: WhyDeferredRelationFamily) -> &'static str {
    match family {
        WhyDeferredRelationFamily::Evolution => "evolution",
        WhyDeferredRelationFamily::Epistemic => "epistemic",
    }
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
                "why",
                "workspace",
                "branch",
                "task",
                "ac",
                "vr",
                "resource",
                "record",
                "session",
                "claim",
                "context",
                "next",
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

    #[test]
    fn cli_runs_next_workflow() {
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
            "Next workflow task",
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

        let next = run(
            Cli::try_parse_from(["workvcs", "next", store, "--session", &session_id])
                .expect("parse next"),
        )
        .expect("next work");
        let claim_id = value(&next, "claim_id");
        assert!(next.contains("selected=true"));
        assert!(next.contains(&format!("task_entity_id={task_id}")));
        assert!(next.contains(&format!("context_focus_entity_id={task_id}")));
        assert!(next.contains("context_runnable_candidates=1"));

        let runnable = run(Cli::try_parse_from([
            "workvcs",
            "runnable",
            "tasks",
            store,
            "--session",
            &session_id,
        ])
        .expect("parse runnable"))
        .expect("runnable after next");
        assert!(runnable.contains(&format!("candidate.0.claim=claimed_by_session:{claim_id}")));
    }

    #[test]
    fn cli_runs_record_finding_workflow() {
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
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Schema validation has no drift",
            "--scope-json",
            r#"{"subject":"schema"}"#,
        ])
        .expect("parse record finding"))
        .expect("create finding record");

        assert!(record.contains("record_kind=finding"));
        assert!(record.contains("record_status=active"));
        assert!(record.contains("record_entity_id="));
        assert!(record.contains("commit_id="));
    }

    #[test]
    fn cli_runs_record_assumption_workflow() {
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
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Serialized writes are sufficient",
        ])
        .expect("parse record assumption"))
        .expect("create assumption record");

        assert!(record.contains("record_kind=assumption"));
        assert!(record.contains("record_status=unverified"));
        assert!(record.contains("record_entity_id="));

        let transition = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
            "--record-version",
            &value(&record, "record_entity_version_id"),
            "--status",
            "validated",
            "--rationale",
            "Confirmed by local validation",
        ])
        .expect("parse assumption status"))
        .expect("transition assumption");
        assert!(transition.contains("previous_record_status=unverified"));
        assert!(transition.contains("record_status=validated"));
    }

    #[test]
    fn cli_runs_record_decision_workflow() {
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
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "decision",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Use SQLite for V0.1 storage",
            "--scope-json",
            r#"{"decision_scope":"storage"}"#,
        ])
        .expect("parse record decision"))
        .expect("create decision record");

        assert!(record.contains("record_kind=decision"));
        assert!(record.contains("record_status=active"));
        assert!(record.contains("record_entity_id="));
    }

    #[test]
    fn cli_runs_question_and_risk_record_workflows() {
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
        let head = value(&workspace, "genesis_commit_id");

        let question = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "question",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Which replay invariant should be exposed next?",
        ])
        .expect("parse record question"))
        .expect("create question record");
        assert!(question.contains("record_kind=question"));
        assert!(question.contains("record_status=active"));

        let risk = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "risk",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&question, "commit_id"),
            "--statement",
            "Manual ordering remains unresolved",
        ])
        .expect("parse record risk"))
        .expect("create risk record");
        assert!(risk.contains("record_kind=risk"));
        assert!(risk.contains("record_status=active"));
    }

    #[test]
    fn cli_lists_record_workflow() {
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
        let head = value(&workspace, "genesis_commit_id");

        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Schema validation has no drift",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let assumption = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--statement",
            "Serialized writes are sufficient",
        ])
        .expect("parse assumption"))
        .expect("create assumption");

        let list = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&assumption, "commit_id"),
        ])
        .expect("parse record list"))
        .expect("list records");
        assert_eq!(value(&list, "records"), "2");
        assert!(list.contains("record_kind=finding"));
        assert!(list.contains("record_kind=assumption"));
        assert!(list.contains("record_status=active"));
        assert!(list.contains("record_status=unverified"));

        let filtered = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&assumption, "commit_id"),
            "--kind",
            "assumption",
        ])
        .expect("parse filtered record list"))
        .expect("list filtered records");
        assert_eq!(value(&filtered, "records"), "1");
        assert!(filtered.contains("record_kind=assumption"));
        assert!(!filtered.contains("record_kind=finding"));
    }

    #[test]
    fn cli_shows_record_workflow() {
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
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Line one\nLine two",
            "--scope-json",
            r#"{"b":2,"a":1}"#,
        ])
        .expect("parse finding"))
        .expect("create finding");
        let show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
        ])
        .expect("parse record show"))
        .expect("show record");

        assert!(show.contains("record_kind=finding"));
        assert!(show.contains("record_status=active"));
        assert!(show.contains("record_statement_json=\"Line one\\nLine two\""));
        assert!(show.contains("record_scope_json={\"a\":1,\"b\":2}"));
        assert_eq!(
            value(&show, "record_entity_version_id"),
            value(&record, "record_entity_version_id")
        );
    }

    #[test]
    fn cli_runs_attempt_record_workflow() {
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
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "attempt",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Run the next validation command",
        ])
        .expect("parse attempt"))
        .expect("create attempt");
        assert!(record.contains("record_kind=attempt"));
        assert!(record.contains("record_status=running"));

        let list = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--kind",
            "attempt",
        ])
        .expect("parse attempt list"))
        .expect("list attempt records");
        assert_eq!(value(&list, "records"), "1");
        assert!(list.contains("record_kind=attempt"));
        assert!(list.contains("record_status=running"));

        let show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
        ])
        .expect("parse attempt show"))
        .expect("show attempt record");
        assert!(show.contains("record_kind=attempt"));
        assert!(show.contains("record_status=running"));
        assert!(show.contains("record_statement_json=\"Run the next validation command\""));

        let terminal = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "attempt-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
            "--record-version",
            &value(&record, "record_entity_version_id"),
            "--status",
            "failed",
            "--rationale",
            "Validation command failed",
        ])
        .expect("parse attempt status"))
        .expect("transition attempt");
        assert!(terminal.contains("previous_record_status=running"));
        assert!(terminal.contains("record_status=failed"));

        let terminal_show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&terminal, "commit_id"),
            "--record",
            &value(&terminal, "record_entity_id"),
        ])
        .expect("parse terminal attempt show"))
        .expect("show terminal attempt record");
        assert!(terminal_show.contains("record_kind=attempt"));
        assert!(terminal_show.contains("record_status=failed"));
    }

    #[test]
    fn cli_runs_handoff_record_workflow() {
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
        let head = value(&workspace, "genesis_commit_id");

        let record = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "handoff",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Continue with the next runnable task",
            "--scope-json",
            r#"{"focus":"task:next"}"#,
        ])
        .expect("parse handoff"))
        .expect("create handoff");
        assert!(record.contains("record_kind=handoff"));
        assert!(record.contains("record_status=active"));

        let list = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "list",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--kind",
            "handoff",
        ])
        .expect("parse handoff list"))
        .expect("list handoff records");
        assert_eq!(value(&list, "records"), "1");
        assert!(list.contains("record_kind=handoff"));

        let show = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "show",
            store,
            "--commit",
            &value(&record, "commit_id"),
            "--record",
            &value(&record, "record_entity_id"),
        ])
        .expect("parse handoff show"))
        .expect("show handoff");
        assert!(show.contains("record_kind=handoff"));
        assert!(show.contains("record_scope_json={\"focus\":\"task:next\"}"));
    }

    #[test]
    fn cli_links_finding_to_invalidated_assumption() {
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
        let head = value(&workspace, "genesis_commit_id");

        let assumption = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption",
            store,
            "--branch",
            &branch,
            "--head",
            &head,
            "--statement",
            "Serialized writes are sufficient",
        ])
        .expect("parse assumption"))
        .expect("create assumption");
        let finding = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "finding",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&assumption, "commit_id"),
            "--statement",
            "Concurrent writer test failed",
        ])
        .expect("parse finding"))
        .expect("create finding");
        let invalidated = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "assumption-status",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&finding, "commit_id"),
            "--record",
            &value(&assumption, "record_entity_id"),
            "--record-version",
            &value(&assumption, "record_entity_version_id"),
            "--status",
            "invalidated",
            "--rationale",
            "Concurrent writer test failed",
        ])
        .expect("parse assumption status"))
        .expect("invalidate assumption");

        let relation = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "link-invalidates",
            store,
            "--branch",
            &branch,
            "--head",
            &value(&invalidated, "commit_id"),
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&assumption, "record_entity_id"),
            "--rationale",
            "Finding invalidates the assumption",
        ])
        .expect("parse link invalidates"))
        .expect("link invalidates");
        assert!(relation.contains("relation_type=invalidates"));
        assert_eq!(
            value(&relation, "source_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(&relation, "target_record_entity_id"),
            value(&assumption, "record_entity_id")
        );

        let state = run(Cli::try_parse_from([
            "workvcs",
            "show-at",
            store,
            "--commit",
            &value(&relation, "commit_id"),
        ])
        .expect("parse show-at"))
        .expect("show relation commit");
        assert!(state.contains("relations=1"));
        assert!(state.contains(&format!("relation={}", value(&relation, "relation_id"))));

        let listed = run(Cli::try_parse_from([
            "workvcs",
            "record",
            "relation-list",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--type",
            "invalidates",
            "--source-record",
            &value(&finding, "record_entity_id"),
            "--target-record",
            &value(&assumption, "record_entity_id"),
        ])
        .expect("parse relation list"))
        .expect("list record relations");
        assert!(listed.contains("relations=1"));
        assert_eq!(
            value(&listed, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
        assert!(listed.contains("relation.0.relation_type=invalidates"));
        assert_eq!(
            value(&listed, "relation.0.source_record_entity_id"),
            value(&finding, "record_entity_id")
        );
        assert_eq!(
            value(&listed, "relation.0.target_record_entity_id"),
            value(&assumption, "record_entity_id")
        );

        let why_finding = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--commit",
            &value(&relation, "commit_id"),
            "--entity",
            &value(&finding, "record_entity_id"),
        ])
        .expect("parse why finding"))
        .expect("why finding");
        assert!(why_finding.contains("target_kind=commit"));
        assert!(why_finding.contains("subject_kind=entity"));
        assert!(why_finding.contains("relation_edges=1"));
        assert!(why_finding.contains("relation.0.relation_kind=record_invalidates"));
        assert!(why_finding.contains("relation.0.direction=outgoing"));
        assert!(why_finding.contains("relation.0.source_entity_kind=record"));
        assert!(why_finding.contains("relation.0.target_entity_kind=record"));
        assert_eq!(
            value(&why_finding, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );

        let why_assumption = run(Cli::try_parse_from([
            "workvcs",
            "why",
            store,
            "--branch",
            &branch,
            "--entity",
            &value(&assumption, "record_entity_id"),
        ])
        .expect("parse why assumption"))
        .expect("why assumption");
        assert!(why_assumption.contains("target_kind=branch_head"));
        assert!(why_assumption.contains("relation.0.relation_kind=record_invalidates"));
        assert!(why_assumption.contains("relation.0.direction=incoming"));
        assert_eq!(
            value(&why_assumption, "relation.0.relation_id"),
            value(&relation, "relation_id")
        );
    }

    fn value(output: &str, key: &str) -> String {
        output
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("missing {key} in output:\n{output}"))
            .to_owned()
    }
}
