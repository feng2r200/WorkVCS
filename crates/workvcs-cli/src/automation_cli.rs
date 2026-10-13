use super::*;
use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use workvcs_core::{
    AutomationArtifactKind, AutomationCatalogManifest, AutomationCatalogSource, AutomationLane,
    AutomationRegistration, AutomationRegistrationProposal, AutomationRolePolicy,
    AutomationSourceStatus, SearchDerivedStatus, SourceArchiveInspectOptions,
    apply_automation_registration, preview_automation_registration, publish_automation_catalog,
    read_automation_catalog, read_automation_registry, revoke_automation_registration,
};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

const AUTOMATION_PROTOCOL_VERSION: u32 = 1;
const MAX_DAEMON_FRAME_BYTES: usize = 16 * 1024;
const MAX_WORKER_STREAM_BYTES: usize = 64 * 1024;
const RECONCILE_QUERY: &str = "automation reconciliation";
const RECONCILE_WORKER_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum AutomationFormatArg {
    Json,
    KeyValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum AutomationRolePolicyArg {
    UserOnly,
    UserAndAssistant,
}

impl From<AutomationRolePolicyArg> for AutomationRolePolicy {
    fn from(value: AutomationRolePolicyArg) -> Self {
        match value {
            AutomationRolePolicyArg::UserOnly => Self::UserOnly,
            AutomationRolePolicyArg::UserAndAssistant => Self::UserAndAssistant,
        }
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum AutomationCommand {
    #[command(about = "Preview explicit local automation registration without writing")]
    RegisterPreview {
        #[arg(long)]
        cwd: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[command(flatten)]
        locator: ProjectLocatorArgs,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        registration_id: Option<String>,
        #[arg(long, required = true)]
        rollout_root: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = AutomationRolePolicyArg::UserAndAssistant)]
        role_policy: AutomationRolePolicyArg,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "Apply one previewed registration against an exact automation basis")]
    RegisterApply {
        #[arg(long)]
        cwd: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[command(flatten)]
        locator: ProjectLocatorArgs,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        registration_id: Option<String>,
        #[arg(long, required = true)]
        rollout_root: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = AutomationRolePolicyArg::UserAndAssistant)]
        role_policy: AutomationRolePolicyArg,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        expected_digest: String,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "List explicit registrations, including revoked history")]
    List {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "Revoke one registration without deleting derived artifacts")]
    Revoke {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        registration_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        expected_digest: String,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "Run one foreground-compatible complete reconciliation round")]
    Reconcile {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, help = "Required acknowledgement that this is one finite round")]
        once: bool,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "Inspect or query one exact immutable global derived catalog")]
    Catalog {
        #[command(subcommand)]
        command: AutomationCatalogCommand,
    },
    #[command(about = "Run or control the optional foreground local automation service")]
    Daemon {
        #[command(subcommand)]
        command: AutomationDaemonCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AutomationCatalogCommand {
    Show {
        #[arg(long)]
        catalog: PathBuf,
    },
    Latest {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Query {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(
            long,
            help = "Exact catalog path; omit to select the newest current-basis catalog"
        )]
        catalog: Option<PathBuf>,
        #[arg(long)]
        query: String,
        #[arg(long)]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AutomationDaemonCommand {
    #[command(about = "Run in the foreground; does not install or enable a startup service")]
    Run {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, default_value_t = 900)]
        periodic_seconds: u64,
    },
    Status {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Reconcile {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Stop {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReconcileReport {
    status: String,
    complete: bool,
    automation_registry_revision: u64,
    automation_registry_digest_sha256: String,
    registrations_considered: usize,
    registrations_ready: usize,
    registrations_quarantined: usize,
    rollout_files_discovered: usize,
    rollout_files_new: usize,
    rollout_files_appended: usize,
    rollout_files_unchanged: usize,
    rollout_files_rewritten: usize,
    rollout_files_deleted: usize,
    rollout_archives_ready: usize,
    rollout_archives_failed: usize,
    canonical_generations_ready: usize,
    canonical_generations_failed: usize,
    catalog_path: PathBuf,
    catalog_manifest_sha256: String,
    canonical_source_written: bool,
    project_registry_written: bool,
    source_files_written: bool,
    existing_artifacts_deleted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFingerprint {
    bytes: u64,
    sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceArchiveReference {
    artifact_path: PathBuf,
    artifact_manifest_sha256: String,
    source_path: PathBuf,
    source_bytes: u64,
    source_sha256: String,
    source_anchor: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceInventoryEntry {
    fingerprint: SourceFingerprint,
    archive: Option<SourceArchiveReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceInventory {
    schema_version: u32,
    observed_at_us: u128,
    files: BTreeMap<String, SourceInventoryEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceInventoryV1 {
    schema_version: u32,
    observed_at_us: u128,
    files: BTreeMap<String, SourceFingerprint>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceChange {
    New,
    Appended,
    Unchanged,
    Rewritten,
}

#[derive(Debug, Serialize)]
struct CatalogQueryReport {
    status: String,
    complete: bool,
    catalog_path: PathBuf,
    catalog_manifest_sha256: String,
    query: String,
    sources_requested: usize,
    sources_succeeded: usize,
    sources_failed: usize,
    result_limit: Option<usize>,
    matched_items: usize,
    omitted_results: usize,
    source_reports: Vec<CatalogQuerySourceReport>,
    items: Vec<CatalogQueryItem>,
}

#[derive(Debug, Serialize)]
struct CatalogLatestReport {
    catalog_path: PathBuf,
    catalog_manifest_sha256: String,
    manifest: AutomationCatalogManifest,
}

#[derive(Debug, Serialize)]
struct CatalogQuerySourceReport {
    registration_id: String,
    project_ref_id: ProjectRefId,
    lane: AutomationLane,
    artifact_kind: AutomationArtifactKind,
    artifact_path: PathBuf,
    source_anchor: String,
    status: String,
    error_code: Option<String>,
    error: Option<String>,
    matched_items: usize,
}

#[derive(Debug, Serialize)]
struct CatalogQueryItem {
    project_ref_id: ProjectRefId,
    lane: AutomationLane,
    artifact_kind: AutomationArtifactKind,
    source_anchor: String,
    source_local_rank: usize,
    source_score: f64,
    federated_score: f64,
    payload: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DaemonRequest {
    version: u32,
    action: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DaemonResponse {
    version: u32,
    status: String,
    action: String,
    message: String,
    worker_exit_code: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DaemonLedger {
    protocol_version: u32,
    pid: u32,
    started_at_us: u128,
    updated_at_us: u128,
    last_trigger: String,
    last_worker_status: Option<String>,
    last_worker_exit_code: Option<i32>,
    last_worker_timed_out: bool,
    last_worker_output_truncated: bool,
    last_worker_evidence_path: Option<PathBuf>,
    last_worker_stdout_bytes: u64,
    last_worker_stdout_sha256: Option<String>,
    last_worker_stderr_bytes: u64,
    last_worker_stderr_sha256: Option<String>,
    last_error: Option<String>,
    last_catalog_path: Option<PathBuf>,
    last_catalog_manifest_sha256: Option<String>,
    last_catalog_complete: Option<bool>,
    watcher_events: u64,
    watcher_errors: u64,
    watcher_coalesced_events: u64,
    last_watcher_error: Option<String>,
    rejected_requests: u64,
    last_protocol_error: Option<String>,
    daemon_enabled_by_default: bool,
    launch_agent_installed: bool,
}

#[derive(Debug)]
struct WorkerOutcome {
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: CapturedStream,
    stderr: CapturedStream,
}

#[derive(Debug)]
struct CapturedStream {
    bytes: Vec<u8>,
    total_bytes: u64,
    sha256: String,
    truncated: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkerEvidence {
    captured_at_us: u128,
    trigger: String,
    exit_code: Option<i32>,
    timed_out: bool,
    stdout_path: PathBuf,
    stdout_bytes: u64,
    stdout_sha256: String,
    stdout_truncated: bool,
    stderr_path: PathBuf,
    stderr_bytes: u64,
    stderr_sha256: String,
    stderr_truncated: bool,
}

#[cfg(unix)]
#[derive(Default)]
struct WatcherHealth {
    events: AtomicU64,
    errors: AtomicU64,
    coalesced_events: AtomicU64,
    last_error: Mutex<Option<String>>,
}

pub(crate) fn run_automation(command: AutomationCommand) -> Result<String> {
    match command {
        AutomationCommand::RegisterPreview {
            cwd,
            registry,
            locator,
            root,
            registration_id,
            rollout_root,
            role_policy,
            format,
        } => {
            let (root, proposal) = registration_proposal(
                cwd,
                registry,
                locator,
                root,
                registration_id,
                rollout_root,
                role_policy.into(),
            )?;
            let preview = preview_automation_registration(&root, proposal)?;
            render_value(&preview, format)
        }
        AutomationCommand::RegisterApply {
            cwd,
            registry,
            locator,
            root,
            registration_id,
            rollout_root,
            role_policy,
            expected_revision,
            expected_digest,
            format,
        } => {
            let (root, proposal) = registration_proposal(
                cwd,
                registry,
                locator,
                root,
                registration_id,
                rollout_root,
                role_policy.into(),
            )?;
            let result = apply_automation_registration(
                &root,
                expected_revision,
                &expected_digest,
                proposal,
            )?;
            render_value(&result, format)
        }
        AutomationCommand::List {
            registry,
            root,
            format,
        } => render_value(
            &read_automation_registry(&resolve_automation_root(root, registry.as_deref())?)?,
            format,
        ),
        AutomationCommand::Revoke {
            registry,
            root,
            registration_id,
            expected_revision,
            expected_digest,
            format,
        } => {
            let root = resolve_automation_root(root, registry.as_deref())?;
            let result = revoke_automation_registration(
                &root,
                expected_revision,
                &expected_digest,
                &registration_id,
            )?;
            render_value(&result, format)
        }
        AutomationCommand::Reconcile {
            registry,
            root,
            once,
            format,
        } => {
            if !once {
                return Err(WorkVcsError::QueryInvalid(
                    "automation reconcile requires --once; use automation daemon run for a resident coordinator"
                        .to_owned(),
                ));
            }
            let root = resolve_automation_root(root, registry.as_deref())?;
            let report = reconcile_once(&root, registry.as_deref())?;
            render_value(&report, format)
        }
        AutomationCommand::Catalog { command } => run_catalog(command),
        AutomationCommand::Daemon { command } => run_daemon_command(command),
    }
}

fn registration_proposal(
    cwd: PathBuf,
    registry: Option<PathBuf>,
    locator: ProjectLocatorArgs,
    root: Option<PathBuf>,
    registration_id: Option<String>,
    rollout_roots: Vec<PathBuf>,
    role_policy: AutomationRolePolicy,
) -> Result<(PathBuf, AutomationRegistrationProposal)> {
    let discovery = discover_project_readonly_with_locator(
        cwd,
        registry,
        locator.project_ref,
        locator.locator_context,
        locator.locator_adapter_context,
    )?;
    let project_ref_id = discovery.project_ref_id.ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid(
            "automation registration requires a verified registry-v2 ProjectRef".to_owned(),
        )
    })?;
    let registry_revision = discovery.registry_revision.ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid(
            "automation registration requires a versioned registry-v2 snapshot".to_owned(),
        )
    })?;
    let automation_root = root.unwrap_or_else(|| default_automation_root(&discovery.registry_path));
    let proposal = AutomationRegistrationProposal {
        registration_id: registration_id.unwrap_or_else(|| project_ref_id.to_string()),
        project_ref_id,
        store_path: PathBuf::from(&discovery.binding.store_path),
        store_id: discovery.binding.store_id,
        workspace_id: discovery.binding.workspace_id,
        branch_id: discovery.binding.branch_id,
        project_root: PathBuf::from(&discovery.current_identity.root),
        rollout_roots,
        role_policy,
        registry_revision,
        registry_digest: discovery.registry_digest.to_string(),
    };
    Ok((automation_root, proposal))
}

fn resolve_automation_root(root: Option<PathBuf>, registry: Option<&Path>) -> Result<PathBuf> {
    if let Some(root) = root {
        if !root.is_absolute() {
            return Err(WorkVcsError::QueryInvalid(
                "automation root must be absolute".to_owned(),
            ));
        }
        return canonicalize_destination(&root);
    }
    let effective = effective_registry_config(registry.map(Path::to_path_buf))?;
    let registry_path = absolute_cli_path("project registry", effective.registry_path)?;
    canonicalize_destination(&default_automation_root(&registry_path))
}

fn default_automation_root(registry_path: &Path) -> PathBuf {
    let mut sidecar = registry_path.as_os_str().to_os_string();
    sidecar.push(".d");
    PathBuf::from(sidecar).join("automation").join("v1")
}

fn canonicalize_destination(path: &Path) -> Result<PathBuf> {
    let mut existing = path;
    let mut missing = Vec::new();
    while !existing.exists() {
        let name = existing.file_name().ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "automation root has no existing ancestor: {}",
                path.display()
            ))
        })?;
        missing.push(name.to_os_string());
        existing = existing.parent().ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "automation root has no existing ancestor: {}",
                path.display()
            ))
        })?;
    }
    let mut canonical = existing
        .canonicalize()
        .map_err(storage_error("canonicalize automation root ancestor"))?;
    for component in missing.iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}

#[cfg(unix)]
struct ReconciliationLock {
    _file: File,
}

#[cfg(unix)]
impl ReconciliationLock {
    fn acquire(root: &Path) -> Result<Self> {
        let path = root.join(".reconcile.lock");
        let file = open_private_file(&path, true)?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Err(WorkVcsError::ControlPlaneInvalid(
                    "another automation reconciliation is already active".to_owned(),
                ));
            }
            return Err(storage_error("lock automation reconciliation")(error));
        }
        Ok(Self { _file: file })
    }
}

