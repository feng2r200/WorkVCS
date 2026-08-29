use clap::{ArgGroup, Parser, Subcommand};
use std::fmt::Write as _;
use std::path::PathBuf;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, AcceptanceCriterionEffectiveStatus,
    ApplicabilityResourceObservationStatus, ApplicabilityResourceStampInput, BranchId,
    CanonicalValue, CommitId, Digest, Engine, EntityId, EntityVersionId, HistoryEntry,
    HistoryQueryOptions, ReplayedState, ResourceCreateOptions, ResourceCreateResult, ResourceId,
    ResourceObservationCreateOptions, ResourceObservationCreateResult, ResourceObservationId,
    Result, StoreInitOptions, TaskCreateCommit, TaskCreateOptions, TaskStatus,
    TaskTransitionCommit, TaskTransitionOptions, VerificationApplicabilityCacheSnapshot,
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
                "task",
                "ac",
                "vr",
                "resource",
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

    fn value(output: &str, key: &str) -> String {
        output
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_else(|| panic!("missing {key} in output:\n{output}"))
            .to_owned()
    }
}