#[cfg(not(unix))]
struct ReconciliationLock;

#[cfg(not(unix))]
impl ReconciliationLock {
    fn acquire(_root: &Path) -> Result<Self> {
        Ok(Self)
    }
}

fn reconcile_once(root: &Path, project_registry: Option<&Path>) -> Result<ReconcileReport> {
    let _reconciliation_lock = ReconciliationLock::acquire(root)?;
    let registry = read_automation_registry(root)?;
    let mut catalog_sources = Vec::new();
    let mut registrations_ready = 0;
    let mut registrations_quarantined = 0;
    let mut rollout_files_discovered = 0;
    let mut rollout_files_new = 0;
    let mut rollout_files_appended = 0;
    let mut rollout_files_unchanged = 0;
    let mut rollout_files_rewritten = 0;
    let mut rollout_files_deleted = 0;
    let mut rollout_archives_ready = 0;
    let mut rollout_archives_failed = 0;
    let mut canonical_generations_ready = 0;
    let mut canonical_generations_failed = 0;
    let registrations = registry
        .registrations
        .iter()
        .filter(|registration| registration.enabled && !registration.revoked)
        .collect::<Vec<_>>();

    for registration in &registrations {
        let identity = SearchSourceIdentity::new(
            registration.project_ref_id,
            registration.store_id,
            registration.workspace_id,
            registration.branch_id,
        );
        let discovery = match rediscover_registration(registration, project_registry) {
            Ok(discovery) => discovery,
            Err(error) => {
                registrations_quarantined += 1;
                canonical_generations_failed += 1;
                catalog_sources.push(failed_catalog_source(
                    registration,
                    AutomationLane::CanonicalCurrent,
                    AutomationArtifactKind::LexicalGeneration,
                    root.join("quarantine").join(&registration.registration_id),
                    "binding_drift",
                    &error.to_string(),
                ));
                run_historical_reconciliation(
                    root,
                    registration,
                    identity,
                    &mut catalog_sources,
                    &mut rollout_files_discovered,
                    &mut rollout_files_new,
                    &mut rollout_files_appended,
                    &mut rollout_files_unchanged,
                    &mut rollout_files_rewritten,
                    &mut rollout_files_deleted,
                    &mut rollout_archives_ready,
                    &mut rollout_archives_failed,
                )?;
                continue;
            }
        };
        registrations_ready += 1;
        let canonical_root = root
            .join("generations")
            .join(&registration.registration_id)
            .join("lexical");
        match Engine::open_readonly(&registration.store_path) {
            Ok(engine) => {
                let mut options =
                    SearchQueryOptions::managed(identity, &canonical_root, RECONCILE_QUERY)?
                        .with_protected_path(discovery.registry_path.clone());
                if let Some(path) = discovery.routing_activation_path.clone() {
                    options = options.with_protected_path(path);
                }
                match engine.search(options) {
                    Ok(result) if result.status == SearchDerivedStatus::Ready => {
                        let manifest = result.manifest.ok_or_else(|| {
                            WorkVcsError::IntegrityInvalid(
                                "ready automation search result omitted its manifest".to_owned(),
                            )
                        })?;
                        canonical_generations_ready += 1;
                        catalog_sources.push(AutomationCatalogSource {
                            registration_id: registration.registration_id.clone(),
                            project_ref_id: registration.project_ref_id,
                            lane: AutomationLane::CanonicalCurrent,
                            artifact_kind: AutomationArtifactKind::LexicalGeneration,
                            artifact_path: result.index_path,
                            artifact_manifest_sha256: sha256_serialized(&manifest)?,
                            source_anchor: format!(
                                "commit:{};state:{}",
                                manifest.source_commit_id, manifest.source_state_digest
                            ),
                            status: AutomationSourceStatus::Ready,
                            error_code: None,
                            error: None,
                        });
                    }
                    Ok(result) => {
                        canonical_generations_failed += 1;
                        catalog_sources.push(failed_catalog_source(
                            registration,
                            AutomationLane::CanonicalCurrent,
                            AutomationArtifactKind::LexicalGeneration,
                            result.index_path,
                            result.reason_code.as_deref().unwrap_or("refresh_required"),
                            result
                                .reason
                                .as_deref()
                                .unwrap_or("derived generation is not ready"),
                        ));
                    }
                    Err(error) => {
                        canonical_generations_failed += 1;
                        catalog_sources.push(failed_catalog_source(
                            registration,
                            AutomationLane::CanonicalCurrent,
                            AutomationArtifactKind::LexicalGeneration,
                            canonical_root.clone(),
                            &error.code().to_string(),
                            &error.to_string(),
                        ));
                    }
                }
            }
            Err(error) => {
                canonical_generations_failed += 1;
                catalog_sources.push(failed_catalog_source(
                    registration,
                    AutomationLane::CanonicalCurrent,
                    AutomationArtifactKind::LexicalGeneration,
                    canonical_root,
                    &error.code().to_string(),
                    &error.to_string(),
                ));
            }
        }
        run_historical_reconciliation(
            root,
            registration,
            identity,
            &mut catalog_sources,
            &mut rollout_files_discovered,
            &mut rollout_files_new,
            &mut rollout_files_appended,
            &mut rollout_files_unchanged,
            &mut rollout_files_rewritten,
            &mut rollout_files_deleted,
            &mut rollout_archives_ready,
            &mut rollout_archives_failed,
        )?;
    }

    let publication = publish_automation_catalog(
        root,
        registry.revision,
        registry.digest_sha256.clone(),
        catalog_sources,
    )?;
    let ready_sources = publication
        .manifest
        .sources
        .iter()
        .filter(|source| source.status == AutomationSourceStatus::Ready)
        .count();
    Ok(ReconcileReport {
        status: if publication.manifest.complete {
            "completed".to_owned()
        } else if ready_sources == 0 {
            "failed".to_owned()
        } else {
            "partial_failure".to_owned()
        },
        complete: publication.manifest.complete,
        automation_registry_revision: registry.revision,
        automation_registry_digest_sha256: registry.digest_sha256,
        registrations_considered: registrations.len(),
        registrations_ready,
        registrations_quarantined,
        rollout_files_discovered,
        rollout_files_new,
        rollout_files_appended,
        rollout_files_unchanged,
        rollout_files_rewritten,
        rollout_files_deleted,
        rollout_archives_ready,
        rollout_archives_failed,
        canonical_generations_ready,
        canonical_generations_failed,
        catalog_path: publication.catalog_path,
        catalog_manifest_sha256: publication.manifest_sha256,
        canonical_source_written: false,
        project_registry_written: false,
        source_files_written: false,
        existing_artifacts_deleted: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_historical_reconciliation(
    root: &Path,
    registration: &AutomationRegistration,
    identity: SearchSourceIdentity,
    catalog_sources: &mut Vec<AutomationCatalogSource>,
    rollout_files_discovered: &mut usize,
    rollout_files_new: &mut usize,
    rollout_files_appended: &mut usize,
    rollout_files_unchanged: &mut usize,
    rollout_files_rewritten: &mut usize,
    rollout_files_deleted: &mut usize,
    rollout_archives_ready: &mut usize,
    rollout_archives_failed: &mut usize,
) -> Result<()> {
    let archive_root = root.join("history").join(&registration.registration_id);
    let previous_inventory = read_source_inventory(root, &registration.registration_id)?;
    let mut current_inventory = BTreeMap::new();

    for rollout_root in &registration.rollout_roots {
        let files = match collect_rollout_files(rollout_root) {
            Ok(files) => files,
            Err(error) => {
                for (path, entry) in previous_inventory
                    .files
                    .iter()
                    .filter(|(path, _)| source_belongs_to_root(path, rollout_root))
                {
                    retain_previous_archive(
                        registration,
                        identity,
                        entry,
                        catalog_sources,
                        rollout_archives_ready,
                        rollout_archives_failed,
                    );
                    current_inventory.insert(path.clone(), entry.clone());
                }
                *rollout_archives_failed += 1;
                catalog_sources.push(failed_catalog_source(
                    registration,
                    AutomationLane::HistoricalReference,
                    AutomationArtifactKind::SourceArchive,
                    rollout_root.clone(),
                    &error.code().to_string(),
                    &error.to_string(),
                ));
                continue;
            }
        };
        *rollout_files_discovered += files.len();
        for file in files {
            let path_key = file
                .to_str()
                .ok_or_else(|| {
                    WorkVcsError::QueryInvalid(
                        "registered rollout path must be valid UTF-8".to_owned(),
                    )
                })?
                .to_owned();
            let previous = previous_inventory.files.get(&path_key);
            let fingerprint = match fingerprint_source(&file) {
                Ok(fingerprint) => fingerprint,
                Err(error) => {
                    record_historical_observation_failure(
                        registration,
                        identity,
                        &path_key,
                        file,
                        previous,
                        &error,
                        &mut current_inventory,
                        catalog_sources,
                        rollout_archives_ready,
                        rollout_archives_failed,
                    );
                    continue;
                }
            };
            let change = classify_source_change(
                &file,
                previous.map(|entry| &entry.fingerprint),
                &fingerprint,
            )?;
            match change {
                SourceChange::New => *rollout_files_new += 1,
                SourceChange::Appended => *rollout_files_appended += 1,
                SourceChange::Unchanged => *rollout_files_unchanged += 1,
                SourceChange::Rewritten => *rollout_files_rewritten += 1,
            }
            let mut entry = SourceInventoryEntry {
                fingerprint: fingerprint.clone(),
                archive: previous.and_then(|entry| entry.archive.clone()),
            };
            let reusable = change == SourceChange::Unchanged
                && entry.archive.as_ref().is_some_and(|reference| {
                    archive_reference_matches_source(reference, &file, &fingerprint)
                });
            if reusable {
                retain_previous_archive(
                    registration,
                    identity,
                    &entry,
                    catalog_sources,
                    rollout_archives_ready,
                    rollout_archives_failed,
                );
            } else {
                let options =
                    SourceArchiveImportOptions::new(identity, &archive_root, vec![file.clone()])?
                        .include_assistant(matches!(
                            registration.role_policy,
                            AutomationRolePolicy::UserAndAssistant
                        ));
                match Engine::import_source_archive(options) {
                    Ok(result) => {
                        let reference = archive_reference(result.archive_path, &result.manifest)?;
                        entry.archive = Some(reference.clone());
                        *rollout_archives_ready += 1;
                        catalog_sources.push(AutomationCatalogSource {
                            registration_id: registration.registration_id.clone(),
                            project_ref_id: registration.project_ref_id,
                            lane: AutomationLane::HistoricalReference,
                            artifact_kind: AutomationArtifactKind::SourceArchive,
                            artifact_path: reference.artifact_path,
                            artifact_manifest_sha256: reference.artifact_manifest_sha256,
                            source_anchor: reference.source_anchor,
                            status: AutomationSourceStatus::Ready,
                            error_code: None,
                            error: None,
                        });
                    }
                    Err(error) => {
                        retain_previous_archive(
                            registration,
                            identity,
                            &entry,
                            catalog_sources,
                            rollout_archives_ready,
                            rollout_archives_failed,
                        );
                        *rollout_archives_failed += 1;
                        catalog_sources.push(failed_catalog_source(
                            registration,
                            AutomationLane::HistoricalReference,
                            AutomationArtifactKind::SourceArchive,
                            file,
                            &error.code().to_string(),
                            &error.to_string(),
                        ));
                    }
                }
            }
            current_inventory.insert(path_key, entry);
        }

        let deleted_sources = previous_inventory
            .files
            .iter()
            .filter(|(path, _)| {
                source_belongs_to_root(path, rollout_root) && !current_inventory.contains_key(*path)
            })
            .map(|(path, entry)| (path.clone(), entry.clone()))
            .collect::<Vec<_>>();
        for (deleted, entry) in deleted_sources {
            *rollout_files_deleted += 1;
            retain_previous_archive(
                registration,
                identity,
                &entry,
                catalog_sources,
                rollout_archives_ready,
                rollout_archives_failed,
            );
            *rollout_archives_failed += 1;
            catalog_sources.push(failed_catalog_source(
                registration,
                AutomationLane::HistoricalReference,
                AutomationArtifactKind::SourceArchive,
                PathBuf::from(&deleted),
                "source_deleted",
                "previously registered rollout source is no longer present",
            ));
            current_inventory.insert(deleted, entry);
        }
    }
    write_source_inventory(root, &registration.registration_id, current_inventory)
}

fn source_belongs_to_root(source: &str, root: &Path) -> bool {
    Path::new(source).starts_with(root)
}

fn retain_previous_archive(
    registration: &AutomationRegistration,
    identity: SearchSourceIdentity,
    entry: &SourceInventoryEntry,
    catalog_sources: &mut Vec<AutomationCatalogSource>,
    rollout_archives_ready: &mut usize,
    rollout_archives_failed: &mut usize,
) {
    let Some(reference) = entry.archive.as_ref() else {
        return;
    };
    match ready_archive_catalog_source(registration, identity, reference) {
        Ok(source) => {
            *rollout_archives_ready += 1;
            catalog_sources.push(source);
        }
        Err(error) => {
            *rollout_archives_failed += 1;
            catalog_sources.push(failed_catalog_source(
                registration,
                AutomationLane::HistoricalReference,
                AutomationArtifactKind::SourceArchive,
                reference.artifact_path.clone(),
                &error.code().to_string(),
                &error.to_string(),
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn record_historical_observation_failure(
    registration: &AutomationRegistration,
    identity: SearchSourceIdentity,
    path_key: &str,
    path: PathBuf,
    previous: Option<&SourceInventoryEntry>,
    error: &WorkVcsError,
    current_inventory: &mut BTreeMap<String, SourceInventoryEntry>,
    catalog_sources: &mut Vec<AutomationCatalogSource>,
    rollout_archives_ready: &mut usize,
    rollout_archives_failed: &mut usize,
) {
    if let Some(entry) = previous {
        retain_previous_archive(
            registration,
            identity,
            entry,
            catalog_sources,
            rollout_archives_ready,
            rollout_archives_failed,
        );
        current_inventory.insert(path_key.to_owned(), entry.clone());
    }
    *rollout_archives_failed += 1;
    catalog_sources.push(failed_catalog_source(
        registration,
        AutomationLane::HistoricalReference,
        AutomationArtifactKind::SourceArchive,
        path,
        &error.code().to_string(),
        &error.to_string(),
    ));
}

fn rediscover_registration(
    registration: &AutomationRegistration,
    project_registry: Option<&Path>,
) -> Result<ProjectDiscovery> {
    let discovery = discover_project_readonly_with_locator(
        registration.project_root.clone(),
        project_registry.map(Path::to_path_buf),
        Some(registration.project_ref_id.to_string()),
        None,
        None,
    )?;
    let exact = discovery.project_ref_id == Some(registration.project_ref_id)
        && discovery.binding.store_id == registration.store_id
        && discovery.binding.workspace_id == registration.workspace_id
        && discovery.binding.branch_id == registration.branch_id
        && Path::new(&discovery.binding.store_path) == registration.store_path
        && discovery.registry_revision == Some(registration.registry_revision)
        && discovery.registry_digest.to_string() == registration.registry_digest;
    if !exact {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation registration {} no longer matches its exact ProjectRef routing basis",
            registration.registration_id
        )));
    }
    Ok(discovery)
}

fn collect_rollout_files(root: &Path) -> Result<Vec<PathBuf>> {
    let metadata = fs::symlink_metadata(root).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "cannot inspect registered rollout root {}: {error}",
            root.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "registered rollout root {} is not a real directory",
            root.display()
        )));
    }
    validate_rollout_permissions(&metadata, root, "registered rollout root")?;
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot read registered rollout directory {}: {error}",
                directory.display()
            ))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "cannot read entry below {}: {error}",
                    directory.display()
                ))
            })?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "cannot inspect rollout candidate {}: {error}",
                    path.display()
                ))
            })?;
            if metadata.file_type().is_symlink() {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "registered rollout tree contains symbolic link {}",
                    path.display()
                )));
            }
            if metadata.is_dir() {
                validate_rollout_permissions(&metadata, &path, "rollout directory")?;
                pending.push(path);
            } else if metadata.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some("jsonl")
            {
                validate_rollout_permissions(&metadata, &path, "rollout source")?;
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn validate_rollout_permissions(metadata: &fs::Metadata, path: &Path, label: &str) -> Result<()> {
    #[cfg(unix)]
    {
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "{label} {} has unsafe ownership or write permissions",
                path.display()
            )));
        }
    }
    Ok(())
}

fn fingerprint_source(path: &Path) -> Result<SourceFingerprint> {
    let metadata = fs::symlink_metadata(path).map_err(storage_error("inspect rollout source"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "rollout source {} is not a real regular file",
            path.display()
        )));
    }
    Ok(SourceFingerprint {
        bytes: metadata.len(),
        sha256: workvcs_core::sha256_file(path, metadata.len())?,
    })
}

fn classify_source_change(
    path: &Path,
    previous: Option<&SourceFingerprint>,
    current: &SourceFingerprint,
) -> Result<SourceChange> {
    let Some(previous) = previous else {
        return Ok(SourceChange::New);
    };
    if previous == current {
        return Ok(SourceChange::Unchanged);
    }
    if current.bytes > previous.bytes && sha256_prefix(path, previous.bytes)? == previous.sha256 {
        return Ok(SourceChange::Appended);
    }
    Ok(SourceChange::Rewritten)
}

fn sha256_prefix(path: &Path, bytes: u64) -> Result<String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(storage_error("open rollout source prefix"))?;
    let mut hasher = Sha256::new();
    let mut remaining = bytes;
    let mut reader = file.take(bytes);
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let read = reader
            .read(&mut buffer)
            .map_err(storage_error("read rollout source prefix"))?;
        if read == 0 {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "rollout source {} became shorter during prefix validation",
                path.display()
            )));
        }
        hasher.update(&buffer[..read]);
        remaining -= read as u64;
    }
    Ok(hex::encode(hasher.finalize()))
}

fn read_source_inventory(root: &Path, registration_id: &str) -> Result<SourceInventory> {
    let path = root
        .join("inventory")
        .join(format!("{registration_id}.json"));
    if !path.exists() {
        return Ok(SourceInventory {
            schema_version: 2,
            observed_at_us: 0,
            files: BTreeMap::new(),
        });
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let mut file = options
        .open(&path)
        .map_err(storage_error("open source inventory"))?;
    #[cfg(unix)]
    {
        let metadata = file
            .metadata()
            .map_err(storage_error("inspect source inventory"))?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            return Err(WorkVcsError::IntegrityInvalid(
                "source inventory is not owner-private".to_owned(),
            ));
        }
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(storage_error("read source inventory"))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!("source inventory is malformed: {error}"))
    })?;
    match value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
    {
        Some(1) => {
            let legacy: SourceInventoryV1 = serde_json::from_value(value).map_err(|error| {
                WorkVcsError::IntegrityInvalid(format!("source inventory v1 is malformed: {error}"))
            })?;
            Ok(SourceInventory {
                schema_version: 2,
                observed_at_us: legacy.observed_at_us,
                files: legacy
                    .files
                    .into_iter()
                    .map(|(path, fingerprint)| {
                        (
                            path,
                            SourceInventoryEntry {
                                fingerprint,
                                archive: None,
                            },
                        )
                    })
                    .collect(),
            })
        }
        Some(2) => serde_json::from_value(value).map_err(|error| {
            WorkVcsError::IntegrityInvalid(format!("source inventory v2 is malformed: {error}"))
        }),
        _ => Err(WorkVcsError::IntegrityInvalid(
            "source inventory has an unsupported schema version".to_owned(),
        )),
    }
}

fn write_source_inventory(
    root: &Path,
    registration_id: &str,
    files: BTreeMap<String, SourceInventoryEntry>,
) -> Result<()> {
    let directory = root.join("inventory");
    ensure_owner_private_directory(&directory)?;
    let destination = directory.join(format!("{registration_id}.json"));
    let temporary = directory.join(format!(
        ".{registration_id}.{}.{}.tmp",
        std::process::id(),
        epoch_micros()
    ));
    let inventory = SourceInventory {
        schema_version: 2,
        observed_at_us: epoch_micros(),
        files,
    };
    let mut bytes = serde_json::to_vec(&inventory).map_err(serialization_error)?;
    bytes.push(b'\n');
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = (|| {
        let mut file = options
            .open(&temporary)
            .map_err(storage_error("create source inventory"))?;
        file.write_all(&bytes)
            .map_err(storage_error("write source inventory"))?;
        file.sync_all()
            .map_err(storage_error("sync source inventory"))?;
        drop(file);
        fs::rename(&temporary, &destination).map_err(storage_error("publish source inventory"))?;
        sync_directory(&directory)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn archive_reference(
    archive_path: PathBuf,
    manifest: &workvcs_core::SourceArchiveManifest,
) -> Result<SourceArchiveReference> {
    let source = manifest.files.as_slice();
    let [source] = source else {
        return Err(WorkVcsError::IntegrityInvalid(
            "automatic rollout archive must contain exactly one source file".to_owned(),
        ));
    };
    Ok(SourceArchiveReference {
        artifact_path: archive_path,
        artifact_manifest_sha256: manifest.manifest_sha256.clone(),
        source_path: source.source_path.clone(),
        source_bytes: source.source_bytes,
        source_sha256: source.source_sha256.clone(),
        source_anchor: format!(
            "path:{};sha256:{}",
            source.source_path.display(),
            source.source_sha256
        ),
    })
}

fn ready_archive_catalog_source(
    registration: &AutomationRegistration,
    identity: SearchSourceIdentity,
    reference: &SourceArchiveReference,
) -> Result<AutomationCatalogSource> {
    let inspection = Engine::inspect_source_archive(SourceArchiveInspectOptions::new(
        identity,
        &reference.artifact_path,
    ))?;
    if !inspection.archive_readback_verified
        || inspection.manifest.manifest_sha256 != reference.artifact_manifest_sha256
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "remembered source archive manifest or readback changed".to_owned(),
        ));
    }
    let inspected = archive_reference(inspection.archive_path, &inspection.manifest)?;
    if &inspected != reference {
        return Err(WorkVcsError::IntegrityInvalid(
            "remembered source archive reference does not match immutable archive contents"
                .to_owned(),
        ));
    }
    Ok(AutomationCatalogSource {
        registration_id: registration.registration_id.clone(),
        project_ref_id: registration.project_ref_id,
        lane: AutomationLane::HistoricalReference,
        artifact_kind: AutomationArtifactKind::SourceArchive,
        artifact_path: reference.artifact_path.clone(),
        artifact_manifest_sha256: reference.artifact_manifest_sha256.clone(),
        source_anchor: reference.source_anchor.clone(),
        status: AutomationSourceStatus::Ready,
        error_code: None,
        error: None,
    })
}

fn archive_reference_matches_source(
    reference: &SourceArchiveReference,
    source: &Path,
    fingerprint: &SourceFingerprint,
) -> bool {
    reference.source_path == source
        && reference.source_bytes == fingerprint.bytes
        && reference.source_sha256 == fingerprint.sha256
}

fn failed_catalog_source(
    registration: &AutomationRegistration,
    lane: AutomationLane,
    artifact_kind: AutomationArtifactKind,
    artifact_path: PathBuf,
    error_code: &str,
    error: &str,
) -> AutomationCatalogSource {
    AutomationCatalogSource {
        registration_id: registration.registration_id.clone(),
        project_ref_id: registration.project_ref_id,
        lane,
        artifact_kind,
        artifact_path,
        artifact_manifest_sha256: "0".repeat(64),
        source_anchor: "unavailable".to_owned(),
        status: AutomationSourceStatus::Failed,
        error_code: Some(error_code.to_owned()),
        error: Some(error.to_owned()),
    }
}

fn run_catalog(command: AutomationCatalogCommand) -> Result<String> {
    match command {
        AutomationCatalogCommand::Show { catalog } => {
            let manifest = read_automation_catalog(&catalog)?;
            render_json(&manifest)
        }
        AutomationCatalogCommand::Latest { registry, root } => {
            let root = resolve_automation_root(root, registry.as_deref())?;
            render_json(&latest_catalog_for_current_basis(&root)?)
        }
        AutomationCatalogCommand::Query {
            registry,
            root,
            catalog,
            query,
            limit,
        } => {
            if limit == Some(0) {
                return Err(WorkVcsError::QueryInvalid(
                    "catalog query limit must be greater than zero".to_owned(),
                ));
            }
            let root = resolve_automation_root(root, registry.as_deref())?;
            let catalog = match catalog {
                Some(catalog) => catalog,
                None => latest_catalog_for_current_basis(&root)?.catalog_path,
            };
            let report = query_catalog(&root, &catalog, &query, limit)?;
            render_json(&report)
        }
    }
}

fn latest_catalog_for_current_basis(root: &Path) -> Result<CatalogLatestReport> {
    let registry = read_automation_registry(root)?;
    let catalogs = root.join("catalogs");
    let entries = fs::read_dir(&catalogs).map_err(storage_error("read automation catalogs"))?;
    let mut selected: Option<(PathBuf, AutomationCatalogManifest)> = None;
    for entry in entries {
        let entry = entry.map_err(storage_error("read automation catalog entry"))?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let manifest = read_automation_catalog(&path)?;
        if manifest.automation_registry_revision != registry.revision
            || manifest.automation_registry_digest_sha256 != registry.digest_sha256
        {
            continue;
        }
        let should_select = selected
            .as_ref()
            .is_none_or(|(selected_path, selected_manifest)| {
                (manifest.created_at_us, &path) > (selected_manifest.created_at_us, selected_path)
            });
        if should_select {
            selected = Some((path, manifest));
        }
    }
    let (catalog_path, manifest) = selected.ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid(
            "no immutable catalog matches the current automation registry basis".to_owned(),
        )
    })?;
    let catalog_manifest_sha256 = catalog_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            WorkVcsError::IntegrityInvalid(
                "selected catalog path has no UTF-8 digest name".to_owned(),
            )
        })?
        .to_owned();
    Ok(CatalogLatestReport {
        catalog_path,
        catalog_manifest_sha256,
        manifest,
    })
}

fn query_catalog(
    root: &Path,
    catalog_path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<CatalogQueryReport> {
    let catalog = read_automation_catalog(catalog_path)?;
    let registry = read_automation_registry(root)?;
    if catalog.automation_registry_revision != registry.revision
        || catalog.automation_registry_digest_sha256 != registry.digest_sha256
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "catalog registration basis is stale: catalog revision {} digest {}, current revision {} digest {}",
            catalog.automation_registry_revision,
            catalog.automation_registry_digest_sha256,
            registry.revision,
            registry.digest_sha256
        )));
    }
    let registrations = registry
        .registrations
        .iter()
        .filter(|item| item.enabled && !item.revoked)
        .map(|item| (item.registration_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut reports = Vec::new();
    let mut items = Vec::new();
    let mut failed = 0;
    let mut succeeded = 0;

    for source in &catalog.sources {
        if source.status != AutomationSourceStatus::Ready {
            failed += 1;
            reports.push(CatalogQuerySourceReport {
                registration_id: source.registration_id.clone(),
                project_ref_id: source.project_ref_id,
                lane: source.lane,
                artifact_kind: source.artifact_kind,
                artifact_path: source.artifact_path.clone(),
                source_anchor: source.source_anchor.clone(),
                status: "failed".to_owned(),
                error_code: source.error_code.clone(),
                error: source.error.clone(),
                matched_items: 0,
            });
            continue;
        }
        let Some(registration) = registrations.get(source.registration_id.as_str()) else {
            failed += 1;
            reports.push(CatalogQuerySourceReport {
                registration_id: source.registration_id.clone(),
                project_ref_id: source.project_ref_id,
                lane: source.lane,
                artifact_kind: source.artifact_kind,
                artifact_path: source.artifact_path.clone(),
                source_anchor: source.source_anchor.clone(),
                status: "revoked_or_unregistered".to_owned(),
                error_code: Some("registration_not_active".to_owned()),
                error: Some("catalog source registration is no longer active".to_owned()),
                matched_items: 0,
            });
            continue;
        };
        let result = query_catalog_source(registration, source, query);
        match result {
            Ok(mut source_items) => {
                succeeded += 1;
                let matched = source_items.len();
                items.append(&mut source_items);
                reports.push(CatalogQuerySourceReport {
                    registration_id: source.registration_id.clone(),
                    project_ref_id: source.project_ref_id,
                    lane: source.lane,
                    artifact_kind: source.artifact_kind,
                    artifact_path: source.artifact_path.clone(),
                    source_anchor: source.source_anchor.clone(),
                    status: "completed".to_owned(),
                    error_code: None,
                    error: None,
                    matched_items: matched,
                });
            }
            Err(error) => {
                failed += 1;
                reports.push(CatalogQuerySourceReport {
                    registration_id: source.registration_id.clone(),
                    project_ref_id: source.project_ref_id,
                    lane: source.lane,
                    artifact_kind: source.artifact_kind,
                    artifact_path: source.artifact_path.clone(),
                    source_anchor: source.source_anchor.clone(),
                    status: "failed".to_owned(),
                    error_code: Some(error.code().to_string()),
                    error: Some(error.to_string()),
                    matched_items: 0,
                });
            }
        }
    }
    items.sort_by(|left, right| {
        right
            .federated_score
            .total_cmp(&left.federated_score)
            .then_with(|| left.project_ref_id.cmp(&right.project_ref_id))
            .then_with(|| left.lane.cmp(&right.lane))
            .then_with(|| left.source_local_rank.cmp(&right.source_local_rank))
    });
    let matched_items = items.len();
    if let Some(limit) = limit {
        items.truncate(limit);
    }
    let complete = failed == 0 && succeeded > 0;
    let catalog_manifest_sha256 = catalog_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            WorkVcsError::IntegrityInvalid("catalog path has no UTF-8 digest name".to_owned())
        })?
        .to_owned();
    Ok(CatalogQueryReport {
        status: if succeeded == 0 {
            "failed".to_owned()
        } else if failed > 0 {
            "partial_failure".to_owned()
        } else {
            "completed".to_owned()
        },
        complete,
        catalog_path: catalog_path.to_path_buf(),
        catalog_manifest_sha256,
        query: query.to_owned(),
        sources_requested: catalog.sources.len(),
        sources_succeeded: succeeded,
        sources_failed: failed,
        result_limit: limit,
        matched_items,
        omitted_results: matched_items.saturating_sub(items.len()),
        source_reports: reports,
        items,
    })
}

fn query_catalog_source(
    registration: &AutomationRegistration,
    source: &AutomationCatalogSource,
    query: &str,
) -> Result<Vec<CatalogQueryItem>> {
    let identity = SearchSourceIdentity::new(
        registration.project_ref_id,
        registration.store_id,
        registration.workspace_id,
        registration.branch_id,
    );
    match source.artifact_kind {
        AutomationArtifactKind::LexicalGeneration => {
            let engine = Engine::open_readonly(&registration.store_path)?;
            let result = engine.search(SearchQueryOptions::new(
                identity,
                &source.artifact_path,
                query,
            )?)?;
            if result.status != SearchDerivedStatus::Ready || !result.canonical_readback_verified {
                return Err(WorkVcsError::IntegrityInvalid(
                    "catalog canonical source is not ready and readback-verified".to_owned(),
                ));
            }
            let manifest = result.manifest.as_ref().ok_or_else(|| {
                WorkVcsError::IntegrityInvalid(
                    "catalog canonical source omitted its manifest".to_owned(),
                )
            })?;
            if sha256_serialized(manifest)? != source.artifact_manifest_sha256 {
                return Err(WorkVcsError::IntegrityInvalid(
                    "catalog canonical source manifest digest changed".to_owned(),
                ));
            }
            result
                .items
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    Ok(CatalogQueryItem {
                        project_ref_id: source.project_ref_id,
                        lane: source.lane,
                        artifact_kind: source.artifact_kind,
                        source_anchor: source.source_anchor.clone(),
                        source_local_rank: index + 1,
                        source_score: item.score,
                        federated_score: 1.0 / (60.0 + (index + 1) as f64),
                        payload: serde_json::to_value(item).map_err(serialization_error)?,
                    })
                })
                .collect()
        }
        AutomationArtifactKind::SourceArchive => {
            let result = Engine::query_source_archive(SourceArchiveQueryOptions::new(
                identity,
                &source.artifact_path,
                query,
            )?)?;
            if !result.archive_readback_verified
                || result.manifest.manifest_sha256 != source.artifact_manifest_sha256
            {
                return Err(WorkVcsError::IntegrityInvalid(
                    "catalog source archive manifest or readback changed".to_owned(),
                ));
            }
            result
                .items
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    Ok(CatalogQueryItem {
                        project_ref_id: source.project_ref_id,
                        lane: source.lane,
                        artifact_kind: source.artifact_kind,
                        source_anchor: source.source_anchor.clone(),
                        source_local_rank: index + 1,
                        source_score: item.score,
                        federated_score: 1.0 / (60.0 + (index + 1) as f64),
                        payload: serde_json::to_value(item).map_err(serialization_error)?,
                    })
                })
                .collect()
        }
    }
}

fn run_daemon_command(command: AutomationDaemonCommand) -> Result<String> {
    match command {
        AutomationDaemonCommand::Run {
            registry,
            root,
            periodic_seconds,
        } => {
            if periodic_seconds == 0 {
                return Err(WorkVcsError::QueryInvalid(
                    "daemon periodic interval must be greater than zero".to_owned(),
                ));
            }
            let root = resolve_automation_root(root, registry.as_deref())?;
            run_daemon(&root, registry.as_deref(), periodic_seconds)?;
            Ok(String::new())
        }
        AutomationDaemonCommand::Status { registry, root } => daemon_request(
            &resolve_automation_root(root, registry.as_deref())?,
            "status",
        ),
        AutomationDaemonCommand::Reconcile { registry, root } => daemon_request(
            &resolve_automation_root(root, registry.as_deref())?,
            "reconcile",
        ),
        AutomationDaemonCommand::Stop { registry, root } => daemon_request(
            &resolve_automation_root(root, registry.as_deref())?,
            "shutdown",
        ),
    }
}

#[cfg(unix)]
fn run_daemon(root: &Path, registry: Option<&Path>, periodic_seconds: u64) -> Result<()> {
    ensure_owner_private_directory(root)?;
    let runtime = root.join("runtime");
    ensure_owner_private_directory(&runtime)?;
    let lock_path = runtime.join("daemon.lock");
    let lock = open_private_file(&lock_path, true)?;
    fs4::FileExt::try_lock(&lock).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "automation daemon is already running or cannot acquire its lock: {error}"
        ))
    })?;
    let socket_path = runtime.join("v1.sock");
    if socket_path.exists() {
        let metadata =
            fs::symlink_metadata(&socket_path).map_err(storage_error("inspect daemon socket"))?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_socket() {
            return Err(WorkVcsError::IntegrityInvalid(
                "automation daemon socket path has an unsafe type".to_owned(),
            ));
        }
        fs::remove_file(&socket_path).map_err(storage_error("remove stale daemon socket"))?;
    }
    let listener = UnixListener::bind(&socket_path).map_err(storage_error("bind daemon socket"))?;
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))
        .map_err(storage_error("protect daemon socket"))?;
    listener
        .set_nonblocking(true)
        .map_err(storage_error("configure daemon socket"))?;

    let (watch_receiver, mut watcher, watcher_health) = start_watcher(root)?;
    let started_at_us = epoch_micros();
    let mut ledger = DaemonLedger {
        protocol_version: AUTOMATION_PROTOCOL_VERSION,
        pid: std::process::id(),
        started_at_us,
        updated_at_us: started_at_us,
        last_trigger: "startup".to_owned(),
        last_worker_status: None,
        last_worker_exit_code: None,
        last_worker_timed_out: false,
        last_worker_output_truncated: false,
        last_worker_evidence_path: None,
        last_worker_stdout_bytes: 0,
        last_worker_stdout_sha256: None,
        last_worker_stderr_bytes: 0,
        last_worker_stderr_sha256: None,
        last_error: None,
        last_catalog_path: None,
        last_catalog_manifest_sha256: None,
        last_catalog_complete: None,
        watcher_events: 0,
        watcher_errors: 0,
        watcher_coalesced_events: 0,
        last_watcher_error: None,
        rejected_requests: 0,
        last_protocol_error: None,
        daemon_enabled_by_default: false,
        launch_agent_installed: false,
    };
    let startup = spawn_reconcile_worker(root, registry)?;
    record_worker(&mut ledger, "startup", startup, root)?;
    project_watcher_health(&watcher_health, &mut ledger);
    write_daemon_ledger(&runtime, &ledger)?;
    let mut periodic_deadline = Instant::now() + Duration::from_secs(periodic_seconds);
    let mut dirty_since: Option<Instant> = None;
    let mut running = true;

    while running {
        while watch_receiver.try_recv().is_ok() {
            dirty_since.get_or_insert_with(Instant::now);
        }
        project_watcher_health(&watcher_health, &mut ledger);
        if dirty_since.is_some_and(|instant| instant.elapsed() >= Duration::from_millis(250)) {
            let outcome = spawn_reconcile_worker(root, registry)?;
            record_worker(&mut ledger, "watcher_dirty_or_overflow", outcome, root)?;
            dirty_since = None;
            project_watcher_health(&watcher_health, &mut ledger);
            write_daemon_ledger(&runtime, &ledger)?;
        }
        if Instant::now() >= periodic_deadline {
            let outcome = spawn_reconcile_worker(root, registry)?;
            record_worker(&mut ledger, "periodic", outcome, root)?;
            periodic_deadline = Instant::now() + Duration::from_secs(periodic_seconds);
            project_watcher_health(&watcher_health, &mut ledger);
            write_daemon_ledger(&runtime, &ledger)?;
        }
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    if let Err(error) = stream.set_read_timeout(Some(Duration::from_secs(2))) {
                        record_protocol_rejection(
                            &mut ledger,
                            &runtime,
                            format!("cannot bound daemon client read: {error}"),
                        )?;
                        continue;
                    }
                    if let Err(error) = stream.set_write_timeout(Some(Duration::from_secs(2))) {
                        record_protocol_rejection(
                            &mut ledger,
                            &runtime,
                            format!("cannot bound daemon client write: {error}"),
                        )?;
                        continue;
                    }
                    if let Err(error) = verify_peer_uid(&stream) {
                        let message = error.to_string();
                        let response = DaemonResponse {
                            version: AUTOMATION_PROTOCOL_VERSION,
                            status: "rejected".to_owned(),
                            action: "unknown".to_owned(),
                            message: message.clone(),
                            worker_exit_code: ledger.last_worker_exit_code,
                        };
                        let response_error = write_daemon_response(&mut stream, &response).err();
                        record_protocol_rejection(
                            &mut ledger,
                            &runtime,
                            rejection_message(message, response_error),
                        )?;
                        continue;
                    }
                    let request = match read_daemon_request(&mut stream) {
                        Ok(request) => request,
                        Err(error) => {
                            let message = error.to_string();
                            let response = DaemonResponse {
                                version: AUTOMATION_PROTOCOL_VERSION,
                                status: "rejected".to_owned(),
                                action: "unknown".to_owned(),
                                message: message.clone(),
                                worker_exit_code: ledger.last_worker_exit_code,
                            };
                            let response_error =
                                write_daemon_response(&mut stream, &response).err();
                            record_protocol_rejection(
                                &mut ledger,
                                &runtime,
                                rejection_message(message, response_error),
                            )?;
                            continue;
                        }
                    };
                    let mut response = DaemonResponse {
                        version: AUTOMATION_PROTOCOL_VERSION,
                        status: "completed".to_owned(),
                        action: request.action.clone(),
                        message: "request completed".to_owned(),
                        worker_exit_code: ledger.last_worker_exit_code,
                    };
                    match request.action.as_str() {
                        "status" => {
                            project_watcher_health(&watcher_health, &mut ledger);
                            project_daemon_status(&ledger, &mut response)?;
                        }
                        "reconcile" => {
                            let outcome = spawn_reconcile_worker(root, registry)?;
                            record_worker(&mut ledger, "manual", outcome, root)?;
                            project_watcher_health(&watcher_health, &mut ledger);
                            response.worker_exit_code = ledger.last_worker_exit_code;
                            project_worker_status(&ledger, &mut response);
                            write_daemon_ledger(&runtime, &ledger)?;
                        }
                        "shutdown" => {
                            ledger.last_trigger = "shutdown".to_owned();
                            ledger.updated_at_us = epoch_micros();
                            write_daemon_ledger(&runtime, &ledger)?;
                            running = false;
                        }
                        _ => {
                            response.status = "rejected".to_owned();
                            response.message = "unsupported daemon action".to_owned();
                            record_protocol_rejection(
                                &mut ledger,
                                &runtime,
                                format!("unsupported daemon action: {}", request.action),
                            )?;
                        }
                    }
                    if let Err(error) = write_daemon_response(&mut stream, &response) {
                        record_protocol_rejection(
                            &mut ledger,
                            &runtime,
                            format!("cannot write daemon response: {error}"),
                        )?;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(storage_error("accept daemon client")(error)),
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(watcher.take());
    drop(listener);
    fs::remove_file(&socket_path).map_err(storage_error("remove daemon socket"))?;
    Ok(())
}

#[cfg(not(unix))]
fn run_daemon(_root: &Path, _registry: Option<&Path>, _periodic_seconds: u64) -> Result<()> {
    Err(WorkVcsError::QueryUnsupported(
        "automation daemon currently requires a Unix local-socket platform".to_owned(),
    ))
}

#[cfg(unix)]
fn start_watcher(
    root: &Path,
) -> Result<(
    Receiver<()>,
    Option<notify::RecommendedWatcher>,
    Arc<WatcherHealth>,
)> {
    use notify::{RecursiveMode, Watcher};
    let registry = read_automation_registry(root)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let health = Arc::new(WatcherHealth::default());
    let callback_health = Arc::clone(&health);
    let callback_sender = sender.clone();
    let mut watcher = notify::recommended_watcher(
        move |event: std::result::Result<notify::Event, notify::Error>| {
            signal_watcher_event(
                &callback_sender,
                &callback_health,
                event.map(|_| ()).map_err(|error| error.to_string()),
            );
        },
    )
    .map_err(|error| WorkVcsError::StorageFailure(format!("cannot create watcher: {error}")))?;
    for registration in registry
        .registrations
        .iter()
        .filter(|item| item.enabled && !item.revoked)
    {
        if let Err(error) = watcher.watch(&registration.store_path, RecursiveMode::NonRecursive) {
            signal_watcher_event(
                &sender,
                &health,
                Err(format!(
                    "cannot watch Store {}: {error}",
                    registration.store_path.display()
                )),
            );
        }
        for rollout_root in &registration.rollout_roots {
            if let Err(error) = watcher.watch(rollout_root, RecursiveMode::Recursive) {
                signal_watcher_event(
                    &sender,
                    &health,
                    Err(format!(
                        "cannot watch rollout root {}: {error}",
                        rollout_root.display()
                    )),
                );
            }
        }
    }
    Ok((receiver, Some(watcher), health))
}

#[cfg(unix)]
fn signal_watcher_event(
    sender: &SyncSender<()>,
    health: &WatcherHealth,
    event: std::result::Result<(), String>,
) {
    match event {
        Ok(()) => {
            health.events.fetch_add(1, Ordering::Relaxed);
        }
        Err(error) => {
            health.errors.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut last_error) = health.last_error.lock() {
                *last_error = Some(bound_text(&error, 2_048));
            }
        }
    }
    match sender.try_send(()) {
        Ok(()) => {}
        Err(TrySendError::Full(())) => {
            health.coalesced_events.fetch_add(1, Ordering::Relaxed);
        }
        Err(TrySendError::Disconnected(())) => {}
    }
}

#[cfg(unix)]
fn project_watcher_health(health: &WatcherHealth, ledger: &mut DaemonLedger) {
    ledger.watcher_events = health.events.load(Ordering::Relaxed);
    ledger.watcher_errors = health.errors.load(Ordering::Relaxed);
    ledger.watcher_coalesced_events = health.coalesced_events.load(Ordering::Relaxed);
    ledger.last_watcher_error = health
        .last_error
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| Some("watcher health state is poisoned".to_owned()));
}

fn spawn_reconcile_worker(root: &Path, registry: Option<&Path>) -> Result<WorkerOutcome> {
    let executable =
        std::env::current_exe().map_err(storage_error("resolve current executable"))?;
    let mut command = ProcessCommand::new(executable);
    command
        .arg("automation")
        .arg("reconcile")
        .arg("--once")
        .arg("--root")
        .arg(root)
        .arg("--format")
        .arg("json");
    if let Some(registry) = registry {
        command.arg("--registry").arg(registry);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(storage_error("spawn reconciliation worker"))?;
    wait_for_worker(&mut child, RECONCILE_WORKER_TIMEOUT)
}

fn wait_for_worker(child: &mut Child, timeout: Duration) -> Result<WorkerOutcome> {
    let stdout = child.stdout.take().ok_or_else(|| {
        WorkVcsError::StorageFailure("reconciliation worker stdout is not captured".to_owned())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        WorkVcsError::StorageFailure("reconciliation worker stderr is not captured".to_owned())
    })?;
    let stdout_reader = std::thread::spawn(move || capture_worker_stream(stdout));
    let stderr_reader = std::thread::spawn(move || capture_worker_stream(stderr));
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(storage_error("wait for reconciliation worker"))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            child
                .kill()
                .map_err(storage_error("terminate timed-out reconciliation worker"))?;
            break child
                .wait()
                .map_err(storage_error("reap timed-out reconciliation worker"))?;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = join_worker_capture(stdout_reader, "stdout")?;
    let stderr = join_worker_capture(stderr_reader, "stderr")?;
    Ok(WorkerOutcome {
        exit_code: status.code(),
        timed_out,
        stdout,
        stderr,
    })
}

fn capture_worker_stream(mut stream: impl Read) -> std::io::Result<CapturedStream> {
    let mut captured = Vec::new();
    let mut total_bytes = 0_u64;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total_bytes = total_bytes.saturating_add(read as u64);
        digest.update(&buffer[..read]);
        let remaining = MAX_WORKER_STREAM_BYTES.saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..read.min(remaining)]);
    }
    Ok(CapturedStream {
        truncated: total_bytes > captured.len() as u64,
        bytes: captured,
        total_bytes,
        sha256: hex::encode(digest.finalize()),
    })
}

fn join_worker_capture(
    handle: std::thread::JoinHandle<std::io::Result<CapturedStream>>,
    label: &str,
) -> Result<CapturedStream> {
    handle
        .join()
        .map_err(|_| {
            WorkVcsError::StorageFailure(format!(
                "reconciliation worker {label} capture thread panicked"
            ))
        })?
        .map_err(storage_error("capture reconciliation worker output"))
}

#[cfg(unix)]
fn persist_worker_evidence(root: &Path, trigger: &str, outcome: &WorkerOutcome) -> Result<PathBuf> {
    let directory = root.join("runtime").join("worker-evidence");
    ensure_owner_private_directory(&directory)?;
    let stdout_path = directory.join("last-worker.stdout.log");
    let stderr_path = directory.join("last-worker.stderr.log");
    let manifest_path = directory.join("last-worker.json");
    replace_private_runtime_file(&stdout_path, &outcome.stdout.bytes)?;
    replace_private_runtime_file(&stderr_path, &outcome.stderr.bytes)?;
    if read_private_runtime_file(&stdout_path, outcome.stdout.bytes.len())? != outcome.stdout.bytes
        || read_private_runtime_file(&stderr_path, outcome.stderr.bytes.len())?
            != outcome.stderr.bytes
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "worker stream evidence failed exact readback".to_owned(),
        ));
    }
    let evidence = WorkerEvidence {
        captured_at_us: epoch_micros(),
        trigger: trigger.to_owned(),
        exit_code: outcome.exit_code,
        timed_out: outcome.timed_out,
        stdout_path,
        stdout_bytes: outcome.stdout.total_bytes,
        stdout_sha256: outcome.stdout.sha256.clone(),
        stdout_truncated: outcome.stdout.truncated,
        stderr_path,
        stderr_bytes: outcome.stderr.total_bytes,
        stderr_sha256: outcome.stderr.sha256.clone(),
        stderr_truncated: outcome.stderr.truncated,
    };
    let mut bytes = serde_json::to_vec(&evidence).map_err(serialization_error)?;
    bytes.push(b'\n');
    replace_private_runtime_file(&manifest_path, &bytes)?;
    let readback = read_private_runtime_file(&manifest_path, bytes.len())?;
    if readback != bytes {
        return Err(WorkVcsError::IntegrityInvalid(
            "worker evidence manifest failed exact readback".to_owned(),
        ));
    }
    Ok(manifest_path)
}

#[cfg(unix)]
fn replace_private_runtime_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        WorkVcsError::QueryInvalid("private runtime file has no parent directory".to_owned())
    })?;
    ensure_owner_private_directory(parent)?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid("private runtime filename must be valid UTF-8".to_owned())
        })?;
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        epoch_micros()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let result = (|| {
        let mut file = options
            .open(&temporary)
            .map_err(storage_error("create private runtime evidence"))?;
        file.write_all(bytes)
            .map_err(storage_error("write private runtime evidence"))?;
        file.sync_all()
            .map_err(storage_error("sync private runtime evidence"))?;
        drop(file);
        fs::rename(&temporary, path).map_err(storage_error("publish private runtime evidence"))?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn read_private_runtime_file(path: &Path, expected_max: usize) -> Result<Vec<u8>> {
    let mut file = open_private_file(path, false)?;
    let metadata = file
        .metadata()
        .map_err(storage_error("inspect private runtime evidence"))?;
    if metadata.len() > expected_max as u64 {
        return Err(WorkVcsError::IntegrityInvalid(
            "private runtime evidence exceeds its expected bound".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(storage_error("read private runtime evidence"))?;
    Ok(bytes)
}

#[cfg(unix)]
fn record_worker(
    ledger: &mut DaemonLedger,
    trigger: &str,
    outcome: WorkerOutcome,
    root: &Path,
) -> Result<()> {
    let evidence_path = persist_worker_evidence(root, trigger, &outcome)?;
    ledger.updated_at_us = epoch_micros();
    ledger.last_trigger = trigger.to_owned();
    ledger.last_worker_status = Some("failed".to_owned());
    ledger.last_worker_exit_code = outcome.exit_code;
    ledger.last_worker_timed_out = outcome.timed_out;
    ledger.last_worker_output_truncated = outcome.stdout.truncated || outcome.stderr.truncated;
    ledger.last_worker_evidence_path = Some(evidence_path.clone());
    ledger.last_worker_stdout_bytes = outcome.stdout.total_bytes;
    ledger.last_worker_stdout_sha256 = Some(outcome.stdout.sha256.clone());
    ledger.last_worker_stderr_bytes = outcome.stderr.total_bytes;
    ledger.last_worker_stderr_sha256 = Some(outcome.stderr.sha256.clone());
    ledger.last_error = None;
    ledger.last_catalog_path = None;
    ledger.last_catalog_manifest_sha256 = None;
    ledger.last_catalog_complete = None;

    if ledger.last_worker_output_truncated {
        ledger.last_error = Some(format!(
            "reconciliation worker output exceeded the {} byte per-stream bound; bounded evidence and full-stream digests are recorded at {}",
            MAX_WORKER_STREAM_BYTES,
            evidence_path.display()
        ));
        return Ok(());
    }
    if outcome.timed_out {
        ledger.last_error = Some(format!(
            "reconciliation worker exceeded {} seconds and was terminated; exact captured evidence: {}",
            RECONCILE_WORKER_TIMEOUT.as_secs(),
            evidence_path.display()
        ));
        return Ok(());
    }
    if outcome.exit_code != Some(0) {
        let evidence = if outcome.stderr.bytes.is_empty() {
            &outcome.stdout.bytes
        } else {
            &outcome.stderr.bytes
        };
        ledger.last_error = Some(format!(
            "reconciliation worker exited with {:?}; exact evidence: {}; excerpt: {}",
            outcome.exit_code,
            evidence_path.display(),
            bound_text(&String::from_utf8_lossy(evidence), 2_048)
        ));
        return Ok(());
    }
    if !outcome.stderr.bytes.is_empty() {
        ledger.last_error = Some(format!(
            "reconciliation worker exited successfully but emitted stderr; exact evidence: {}; excerpt: {}",
            evidence_path.display(),
            bound_text(&String::from_utf8_lossy(&outcome.stderr.bytes), 2_048)
        ));
        return Ok(());
    }

    let report: ReconcileReport = match serde_json::from_slice(&outcome.stdout.bytes) {
        Ok(report) => report,
        Err(error) => {
            ledger.last_error = Some(format!(
                "reconciliation worker stdout is not a valid report: {error}; exact evidence: {}",
                evidence_path.display()
            ));
            return Ok(());
        }
    };
    let ready_sources = report
        .canonical_generations_ready
        .saturating_add(report.rollout_archives_ready);
    let expected_status = if report.complete {
        "completed"
    } else if ready_sources == 0 {
        "failed"
    } else {
        "partial_failure"
    };
    if report.status != expected_status {
        ledger.last_error = Some(format!(
            "reconciliation worker report status {:?} contradicts complete={}; exact evidence: {}",
            report.status,
            report.complete,
            evidence_path.display()
        ));
        return Ok(());
    }
    let latest = match latest_catalog_for_current_basis(root) {
        Ok(latest) => latest,
        Err(error) => {
            ledger.last_error = Some(format!(
                "reconciliation worker completed but catalog readback failed: {error}; exact evidence: {}",
                evidence_path.display()
            ));
            return Ok(());
        }
    };
    if latest.catalog_path != report.catalog_path
        || latest.catalog_manifest_sha256 != report.catalog_manifest_sha256
        || latest.manifest.complete != report.complete
        || latest.manifest.automation_registry_revision != report.automation_registry_revision
        || latest.manifest.automation_registry_digest_sha256
            != report.automation_registry_digest_sha256
    {
        ledger.last_error = Some(format!(
            "reconciliation worker report does not match exact current-basis catalog readback; exact evidence: {}",
            evidence_path.display()
        ));
        return Ok(());
    }
    ledger.last_worker_status = Some(report.status);
    ledger.last_catalog_path = Some(latest.catalog_path);
    ledger.last_catalog_manifest_sha256 = Some(latest.catalog_manifest_sha256);
    ledger.last_catalog_complete = Some(latest.manifest.complete);
    Ok(())
}

fn bound_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...[truncated]", &value[..end])
}

fn project_worker_status(ledger: &DaemonLedger, response: &mut DaemonResponse) {
    if ledger.last_worker_status.as_deref() == Some("failed")
        || ledger.last_worker_exit_code != Some(0)
        || ledger.last_error.is_some()
    {
        response.status = "failed".to_owned();
        response.message = ledger
            .last_error
            .clone()
            .unwrap_or_else(|| "reconciliation worker failed".to_owned());
    } else if ledger.last_worker_status.as_deref() == Some("partial_failure")
        && ledger.last_catalog_complete == Some(false)
    {
        response.status = "partial_failure".to_owned();
        response.message =
            "reconciliation completed with an explicitly incomplete catalog".to_owned();
    } else if ledger.last_worker_status.as_deref() != Some("completed")
        || ledger.last_catalog_complete != Some(true)
    {
        response.status = "failed".to_owned();
        response.message = "reconciliation produced no verified catalog readback".to_owned();
    }
}

#[cfg(unix)]
fn project_daemon_status(ledger: &DaemonLedger, response: &mut DaemonResponse) -> Result<()> {
    response.worker_exit_code = ledger.last_worker_exit_code;
    project_worker_status(ledger, response);
    response.message = serde_json::to_string(ledger).map_err(serialization_error)?;
    Ok(())
}

#[cfg(unix)]
fn record_protocol_rejection(
    ledger: &mut DaemonLedger,
    runtime: &Path,
    message: String,
) -> Result<()> {
    ledger.updated_at_us = epoch_micros();
    ledger.rejected_requests = ledger.rejected_requests.saturating_add(1);
    ledger.last_protocol_error = Some(message);
    write_daemon_ledger(runtime, ledger)
}

#[cfg(unix)]
fn rejection_message(message: String, response_error: Option<WorkVcsError>) -> String {
    match response_error {
        Some(error) => format!("{message}; rejection response failed: {error}"),
        None => message,
    }
}

#[cfg(unix)]
fn daemon_request(root: &Path, action: &str) -> Result<String> {
    let socket = root.join("runtime").join("v1.sock");
    let metadata = fs::symlink_metadata(&socket).map_err(storage_error("inspect daemon socket"))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_socket() {
        return Err(WorkVcsError::IntegrityInvalid(
            "automation daemon socket has an unsafe type".to_owned(),
        ));
    }
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
        return Err(WorkVcsError::IntegrityInvalid(
            "automation daemon socket is not owner-private".to_owned(),
        ));
    }
    let mut stream =
        UnixStream::connect(&socket).map_err(storage_error("connect daemon socket"))?;
    let request = DaemonRequest {
        version: AUTOMATION_PROTOCOL_VERSION,
        action: action.to_owned(),
    };
    let mut bytes = serde_json::to_vec(&request).map_err(serialization_error)?;
    bytes.push(b'\n');
    stream
        .write_all(&bytes)
        .map_err(storage_error("write daemon request"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(storage_error("finish daemon request"))?;
    let response = read_bounded_json::<DaemonResponse>(&mut stream, "daemon response")?;
    render_json(&response)
}

#[cfg(not(unix))]
fn daemon_request(_root: &Path, _action: &str) -> Result<String> {
    Err(WorkVcsError::QueryUnsupported(
        "automation daemon currently requires a Unix local-socket platform".to_owned(),
    ))
}

#[cfg(unix)]
fn verify_peer_uid(stream: &UnixStream) -> Result<()> {
    let peer_uid = peer_uid(stream)?;
    let current_uid = unsafe { libc::geteuid() };
    if peer_uid != current_uid {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation daemon peer uid {peer_uid} does not match current uid {current_uid}"
        )));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn peer_uid(stream: &UnixStream) -> Result<libc::uid_t> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result != 0 {
        return Err(storage_error("read daemon peer identity")(
            std::io::Error::last_os_error(),
        ));
    }
    Ok(uid)
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> Result<libc::uid_t> {
    let mut credentials: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    if result != 0 {
        return Err(storage_error("read daemon peer identity")(
            std::io::Error::last_os_error(),
        ));
    }
    Ok(credentials.uid)
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn peer_uid(_stream: &UnixStream) -> Result<libc::uid_t> {
    Err(WorkVcsError::QueryUnsupported(
        "same-user peer verification is unsupported on this Unix platform".to_owned(),
    ))
}

#[cfg(unix)]
fn read_daemon_request(stream: &mut UnixStream) -> Result<DaemonRequest> {
    let request = read_bounded_json::<DaemonRequest>(stream, "daemon request")?;
    if request.version != AUTOMATION_PROTOCOL_VERSION {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "daemon protocol version {} does not match {}",
            request.version, AUTOMATION_PROTOCOL_VERSION
        )));
    }
    Ok(request)
}

fn read_bounded_json<T: for<'de> Deserialize<'de>>(
    reader: &mut impl Read,
    label: &str,
) -> Result<T> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_DAEMON_FRAME_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(storage_error("read daemon frame"))?;
    if bytes.len() > MAX_DAEMON_FRAME_BYTES {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{label} exceeds the bounded frame size"
        )));
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("{label} is malformed JSON: {error}"))
    })
}

#[cfg(unix)]
fn write_daemon_response(stream: &mut UnixStream, response: &DaemonResponse) -> Result<()> {
    let mut bytes = serde_json::to_vec(response).map_err(serialization_error)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_DAEMON_FRAME_BYTES {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "daemon response exceeds the bounded frame size".to_owned(),
        ));
    }
    stream
        .write_all(&bytes)
        .map_err(storage_error("write daemon response"))
}

#[cfg(unix)]
fn ensure_owner_private_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(storage_error("create automation runtime directory"))?;
    let metadata =
        fs::symlink_metadata(path).map_err(storage_error("inspect runtime directory"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automation runtime directory has unsafe type or ownership".to_owned(),
        ));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(storage_error("protect automation runtime directory"))
}

#[cfg(unix)]
fn open_private_file(path: &Path, create: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(create)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(storage_error("open private runtime file"))?;
    let metadata = file
        .metadata()
        .map_err(storage_error("inspect private runtime file"))?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "automation runtime file is not owner-private".to_owned(),
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn write_daemon_ledger(runtime: &Path, ledger: &DaemonLedger) -> Result<()> {
    let destination = runtime.join("status.json");
    let temporary = runtime.join(format!(
        ".status.{}.{}.tmp",
        std::process::id(),
        epoch_micros()
    ));
    let bytes = [
        serde_json::to_vec(ledger)
            .map_err(serialization_error)?
            .as_slice(),
        b"\n",
    ]
    .concat();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let result = (|| {
        let mut file = options
            .open(&temporary)
            .map_err(storage_error("create daemon status"))?;
        file.write_all(&bytes)
            .map_err(storage_error("write daemon status"))?;
        file.sync_all()
            .map_err(storage_error("sync daemon status"))?;
        drop(file);
        fs::rename(&temporary, &destination).map_err(storage_error("publish daemon status"))?;
        sync_directory(runtime)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn epoch_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
}

fn sha256_serialized<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(serialization_error)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn render_value<T: Serialize>(value: &T, format: AutomationFormatArg) -> Result<String> {
    match format {
        AutomationFormatArg::Json => render_json(value),
        AutomationFormatArg::KeyValue => {
            let value = serde_json::to_value(value).map_err(serialization_error)?;
            let object = value.as_object().ok_or_else(|| {
                WorkVcsError::CanonicalEncodingInvalid(
                    "automation key-value output requires an object".to_owned(),
                )
            })?;
            let mut output = String::new();
            for (key, value) in object {
                writeln!(
                    output,
                    "{}={}",
                    key,
                    escape_key_value(&render_json_scalar(value)?)
                )
                .expect("write to String");
            }
            Ok(output)
        }
    }
}

fn render_json<T: Serialize>(value: &T) -> Result<String> {
    let mut output = serde_json::to_string_pretty(value).map_err(serialization_error)?;
    output.push('\n');
    Ok(output)
}

fn render_json_scalar(value: &serde_json::Value) -> Result<String> {
    match value {
        serde_json::Value::Null => Ok("none".to_owned()),
        serde_json::Value::Bool(value) => Ok(value.to_string()),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        serde_json::Value::String(value) => Ok(value.clone()),
        _ => serde_json::to_string(value).map_err(serialization_error),
    }
}

fn serialization_error(error: serde_json::Error) -> WorkVcsError {
    WorkVcsError::CanonicalEncodingInvalid(format!("cannot serialize automation result: {error}"))
}

fn storage_error(context: &'static str) -> impl Fn(std::io::Error) -> WorkVcsError {
    move |error| WorkVcsError::StorageFailure(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_test_rollout(path: &Path, text: &str) {
        let thread = "01900000-0000-7000-8000-000000000001";
        let user = "01900000-0000-7000-8000-000000000002";
        let assistant = "01900000-0000-7000-8000-000000000003";
        let lines = [
            serde_json::json!({"type":"session_meta","timestamp":"2026-01-01T00:00:00Z","payload":{"id":thread}}),
            serde_json::json!({"type":"response_item","timestamp":"2026-01-01T00:00:01Z","payload":{"type":"message","role":"user","id":user,"content":[{"type":"input_text","text":text}]}}),
            serde_json::json!({"type":"response_item","timestamp":"2026-01-01T00:00:02Z","payload":{"type":"message","role":"assistant","id":assistant,"content":[{"type":"output_text","text":"historical answer"}]}}),
        ];
        let bytes = lines
            .into_iter()
            .map(|line| serde_json::to_string(&line).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, format!("{bytes}\n")).unwrap();
    }

    fn automation_fixture(
        temporary: &TempDir,
    ) -> (PathBuf, AutomationRegistration, PathBuf, PathBuf) {
        let automation_root = temporary.path().join("automation");
        let project_root = temporary.path().join("project");
        let rollout_root = temporary.path().join("rollouts");
        fs::create_dir(&automation_root).unwrap();
        fs::create_dir(&project_root).unwrap();
        fs::create_dir(&rollout_root).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&automation_root, fs::Permissions::from_mode(0o700)).unwrap();
        let source = rollout_root.join("thread.jsonl");
        write_test_rollout(&source, "retained historical evidence");
        let store_path = temporary.path().join("unavailable-store.sqlite");
        fs::write(&store_path, b"not a Store").unwrap();
        let registration = AutomationRegistration {
            registration_id: "registration-1".to_owned(),
            revision: 1,
            project_ref_id: ProjectRefId::new_v7(),
            store_path,
            store_id: StoreId::new_v7(),
            workspace_id: WorkspaceId::new_v7(),
            branch_id: BranchId::new_v7(),
            project_root,
            rollout_roots: vec![rollout_root.clone()],
            role_policy: AutomationRolePolicy::UserAndAssistant,
            registry_revision: 1,
            registry_digest: "sha256:unavailable-test-basis".to_owned(),
            enabled: true,
            revoked: false,
            created_at_us: 1,
            updated_at_us: 1,
        };
        let body = serde_json::json!({
            "format_version": 1,
            "revision": 1,
            "registrations": [registration.clone()],
        });
        let registry_path = automation_root.join("registrations.json");
        fs::write(&registry_path, serde_json::to_vec(&body).unwrap()).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&registry_path, fs::Permissions::from_mode(0o600)).unwrap();
        (automation_root, registration, rollout_root, source)
    }

    #[cfg(unix)]
    fn test_ledger() -> DaemonLedger {
        DaemonLedger {
            protocol_version: AUTOMATION_PROTOCOL_VERSION,
            pid: 1,
            started_at_us: 1,
            updated_at_us: 1,
            last_trigger: "startup".to_owned(),
            last_worker_status: None,
            last_worker_exit_code: None,
            last_worker_timed_out: false,
            last_worker_output_truncated: false,
            last_worker_evidence_path: None,
            last_worker_stdout_bytes: 0,
            last_worker_stdout_sha256: None,
            last_worker_stderr_bytes: 0,
            last_worker_stderr_sha256: None,
            last_error: None,
            last_catalog_path: None,
            last_catalog_manifest_sha256: None,
            last_catalog_complete: None,
            watcher_events: 0,
            watcher_errors: 0,
            watcher_coalesced_events: 0,
            last_watcher_error: None,
            rejected_requests: 0,
            last_protocol_error: None,
            daemon_enabled_by_default: false,
            launch_agent_installed: false,
        }
    }

    #[test]
    fn bounded_protocol_rejects_wrong_version_and_oversized_frame() {
        let request = DaemonRequest {
            version: 2,
            action: "status".to_owned(),
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let parsed: DaemonRequest = read_bounded_json(&mut bytes.as_slice(), "request").unwrap();
        assert_eq!(parsed.version, 2);
        let oversized = vec![b'x'; MAX_DAEMON_FRAME_BYTES + 1];
        let error =
            read_bounded_json::<DaemonRequest>(&mut oversized.as_slice(), "request").unwrap_err();
        assert!(matches!(error, WorkVcsError::ControlPlaneInvalid(_)));
    }

    #[test]
    fn rollout_discovery_is_sorted_and_rejects_symlinks() {
        let temporary = TempDir::new().unwrap();
        fs::write(temporary.path().join("b.jsonl"), b"{}\n").unwrap();
        fs::write(temporary.path().join("a.jsonl"), b"{}\n").unwrap();
        fs::write(temporary.path().join("ignored.txt"), b"ignored").unwrap();
        let files = collect_rollout_files(temporary.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("a.jsonl"));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                temporary.path().join("a.jsonl"),
                temporary.path().join("unsafe.jsonl"),
            )
            .unwrap();
            assert!(collect_rollout_files(temporary.path()).is_err());
        }
    }

    #[test]
    fn source_inventory_classifies_new_unchanged_append_rewrite_and_delete() {
        let temporary = TempDir::new().unwrap();
        let source = temporary.path().join("rollout.jsonl");
        fs::write(&source, b"first\n").unwrap();

        let first = fingerprint_source(&source).unwrap();
        assert_eq!(
            classify_source_change(&source, None, &first).unwrap(),
            SourceChange::New
        );
        assert_eq!(
            classify_source_change(&source, Some(&first), &first).unwrap(),
            SourceChange::Unchanged
        );

        OpenOptions::new()
            .append(true)
            .open(&source)
            .unwrap()
            .write_all(b"second\n")
            .unwrap();
        let appended = fingerprint_source(&source).unwrap();
        assert_eq!(
            classify_source_change(&source, Some(&first), &appended).unwrap(),
            SourceChange::Appended
        );

        fs::write(&source, b"changed bytes\n").unwrap();
        let rewritten = fingerprint_source(&source).unwrap();
        assert_eq!(
            classify_source_change(&source, Some(&appended), &rewritten).unwrap(),
            SourceChange::Rewritten
        );

        let automation_root = temporary.path().join("automation");
        let reference = SourceArchiveReference {
            artifact_path: temporary.path().join("archive"),
            artifact_manifest_sha256: "a".repeat(64),
            source_path: source.clone(),
            source_bytes: rewritten.bytes,
            source_sha256: rewritten.sha256.clone(),
            source_anchor: format!("path:{};sha256:{}", source.display(), rewritten.sha256),
        };
        let mut files = BTreeMap::new();
        files.insert(
            source.display().to_string(),
            SourceInventoryEntry {
                fingerprint: rewritten.clone(),
                archive: Some(reference.clone()),
            },
        );
        write_source_inventory(&automation_root, "registration-1", files.clone()).unwrap();
        let persisted = read_source_inventory(&automation_root, "registration-1").unwrap();
        assert_eq!(persisted.schema_version, 2);
        assert_eq!(persisted.files, files);
        assert!(archive_reference_matches_source(
            &reference, &source, &rewritten
        ));

        let legacy = SourceInventoryV1 {
            schema_version: 1,
            observed_at_us: 7,
            files: BTreeMap::from([(source.display().to_string(), rewritten.clone())]),
        };
        fs::write(
            automation_root
                .join("inventory")
                .join("registration-1.json"),
            serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        let migrated = read_source_inventory(&automation_root, "registration-1").unwrap();
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.observed_at_us, 7);
        assert_eq!(
            migrated
                .files
                .get(source.to_str().unwrap())
                .unwrap()
                .archive,
            None
        );
        fs::remove_file(&source).unwrap();
        assert!(files.keys().all(|path| !Path::new(path).exists()));

        let mut current = BTreeMap::new();
        let deleted = files
            .iter()
            .filter(|(path, _)| !current.contains_key(*path))
            .map(|(path, entry)| (path.clone(), entry.clone()))
            .collect::<Vec<_>>();
        for (path, entry) in deleted {
            current.insert(path, entry);
        }
        assert_eq!(current, files, "deleted source tombstones must persist");
    }

    #[test]
    fn canonical_failure_does_not_skip_history_and_missing_root_retains_archive() {
        let temporary = TempDir::new().unwrap();
        let (root, registration, rollout_root, source) = automation_fixture(&temporary);
        let first = reconcile_once(&root, None).unwrap();
        assert_eq!(first.status, "partial_failure");
        assert_eq!(first.canonical_generations_failed, 1);
        assert_eq!(first.rollout_archives_ready, 1);
        let first_inventory = read_source_inventory(&root, &registration.registration_id).unwrap();
        let remembered = first_inventory
            .files
            .get(source.to_str().unwrap())
            .unwrap()
            .archive
            .clone()
            .unwrap();

        fs::remove_file(&source).unwrap();
        fs::remove_dir(&rollout_root).unwrap();
        let second = reconcile_once(&root, None).unwrap();
        assert_eq!(second.status, "partial_failure");
        assert_eq!(second.rollout_archives_ready, 1);
        assert!(second.rollout_archives_failed >= 1);
        let manifest = read_automation_catalog(&second.catalog_path).unwrap();
        assert!(manifest.sources.iter().any(|item| {
            item.lane == AutomationLane::HistoricalReference
                && item.status == AutomationSourceStatus::Ready
                && item.artifact_path == remembered.artifact_path
        }));
        assert!(manifest.sources.iter().any(|item| {
            item.lane == AutomationLane::HistoricalReference
                && item.status == AutomationSourceStatus::Failed
                && item.artifact_path == rollout_root
        }));
        let second_inventory = read_source_inventory(&root, &registration.registration_id).unwrap();
        assert_eq!(
            second_inventory
                .files
                .get(source.to_str().unwrap())
                .unwrap()
                .archive
                .as_ref(),
            Some(&remembered)
        );
        #[cfg(unix)]
        {
            let (_receiver, _watcher, health) = start_watcher(&root).unwrap();
            let mut ledger = test_ledger();
            project_watcher_health(&health, &mut ledger);
            assert!(ledger.watcher_errors >= 1);
            assert!(
                ledger
                    .last_watcher_error
                    .as_deref()
                    .is_some_and(|message| message.contains("rollout root"))
            );
        }
    }

    #[test]
    fn total_source_failure_is_failed_for_reconcile_and_query() {
        let temporary = TempDir::new().unwrap();
        let (root, _registration, _rollout_root, source) = automation_fixture(&temporary);
        fs::write(&source, b"not-json\n").unwrap();

        let report = reconcile_once(&root, None).unwrap();
        assert_eq!(report.status, "failed");
        assert!(!report.complete);
        assert_eq!(report.rollout_archives_ready, 0);
        assert!(report.rollout_archives_failed >= 1);
        assert_eq!(report.canonical_generations_ready, 0);
        assert!(report.canonical_generations_failed >= 1);

        let query = query_catalog(&root, &report.catalog_path, "unavailable", None).unwrap();
        assert_eq!(query.status, "failed");
        assert!(!query.complete);
        assert_eq!(query.sources_succeeded, 0);
        assert_eq!(query.sources_failed, query.sources_requested);
        assert!(query.items.is_empty());
    }

    fn assert_zero_source_failure(root: &Path) {
        let report = reconcile_once(root, None).unwrap();
        assert_eq!(report.status, "failed");
        assert!(!report.complete);
        assert_eq!(report.registrations_considered, 0);
        assert_eq!(report.canonical_generations_ready, 0);
        assert_eq!(report.rollout_archives_ready, 0);
        let catalog = read_automation_catalog(&report.catalog_path).unwrap();
        assert!(!catalog.complete);
        assert!(catalog.sources.is_empty());

        let query = query_catalog(root, &report.catalog_path, "no sources", None).unwrap();
        assert_eq!(query.status, "failed");
        assert!(!query.complete);
        assert_eq!(query.sources_requested, 0);
        assert_eq!(query.sources_succeeded, 0);
        assert_eq!(query.sources_failed, 0);
        assert!(query.items.is_empty());
    }

    #[test]
    fn empty_registry_reconcile_and_query_are_failed() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("empty-automation");
        fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        assert_zero_source_failure(&root);
    }

    #[test]
    fn all_revoked_registry_reconcile_and_query_are_failed() {
        let temporary = TempDir::new().unwrap();
        let (root, registration, _rollout_root, _source) = automation_fixture(&temporary);
        let registry = read_automation_registry(&root).unwrap();
        let revoked = revoke_automation_registration(
            &root,
            registry.revision,
            &registry.digest_sha256,
            &registration.registration_id,
        )
        .unwrap();
        assert!(revoked.registrations.iter().all(|item| item.revoked));
        assert_zero_source_failure(&root);
    }

    #[test]
    fn user_only_archive_excludes_assistant_and_matches_explicit_source_query() {
        let temporary = TempDir::new().unwrap();
        let (root, registration, _rollout_root, _source) = automation_fixture(&temporary);
        let registry_path = root.join("registrations.json");
        let mut registry: serde_json::Value =
            serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
        registry["registrations"][0]["role_policy"] = serde_json::json!("user_only");
        fs::write(&registry_path, serde_json::to_vec(&registry).unwrap()).unwrap();

        let report = reconcile_once(&root, None).unwrap();
        assert_eq!(report.status, "partial_failure");
        let catalog = read_automation_catalog(&report.catalog_path).unwrap();
        let history = catalog
            .sources
            .iter()
            .find(|source| {
                source.lane == AutomationLane::HistoricalReference
                    && source.artifact_kind == AutomationArtifactKind::SourceArchive
                    && source.status == AutomationSourceStatus::Ready
            })
            .unwrap();
        let identity = SearchSourceIdentity::new(
            registration.project_ref_id,
            registration.store_id,
            registration.workspace_id,
            registration.branch_id,
        );

        let user_query = "retained historical evidence";
        let catalog_user = query_catalog(&root, &report.catalog_path, user_query, None).unwrap();
        let explicit_user = Engine::query_source_archive(
            SourceArchiveQueryOptions::new(identity, &history.artifact_path, user_query).unwrap(),
        )
        .unwrap();
        assert!(!explicit_user.manifest.include_assistant);
        assert!(
            explicit_user
                .items
                .iter()
                .all(|item| item.document.source_role == "user")
        );
        assert_eq!(catalog_user.matched_items, explicit_user.items.len());
        assert_eq!(
            catalog_user
                .items
                .iter()
                .map(|item| item.payload.clone())
                .collect::<Vec<_>>(),
            explicit_user
                .items
                .iter()
                .map(|item| serde_json::to_value(item).unwrap())
                .collect::<Vec<_>>()
        );
        assert!(
            catalog_user
                .items
                .iter()
                .enumerate()
                .all(|(index, item)| item.source_local_rank == index + 1
                    && item.federated_score == 1.0 / (60.0 + (index + 1) as f64))
        );

        let catalog_assistant = query_catalog(&root, &report.catalog_path, "answer", None).unwrap();
        let explicit_assistant = Engine::query_source_archive(
            SourceArchiveQueryOptions::new(identity, &history.artifact_path, "answer").unwrap(),
        )
        .unwrap();
        assert_eq!(catalog_assistant.matched_items, 0);
        assert!(catalog_assistant.items.is_empty());
        assert!(explicit_assistant.items.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unsafe_rollout_root_retains_last_validated_archive() {
        let temporary = TempDir::new().unwrap();
        let (root, registration, rollout_root, source) = automation_fixture(&temporary);
        reconcile_once(&root, None).unwrap();
        let remembered = read_source_inventory(&root, &registration.registration_id)
            .unwrap()
            .files
            .get(source.to_str().unwrap())
            .unwrap()
            .archive
            .clone()
            .unwrap();
        fs::set_permissions(&rollout_root, fs::Permissions::from_mode(0o777)).unwrap();
        let report = reconcile_once(&root, None).unwrap();
        fs::set_permissions(&rollout_root, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(report.status, "partial_failure");
        let manifest = read_automation_catalog(&report.catalog_path).unwrap();
        assert!(manifest.sources.iter().any(|item| {
            item.status == AutomationSourceStatus::Ready
                && item.artifact_path == remembered.artifact_path
        }));
        assert!(manifest.sources.iter().any(|item| {
            item.status == AutomationSourceStatus::Failed && item.artifact_path == rollout_root
        }));
    }

    #[test]
    fn fingerprint_failure_preserves_prior_archive_and_failed_observation() {
        let temporary = TempDir::new().unwrap();
        let (root, registration, _rollout_root, source) = automation_fixture(&temporary);
        reconcile_once(&root, None).unwrap();
        let inventory = read_source_inventory(&root, &registration.registration_id).unwrap();
        let path_key = source.to_str().unwrap().to_owned();
        let previous = inventory.files.get(&path_key).unwrap();
        fs::remove_file(&source).unwrap();
        let error = fingerprint_source(&source).unwrap_err();
        let identity = SearchSourceIdentity::new(
            registration.project_ref_id,
            registration.store_id,
            registration.workspace_id,
            registration.branch_id,
        );
        let mut current = BTreeMap::new();
        let mut sources = Vec::new();
        let mut ready = 0;
        let mut failed = 0;
        record_historical_observation_failure(
            &registration,
            identity,
            &path_key,
            source,
            Some(previous),
            &error,
            &mut current,
            &mut sources,
            &mut ready,
            &mut failed,
        );
        assert_eq!(current.get(&path_key), Some(previous));
        assert_eq!(ready, 1);
        assert_eq!(failed, 1);
        assert!(
            sources
                .iter()
                .any(|item| item.status == AutomationSourceStatus::Ready)
        );
        assert!(
            sources
                .iter()
                .any(|item| item.status == AutomationSourceStatus::Failed)
        );
    }

    #[cfg(unix)]
    #[test]
    fn rollout_discovery_rejects_writable_source_tree() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("rollouts");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o777)).unwrap();
        let error = collect_rollout_files(&root).unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));
    }

    #[cfg(unix)]
    #[test]
    fn automation_destination_canonicalizes_symlinked_existing_ancestor() {
        let temporary = TempDir::new().unwrap();
        let real = temporary.path().join("real");
        fs::create_dir(&real).unwrap();
        let alias = temporary.path().join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        let resolved = canonicalize_destination(&alias.join("new").join("derived")).unwrap();
        assert_eq!(
            resolved,
            real.canonicalize().unwrap().join("new").join("derived")
        );
    }

    #[test]
    fn latest_catalog_is_discovered_without_a_mutable_pointer() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("automation");
        let registry = read_automation_registry(&root).unwrap();
        let published = publish_automation_catalog(
            &root,
            registry.revision,
            registry.digest_sha256,
            Vec::new(),
        )
        .unwrap();
        let latest = latest_catalog_for_current_basis(&root).unwrap();
        assert_eq!(latest.catalog_path, published.catalog_path);
        assert_eq!(latest.catalog_manifest_sha256, published.manifest_sha256);
        assert!(!latest.manifest.complete);
        assert!(latest.manifest.sources.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn watcher_signal_is_capacity_one_and_records_errors() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let health = WatcherHealth::default();
        signal_watcher_event(&sender, &health, Ok(()));
        signal_watcher_event(&sender, &health, Ok(()));
        signal_watcher_event(
            &sender,
            &health,
            Err("watch backend reported overflow".to_owned()),
        );
        assert_eq!(receiver.try_recv(), Ok(()));
        let mut ledger = test_ledger();
        project_watcher_health(&health, &mut ledger);
        assert_eq!(ledger.watcher_events, 2);
        assert_eq!(ledger.watcher_errors, 1);
        assert_eq!(ledger.watcher_coalesced_events, 2);
        assert_eq!(
            ledger.last_watcher_error.as_deref(),
            Some("watch backend reported overflow")
        );
    }

    #[cfg(unix)]
    #[test]
    fn worker_capture_preserves_nonzero_and_timeout_evidence() {
        let mut failed = ProcessCommand::new("/bin/sh")
            .arg("-c")
            .arg("printf 'raw-worker-stdout'; printf 'raw-worker-stderr' >&2; exit 7")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let failed = wait_for_worker(&mut failed, Duration::from_secs(2)).unwrap();
        assert_eq!(failed.exit_code, Some(7));
        assert!(!failed.timed_out);
        assert_eq!(failed.stdout.bytes, b"raw-worker-stdout");
        assert_eq!(failed.stderr.bytes, b"raw-worker-stderr");

        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("automation");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let mut ledger = test_ledger();
        record_worker(&mut ledger, "test-nonzero", failed, &root).unwrap();
        assert_eq!(ledger.last_worker_status.as_deref(), Some("failed"));
        assert!(
            ledger
                .last_error
                .as_deref()
                .unwrap()
                .contains("raw-worker-stderr")
        );
        let evidence_path = ledger.last_worker_evidence_path.as_ref().unwrap();
        let evidence: WorkerEvidence =
            serde_json::from_slice(&fs::read(evidence_path).unwrap()).unwrap();
        assert_eq!(
            fs::read(evidence.stdout_path).unwrap(),
            b"raw-worker-stdout"
        );
        assert_eq!(
            fs::read(evidence.stderr_path).unwrap(),
            b"raw-worker-stderr"
        );

        let mut sleeping = ProcessCommand::new("/bin/sh")
            .arg("-c")
            .arg("sleep 2")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let timed_out = wait_for_worker(&mut sleeping, Duration::from_millis(20)).unwrap();
        assert!(timed_out.timed_out);
        record_worker(&mut ledger, "test-timeout", timed_out, &root).unwrap();
        assert!(ledger.last_worker_timed_out);
        assert_eq!(ledger.last_worker_status.as_deref(), Some("failed"));
        assert!(ledger.last_error.as_deref().unwrap().contains("terminated"));

        record_worker(
            &mut ledger,
            "test-malformed",
            WorkerOutcome {
                exit_code: Some(0),
                timed_out: false,
                stdout: capture_worker_stream(b"not-json".as_slice()).unwrap(),
                stderr: capture_worker_stream(b"".as_slice()).unwrap(),
            },
            &root,
        )
        .unwrap();
        assert!(
            ledger
                .last_error
                .as_deref()
                .unwrap()
                .contains("not a valid report")
        );

        let oversized = vec![b'x'; MAX_WORKER_STREAM_BYTES + 1];
        record_worker(
            &mut ledger,
            "test-overflow",
            WorkerOutcome {
                exit_code: Some(0),
                timed_out: false,
                stdout: capture_worker_stream(oversized.as_slice()).unwrap(),
                stderr: capture_worker_stream(b"".as_slice()).unwrap(),
            },
            &root,
        )
        .unwrap();
        assert!(ledger.last_worker_output_truncated);
        assert!(ledger.last_error.as_deref().unwrap().contains("exceeded"));
    }

    #[cfg(unix)]
    #[test]
    fn worker_report_projects_failed_catalog_without_false_success() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("automation");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let registry = read_automation_registry(&root).unwrap();
        let source = AutomationCatalogSource {
            registration_id: "registration-1".to_owned(),
            project_ref_id: ProjectRefId::new_v7(),
            lane: AutomationLane::HistoricalReference,
            artifact_kind: AutomationArtifactKind::SourceArchive,
            artifact_path: root.join("missing"),
            artifact_manifest_sha256: "0".repeat(64),
            source_anchor: "unavailable".to_owned(),
            status: AutomationSourceStatus::Failed,
            error_code: Some("source_missing".to_owned()),
            error: Some("source is missing".to_owned()),
        };
        let catalog = publish_automation_catalog(
            &root,
            registry.revision,
            registry.digest_sha256.clone(),
            vec![source],
        )
        .unwrap();
        let report = ReconcileReport {
            status: "failed".to_owned(),
            complete: false,
            automation_registry_revision: registry.revision,
            automation_registry_digest_sha256: registry.digest_sha256,
            registrations_considered: 1,
            registrations_ready: 0,
            registrations_quarantined: 1,
            rollout_files_discovered: 0,
            rollout_files_new: 0,
            rollout_files_appended: 0,
            rollout_files_unchanged: 0,
            rollout_files_rewritten: 0,
            rollout_files_deleted: 0,
            rollout_archives_ready: 0,
            rollout_archives_failed: 1,
            canonical_generations_ready: 0,
            canonical_generations_failed: 1,
            catalog_path: catalog.catalog_path,
            catalog_manifest_sha256: catalog.manifest_sha256,
            canonical_source_written: false,
            project_registry_written: false,
            source_files_written: false,
            existing_artifacts_deleted: false,
        };
        let bytes = serde_json::to_vec(&report).unwrap();
        let mut ledger = test_ledger();
        record_worker(
            &mut ledger,
            "manual",
            WorkerOutcome {
                exit_code: Some(0),
                timed_out: false,
                stdout: capture_worker_stream(bytes.as_slice()).unwrap(),
                stderr: capture_worker_stream([].as_slice()).unwrap(),
            },
            &root,
        )
        .unwrap();
        assert_eq!(ledger.last_worker_status.as_deref(), Some("failed"));
        assert!(ledger.last_error.is_none());
        let mut response = DaemonResponse {
            version: AUTOMATION_PROTOCOL_VERSION,
            status: "completed".to_owned(),
            action: "reconcile".to_owned(),
            message: "request completed".to_owned(),
            worker_exit_code: Some(0),
        };
        project_worker_status(&ledger, &mut response);
        assert_eq!(response.status, "failed");
    }

    #[cfg(unix)]
    #[test]
    fn daemon_status_projects_partial_and_failed_worker_state() {
        let mut ledger = test_ledger();
        ledger.last_worker_status = Some("partial_failure".to_owned());
        ledger.last_worker_exit_code = Some(0);
        ledger.last_catalog_complete = Some(false);
        ledger.last_worker_evidence_path = Some(PathBuf::from("/evidence/last-worker.json"));
        ledger.last_worker_stdout_sha256 = Some("a".repeat(64));
        ledger.last_catalog_path = Some(PathBuf::from("/catalogs/partial"));
        ledger.last_catalog_manifest_sha256 = Some("b".repeat(64));
        ledger.watcher_events = 7;
        ledger.watcher_errors = 2;
        let mut response = DaemonResponse {
            version: AUTOMATION_PROTOCOL_VERSION,
            status: "completed".to_owned(),
            action: "status".to_owned(),
            message: "request completed".to_owned(),
            worker_exit_code: None,
        };
        project_daemon_status(&ledger, &mut response).unwrap();
        assert_eq!(response.action, "status");
        assert_eq!(response.status, "partial_failure");
        assert_eq!(response.worker_exit_code, Some(0));
        let projected: DaemonLedger = serde_json::from_str(&response.message).unwrap();
        assert_eq!(
            projected.last_worker_status.as_deref(),
            Some("partial_failure")
        );
        assert_eq!(
            projected.last_worker_evidence_path,
            ledger.last_worker_evidence_path
        );
        assert_eq!(
            projected.last_worker_stdout_sha256,
            ledger.last_worker_stdout_sha256
        );
        assert_eq!(projected.last_catalog_path, ledger.last_catalog_path);
        assert_eq!(
            projected.last_catalog_manifest_sha256,
            ledger.last_catalog_manifest_sha256
        );
        assert_eq!(projected.watcher_events, 7);
        assert_eq!(projected.watcher_errors, 2);

        ledger.last_worker_status = Some("failed".to_owned());
        ledger.last_worker_exit_code = Some(2);
        ledger.last_error = Some("exact persisted worker failure".to_owned());
        response.status = "completed".to_owned();
        response.message = "request completed".to_owned();
        response.worker_exit_code = None;
        project_daemon_status(&ledger, &mut response).unwrap();
        assert_eq!(response.action, "status");
        assert_eq!(response.status, "failed");
        assert_eq!(response.worker_exit_code, Some(2));
        let projected: DaemonLedger = serde_json::from_str(&response.message).unwrap();
        assert_eq!(projected.last_worker_status.as_deref(), Some("failed"));
        assert_eq!(
            projected.last_error.as_deref(),
            Some("exact persisted worker failure")
        );
        assert_eq!(
            projected.last_worker_evidence_path,
            ledger.last_worker_evidence_path
        );
        assert_eq!(projected.last_catalog_path, ledger.last_catalog_path);
    }

    #[cfg(unix)]
    #[test]
    fn protocol_rejection_is_persisted_without_becoming_worker_success() {
        let temporary = TempDir::new().unwrap();
        let runtime = temporary.path().join("runtime");
        ensure_owner_private_directory(&runtime).unwrap();
        let mut ledger = test_ledger();
        ledger.last_worker_status = Some("completed".to_owned());
        ledger.last_worker_exit_code = Some(0);
        record_protocol_rejection(&mut ledger, &runtime, "request is malformed".to_owned())
            .unwrap();
        let persisted: DaemonLedger =
            serde_json::from_slice(&fs::read(runtime.join("status.json")).unwrap()).unwrap();
        assert_eq!(persisted.rejected_requests, 1);
        assert_eq!(
            persisted.last_protocol_error.as_deref(),
            Some("request is malformed")
        );
        assert_eq!(persisted.last_worker_exit_code, Some(0));
        assert!(persisted.last_error.is_none());

        ledger.last_worker_status = Some("partial_failure".to_owned());
        ledger.last_catalog_complete = Some(false);
        let mut response = DaemonResponse {
            version: AUTOMATION_PROTOCOL_VERSION,
            status: "completed".to_owned(),
            action: "reconcile".to_owned(),
            message: "request completed".to_owned(),
            worker_exit_code: Some(0),
        };
        project_worker_status(&ledger, &mut response);
        assert_eq!(response.status, "partial_failure");
        assert!(response.message.contains("incomplete catalog"));

        ledger.last_error = Some("exact worker failure".to_owned());
        project_worker_status(&ledger, &mut response);
        assert_eq!(response.status, "failed");
        assert_eq!(response.message, "exact worker failure");
    }
}
