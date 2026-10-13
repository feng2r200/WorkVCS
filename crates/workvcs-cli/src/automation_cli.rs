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
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use workvcs_core::{
    AutomationArtifactKind, AutomationCatalogManifest, AutomationCatalogSource, AutomationLane,
    AutomationRegistration, AutomationRegistrationProposal, AutomationRolePolicy,
    AutomationSourcePolicyEntry, AutomationSourcePolicyProposal, AutomationSourceStatus,
    RegistryDerivedCatalogBasis, SearchDerivedStatus, SourceArchiveInspectOptions,
    apply_automation_registration, apply_automation_source_policy, automation_source_policy_path,
    preview_automation_registration, preview_automation_source_policy, publish_automation_catalog,
    publish_registry_derived_automation_catalog, read_automation_catalog, read_automation_registry,
    read_automation_source_policy, revoke_automation_registration, revoke_automation_source_policy,
};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

const AUTOMATION_PROTOCOL_VERSION: u32 = 1;
const MAX_DAEMON_FRAME_BYTES: usize = 16 * 1024;
const MAX_WORKER_STREAM_BYTES: usize = 64 * 1024;
const RECONCILE_QUERY: &str = "automation reconciliation";
#[cfg(unix)]
const DAEMON_PROTOCOL_IO_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(unix)]
static DAEMON_TERMINATION_SIGNAL: AtomicI32 = AtomicI32::new(0);

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
    #[command(about = "Manage the separate global rollout-root source policy")]
    SourcePolicy {
        #[command(subcommand)]
        command: AutomationSourcePolicyCommand,
    },
    #[command(
        about = "List the active ProjectRef registry members for registry-derived automation"
    )]
    Membership {
        #[arg(long)]
        registry: Option<PathBuf>,
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
pub(crate) enum AutomationSourcePolicyCommand {
    #[command(about = "Preview one global source-policy entry without writing")]
    Preview {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        rollout_root: PathBuf,
        #[arg(long, value_enum, default_value_t = AutomationRolePolicyArg::UserOnly)]
        role_policy: AutomationRolePolicyArg,
        #[arg(long = "exclude-project-ref")]
        excluded_project_refs: Vec<String>,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "Apply one global source-policy entry against an exact policy basis")]
    Apply {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        rollout_root: PathBuf,
        #[arg(long, value_enum, default_value_t = AutomationRolePolicyArg::UserOnly)]
        role_policy: AutomationRolePolicyArg,
        #[arg(long = "exclude-project-ref")]
        excluded_project_refs: Vec<String>,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        expected_digest: String,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "List global source-policy entries, including revoked history")]
    List {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
    },
    #[command(about = "Revoke one global source-policy entry without deleting derived artifacts")]
    Revoke {
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long)]
        expected_digest: String,
        #[arg(long, value_enum, default_value_t = AutomationFormatArg::Json)]
        format: AutomationFormatArg,
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
        #[arg(
            long,
            default_value_t = 0,
            help = "Optional reconciliation worker wall-clock deadline; 0 disables automatic termination"
        )]
        worker_timeout_seconds: u64,
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
    membership_mode: String,
    automation_registry_revision: u64,
    automation_registry_digest_sha256: String,
    project_registry_id: Option<String>,
    project_registry_revision: Option<u64>,
    project_registry_digest: Option<String>,
    source_policy_revision: Option<u64>,
    source_policy_digest_sha256: Option<String>,
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
struct RegistryDerivedMember {
    project_ref_id: ProjectRefId,
    store_path: PathBuf,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
}

impl RegistryDerivedMember {
    fn identity(&self) -> SearchSourceIdentity {
        SearchSourceIdentity::new(
            self.project_ref_id,
            self.store_id,
            self.workspace_id,
            self.branch_id,
        )
    }

    fn catalog_member_id(&self) -> String {
        format!("project-ref:{}", self.project_ref_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RegistryDerivedMembership {
    registry_path: PathBuf,
    registry_id: String,
    registry_revision: u64,
    registry_digest: String,
    routing_activation_path: PathBuf,
    members: Vec<RegistryDerivedMember>,
    #[serde(skip)]
    historical_owner_index: BTreeMap<HistoricalOwnerKey, RegistryDerivedMember>,
    #[serde(skip)]
    historical_owner_boundaries: Vec<PathBuf>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HistoricalOwnerKey {
    identity_kind: String,
    identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HistoricalOwnerFailure {
    code: String,
    error: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HistoricalOwnerObservation {
    identity: HistoricalOwnerKey,
    metadata_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HistoricalAliasFingerprint {
    anchor: PathBuf,
    link_path: PathBuf,
    link_target: PathBuf,
    link_target_sha256: String,
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HistoricalProbeResolution {
    original_cwd: PathBuf,
    boundary: PathBuf,
    probe_cwd: PathBuf,
    alias: Option<HistoricalAliasFingerprint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HistoricalOwnerCacheEntry {
    resolution: HistoricalProbeResolution,
    observation: HistoricalOwnerObservation,
    owner: std::result::Result<RegistryDerivedMember, HistoricalOwnerFailure>,
}

type HistoricalOwnerCache = BTreeMap<PathBuf, HistoricalOwnerCacheEntry>;

type SourceArchiveImporter<'a> =
    dyn FnMut(SourceArchiveImportOptions) -> Result<SourceArchiveImportResult> + 'a;

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalSourceInventoryEntry {
    source_id: String,
    member: RegistryDerivedMember,
    fingerprint: SourceFingerprint,
    archive: Option<SourceArchiveReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalSourceInventory {
    schema_version: u32,
    observed_at_us: u128,
    files: BTreeMap<String, GlobalSourceInventoryEntry>,
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

#[derive(Clone, Debug)]
struct CatalogSourceBinding {
    identity: SearchSourceIdentity,
    store_path: PathBuf,
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
    launch_agent_installed: Option<bool>,
    worker_active: bool,
    active_worker_pid: Option<u32>,
    active_worker_trigger: Option<String>,
    active_worker_started_at_us: Option<u128>,
    worker_timeout_seconds: Option<u64>,
    pending_reconcile: bool,
    last_worker_duration_ms: Option<u128>,
    last_worker_cancelled: bool,
}

#[derive(Debug)]
struct WorkerOutcome {
    exit_code: Option<i32>,
    timed_out: bool,
    cancelled: bool,
    duration_ms: u128,
    timeout_seconds: Option<u64>,
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
    cancelled: bool,
    duration_ms: u128,
    timeout_seconds: Option<u64>,
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
struct ActiveWorker {
    child: Option<Child>,
    process_group_id: libc::pid_t,
    stdout_reader: Option<std::thread::JoinHandle<std::io::Result<CapturedStream>>>,
    stderr_reader: Option<std::thread::JoinHandle<std::io::Result<CapturedStream>>>,
    trigger: String,
    started: Instant,
    started_at_us: u128,
    timeout_seconds: Option<u64>,
}

#[cfg(unix)]
impl Drop for ActiveWorker {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            match worker_has_exited_without_reaping(&child) {
                Ok(true) => {
                    let _ = terminate_remaining_worker_process_group(&child, self.process_group_id);
                }
                Ok(false) | Err(_) => {
                    let _ = terminate_worker_process_group(&child, self.process_group_id);
                }
            }
            let _ = child.wait();
        }
        if let Some(reader) = self.stdout_reader.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
    }
}

#[cfg(unix)]
struct CompletedWorker {
    trigger: String,
    outcome: WorkerOutcome,
}

#[cfg(unix)]
struct DaemonSignalHandlers {
    previous_sigterm: libc::sigaction,
    previous_sigint: libc::sigaction,
}

#[cfg(unix)]
impl Drop for DaemonSignalHandlers {
    fn drop(&mut self) {
        unsafe {
            libc::sigaction(libc::SIGTERM, &self.previous_sigterm, std::ptr::null_mut());
            libc::sigaction(libc::SIGINT, &self.previous_sigint, std::ptr::null_mut());
        }
        DAEMON_TERMINATION_SIGNAL.store(0, Ordering::SeqCst);
    }
}

#[cfg(unix)]
extern "C" fn daemon_termination_signal_handler(signal: libc::c_int) {
    let _ =
        DAEMON_TERMINATION_SIGNAL.compare_exchange(0, signal, Ordering::SeqCst, Ordering::SeqCst);
}

#[cfg(unix)]
fn install_daemon_signal_handlers() -> Result<DaemonSignalHandlers> {
    DAEMON_TERMINATION_SIGNAL.store(0, Ordering::SeqCst);
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = daemon_termination_signal_handler as *const () as usize;
    action.sa_flags = 0;
    unsafe {
        libc::sigemptyset(&mut action.sa_mask);
    }
    let mut previous_sigterm: libc::sigaction = unsafe { std::mem::zeroed() };
    if unsafe { libc::sigaction(libc::SIGTERM, &action, &mut previous_sigterm) } != 0 {
        return Err(storage_error("install daemon SIGTERM handler")(
            std::io::Error::last_os_error(),
        ));
    }
    let mut previous_sigint: libc::sigaction = unsafe { std::mem::zeroed() };
    if unsafe { libc::sigaction(libc::SIGINT, &action, &mut previous_sigint) } != 0 {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::sigaction(libc::SIGTERM, &previous_sigterm, std::ptr::null_mut());
        }
        return Err(storage_error("install daemon SIGINT handler")(error));
    }
    Ok(DaemonSignalHandlers {
        previous_sigterm,
        previous_sigint,
    })
}

#[cfg(unix)]
fn take_daemon_termination_signal() -> Option<i32> {
    let signal = DAEMON_TERMINATION_SIGNAL.swap(0, Ordering::SeqCst);
    (signal != 0).then_some(signal)
}

#[cfg(unix)]
#[derive(Default)]
struct PendingReconcile {
    manual: bool,
    watcher: bool,
    periodic: bool,
}

#[cfg(unix)]
impl PendingReconcile {
    fn any(&self) -> bool {
        self.manual || self.watcher || self.periodic
    }

    fn take_trigger(&mut self) -> Option<&'static str> {
        let trigger = if self.manual {
            Some("manual")
        } else if self.watcher {
            Some("watcher_dirty_or_overflow")
        } else if self.periodic {
            Some("periodic")
        } else {
            None
        };
        if trigger.is_some() {
            *self = Self::default();
        }
        trigger
    }
}

#[cfg(unix)]
#[derive(Default)]
struct WatcherHealth {
    events: AtomicU64,
    errors: AtomicU64,
    coalesced_events: AtomicU64,
    last_error: Mutex<Option<String>>,
}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
enum WatchFilterRule {
    Exact(PathBuf),
    StoreFamily(PathBuf),
    Tree(PathBuf),
}

#[cfg(unix)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct WatchFilterSet {
    rules: Vec<WatchFilterRule>,
}

#[cfg(unix)]
impl WatchFilterSet {
    fn matches(&self, event: &notify::Event) -> bool {
        event.need_rescan()
            || event
                .paths
                .iter()
                .any(|path| self.rules.iter().any(|rule| watch_rule_matches(rule, path)))
    }
}

#[cfg(unix)]
#[derive(Debug, Default)]
struct WatchTargetPlan {
    backend_targets: BTreeMap<PathBuf, (bool, &'static str)>,
    filter: WatchFilterSet,
    discovery_error: Option<String>,
}

#[cfg(unix)]
struct DaemonWatcher {
    receiver: Receiver<()>,
    watcher: Option<notify::RecommendedWatcher>,
    health: Arc<WatcherHealth>,
    filter: Arc<Mutex<WatchFilterSet>>,
    backend_targets: BTreeMap<PathBuf, (bool, &'static str)>,
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
        AutomationCommand::SourcePolicy { command } => run_source_policy(command),
        AutomationCommand::Membership { registry, format } => {
            render_value(&registry_derived_membership(registry.as_deref())?, format)
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

fn run_source_policy(command: AutomationSourcePolicyCommand) -> Result<String> {
    match command {
        AutomationSourcePolicyCommand::Preview {
            registry,
            root,
            source_id,
            rollout_root,
            role_policy,
            excluded_project_refs,
            format,
        } => {
            let (root, proposal) = source_policy_proposal(
                root,
                registry,
                source_id,
                rollout_root,
                role_policy.into(),
                excluded_project_refs,
            )?;
            render_value(&preview_automation_source_policy(&root, proposal)?, format)
        }
        AutomationSourcePolicyCommand::Apply {
            registry,
            root,
            source_id,
            rollout_root,
            role_policy,
            excluded_project_refs,
            expected_revision,
            expected_digest,
            format,
        } => {
            let (root, proposal) = source_policy_proposal(
                root,
                registry,
                source_id,
                rollout_root,
                role_policy.into(),
                excluded_project_refs,
            )?;
            let result = apply_automation_source_policy(
                &root,
                expected_revision,
                &expected_digest,
                proposal,
            )?;
            render_value(&result, format)
        }
        AutomationSourcePolicyCommand::List {
            registry,
            root,
            format,
        } => render_value(
            &read_automation_source_policy(&resolve_automation_root(root, registry.as_deref())?)?,
            format,
        ),
        AutomationSourcePolicyCommand::Revoke {
            registry,
            root,
            source_id,
            expected_revision,
            expected_digest,
            format,
        } => {
            let root = resolve_automation_root(root, registry.as_deref())?;
            let result = revoke_automation_source_policy(
                &root,
                expected_revision,
                &expected_digest,
                &source_id,
            )?;
            render_value(&result, format)
        }
    }
}

fn source_policy_proposal(
    root: Option<PathBuf>,
    registry: Option<PathBuf>,
    source_id: String,
    rollout_root: PathBuf,
    role_policy: AutomationRolePolicy,
    excluded_project_refs: Vec<String>,
) -> Result<(PathBuf, AutomationSourcePolicyProposal)> {
    let root = resolve_automation_root(root, registry.as_deref())?;
    let excluded_project_refs = excluded_project_refs
        .into_iter()
        .map(|value| ProjectRefId::parse_canonical(&value))
        .collect::<Result<Vec<_>>>()?;
    Ok((
        root,
        AutomationSourcePolicyProposal {
            source_id,
            rollout_root,
            role_policy,
            excluded_project_refs,
        },
    ))
}

fn registry_derived_membership(
    project_registry: Option<&Path>,
) -> Result<RegistryDerivedMembership> {
    let effective = effective_registry_config(project_registry.map(Path::to_path_buf))?;
    let registry_path = canonical_registry_path(
        absolute_cli_path("project registry", effective.registry_path.clone())?,
        false,
    )?;
    let (registry, digest) = match load_project_registry_readonly(&registry_path, true)? {
        LoadedProjectRegistry::V1 { .. } => {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "registry-derived automation requires an active registry v2".to_owned(),
            ));
        }
        LoadedProjectRegistry::V2 { registry, digest } => (registry, digest),
    };
    let activation = require_active_read_routing(&effective, &registry_path, &registry)?;
    let members = registry
        .bindings()
        .iter()
        .map(|binding| RegistryDerivedMember {
            project_ref_id: binding.project_ref_id(),
            store_path: PathBuf::from(binding.store_path().as_str()),
            store_id: binding.store_id(),
            workspace_id: binding.workspace_id(),
            branch_id: binding.branch_id(),
        })
        .collect::<Vec<_>>();
    let members_by_project = members
        .iter()
        .map(|member| (member.project_ref_id, member.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut historical_owner_index = BTreeMap::new();
    let mut historical_owner_boundaries = Vec::new();
    for locator in registry.locators().iter().filter(|locator| {
        locator.role() == LocatorRole::Identity && locator.state() == LocatorState::Active
    }) {
        let identity_kind = match locator.authority() {
            LocatorAuthority::SemanticProject => continue,
            LocatorAuthority::Repository => "git-common-dir",
            LocatorAuthority::Cwd => "cwd",
        };
        let Some(member) = members_by_project.get(&locator.project_ref_id()).cloned() else {
            continue;
        };
        let key = HistoricalOwnerKey {
            identity_kind: identity_kind.to_owned(),
            identity: locator.normalized_value().to_owned(),
        };
        let identity_path = PathBuf::from(locator.normalized_value());
        let boundaries = if locator.authority() == LocatorAuthority::Repository {
            historical_repository_boundaries(&identity_path)
        } else {
            vec![identity_path]
        };
        for boundary in boundaries {
            historical_owner_boundaries.extend(
                historical_cwd_aliases_without_io(&boundary).unwrap_or_else(|_| vec![boundary]),
            );
        }
        if let Some(previous) = historical_owner_index.insert(key.clone(), member.clone())
            && previous != member
        {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "active historical owner locator {}:{} maps to multiple registry members",
                key.identity_kind, key.identity
            )));
        }
    }
    historical_owner_boundaries.sort();
    historical_owner_boundaries.dedup();
    Ok(RegistryDerivedMembership {
        registry_path,
        registry_id: registry.registry_id().to_string(),
        registry_revision: registry.revision(),
        registry_digest: digest.to_string(),
        routing_activation_path: activation.path,
        members,
        historical_owner_index,
        historical_owner_boundaries,
    })
}

fn historical_repository_boundaries(common_dir: &Path) -> Vec<PathBuf> {
    let mut boundaries = Vec::new();
    if common_dir.file_name().and_then(|value| value.to_str()) == Some(".git")
        && let Some(parent) = common_dir.parent()
        && parent.parent().is_some()
    {
        boundaries.push(parent.to_path_buf());
    }
    let worktrees = common_dir.join("worktrees");
    let Ok(entries) = fs::read_dir(&worktrees) else {
        return boundaries;
    };
    for entry in entries.flatten() {
        let admin = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&admin) else {
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let Ok(bytes) =
            read_historical_git_metadata(&admin.join("gitdir"), "Git worktree backlink")
        else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let target = Path::new(text.trim_end_matches(['\r', '\n']));
        let target = if target.is_absolute() {
            target.to_path_buf()
        } else {
            admin.join(target)
        };
        let Ok(target) = normalize_historical_cwd_without_io(&target) else {
            continue;
        };
        if target.file_name().and_then(|value| value.to_str()) != Some(".git") {
            continue;
        }
        if let Some(root) = target.parent()
            && root.parent().is_some()
        {
            boundaries.push(root.to_path_buf());
        }
    }
    boundaries.sort();
    boundaries.dedup();
    boundaries
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
    if automation_source_policy_path(root).exists() {
        reconcile_registry_derived_once(root, project_registry)
    } else {
        reconcile_legacy_once(root, project_registry)
    }
}

fn reconcile_legacy_once(root: &Path, project_registry: Option<&Path>) -> Result<ReconcileReport> {
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
        membership_mode: "legacy_registration".to_owned(),
        automation_registry_revision: registry.revision,
        automation_registry_digest_sha256: registry.digest_sha256,
        project_registry_id: None,
        project_registry_revision: None,
        project_registry_digest: None,
        source_policy_revision: None,
        source_policy_digest_sha256: None,
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

fn reconcile_registry_derived_once(
    root: &Path,
    project_registry: Option<&Path>,
) -> Result<ReconcileReport> {
    let mut importer = Engine::import_source_archive;
    reconcile_registry_derived_once_with_importer(root, project_registry, &mut importer)
}

fn reconcile_registry_derived_once_with_importer(
    root: &Path,
    project_registry: Option<&Path>,
    importer: &mut SourceArchiveImporter<'_>,
) -> Result<ReconcileReport> {
    let _reconciliation_lock = ReconciliationLock::acquire(root)?;
    let membership = registry_derived_membership(project_registry)?;
    let source_policy = read_automation_source_policy(root)?;
    let active_sources = source_policy
        .sources
        .iter()
        .filter(|source| source.enabled && !source.revoked)
        .collect::<Vec<_>>();
    let mut catalog_sources = Vec::new();
    let mut canonical_generations_ready = 0;
    let mut canonical_generations_failed = 0;
    let mut rollout_files_discovered = 0;
    let mut rollout_files_new = 0;
    let mut rollout_files_appended = 0;
    let mut rollout_files_unchanged = 0;
    let mut rollout_files_rewritten = 0;
    let mut rollout_files_deleted = 0;
    let mut rollout_archives_ready = 0;
    let mut rollout_archives_failed = 0;

    for member in &membership.members {
        let canonical_root = root
            .join("generations")
            .join("registry-derived")
            .join(member.project_ref_id.to_string())
            .join("lexical");
        match Engine::open_readonly(&member.store_path) {
            Ok(engine) => {
                let mut options = SearchQueryOptions::managed(
                    member.identity(),
                    &canonical_root,
                    RECONCILE_QUERY,
                )?
                .with_protected_path(membership.registry_path.clone());
                options = options.with_protected_path(membership.routing_activation_path.clone());
                match engine.search(options) {
                    Ok(result) if result.status == SearchDerivedStatus::Ready => {
                        let manifest = result.manifest.ok_or_else(|| {
                            WorkVcsError::IntegrityInvalid(
                                "ready registry-derived search result omitted its manifest"
                                    .to_owned(),
                            )
                        })?;
                        canonical_generations_ready += 1;
                        catalog_sources.push(AutomationCatalogSource {
                            registration_id: member.catalog_member_id(),
                            project_ref_id: member.project_ref_id,
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
                        catalog_sources.push(failed_global_catalog_source(
                            member,
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
                        catalog_sources.push(failed_global_catalog_source(
                            member,
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
                catalog_sources.push(failed_global_catalog_source(
                    member,
                    AutomationLane::CanonicalCurrent,
                    AutomationArtifactKind::LexicalGeneration,
                    canonical_root,
                    &error.code().to_string(),
                    &error.to_string(),
                ));
            }
        }
    }

    let previous_inventory = read_global_source_inventory(root)?;
    let mut current_inventory = BTreeMap::new();
    let mut historical_owner_cache = HistoricalOwnerCache::new();
    for source in active_sources {
        run_registry_derived_historical_reconciliation(
            root,
            source,
            &membership,
            &mut historical_owner_cache,
            &previous_inventory,
            &mut current_inventory,
            &mut catalog_sources,
            &mut rollout_files_discovered,
            &mut rollout_files_new,
            &mut rollout_files_appended,
            &mut rollout_files_unchanged,
            &mut rollout_files_rewritten,
            &mut rollout_files_deleted,
            &mut rollout_archives_ready,
            &mut rollout_archives_failed,
            importer,
        )?;
    }
    validate_historical_owner_cache(&historical_owner_cache, &membership)?;
    let current_membership = registry_derived_membership(Some(&membership.registry_path))?;
    if current_membership != membership {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "ProjectRef registry membership or routing activation changed during registry-derived reconciliation"
                .to_owned(),
        ));
    }
    write_global_source_inventory(root, current_inventory)?;

    let publication = publish_registry_derived_automation_catalog(
        root,
        RegistryDerivedCatalogBasis {
            project_registry_id: membership.registry_id.clone(),
            project_registry_revision: membership.registry_revision,
            project_registry_digest: membership.registry_digest.clone(),
            source_policy_revision: source_policy.revision,
            source_policy_digest_sha256: source_policy.digest_sha256.clone(),
        },
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
        membership_mode: "registry_derived".to_owned(),
        automation_registry_revision: 0,
        automation_registry_digest_sha256: "0".repeat(64),
        project_registry_id: Some(membership.registry_id),
        project_registry_revision: Some(membership.registry_revision),
        project_registry_digest: Some(membership.registry_digest),
        source_policy_revision: Some(source_policy.revision),
        source_policy_digest_sha256: Some(source_policy.digest_sha256),
        registrations_considered: membership.members.len(),
        registrations_ready: membership.members.len(),
        registrations_quarantined: 0,
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
fn run_registry_derived_historical_reconciliation(
    root: &Path,
    source_policy: &AutomationSourcePolicyEntry,
    membership: &RegistryDerivedMembership,
    historical_owner_cache: &mut HistoricalOwnerCache,
    previous_inventory: &GlobalSourceInventory,
    current_inventory: &mut BTreeMap<String, GlobalSourceInventoryEntry>,
    catalog_sources: &mut Vec<AutomationCatalogSource>,
    rollout_files_discovered: &mut usize,
    rollout_files_new: &mut usize,
    rollout_files_appended: &mut usize,
    rollout_files_unchanged: &mut usize,
    rollout_files_rewritten: &mut usize,
    rollout_files_deleted: &mut usize,
    rollout_archives_ready: &mut usize,
    rollout_archives_failed: &mut usize,
    importer: &mut SourceArchiveImporter<'_>,
) -> Result<()> {
    let archive_root = root
        .join("history")
        .join("registry-derived")
        .join(&source_policy.source_id);
    let files = match collect_rollout_files(&source_policy.rollout_root) {
        Ok(files) => files,
        Err(error) => {
            let mut retained = 0;
            for (path, entry) in previous_inventory
                .files
                .iter()
                .filter(|(_, entry)| entry.source_id == source_policy.source_id)
            {
                retained += 1;
                current_inventory.insert(path.clone(), entry.clone());
                retain_or_quarantine_global_archive(
                    source_policy,
                    membership,
                    entry,
                    source_policy.rollout_root.clone(),
                    &error.code().to_string(),
                    &error.to_string(),
                    catalog_sources,
                    rollout_archives_ready,
                    rollout_archives_failed,
                )?;
            }
            *rollout_archives_failed += 1;
            if retained == 0 {
                catalog_sources.push(failed_unattributed_catalog_source(
                    source_policy.rollout_root.clone(),
                    &error.code().to_string(),
                    &error.to_string(),
                )?);
            }
            return Ok(());
        }
    };
    *rollout_files_discovered += files.len();
    for file in files {
        let path_key = file
            .to_str()
            .ok_or_else(|| {
                WorkVcsError::QueryInvalid(
                    "registry-derived rollout path must be valid UTF-8".to_owned(),
                )
            })?
            .to_owned();
        let previous = previous_inventory
            .files
            .get(&path_key)
            .filter(|entry| entry.source_id == source_policy.source_id);
        let fingerprint = match fingerprint_source(&file) {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                if let Some(entry) = previous {
                    current_inventory.insert(path_key.clone(), entry.clone());
                    retain_or_quarantine_global_archive(
                        source_policy,
                        membership,
                        entry,
                        file,
                        &error.code().to_string(),
                        &error.to_string(),
                        catalog_sources,
                        rollout_archives_ready,
                        rollout_archives_failed,
                    )?;
                } else {
                    catalog_sources.push(failed_unattributed_catalog_source(
                        file,
                        &error.code().to_string(),
                        &error.to_string(),
                    )?);
                }
                *rollout_archives_failed += 1;
                continue;
            }
        };
        let member =
            match resolve_registry_derived_source_member(&file, membership, historical_owner_cache)
            {
                Ok(member) if !source_policy_excludes_member(source_policy, &member) => member,
                Ok(member) => {
                    let error = WorkVcsError::ControlPlaneInvalid(format!(
                        "rollout source {} resolves to excluded ProjectRef {}",
                        file.display(),
                        member.project_ref_id
                    ));
                    if let Some(entry) = previous {
                        current_inventory.insert(path_key.clone(), entry.clone());
                        catalog_sources.push(failed_global_catalog_source(
                            &entry.member,
                            AutomationLane::HistoricalReference,
                            AutomationArtifactKind::SourceArchive,
                            file,
                            &error.code().to_string(),
                            &error.to_string(),
                        ));
                    } else {
                        catalog_sources.push(failed_unattributed_catalog_source(
                            file,
                            &error.code().to_string(),
                            &error.to_string(),
                        )?);
                    }
                    *rollout_archives_failed += 1;
                    continue;
                }
                Err(error) => {
                    if let Some(entry) = previous {
                        current_inventory.insert(path_key.clone(), entry.clone());
                        catalog_sources.push(failed_global_catalog_source(
                            &entry.member,
                            AutomationLane::HistoricalReference,
                            AutomationArtifactKind::SourceArchive,
                            file,
                            &error.code,
                            &error.error,
                        ));
                    } else {
                        catalog_sources.push(failed_unattributed_catalog_source(
                            file,
                            &error.code,
                            &error.error,
                        )?);
                    }
                    *rollout_archives_failed += 1;
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
        let prior_archive = previous
            .filter(|entry| entry.member == member)
            .and_then(|entry| entry.archive.clone());
        let mut entry = GlobalSourceInventoryEntry {
            source_id: source_policy.source_id.clone(),
            member: member.clone(),
            fingerprint: fingerprint.clone(),
            archive: prior_archive,
        };
        let reusable = change == SourceChange::Unchanged
            && entry.archive.as_ref().is_some_and(|reference| {
                archive_reference_matches_source(reference, &file, &fingerprint)
            });
        if reusable {
            retain_global_archive(
                &member,
                &entry,
                catalog_sources,
                rollout_archives_ready,
                rollout_archives_failed,
            );
        } else {
            let member_archive_root = archive_root.join(member.project_ref_id.to_string());
            let options = SourceArchiveImportOptions::new(
                member.identity(),
                &member_archive_root,
                vec![file.clone()],
            )?
            .include_assistant(matches!(
                source_policy.role_policy,
                AutomationRolePolicy::UserAndAssistant
            ));
            match importer(options) {
                Ok(result) => {
                    let reference = archive_reference(result.archive_path, &result.manifest)?;
                    if !archive_reference_matches_source(&reference, &file, &fingerprint) {
                        let error = source_changed_during_archive_import_error(
                            &file,
                            &fingerprint,
                            &reference,
                            result.outcome,
                            entry.archive.is_some(),
                        );
                        retain_global_archive(
                            &member,
                            &entry,
                            catalog_sources,
                            rollout_archives_ready,
                            rollout_archives_failed,
                        );
                        *rollout_archives_failed += 1;
                        catalog_sources.push(failed_global_catalog_source(
                            &member,
                            AutomationLane::HistoricalReference,
                            AutomationArtifactKind::SourceArchive,
                            file,
                            &error.code().to_string(),
                            &error.to_string(),
                        ));
                    } else {
                        entry.archive = Some(reference.clone());
                        *rollout_archives_ready += 1;
                        catalog_sources.push(AutomationCatalogSource {
                            registration_id: member.catalog_member_id(),
                            project_ref_id: member.project_ref_id,
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
                }
                Err(error) => {
                    retain_global_archive(
                        &member,
                        &entry,
                        catalog_sources,
                        rollout_archives_ready,
                        rollout_archives_failed,
                    );
                    *rollout_archives_failed += 1;
                    catalog_sources.push(failed_global_catalog_source(
                        &member,
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
        .filter(|(path, entry)| {
            entry.source_id == source_policy.source_id && !current_inventory.contains_key(*path)
        })
        .map(|(path, entry)| (path.clone(), entry.clone()))
        .collect::<Vec<_>>();
    for (deleted, entry) in deleted_sources {
        *rollout_files_deleted += 1;
        current_inventory.insert(deleted.clone(), entry.clone());
        retain_or_quarantine_global_archive(
            source_policy,
            membership,
            &entry,
            PathBuf::from(&deleted),
            "source_deleted",
            "previously configured rollout source is no longer present",
            catalog_sources,
            rollout_archives_ready,
            rollout_archives_failed,
        )?;
        *rollout_archives_failed += 1;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn retain_or_quarantine_global_archive(
    source_policy: &AutomationSourcePolicyEntry,
    membership: &RegistryDerivedMembership,
    entry: &GlobalSourceInventoryEntry,
    artifact_path: PathBuf,
    error_code: &str,
    error: &str,
    catalog_sources: &mut Vec<AutomationCatalogSource>,
    rollout_archives_ready: &mut usize,
    rollout_archives_failed: &mut usize,
) -> Result<()> {
    if let Some(member) = active_registry_member(membership, &entry.member)
        && !source_policy_excludes_member(source_policy, &member)
    {
        retain_global_archive(
            &member,
            entry,
            catalog_sources,
            rollout_archives_ready,
            rollout_archives_failed,
        );
    }
    catalog_sources.push(failed_global_catalog_source(
        &entry.member,
        AutomationLane::HistoricalReference,
        AutomationArtifactKind::SourceArchive,
        artifact_path,
        error_code,
        error,
    ));
    Ok(())
}

fn retain_global_archive(
    member: &RegistryDerivedMember,
    entry: &GlobalSourceInventoryEntry,
    catalog_sources: &mut Vec<AutomationCatalogSource>,
    rollout_archives_ready: &mut usize,
    rollout_archives_failed: &mut usize,
) {
    let Some(reference) = entry.archive.as_ref() else {
        return;
    };
    match ready_global_archive_catalog_source(member, reference) {
        Ok(source) => {
            *rollout_archives_ready += 1;
            catalog_sources.push(source);
        }
        Err(error) => {
            *rollout_archives_failed += 1;
            catalog_sources.push(failed_global_catalog_source(
                member,
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

fn global_source_inventory_path(root: &Path) -> PathBuf {
    root.join("inventory").join("registry-derived.json")
}

fn read_global_source_inventory(root: &Path) -> Result<GlobalSourceInventory> {
    let path = global_source_inventory_path(root);
    if !path.exists() {
        return Ok(GlobalSourceInventory {
            schema_version: 1,
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
        .map_err(storage_error("open registry-derived source inventory"))?;
    #[cfg(unix)]
    {
        let metadata = file
            .metadata()
            .map_err(storage_error("inspect registry-derived source inventory"))?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            return Err(WorkVcsError::IntegrityInvalid(
                "registry-derived source inventory is not owner-private".to_owned(),
            ));
        }
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(storage_error("read registry-derived source inventory"))?;
    let inventory: GlobalSourceInventory = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!(
            "registry-derived source inventory is malformed: {error}"
        ))
    })?;
    if inventory.schema_version != 1 {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "registry-derived source inventory has unsupported schema version {}",
            inventory.schema_version
        )));
    }
    Ok(inventory)
}

fn write_global_source_inventory(
    root: &Path,
    files: BTreeMap<String, GlobalSourceInventoryEntry>,
) -> Result<()> {
    let directory = root.join("inventory");
    ensure_owner_private_directory(&directory)?;
    let destination = global_source_inventory_path(root);
    let temporary = directory.join(format!(
        ".registry-derived.{}.{}.tmp",
        std::process::id(),
        epoch_micros()
    ));
    let inventory = GlobalSourceInventory {
        schema_version: 1,
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
            .map_err(storage_error("create registry-derived source inventory"))?;
        file.write_all(&bytes)
            .map_err(storage_error("write registry-derived source inventory"))?;
        file.sync_all()
            .map_err(storage_error("sync registry-derived source inventory"))?;
        drop(file);
        fs::rename(&temporary, &destination)
            .map_err(storage_error("publish registry-derived source inventory"))?;
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

fn source_changed_during_archive_import_error(
    source: &Path,
    pre_fingerprint: &SourceFingerprint,
    imported: &SourceArchiveReference,
    outcome: workvcs_core::SourceArchiveImportOutcome,
    prior_archive_present: bool,
) -> WorkVcsError {
    let outcome = match outcome {
        workvcs_core::SourceArchiveImportOutcome::Imported => "imported",
        workvcs_core::SourceArchiveImportOutcome::Reused => "reused",
    };
    let prior_archive = if prior_archive_present {
        "present"
    } else {
        "absent"
    };
    WorkVcsError::IntegrityInvalid(format!(
        "source_changed_during_archive_import: path={} pre_bytes={} pre_sha256={} import_bytes={} import_sha256={} outcome={} prior_archive={}",
        source.display(),
        pre_fingerprint.bytes,
        pre_fingerprint.sha256,
        imported.source_bytes,
        imported.source_sha256,
        outcome,
        prior_archive,
    ))
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

fn ready_global_archive_catalog_source(
    member: &RegistryDerivedMember,
    reference: &SourceArchiveReference,
) -> Result<AutomationCatalogSource> {
    let inspection = Engine::inspect_source_archive(SourceArchiveInspectOptions::new(
        member.identity(),
        &reference.artifact_path,
    ))?;
    if !inspection.archive_readback_verified
        || inspection.manifest.manifest_sha256 != reference.artifact_manifest_sha256
    {
        return Err(WorkVcsError::IntegrityInvalid(
            "remembered registry-derived source archive manifest or readback changed".to_owned(),
        ));
    }
    let inspected = archive_reference(inspection.archive_path, &inspection.manifest)?;
    if &inspected != reference {
        return Err(WorkVcsError::IntegrityInvalid(
            "remembered registry-derived source archive reference does not match immutable archive contents"
                .to_owned(),
        ));
    }
    Ok(AutomationCatalogSource {
        registration_id: member.catalog_member_id(),
        project_ref_id: member.project_ref_id,
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

fn failed_global_catalog_source(
    member: &RegistryDerivedMember,
    lane: AutomationLane,
    artifact_kind: AutomationArtifactKind,
    artifact_path: PathBuf,
    error_code: &str,
    error: &str,
) -> AutomationCatalogSource {
    AutomationCatalogSource {
        registration_id: member.catalog_member_id(),
        project_ref_id: member.project_ref_id,
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

fn failed_unattributed_catalog_source(
    artifact_path: PathBuf,
    error_code: &str,
    error: &str,
) -> Result<AutomationCatalogSource> {
    // Catalog v1 sources require a ProjectRef even for failures.  This fixed
    // non-member sentinel keeps an attribution failure visible without
    // asserting that it belongs to a real ProjectRef.
    let project_ref_id = ProjectRefId::parse_canonical("01900000-0000-7000-8000-000000000000")?;
    Ok(AutomationCatalogSource {
        registration_id: "registry-derived-quarantine".to_owned(),
        project_ref_id,
        lane: AutomationLane::HistoricalReference,
        artifact_kind: AutomationArtifactKind::SourceArchive,
        artifact_path,
        artifact_manifest_sha256: "0".repeat(64),
        source_anchor: "quarantine:unattributed".to_owned(),
        status: AutomationSourceStatus::Failed,
        error_code: Some(error_code.to_owned()),
        error: Some(error.to_owned()),
    })
}

fn rollout_cwd_from_session_meta(source: &Path) -> Result<PathBuf> {
    let bytes = fs::read(source).map_err(storage_error("read rollout metadata"))?;
    let mut observed = None::<String>;
    let mut session_meta_records = 0_usize;
    for (offset, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(line).map_err(|error| {
            WorkVcsError::QueryInvalid(format!(
                "rollout source {} line {} is invalid JSON: {error}",
                source.display(),
                offset + 1
            ))
        })?;
        if value.get("type").and_then(serde_json::Value::as_str) != Some("session_meta") {
            continue;
        }
        session_meta_records += 1;
        let cwd = value
            .get("payload")
            .and_then(|payload| payload.get("cwd"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "rollout source {} session_meta has no trusted cwd",
                    source.display()
                ))
            })?;
        let candidate = Path::new(cwd);
        if !candidate.is_absolute()
            || candidate.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::CurDir | std::path::Component::ParentDir
                )
            })
        {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "rollout source {} session_meta cwd is not an absolute normalized path",
                source.display()
            )));
        }
        if let Some(previous) = &observed {
            if previous != cwd {
                return Err(WorkVcsError::ControlPlaneInvalid(format!(
                    "rollout source {} has inconsistent session_meta cwd values",
                    source.display()
                )));
            }
        } else {
            observed = Some(cwd.to_owned());
        }
    }
    if session_meta_records == 0 {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "rollout source {} has no session_meta metadata",
            source.display()
        )));
    }
    observed.map(PathBuf::from).ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "rollout source {} has no usable session_meta cwd",
            source.display()
        ))
    })
}

fn resolve_registry_derived_source_member(
    source: &Path,
    membership: &RegistryDerivedMembership,
    cache: &mut HistoricalOwnerCache,
) -> std::result::Result<RegistryDerivedMember, HistoricalOwnerFailure> {
    let failure = |error: WorkVcsError, context: String| HistoricalOwnerFailure {
        code: error.code().to_string(),
        error: format!("{context}: {error}"),
    };
    let cwd = rollout_cwd_from_session_meta(source).map_err(|error| {
        failure(
            error,
            format!(
                "cannot recover historical owner for rollout source {}",
                source.display()
            ),
        )
    })?;
    let normalized_cwd = normalize_historical_cwd_without_io(&cwd).map_err(|error| {
        failure(
            error,
            format!(
                "cannot normalize historical rollout cwd {} without filesystem access",
                cwd.display()
            ),
        )
    })?;
    let lexical_cwds = historical_cwd_aliases_without_io(&normalized_cwd).map_err(|error| {
        failure(
            error,
            format!(
                "cannot derive historical rollout cwd aliases for {} without filesystem access",
                cwd.display()
            ),
        )
    })?;
    let selected = select_historical_probe_path(
        &normalized_cwd,
        &membership.historical_owner_boundaries,
        &lexical_cwds,
    );
    let cached = cache.get(&normalized_cwd);
    let selected = match (selected, cached) {
        (Ok(selected), _) => selected,
        (Err(error), Some(cached)) => {
            return Err(failure(
                WorkVcsError::ControlPlaneInvalid(format!(
                    "historical rollout cwd {} could not reproduce its cached local resolution {:?}: {error}",
                    cwd.display(),
                    cached.resolution
                )),
                format!(
                    "cannot reuse historical owner attribution for rollout source {}",
                    source.display()
                ),
            ));
        }
        (Err(error), None) => {
            return Err(failure(
                error,
                format!(
                    "cannot safely enter historical rollout cwd {} from an active local identity boundary",
                    cwd.display()
                ),
            ));
        }
    };
    let Some(resolution) = selected else {
        if let Some(cached) = cached {
            return Err(failure(
                WorkVcsError::ControlPlaneInvalid(format!(
                    "historical rollout cwd {} no longer reproduces cached local resolution {:?}",
                    cwd.display(),
                    cached.resolution
                )),
                format!(
                    "cannot reuse historical owner attribution for rollout source {}",
                    source.display()
                ),
            ));
        }
        let error = WorkVcsError::ProjectOwnerUnbound {
            registry_path: membership.registry_path.display().to_string(),
            resolution_rank: "verified-local-boundary".to_owned(),
            unmapped_locators: 1,
            resolution_diagnostics: 0,
        };
        return Err(failure(
            error,
            format!(
                "historical rollout cwd {} is outside every active local identity boundary in registry revision {}",
                cwd.display(),
                membership.registry_revision
            ),
        ));
    };
    let observation = observe_historical_owner(&resolution.probe_cwd).map_err(|error| {
        failure(
            error,
            format!(
                "cannot inspect historical rollout cwd {} without an external Git process",
                cwd.display()
            ),
        )
    })?;
    if let Some(cached) = cached {
        if cached.resolution != resolution {
            return Err(failure(
                WorkVcsError::ControlPlaneInvalid(format!(
                    "historical rollout cwd {} changed local resolution during reconciliation from {:?} to {:?}",
                    cwd.display(),
                    cached.resolution,
                    resolution
                )),
                format!(
                    "cannot reuse historical owner attribution for rollout source {}",
                    source.display()
                ),
            ));
        }
        if cached.observation != observation {
            return Err(failure(
                WorkVcsError::ControlPlaneInvalid(format!(
                    "historical rollout cwd {} changed identity metadata during reconciliation from {}:{}@{} to {}:{}@{}",
                    resolution.probe_cwd.display(),
                    cached.observation.identity.identity_kind,
                    cached.observation.identity.identity,
                    cached.observation.metadata_sha256,
                    observation.identity.identity_kind,
                    observation.identity.identity,
                    observation.metadata_sha256
                )),
                format!(
                    "cannot reuse historical owner attribution for rollout source {}",
                    source.display()
                ),
            ));
        }
        return cached.owner.clone();
    }
    let key = &observation.identity;
    let resolved = membership
        .historical_owner_index
        .get(key)
        .cloned()
        .ok_or_else(|| {
            let error = WorkVcsError::ProjectOwnerUnbound {
                registry_path: membership.registry_path.display().to_string(),
                resolution_rank: if key.identity_kind == "git-common-dir" {
                    "repository"
                } else {
                    "cwd"
                }
                .to_owned(),
                unmapped_locators: 1,
                resolution_diagnostics: 0,
            };
            failure(
                error,
                format!(
                    "historical rollout cwd {} has no exact active {} owner locator for {} in registry revision {}",
                    cwd.display(),
                    key.identity_kind,
                    key.identity,
                    membership.registry_revision
                ),
            )
        });
    cache.insert(
        normalized_cwd,
        HistoricalOwnerCacheEntry {
            resolution,
            observation,
            owner: resolved.clone(),
        },
    );
    resolved
}

fn validate_historical_owner_cache(
    cache: &HistoricalOwnerCache,
    membership: &RegistryDerivedMembership,
) -> Result<()> {
    for (original_cwd, cached) in cache {
        let candidates = historical_cwd_aliases_without_io(original_cwd).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "historical rollout cwd {} aliases could not be revalidated after reconciliation: {error}",
                original_cwd.display()
            ))
        })?;
        let current_resolution = select_historical_probe_path(
            original_cwd,
            &membership.historical_owner_boundaries,
            &candidates,
        )
        .map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "historical rollout cwd {} local resolution could not be revalidated after reconciliation: {error}",
                original_cwd.display()
            ))
        })?
        .ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "historical rollout cwd {} no longer resolves through its cached active local identity boundary",
                original_cwd.display()
            ))
        })?;
        if current_resolution != cached.resolution {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "historical rollout cwd {} changed local resolution during reconciliation from {:?} to {:?}",
                original_cwd.display(),
                cached.resolution,
                current_resolution
            )));
        }
        let current = observe_historical_owner(&current_resolution.probe_cwd).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "historical rollout cwd {} could not be revalidated after reconciliation: {error}",
                original_cwd.display()
            ))
        })?;
        if current != cached.observation {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "historical rollout cwd {} changed identity metadata during reconciliation from {}:{}@{} to {}:{}@{}",
                original_cwd.display(),
                cached.observation.identity.identity_kind,
                cached.observation.identity.identity,
                cached.observation.metadata_sha256,
                current.identity.identity_kind,
                current.identity.identity,
                current.metadata_sha256
            )));
        }
    }
    Ok(())
}

fn normalize_historical_cwd_without_io(cwd: &Path) -> Result<PathBuf> {
    if !cwd.is_absolute() {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "historical rollout cwd {} is not absolute",
            cwd.display()
        )));
    }
    let mut normalized = PathBuf::new();
    for component in cwd.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => normalized.push(value),
            std::path::Component::ParentDir => {
                return Err(WorkVcsError::ControlPlaneInvalid(format!(
                    "historical rollout cwd {} contains a parent-directory component",
                    cwd.display()
                )));
            }
        }
    }
    Ok(normalized)
}

fn verify_historical_probe_path(boundary: &Path, candidate: &Path) -> Result<PathBuf> {
    let relative = candidate.strip_prefix(boundary).map_err(|_| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "historical cwd {} is outside active identity boundary {}",
            candidate.display(),
            boundary.display()
        ))
    })?;
    verify_absolute_directory_path_without_symlinks(
        boundary,
        "active historical identity boundary",
    )?;
    let mut cursor = boundary.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "historical cwd {} has a non-normal component below active boundary {}",
                candidate.display(),
                boundary.display()
            )));
        }
        cursor.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&cursor).map_err(|error| {
            WorkVcsError::QueryInvalid(format!(
                "cannot inspect historical cwd component {}: {error}",
                cursor.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "historical cwd component {} is a symlink that could escape its active identity boundary",
                cursor.display()
            )));
        }
        if !metadata.is_dir() {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "historical cwd component {} is not a directory",
                cursor.display()
            )));
        }
    }
    Ok(candidate.to_path_buf())
}

fn select_historical_probe_path(
    original_cwd: &Path,
    boundaries: &[PathBuf],
    candidates: &[PathBuf],
) -> Result<Option<HistoricalProbeResolution>> {
    let mut matches = boundaries
        .iter()
        .flat_map(|boundary| {
            candidates.iter().filter_map(move |candidate| {
                candidate
                    .starts_with(boundary)
                    .then_some((boundary, candidate))
            })
        })
        .collect::<Vec<_>>();
    if let Some(maximum_depth) = matches
        .iter()
        .map(|(boundary, _)| boundary.components().count())
        .max()
    {
        matches.retain(|(boundary, _)| boundary.components().count() == maximum_depth);
        let mut last_error = None;
        for (boundary, candidate) in matches {
            match verify_historical_probe_path(boundary, candidate) {
                Ok(path) => {
                    return Ok(Some(HistoricalProbeResolution {
                        original_cwd: original_cwd.to_path_buf(),
                        boundary: boundary.clone(),
                        probe_cwd: path,
                        alias: None,
                    }));
                }
                Err(error) => last_error = Some(error),
            }
        }
        return Err(last_error.expect("a matching historical boundary has a validation result"));
    }

    select_dynamic_historical_alias(original_cwd, boundaries, candidates)
}

#[cfg(unix)]
fn select_dynamic_historical_alias(
    original_cwd: &Path,
    boundaries: &[PathBuf],
    candidates: &[PathBuf],
) -> Result<Option<HistoricalProbeResolution>> {
    let mut resolutions = Vec::new();
    let mut safely_inspected_candidate = false;
    let mut last_anchor_error = None;
    for candidate_cwd in candidates {
        for boundary in boundaries {
            let Some(anchor) = common_historical_path_ancestor(candidate_cwd, boundary) else {
                continue;
            };
            if anchor
                .components()
                .filter(|component| matches!(component, std::path::Component::Normal(_)))
                .count()
                < 2
            {
                continue;
            }
            let relative = candidate_cwd
                .strip_prefix(&anchor)
                .expect("shared path ancestor is a prefix");
            let Some(std::path::Component::Normal(alias_name)) = relative.components().next()
            else {
                continue;
            };
            if let Err(error) = verify_absolute_directory_path_without_symlinks(
                &anchor,
                "historical alias trusted anchor",
            ) {
                last_anchor_error = Some(error);
                continue;
            }
            let alias_path = anchor.join(alias_name);
            let metadata = match fs::symlink_metadata(&alias_path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    safely_inspected_candidate = true;
                    continue;
                }
                Err(error) => {
                    return Err(WorkVcsError::QueryInvalid(format!(
                        "cannot inspect historical alias candidate {}: {error}",
                        alias_path.display()
                    )));
                }
            };
            safely_inspected_candidate = true;
            if !metadata.file_type().is_symlink() {
                continue;
            }
            let fingerprint = snapshot_historical_alias(&anchor, &alias_path)?;
            let target = normalize_absolute_single_hop_alias_target(&fingerprint.link_target)?;
            let suffix = candidate_cwd.strip_prefix(&alias_path).map_err(|_| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "historical alias candidate {} is not a prefix of {}",
                    alias_path.display(),
                    candidate_cwd.display()
                ))
            })?;
            let expanded = normalize_historical_cwd_without_io(&target.join(suffix))?;
            if !expanded.starts_with(boundary) {
                continue;
            }
            let probe_cwd = verify_historical_probe_path(boundary, &expanded)?;
            let stable_fingerprint = snapshot_historical_alias(&anchor, &alias_path)?;
            if stable_fingerprint != fingerprint {
                return Err(WorkVcsError::ControlPlaneInvalid(format!(
                    "historical alias {} changed while its active-boundary resolution was being verified",
                    alias_path.display()
                )));
            }
            resolutions.push(HistoricalProbeResolution {
                original_cwd: original_cwd.to_path_buf(),
                boundary: boundary.clone(),
                probe_cwd,
                alias: Some(stable_fingerprint),
            });
        }
    }
    resolutions.sort_by(|left, right| {
        left.probe_cwd
            .cmp(&right.probe_cwd)
            .then_with(|| {
                right
                    .boundary
                    .components()
                    .count()
                    .cmp(&left.boundary.components().count())
            })
            .then_with(|| left.boundary.cmp(&right.boundary))
    });
    resolutions.dedup();
    let Some(first) = resolutions.first() else {
        if !safely_inspected_candidate && let Some(error) = last_anchor_error {
            return Err(error);
        }
        return Ok(None);
    };
    if resolutions
        .iter()
        .any(|candidate| candidate.probe_cwd != first.probe_cwd)
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "historical rollout cwd {} has ambiguous active-boundary alias resolutions",
            original_cwd.display()
        )));
    }
    Ok(resolutions.into_iter().next())
}

#[cfg(not(unix))]
fn select_dynamic_historical_alias(
    _original_cwd: &Path,
    _boundaries: &[PathBuf],
    _candidates: &[PathBuf],
) -> Result<Option<HistoricalProbeResolution>> {
    Ok(None)
}

fn common_historical_path_ancestor(left: &Path, right: &Path) -> Option<PathBuf> {
    let mut shared = PathBuf::new();
    let mut matched = false;
    for (left_component, right_component) in left.components().zip(right.components()) {
        if left_component != right_component {
            break;
        }
        shared.push(left_component.as_os_str());
        matched = true;
    }
    matched.then_some(shared)
}

#[cfg(unix)]
fn historical_alias_metadata_fingerprint(
    anchor: &Path,
    link_path: &Path,
    link_target: &Path,
    metadata: &fs::Metadata,
) -> HistoricalAliasFingerprint {
    let mut digest = Sha256::new();
    digest.update(link_target.as_os_str().as_bytes());
    HistoricalAliasFingerprint {
        anchor: anchor.to_path_buf(),
        link_path: link_path.to_path_buf(),
        link_target: link_target.to_path_buf(),
        link_target_sha256: hex::encode(digest.finalize()),
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(unix)]
fn snapshot_historical_alias(
    anchor: &Path,
    link_path: &Path,
) -> Result<HistoricalAliasFingerprint> {
    let before = fs::symlink_metadata(link_path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "cannot inspect historical alias {} before reading it: {error}",
            link_path.display()
        ))
    })?;
    if !before.file_type().is_symlink() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "historical alias {} changed to a non-symlink",
            link_path.display()
        )));
    }
    let target = fs::read_link(link_path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "cannot read historical alias {}: {error}",
            link_path.display()
        ))
    })?;
    let after = fs::symlink_metadata(link_path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "cannot inspect historical alias {} after reading it: {error}",
            link_path.display()
        ))
    })?;
    if !after.file_type().is_symlink() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "historical alias {} changed to a non-symlink while it was read",
            link_path.display()
        )));
    }
    let before = historical_alias_metadata_fingerprint(anchor, link_path, &target, &before);
    let after = historical_alias_metadata_fingerprint(anchor, link_path, &target, &after);
    if before != after {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "historical alias {} changed while it was being read",
            link_path.display()
        )));
    }
    Ok(after)
}

#[cfg(unix)]
fn normalize_absolute_single_hop_alias_target(target: &Path) -> Result<PathBuf> {
    if !target.is_absolute() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "historical alias target {} is relative; only absolute single-hop aliases are supported",
            target.display()
        )));
    }
    let mut normalized = PathBuf::new();
    for component in target.components() {
        match component {
            std::path::Component::RootDir | std::path::Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            std::path::Component::Prefix(_)
            | std::path::Component::CurDir
            | std::path::Component::ParentDir => {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "historical alias target {} is not an absolute normalized single-hop path",
                    target.display()
                )));
            }
        }
    }
    if normalized.as_os_str().as_bytes() != target.as_os_str().as_bytes() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "historical alias target {} is not an absolute normalized single-hop path",
            target.display()
        )));
    }
    Ok(normalized)
}

fn verify_absolute_directory_path_without_symlinks(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute() {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{label} {} is not absolute",
            path.display()
        )));
    }
    let mut cursor = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => cursor.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                cursor.push(component.as_os_str());
                continue;
            }
            std::path::Component::Normal(value) => cursor.push(value),
            std::path::Component::CurDir | std::path::Component::ParentDir => {
                return Err(WorkVcsError::ControlPlaneInvalid(format!(
                    "{label} {} contains a non-normal component",
                    path.display()
                )));
            }
        }
        let metadata = fs::symlink_metadata(&cursor).map_err(|error| {
            WorkVcsError::QueryInvalid(format!(
                "cannot inspect {label} component {}: {error}",
                cursor.display()
            ))
        })?;
        if metadata.file_type().is_symlink() {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "{label} component {} is a symlink",
                cursor.display()
            )));
        }
        if !metadata.is_dir() {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "{label} component {} is not a directory",
                cursor.display()
            )));
        }
    }
    Ok(())
}

fn historical_cwd_aliases_without_io(cwd: &Path) -> Result<Vec<PathBuf>> {
    let normalized = normalize_historical_cwd_without_io(cwd)?;
    let mut aliases = vec![normalized.clone()];
    #[cfg(target_os = "macos")]
    for public_root in ["/var", "/tmp", "/etc"] {
        if normalized.starts_with(public_root) {
            let relative = normalized.strip_prefix("/").expect("absolute path");
            aliases.push(Path::new("/private").join(relative));
            break;
        }
        let private_root = Path::new("/private").join(
            Path::new(public_root)
                .strip_prefix("/")
                .expect("public root is absolute"),
        );
        if normalized.starts_with(&private_root) {
            let relative = normalized
                .strip_prefix("/private")
                .expect("private path prefix");
            aliases.push(Path::new("/").join(relative));
            break;
        }
    }
    aliases.sort();
    aliases.dedup();
    Ok(aliases)
}

fn read_historical_git_metadata(path: &Path, label: &str) -> Result<Vec<u8>> {
    const MAX_METADATA_BYTES: u64 = 16 * 1024;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        WorkVcsError::QueryInvalid(format!(
            "cannot inspect {label} {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} must be a regular file, not a symlink or special file",
            path.display()
        )));
    }
    if metadata.len() > MAX_METADATA_BYTES {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} exceeds the {MAX_METADATA_BYTES} byte safety bound",
            path.display()
        )));
    }
    fs::read(path).map_err(storage_error("read historical Git identity metadata"))
}

fn hash_historical_metadata(digest: &mut Sha256, label: &str, bytes: &[u8]) {
    digest.update((label.len() as u64).to_le_bytes());
    digest.update(label.as_bytes());
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
}

fn observe_historical_owner(cwd: &Path) -> Result<HistoricalOwnerObservation> {
    verify_absolute_directory_path_without_symlinks(cwd, "historical rollout cwd")?;
    let identity = resolve_project_identity_without_git_subprocess(cwd)?;
    let key = HistoricalOwnerKey {
        identity_kind: identity.kind,
        identity: identity.identity,
    };
    let mut digest = Sha256::new();
    hash_historical_metadata(&mut digest, "identity-kind", key.identity_kind.as_bytes());
    hash_historical_metadata(&mut digest, "identity", key.identity.as_bytes());
    hash_historical_metadata(&mut digest, "root", identity.root.as_bytes());
    if key.identity_kind == "git-common-dir" {
        let marker = PathBuf::from(&identity.root).join(".git");
        let marker_metadata = fs::symlink_metadata(&marker)
            .map_err(storage_error("inspect historical Git metadata marker"))?;
        if marker_metadata.is_dir() {
            hash_historical_metadata(&mut digest, "marker-kind", b"directory");
            let head = read_historical_git_metadata(&marker.join("HEAD"), "Git HEAD")?;
            hash_historical_metadata(&mut digest, "head", &head);
        } else if marker_metadata.is_file() && !marker_metadata.file_type().is_symlink() {
            hash_historical_metadata(&mut digest, "marker-kind", b"worktree-pointer");
            let pointer = read_historical_git_metadata(&marker, "Git worktree pointer")?;
            hash_historical_metadata(&mut digest, "worktree-pointer", &pointer);
            let pointer_text = std::str::from_utf8(&pointer).map_err(|error| {
                WorkVcsError::IntegrityInvalid(format!(
                    "Git worktree pointer {} is not UTF-8: {error}",
                    marker.display()
                ))
            })?;
            let target = pointer_text
                .trim_end_matches(['\r', '\n'])
                .strip_prefix("gitdir: ")
                .ok_or_else(|| {
                    WorkVcsError::IntegrityInvalid(format!(
                        "Git worktree pointer {} has an invalid format",
                        marker.display()
                    ))
                })?;
            let target = Path::new(target);
            let target = if target.is_absolute() {
                target.to_path_buf()
            } else {
                marker
                    .parent()
                    .expect("Git marker has a parent")
                    .join(target)
            };
            let admin = canonical_existing_path("Git worktree metadata directory", &target)?;
            for (label, name) in [
                ("commondir", "commondir"),
                ("gitdir", "gitdir"),
                ("head", "HEAD"),
            ] {
                let bytes = read_historical_git_metadata(&admin.join(name), label)?;
                hash_historical_metadata(&mut digest, label, &bytes);
            }
        } else {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "Git metadata marker {} changed to an unsafe type",
                marker.display()
            )));
        }
    }
    Ok(HistoricalOwnerObservation {
        identity: key,
        metadata_sha256: hex::encode(digest.finalize()),
    })
}

fn source_policy_excludes_member(
    source: &AutomationSourcePolicyEntry,
    member: &RegistryDerivedMember,
) -> bool {
    source
        .excluded_project_refs
        .binary_search(&member.project_ref_id)
        .is_ok()
}

fn active_registry_member(
    membership: &RegistryDerivedMembership,
    candidate: &RegistryDerivedMember,
) -> Option<RegistryDerivedMember> {
    membership
        .members
        .iter()
        .find(|member| *member == candidate)
        .cloned()
}

fn run_catalog(command: AutomationCatalogCommand) -> Result<String> {
    match command {
        AutomationCatalogCommand::Show { catalog } => {
            let manifest = read_automation_catalog(&catalog)?;
            render_json(&manifest)
        }
        AutomationCatalogCommand::Latest { registry, root } => {
            let root = resolve_automation_root(root, registry.as_deref())?;
            render_json(&latest_catalog_for_current_basis(
                &root,
                registry.as_deref(),
            )?)
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
                None => latest_catalog_for_current_basis(&root, registry.as_deref())?.catalog_path,
            };
            let report = query_catalog(&root, registry.as_deref(), &catalog, &query, limit)?;
            render_json(&report)
        }
    }
}

fn latest_catalog_for_current_basis(
    root: &Path,
    project_registry: Option<&Path>,
) -> Result<CatalogLatestReport> {
    let global_mode = automation_source_policy_path(root).exists();
    let membership = global_mode
        .then(|| registry_derived_membership(project_registry))
        .transpose()?;
    let source_policy = global_mode
        .then(|| read_automation_source_policy(root))
        .transpose()?;
    let legacy_registry = (!global_mode)
        .then(|| read_automation_registry(root))
        .transpose()?;
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
        let matches = if let (Some(membership), Some(source_policy)) = (&membership, &source_policy)
        {
            manifest.project_registry_id.as_deref() == Some(membership.registry_id.as_str())
                && manifest.project_registry_revision == Some(membership.registry_revision)
                && manifest.project_registry_digest.as_deref()
                    == Some(membership.registry_digest.as_str())
                && manifest.source_policy_revision == Some(source_policy.revision)
                && manifest.source_policy_digest_sha256.as_deref()
                    == Some(source_policy.digest_sha256.as_str())
        } else if let Some(registry) = &legacy_registry {
            manifest.project_registry_id.is_none()
                && manifest.project_registry_revision.is_none()
                && manifest.automation_registry_revision == registry.revision
                && manifest.automation_registry_digest_sha256 == registry.digest_sha256
        } else {
            false
        };
        if !matches {
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
        WorkVcsError::ControlPlaneInvalid(if global_mode {
            "no immutable catalog matches the current ProjectRef registry and source-policy basis"
                .to_owned()
        } else {
            "no immutable catalog matches the current automation registry basis".to_owned()
        })
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
    project_registry: Option<&Path>,
    catalog_path: &Path,
    query: &str,
    limit: Option<usize>,
) -> Result<CatalogQueryReport> {
    let catalog = read_automation_catalog(catalog_path)?;
    let bindings = if catalog.project_registry_revision.is_some() {
        if !automation_source_policy_path(root).exists() {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "registry-derived catalog is stale because global source-policy mode is no longer active"
                    .to_owned(),
            ));
        }
        let membership = registry_derived_membership(project_registry)?;
        let source_policy = read_automation_source_policy(root)?;
        if catalog.project_registry_id.as_deref() != Some(membership.registry_id.as_str())
            || catalog.project_registry_revision != Some(membership.registry_revision)
            || catalog.project_registry_digest.as_deref()
                != Some(membership.registry_digest.as_str())
            || catalog.source_policy_revision != Some(source_policy.revision)
            || catalog.source_policy_digest_sha256.as_deref()
                != Some(source_policy.digest_sha256.as_str())
        {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "registry-derived catalog membership or source-policy basis is stale".to_owned(),
            ));
        }
        membership
            .members
            .into_iter()
            .map(|member| {
                (
                    member.catalog_member_id(),
                    CatalogSourceBinding {
                        identity: member.identity(),
                        store_path: member.store_path,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>()
    } else {
        if automation_source_policy_path(root).exists() {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "legacy catalog is stale because global source-policy mode is active".to_owned(),
            ));
        }
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
        registry
            .registrations
            .iter()
            .filter(|item| item.enabled && !item.revoked)
            .map(|item| {
                (
                    item.registration_id.clone(),
                    CatalogSourceBinding {
                        identity: SearchSourceIdentity::new(
                            item.project_ref_id,
                            item.store_id,
                            item.workspace_id,
                            item.branch_id,
                        ),
                        store_path: item.store_path.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
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
        let Some(binding) = bindings.get(source.registration_id.as_str()) else {
            failed += 1;
            reports.push(CatalogQuerySourceReport {
                registration_id: source.registration_id.clone(),
                project_ref_id: source.project_ref_id,
                lane: source.lane,
                artifact_kind: source.artifact_kind,
                artifact_path: source.artifact_path.clone(),
                source_anchor: source.source_anchor.clone(),
                status: "not_in_current_membership".to_owned(),
                error_code: Some("membership_not_active".to_owned()),
                error: Some("catalog source membership is no longer active".to_owned()),
                matched_items: 0,
            });
            continue;
        };
        let result = query_catalog_source(binding, source, query);
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
    binding: &CatalogSourceBinding,
    source: &AutomationCatalogSource,
    query: &str,
) -> Result<Vec<CatalogQueryItem>> {
    match source.artifact_kind {
        AutomationArtifactKind::LexicalGeneration => {
            let engine = Engine::open_readonly(&binding.store_path)?;
            let result = engine.search(SearchQueryOptions::new(
                binding.identity,
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
                binding.identity,
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
            worker_timeout_seconds,
        } => {
            if periodic_seconds == 0 {
                return Err(WorkVcsError::QueryInvalid(
                    "daemon periodic interval must be greater than zero".to_owned(),
                ));
            }
            let root = resolve_automation_root(root, registry.as_deref())?;
            run_daemon(
                &root,
                registry.as_deref(),
                periodic_seconds,
                (worker_timeout_seconds > 0).then_some(worker_timeout_seconds),
            )?;
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
fn run_daemon(
    root: &Path,
    registry: Option<&Path>,
    periodic_seconds: u64,
    worker_timeout_seconds: Option<u64>,
) -> Result<()> {
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
    let _signal_handlers = install_daemon_signal_handlers()?;
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

    let mut watcher = start_watcher(root, registry)?;
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
        launch_agent_installed: None,
        worker_active: false,
        active_worker_pid: None,
        active_worker_trigger: None,
        active_worker_started_at_us: None,
        worker_timeout_seconds,
        pending_reconcile: false,
        last_worker_duration_ms: None,
        last_worker_cancelled: false,
    };
    let mut active_worker = Some(start_reconcile_worker(
        root,
        registry,
        "startup",
        worker_timeout_seconds,
    )?);
    project_active_worker(&active_worker, &mut ledger);
    project_watcher_health(&watcher.health, &mut ledger);
    write_daemon_ledger(&runtime, &ledger)?;
    let mut periodic_deadline = Instant::now() + Duration::from_secs(periodic_seconds);
    let mut dirty_since: Option<Instant> = None;
    let mut pending_reconcile = PendingReconcile::default();
    let mut running = true;

    while running {
        if let Some(signal) = take_daemon_termination_signal() {
            let trigger = match signal {
                libc::SIGTERM => "signal_sigterm",
                libc::SIGINT => "signal_sigint",
                _ => "signal_termination",
            };
            complete_daemon_shutdown(
                &mut active_worker,
                &mut pending_reconcile,
                &mut ledger,
                &runtime,
                root,
                registry,
                trigger,
            )?;
            running = false;
            continue;
        }
        if let Some(completed) = poll_active_worker(&mut active_worker)? {
            record_worker_with_registry(
                &mut ledger,
                &completed.trigger,
                completed.outcome,
                root,
                registry,
            )?;
            refresh_watcher(&mut watcher, root, registry);
            if let Some(trigger) = pending_reconcile.take_trigger() {
                active_worker = Some(start_reconcile_worker(
                    root,
                    registry,
                    trigger,
                    worker_timeout_seconds,
                )?);
            }
            project_active_worker(&active_worker, &mut ledger);
            ledger.pending_reconcile = pending_reconcile.any();
            project_watcher_health(&watcher.health, &mut ledger);
            write_daemon_ledger(&runtime, &ledger)?;
        }
        while watcher.receiver.try_recv().is_ok() {
            dirty_since.get_or_insert_with(Instant::now);
        }
        project_watcher_health(&watcher.health, &mut ledger);
        if dirty_since.is_some_and(|instant| instant.elapsed() >= Duration::from_millis(250)) {
            if active_worker.is_some() {
                pending_reconcile.watcher = true;
            } else {
                active_worker = Some(start_reconcile_worker(
                    root,
                    registry,
                    "watcher_dirty_or_overflow",
                    worker_timeout_seconds,
                )?);
            }
            dirty_since = None;
            project_active_worker(&active_worker, &mut ledger);
            ledger.pending_reconcile = pending_reconcile.any();
            project_watcher_health(&watcher.health, &mut ledger);
            write_daemon_ledger(&runtime, &ledger)?;
        }
        if Instant::now() >= periodic_deadline {
            if active_worker.is_some() {
                pending_reconcile.periodic = true;
            } else {
                active_worker = Some(start_reconcile_worker(
                    root,
                    registry,
                    "periodic",
                    worker_timeout_seconds,
                )?);
            }
            periodic_deadline = Instant::now() + Duration::from_secs(periodic_seconds);
            project_active_worker(&active_worker, &mut ledger);
            ledger.pending_reconcile = pending_reconcile.any();
            project_watcher_health(&watcher.health, &mut ledger);
            write_daemon_ledger(&runtime, &ledger)?;
        }
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    if let Err(error) = verify_peer_uid(&stream) {
                        let message = error.to_string();
                        let response = DaemonResponse {
                            version: AUTOMATION_PROTOCOL_VERSION,
                            status: "rejected".to_owned(),
                            action: "unknown".to_owned(),
                            message: message.clone(),
                            worker_exit_code: ledger.last_worker_exit_code,
                        };
                        let response_error = write_daemon_response(
                            &mut stream,
                            &response,
                            Instant::now() + DAEMON_PROTOCOL_IO_TIMEOUT,
                        )
                        .err();
                        record_protocol_rejection(
                            &mut ledger,
                            &runtime,
                            rejection_message(message, response_error),
                        )?;
                        continue;
                    }
                    let request = match read_daemon_request(
                        &mut stream,
                        Instant::now() + DAEMON_PROTOCOL_IO_TIMEOUT,
                    ) {
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
                            let response_error = write_daemon_response(
                                &mut stream,
                                &response,
                                Instant::now() + DAEMON_PROTOCOL_IO_TIMEOUT,
                            )
                            .err();
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
                            project_watcher_health(&watcher.health, &mut ledger);
                            project_daemon_status(&ledger, &mut response)?;
                        }
                        "reconcile" => {
                            if active_worker.is_some() {
                                pending_reconcile.manual = true;
                                response.message =
                                    "reconciliation accepted and coalesced behind the active worker"
                                        .to_owned();
                            } else {
                                active_worker = Some(start_reconcile_worker(
                                    root,
                                    registry,
                                    "manual",
                                    worker_timeout_seconds,
                                )?);
                                response.message = "reconciliation accepted and started".to_owned();
                            }
                            response.status = "accepted".to_owned();
                            response.worker_exit_code = None;
                            project_active_worker(&active_worker, &mut ledger);
                            ledger.pending_reconcile = pending_reconcile.any();
                            project_watcher_health(&watcher.health, &mut ledger);
                            write_daemon_ledger(&runtime, &ledger)?;
                        }
                        "shutdown" => {
                            complete_daemon_shutdown(
                                &mut active_worker,
                                &mut pending_reconcile,
                                &mut ledger,
                                &runtime,
                                root,
                                registry,
                                "shutdown",
                            )?;
                            response.worker_exit_code = ledger.last_worker_exit_code;
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
                    if let Err(error) = write_daemon_response(
                        &mut stream,
                        &response,
                        Instant::now() + DAEMON_PROTOCOL_IO_TIMEOUT,
                    ) {
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
    drop(watcher.watcher.take());
    drop(listener);
    fs::remove_file(&socket_path).map_err(storage_error("remove daemon socket"))?;
    Ok(())
}

#[cfg(unix)]
fn complete_daemon_shutdown(
    active_worker: &mut Option<ActiveWorker>,
    pending_reconcile: &mut PendingReconcile,
    ledger: &mut DaemonLedger,
    runtime: &Path,
    root: &Path,
    registry: Option<&Path>,
    trigger: &str,
) -> Result<()> {
    if let Some(completed) = cancel_active_worker(active_worker)? {
        record_worker_with_registry(
            ledger,
            &completed.trigger,
            completed.outcome,
            root,
            registry,
        )?;
    }
    *pending_reconcile = PendingReconcile::default();
    project_active_worker(active_worker, ledger);
    ledger.pending_reconcile = false;
    ledger.last_trigger = trigger.to_owned();
    ledger.updated_at_us = epoch_micros();
    write_daemon_ledger(runtime, ledger)
}

#[cfg(not(unix))]
fn run_daemon(
    _root: &Path,
    _registry: Option<&Path>,
    _periodic_seconds: u64,
    _worker_timeout_seconds: Option<u64>,
) -> Result<()> {
    Err(WorkVcsError::QueryUnsupported(
        "automation daemon currently requires a Unix local-socket platform".to_owned(),
    ))
}

#[cfg(unix)]
fn start_watcher(root: &Path, project_registry: Option<&Path>) -> Result<DaemonWatcher> {
    let plan = watcher_target_plan(root, project_registry)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let health = Arc::new(WatcherHealth::default());
    let callback_health = Arc::clone(&health);
    let callback_sender = sender.clone();
    let filter = Arc::new(Mutex::new(WatchFilterSet::default()));
    let callback_filter = Arc::clone(&filter);
    let watcher = notify::recommended_watcher(
        move |event: std::result::Result<notify::Event, notify::Error>| match event {
            Ok(event)
                if watcher_event_requires_reconcile(&event)
                    && callback_filter
                        .lock()
                        .map(|filter| filter.matches(&event))
                        .unwrap_or(true) =>
            {
                signal_watcher_event(&callback_sender, &callback_health, Ok(()));
            }
            Ok(_) => {}
            Err(error) => {
                signal_watcher_event(&callback_sender, &callback_health, Err(error.to_string()))
            }
        },
    )
    .map_err(|error| WorkVcsError::StorageFailure(format!("cannot create watcher: {error}")))?;
    let mut state = DaemonWatcher {
        receiver,
        watcher: Some(watcher),
        health,
        filter,
        backend_targets: BTreeMap::new(),
    };
    replace_watcher_targets(&mut state, plan);
    Ok(state)
}

#[cfg(unix)]
fn refresh_watcher(state: &mut DaemonWatcher, root: &Path, project_registry: Option<&Path>) {
    match watcher_target_plan(root, project_registry) {
        Ok(plan) => replace_watcher_targets(state, plan),
        Err(error) => record_watcher_error(
            &state.health,
            format!("cannot refresh watcher targets: {error}"),
        ),
    }
}

#[cfg(unix)]
fn watcher_target_plan(root: &Path, project_registry: Option<&Path>) -> Result<WatchTargetPlan> {
    if automation_source_policy_path(root).exists() {
        Ok(registry_derived_watcher_target_plan(root, project_registry))
    } else {
        legacy_watcher_target_plan(root)
    }
}

#[cfg(unix)]
fn legacy_watcher_target_plan(root: &Path) -> Result<WatchTargetPlan> {
    let registry = read_automation_registry(root)?;
    let mut plan = WatchTargetPlan::default();
    add_exact_watch(
        &mut plan,
        automation_source_policy_path(root),
        "automation source policy",
    );
    for registration in registry
        .registrations
        .iter()
        .filter(|item| item.enabled && !item.revoked)
    {
        add_store_watch(&mut plan, registration.store_path.clone());
        for rollout_root in &registration.rollout_roots {
            add_tree_watch(&mut plan, rollout_root.clone(), "rollout root");
        }
    }
    Ok(plan)
}

#[cfg(unix)]
fn registry_derived_watcher_target_plan(
    root: &Path,
    project_registry: Option<&Path>,
) -> WatchTargetPlan {
    let mut plan = WatchTargetPlan::default();
    add_exact_watch(
        &mut plan,
        automation_source_policy_path(root),
        "automation source policy",
    );
    let mut errors = Vec::new();

    match effective_registry_config(project_registry.map(Path::to_path_buf)).and_then(|effective| {
        let registry_path = canonical_registry_path(
            absolute_cli_path("project registry", effective.registry_path.clone())?,
            false,
        )?;
        let activation_path = routing_activation_path(&effective, &registry_path);
        Ok((registry_path, activation_path))
    }) {
        Ok((registry_path, activation_path)) => {
            add_exact_watch(&mut plan, registry_path, "ProjectRef registry");
            add_exact_watch(&mut plan, activation_path, "routing activation");
        }
        Err(error) => errors.push(format!("cannot resolve global registry controls: {error}")),
    }

    match registry_derived_membership(project_registry) {
        Ok(membership) => {
            for member in membership.members {
                add_store_watch(&mut plan, member.store_path);
            }
        }
        Err(error) => errors.push(format!("cannot resolve global Store membership: {error}")),
    }

    match read_automation_source_policy(root) {
        Ok(policy) => {
            for source in policy
                .sources
                .iter()
                .filter(|source| source.enabled && !source.revoked)
            {
                add_tree_watch(&mut plan, source.rollout_root.clone(), "rollout root");
            }
        }
        Err(error) => errors.push(format!("cannot read automation source policy: {error}")),
    }

    if !errors.is_empty() {
        plan.discovery_error = Some(errors.join("; "));
    }
    plan
}

#[cfg(unix)]
fn add_exact_watch(plan: &mut WatchTargetPlan, path: PathBuf, label: &'static str) {
    let path = normalized_watch_path(path);
    if let Some(parent) = path.parent() {
        add_backend_watch(plan, parent.to_path_buf(), false, label);
    }
    plan.filter.rules.push(WatchFilterRule::Exact(path));
}

#[cfg(unix)]
fn add_store_watch(plan: &mut WatchTargetPlan, path: PathBuf) {
    let path = normalized_watch_path(path);
    if let Some(parent) = path.parent() {
        add_backend_watch(plan, parent.to_path_buf(), false, "Store family");
    }
    plan.filter.rules.push(WatchFilterRule::StoreFamily(path));
}

#[cfg(unix)]
fn add_tree_watch(plan: &mut WatchTargetPlan, path: PathBuf, label: &'static str) {
    let path = normalized_watch_path(path);
    add_backend_watch(plan, path.clone(), true, label);
    plan.filter.rules.push(WatchFilterRule::Tree(path));
}

#[cfg(unix)]
fn normalized_watch_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .and_then(|parent| path.file_name().map(|name| parent.join(name)))
            .unwrap_or(path)
    })
}

#[cfg(unix)]
fn add_backend_watch(
    plan: &mut WatchTargetPlan,
    path: PathBuf,
    recursive: bool,
    label: &'static str,
) {
    plan.backend_targets
        .entry(path)
        .and_modify(|(existing_recursive, _)| *existing_recursive |= recursive)
        .or_insert((recursive, label));
}

#[cfg(unix)]
fn replace_watcher_targets(state: &mut DaemonWatcher, plan: WatchTargetPlan) {
    use notify::{RecursiveMode, Watcher};

    let Some(watcher) = state.watcher.as_mut() else {
        return;
    };
    for path in state.backend_targets.keys() {
        let _ = watcher.unwatch(path);
    }
    state.backend_targets.clear();
    for (path, (recursive, label)) in &plan.backend_targets {
        let mode = if *recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        match watcher.watch(path, mode) {
            Ok(()) => {
                state
                    .backend_targets
                    .insert(path.clone(), (*recursive, *label));
            }
            Err(error) => record_watcher_error(
                &state.health,
                format!("cannot watch {label} {}: {error}", path.display()),
            ),
        }
    }
    match state.filter.lock() {
        Ok(mut current) => *current = plan.filter,
        Err(_) => {
            record_watcher_error(&state.health, "watcher filter state is poisoned".to_owned())
        }
    }
    if let Some(error) = plan.discovery_error {
        record_watcher_error(&state.health, error);
    }
}

#[cfg(unix)]
fn watch_rule_matches(rule: &WatchFilterRule, path: &Path) -> bool {
    match rule {
        WatchFilterRule::Exact(expected) => path == expected,
        WatchFilterRule::Tree(root) => path.starts_with(root),
        WatchFilterRule::StoreFamily(store) => {
            path == store
                || ["-wal", "-shm", "-journal"].iter().any(|suffix| {
                    let mut candidate = store.as_os_str().to_os_string();
                    candidate.push(suffix);
                    path.as_os_str() == candidate
                })
        }
    }
}

#[cfg(unix)]
fn watcher_event_requires_reconcile(event: &notify::Event) -> bool {
    // Access modes describe how a handle was opened, not whether source state
    // changed. Linux may therefore emit close-write while a read-only
    // reconciliation closes a database handle opened with write capability.
    // Actual source mutations still arrive as create, modify, or remove, while
    // Any/Other and explicit rescan flags remain fail-safe dirty signals.
    event.need_rescan() || !matches!(event.kind, notify::EventKind::Access(_))
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
            record_watcher_error(health, error);
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
fn record_watcher_error(health: &WatcherHealth, error: String) {
    health.errors.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut last_error) = health.last_error.lock() {
        *last_error = Some(bound_text(&error, 2_048));
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

#[cfg(unix)]
fn start_reconcile_worker(
    root: &Path,
    registry: Option<&Path>,
    trigger: &str,
    timeout_seconds: Option<u64>,
) -> Result<ActiveWorker> {
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
    command.process_group(0);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(storage_error("spawn reconciliation worker"))?;
    let process_group_id = libc::pid_t::try_from(child.id()).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "reconciliation worker PID cannot identify its process group: {error}"
        ))
    })?;
    if process_group_id <= 0 {
        let _ = child.kill();
        let _ = child.wait();
        return Err(WorkVcsError::StorageFailure(
            "reconciliation worker process group must be positive".to_owned(),
        ));
    }
    let stdout = child.stdout.take().ok_or_else(|| {
        WorkVcsError::StorageFailure("reconciliation worker stdout is not captured".to_owned())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        WorkVcsError::StorageFailure("reconciliation worker stderr is not captured".to_owned())
    })?;
    Ok(ActiveWorker {
        child: Some(child),
        process_group_id,
        stdout_reader: Some(std::thread::spawn(move || capture_worker_stream(stdout))),
        stderr_reader: Some(std::thread::spawn(move || capture_worker_stream(stderr))),
        trigger: trigger.to_owned(),
        started: Instant::now(),
        started_at_us: epoch_micros(),
        timeout_seconds,
    })
}

#[cfg(unix)]
fn poll_active_worker(active: &mut Option<ActiveWorker>) -> Result<Option<CompletedWorker>> {
    poll_active_worker_before_terminate(active, || {})
}

#[cfg(unix)]
fn poll_active_worker_before_terminate(
    active: &mut Option<ActiveWorker>,
    before_terminate: impl FnOnce(),
) -> Result<Option<CompletedWorker>> {
    let Some(worker) = active.as_mut() else {
        return Ok(None);
    };
    if worker_has_exited_without_reaping(
        worker
            .child
            .as_ref()
            .expect("active worker child is present"),
    )? {
        let child = worker
            .child
            .as_mut()
            .expect("active worker child is present");
        terminate_remaining_worker_process_group(child, worker.process_group_id)?;
        let status = child
            .wait()
            .map_err(storage_error("reap completed reconciliation worker"))?;
        let worker = active.take().expect("active worker is present");
        return finish_active_worker(worker, status, false, false).map(Some);
    }
    if worker
        .timeout_seconds
        .is_some_and(|seconds| worker.started.elapsed() >= Duration::from_secs(seconds))
    {
        before_terminate();
        let (status, terminated) = terminate_and_reap(
            worker
                .child
                .as_mut()
                .expect("active worker child is present"),
            worker.process_group_id,
            "timed-out reconciliation worker",
        )?;
        let worker = active.take().expect("active worker is present");
        return finish_active_worker(worker, status, terminated, false).map(Some);
    }
    Ok(None)
}

#[cfg(unix)]
fn cancel_active_worker(active: &mut Option<ActiveWorker>) -> Result<Option<CompletedWorker>> {
    cancel_active_worker_before_terminate(active, || {})
}

#[cfg(unix)]
fn cancel_active_worker_before_terminate(
    active: &mut Option<ActiveWorker>,
    before_terminate: impl FnOnce(),
) -> Result<Option<CompletedWorker>> {
    let Some(worker) = active.as_mut() else {
        return Ok(None);
    };
    let child = worker
        .child
        .as_mut()
        .expect("active worker child is present");
    let (status, cancelled) = if worker_has_exited_without_reaping(child)? {
        terminate_remaining_worker_process_group(child, worker.process_group_id)?;
        (
            child
                .wait()
                .map_err(storage_error("reap completed reconciliation worker"))?,
            false,
        )
    } else {
        before_terminate();
        terminate_and_reap(
            child,
            worker.process_group_id,
            "reconciliation worker during shutdown",
        )?
    };
    let worker = active.take().expect("active worker is present");
    finish_active_worker(worker, status, false, cancelled).map(Some)
}

#[cfg(unix)]
fn terminate_and_reap(
    child: &mut Child,
    process_group_id: libc::pid_t,
    operation: &'static str,
) -> Result<(std::process::ExitStatus, bool)> {
    if let Err(error) = terminate_worker_process_group(child, process_group_id) {
        if let Some(status) = child.try_wait().map_err(|wait_error| {
            WorkVcsError::StorageFailure(format!(
                "cannot inspect {operation} after process-group termination failed with {error}: {wait_error}"
            ))
        })? {
            return Ok((status, false));
        }
        return Err(WorkVcsError::StorageFailure(format!(
            "cannot terminate {operation}: {error}"
        )));
    }
    let status = child.wait().map_err(|error| {
        WorkVcsError::StorageFailure(format!("cannot reap {operation}: {error}"))
    })?;
    Ok((status, status.signal() == Some(libc::SIGKILL)))
}

#[cfg(unix)]
fn worker_has_exited_without_reaping(child: &Child) -> Result<bool> {
    let child_id = libc::id_t::try_from(child.id()).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "reconciliation worker PID cannot be observed without reaping: {error}"
        ))
    })?;
    loop {
        let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                child_id,
                information.as_mut_ptr(),
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result == 0 {
            let information = unsafe { information.assume_init() };
            return Ok(unsafe { information.si_pid() } != 0);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        return Err(WorkVcsError::StorageFailure(format!(
            "cannot inspect reconciliation worker without reaping: {error}"
        )));
    }
}

#[cfg(unix)]
fn terminate_worker_process_group(
    child: &Child,
    process_group_id: libc::pid_t,
) -> std::io::Result<()> {
    let child_pid = libc::pid_t::try_from(child.id()).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("worker PID cannot identify its process group: {error}"),
        )
    })?;
    if process_group_id <= 0 || process_group_id != child_pid {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("worker process group {process_group_id} does not match child PID {child_pid}"),
        ));
    }
    let result = unsafe { libc::kill(-process_group_id, libc::SIGKILL) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn terminate_remaining_worker_process_group(
    child: &Child,
    process_group_id: libc::pid_t,
) -> Result<()> {
    match terminate_worker_process_group(child, process_group_id) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(()),
        // Darwin reports EPERM when the observed, unreaped group leader is the
        // only remaining process (a zombie cannot receive SIGKILL). This path
        // is used only after waitid(WNOWAIT) proved that our own leader exited;
        // active timeout/cancellation still treats EPERM as a hard failure.
        #[cfg(target_os = "macos")]
        Err(error) if error.raw_os_error() == Some(libc::EPERM) => Ok(()),
        Err(error) => Err(WorkVcsError::StorageFailure(format!(
            "cannot terminate remaining reconciliation worker process group: {error}"
        ))),
    }
}

#[cfg(unix)]
fn finish_active_worker(
    mut worker: ActiveWorker,
    status: std::process::ExitStatus,
    timed_out: bool,
    cancelled: bool,
) -> Result<CompletedWorker> {
    let duration_ms = worker.started.elapsed().as_millis();
    worker.child.take();
    let stdout = join_worker_capture(
        worker
            .stdout_reader
            .take()
            .expect("active worker stdout reader is present"),
        "stdout",
    )?;
    let stderr = join_worker_capture(
        worker
            .stderr_reader
            .take()
            .expect("active worker stderr reader is present"),
        "stderr",
    )?;
    let trigger = std::mem::take(&mut worker.trigger);
    Ok(CompletedWorker {
        trigger,
        outcome: WorkerOutcome {
            exit_code: status.code(),
            timed_out,
            cancelled,
            duration_ms,
            timeout_seconds: worker.timeout_seconds,
            stdout,
            stderr,
        },
    })
}

#[cfg(unix)]
fn project_active_worker(active: &Option<ActiveWorker>, ledger: &mut DaemonLedger) {
    ledger.updated_at_us = epoch_micros();
    ledger.worker_active = active.is_some();
    ledger.active_worker_pid = active
        .as_ref()
        .and_then(|worker| worker.child.as_ref().map(Child::id));
    ledger.active_worker_trigger = active.as_ref().map(|worker| worker.trigger.clone());
    ledger.active_worker_started_at_us = active.as_ref().map(|worker| worker.started_at_us);
    if let Some(worker) = active {
        ledger.last_trigger = worker.trigger.clone();
    }
}

#[cfg(test)]
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
            let process_group_id = libc::pid_t::try_from(child.id()).map_err(|error| {
                WorkVcsError::StorageFailure(format!(
                    "test worker PID cannot identify its process group: {error}"
                ))
            })?;
            let (status, terminated) =
                terminate_and_reap(child, process_group_id, "timed-out reconciliation worker")?;
            timed_out = terminated;
            break status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = join_worker_capture(stdout_reader, "stdout")?;
    let stderr = join_worker_capture(stderr_reader, "stderr")?;
    Ok(WorkerOutcome {
        exit_code: status.code(),
        timed_out,
        cancelled: false,
        duration_ms: started.elapsed().as_millis(),
        timeout_seconds: Some(timeout.as_secs()),
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
        cancelled: outcome.cancelled,
        duration_ms: outcome.duration_ms,
        timeout_seconds: outcome.timeout_seconds,
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

#[cfg(all(unix, test))]
fn record_worker(
    ledger: &mut DaemonLedger,
    trigger: &str,
    outcome: WorkerOutcome,
    root: &Path,
) -> Result<()> {
    record_worker_with_registry(ledger, trigger, outcome, root, None)
}

#[cfg(unix)]
fn record_worker_with_registry(
    ledger: &mut DaemonLedger,
    trigger: &str,
    outcome: WorkerOutcome,
    root: &Path,
    project_registry: Option<&Path>,
) -> Result<()> {
    let evidence_path = persist_worker_evidence(root, trigger, &outcome)?;
    ledger.updated_at_us = epoch_micros();
    ledger.last_trigger = trigger.to_owned();
    ledger.last_worker_status = Some("failed".to_owned());
    ledger.last_worker_exit_code = outcome.exit_code;
    ledger.last_worker_timed_out = outcome.timed_out;
    ledger.last_worker_cancelled = outcome.cancelled;
    ledger.last_worker_duration_ms = Some(outcome.duration_ms);
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
    ledger.worker_active = false;
    ledger.active_worker_pid = None;
    ledger.active_worker_trigger = None;
    ledger.active_worker_started_at_us = None;

    if outcome.cancelled {
        ledger.last_error = Some(format!(
            "reconciliation worker was cancelled during daemon shutdown; exact captured evidence: {}",
            evidence_path.display()
        ));
        return Ok(());
    }
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
            "reconciliation worker exceeded the explicitly configured {} second deadline and was terminated; exact captured evidence: {}",
            outcome.timeout_seconds.unwrap_or_default(),
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
    let latest = match latest_catalog_for_current_basis(root, project_registry) {
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
        || latest.manifest.project_registry_id != report.project_registry_id
        || latest.manifest.project_registry_revision != report.project_registry_revision
        || latest.manifest.project_registry_digest != report.project_registry_digest
        || latest.manifest.source_policy_revision != report.source_policy_revision
        || latest.manifest.source_policy_digest_sha256 != report.source_policy_digest_sha256
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
    if ledger.worker_active {
        response.status = "in_progress".to_owned();
        response.message = "reconciliation worker is active".to_owned();
        response.worker_exit_code = None;
    } else if ledger.last_worker_status.as_deref() == Some("failed")
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
    let protocol_deadline = Instant::now() + DAEMON_PROTOCOL_IO_TIMEOUT;
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
    write_all_before_deadline(&mut stream, &bytes, protocol_deadline, "daemon request")?;
    daemon_protocol_time_remaining(protocol_deadline, "daemon request")?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(storage_error("finish daemon request"))?;
    let response = read_bounded_json_before_deadline::<DaemonResponse>(
        &mut stream,
        "daemon response",
        protocol_deadline,
    )?;
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
fn read_daemon_request(stream: &mut UnixStream, deadline: Instant) -> Result<DaemonRequest> {
    let request =
        read_bounded_json_before_deadline::<DaemonRequest>(stream, "daemon request", deadline)?;
    if request.version != AUTOMATION_PROTOCOL_VERSION {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "daemon protocol version {} does not match {}",
            request.version, AUTOMATION_PROTOCOL_VERSION
        )));
    }
    Ok(request)
}

#[cfg(test)]
fn read_bounded_json<T: for<'de> Deserialize<'de>>(
    reader: &mut impl Read,
    label: &str,
) -> Result<T> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_DAEMON_FRAME_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(storage_error("read daemon frame"))?;
    decode_bounded_json(&bytes, label)
}

fn decode_bounded_json<T: for<'de> Deserialize<'de>>(bytes: &[u8], label: &str) -> Result<T> {
    if bytes.len() > MAX_DAEMON_FRAME_BYTES {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "{label} exceeds the bounded frame size"
        )));
    }
    serde_json::from_slice(bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("{label} is malformed JSON: {error}"))
    })
}

#[cfg(unix)]
fn daemon_protocol_time_remaining(deadline: Instant, label: &str) -> Result<Duration> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| {
            WorkVcsError::StorageFailure(format!(
                "{label} exceeded the {:?} daemon protocol deadline",
                DAEMON_PROTOCOL_IO_TIMEOUT
            ))
        })?;
    Ok(remaining)
}

#[cfg(unix)]
fn read_bounded_json_before_deadline<T: for<'de> Deserialize<'de>>(
    stream: &mut UnixStream,
    label: &str,
    deadline: Instant,
) -> Result<T> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let remaining = daemon_protocol_time_remaining(deadline, label)?;
        stream.set_read_timeout(Some(remaining)).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot bound {label} frame read to remaining deadline {remaining:?}: {error}"
            ))
        })?;
        let available = (MAX_DAEMON_FRAME_BYTES + 1).saturating_sub(bytes.len());
        if available == 0 {
            return decode_bounded_json(&bytes, label);
        }
        let chunk_len = available.min(buffer.len());
        let read = match stream.read(&mut buffer[..chunk_len]) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(WorkVcsError::StorageFailure(format!(
                    "{label} exceeded the {:?} daemon protocol deadline",
                    DAEMON_PROTOCOL_IO_TIMEOUT
                )));
            }
            Err(error) => return Err(storage_error("read daemon frame")(error)),
        };
        if read == 0 {
            return decode_bounded_json(&bytes, label);
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_DAEMON_FRAME_BYTES || bytes.contains(&b'\n') {
            return decode_bounded_json(&bytes, label);
        }
    }
}

#[cfg(unix)]
fn write_all_before_deadline(
    stream: &mut UnixStream,
    mut bytes: &[u8],
    deadline: Instant,
    label: &str,
) -> Result<()> {
    while !bytes.is_empty() {
        let remaining = daemon_protocol_time_remaining(deadline, label)?;
        stream.set_write_timeout(Some(remaining)).map_err(|error| {
            WorkVcsError::StorageFailure(format!(
                "cannot bound {label} frame write to remaining deadline {remaining:?}: {error}"
            ))
        })?;
        match stream.write(bytes) {
            Ok(0) => {
                return Err(WorkVcsError::StorageFailure(format!(
                    "cannot write {label}: socket made no progress"
                )));
            }
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(WorkVcsError::StorageFailure(format!(
                    "{label} exceeded the {:?} daemon protocol deadline",
                    DAEMON_PROTOCOL_IO_TIMEOUT
                )));
            }
            Err(error) => return Err(storage_error("write daemon frame")(error)),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn write_daemon_response(
    stream: &mut UnixStream,
    response: &DaemonResponse,
    deadline: Instant,
) -> Result<()> {
    let mut bytes = serde_json::to_vec(response).map_err(serialization_error)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_DAEMON_FRAME_BYTES {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "daemon response exceeds the bounded frame size".to_owned(),
        ));
    }
    write_all_before_deadline(stream, &bytes, deadline, "daemon response")
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
    use workvcs_core::automation::AUTOMATION_CATALOG_FORMAT_VERSION;

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

    struct RegistryDerivedAutomationFixture {
        automation_root: PathBuf,
        registry_path: PathBuf,
        rollout_root: PathBuf,
        project_ref_id: ProjectRefId,
        project_ref_ids: Vec<ProjectRefId>,
        project_roots: Vec<PathBuf>,
    }

    fn write_registry_derived_test_rollout(path: &Path, cwd: &Path, text: &str) {
        let thread = "01900000-0000-7000-8000-000000000011";
        let user = "01900000-0000-7000-8000-000000000012";
        let assistant = "01900000-0000-7000-8000-000000000013";
        let lines = [
            serde_json::json!({
                "type":"session_meta",
                "timestamp":"2026-01-01T00:00:00Z",
                "payload":{"id":thread,"cwd":cwd}
            }),
            serde_json::json!({
                "type":"response_item",
                "timestamp":"2026-01-01T00:00:01Z",
                "payload":{"type":"message","role":"user","id":user,"content":[{"type":"input_text","text":text}]}
            }),
            serde_json::json!({
                "type":"response_item",
                "timestamp":"2026-01-01T00:00:02Z",
                "payload":{"type":"message","role":"assistant","id":assistant,"content":[{"type":"output_text","text":"assistant text must be excluded"}]}
            }),
        ];
        let bytes = lines
            .into_iter()
            .map(|line| serde_json::to_string(&line).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, format!("{bytes}\n")).unwrap();
    }

    fn append_registry_derived_test_message(path: &Path, message_id: &str, text: &str) {
        let line = serde_json::json!({
            "type":"response_item",
            "timestamp":"2026-01-01T00:00:03Z",
            "payload":{
                "type":"message",
                "role":"user",
                "id":message_id,
                "content":[{"type":"input_text","text":text}]
            }
        });
        let mut file = OpenOptions::new().append(true).open(path).unwrap();
        writeln!(file, "{}", serde_json::to_string(&line).unwrap()).unwrap();
        file.sync_all().unwrap();
    }

    fn registry_derived_automation_fixture(
        temporary: &TempDir,
    ) -> RegistryDerivedAutomationFixture {
        registry_derived_automation_fixture_with_project_count(temporary, 1)
    }

    fn registry_derived_automation_fixture_with_project_count(
        temporary: &TempDir,
        project_count: usize,
    ) -> RegistryDerivedAutomationFixture {
        assert!(project_count > 0);
        let fixture_root = temporary.path().canonicalize().unwrap();
        let automation_root = fixture_root.join("automation");
        let rollout_root = fixture_root.join("rollouts");
        let registry_path = fixture_root.join("project-bindings.json");
        fs::create_dir(&automation_root).unwrap();
        fs::create_dir(&rollout_root).unwrap();
        #[cfg(unix)]
        {
            fs::set_permissions(&automation_root, fs::Permissions::from_mode(0o700)).unwrap();
            fs::set_permissions(&rollout_root, fs::Permissions::from_mode(0o700)).unwrap();
        }

        let registry_id = workvcs_core::RegistryId::new_v7();
        let mut projects = Vec::new();
        let mut locators = Vec::new();
        let mut bindings = Vec::new();
        let mut project_ref_ids = Vec::new();
        let mut project_roots = Vec::new();
        for index in 0..project_count {
            let project_root = fixture_root.join(if index == 0 {
                "project".to_owned()
            } else {
                format!("project-{index}")
            });
            let store_path = fixture_root.join(if index == 0 {
                "project.sqlite".to_owned()
            } else {
                format!("project-{index}.sqlite")
            });
            fs::create_dir(&project_root).unwrap();
            let label = format!("registry-derived-automation-{index}");
            let mut engine =
                Engine::init(&store_path, StoreInitOptions::new(&label).unwrap()).unwrap();
            let store_id = engine.store_info().unwrap().store_id;
            let workspace = engine
                .create_workspace(WorkspaceInitOptions::new(&label).unwrap())
                .unwrap();
            engine
                .create_record(
                    RecordCreateOptions::finding(
                        workspace.initial_branch_id,
                        workspace.genesis_commit_id,
                        format!("registry derived canonical evidence {index}"),
                    )
                    .unwrap(),
                )
                .unwrap();

            let project_root = project_root.canonicalize().unwrap();
            let store_path = store_path.canonicalize().unwrap();
            let project_ref_id = ProjectRefId::new_v7();
            projects.push(serde_json::json!({
                "project_ref_id": project_ref_id,
                "maturity": "provisional",
                "display_name": null,
                "created_at": "2026-01-01T00:00:00Z",
                "created_by": "cwd_first_write"
            }));
            locators.push(serde_json::json!({
                "locator_id": workvcs_core::ProjectLocatorId::new_v7(),
                "project_ref_id": project_ref_id,
                "role": "identity",
                "authority": "cwd",
                "provider": "filesystem",
                "namespace": format!("registry:{registry_id}"),
                "kind": "canonical_directory",
                "normalized_value": project_root,
                "assurance": "verified_derived",
                "source_adapter": "workvcs-filesystem-v1",
                "evidence_digest": ControlPlaneDigest::raw(b"registry-derived-automation"),
                "observed_at": "2026-01-01T00:00:00Z",
                "state": "active"
            }));
            bindings.push(serde_json::json!({
                "project_ref_id": project_ref_id,
                "store_path": store_path,
                "store_id": store_id,
                "workspace_id": workspace.workspace_id,
                "branch_id": workspace.initial_branch_id,
                "bound_at": "2026-01-01T00:00:00Z",
                "binding_source": "first_write"
            }));
            project_ref_ids.push(project_ref_id);
            project_roots.push(project_root);
        }
        projects.sort_by_key(|value| value["project_ref_id"].as_str().unwrap().to_owned());
        locators.sort_by_key(|value| value["locator_id"].as_str().unwrap().to_owned());
        bindings.sort_by_key(|value| value["project_ref_id"].as_str().unwrap().to_owned());
        let registry_value = serde_json::json!({
            "version": 2,
            "registry_id": registry_id,
            "revision": 1,
            "projects": projects,
            "locators": locators,
            "bindings": bindings,
            "links": [],
            "observations": [],
            "migration": null
        });
        let registry =
            ProjectRegistryV2::from_json_bytes(&serde_json::to_vec(&registry_value).unwrap())
                .unwrap();
        fs::write(&registry_path, registry.stored_json_bytes().unwrap()).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&registry_path, fs::Permissions::from_mode(0o600)).unwrap();

        let effective = effective_registry_config(Some(registry_path.clone())).unwrap();
        let activation_path = routing_activation_path(&effective, &registry_path);
        fs::create_dir_all(activation_path.parent().unwrap()).unwrap();
        let activation = RoutingActivationCandidate::for_registry(&registry).unwrap();
        fs::write(&activation_path, activation.stored_json_bytes().unwrap()).unwrap();
        #[cfg(unix)]
        {
            fs::set_permissions(
                activation_path.parent().unwrap(),
                fs::Permissions::from_mode(0o700),
            )
            .unwrap();
            fs::set_permissions(&activation_path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        write_registry_derived_test_rollout(
            &rollout_root.join("thread.jsonl"),
            &project_roots[0],
            "registry derived historical evidence",
        );
        RegistryDerivedAutomationFixture {
            automation_root,
            registry_path,
            rollout_root,
            project_ref_id: project_ref_ids[0],
            project_ref_ids,
            project_roots,
        }
    }

    fn install_registry_derived_source_policy(
        fixture: &RegistryDerivedAutomationFixture,
        excluded_project_refs: Vec<ProjectRefId>,
    ) {
        let proposal = AutomationSourcePolicyProposal {
            source_id: "rollouts".to_owned(),
            rollout_root: fixture.rollout_root.clone(),
            role_policy: AutomationRolePolicy::UserOnly,
            excluded_project_refs,
        };
        let preview =
            preview_automation_source_policy(&fixture.automation_root, proposal.clone()).unwrap();
        apply_automation_source_policy(
            &fixture.automation_root,
            preview.policy_revision,
            &preview.policy_digest_sha256,
            proposal,
        )
        .unwrap();
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
            launch_agent_installed: None,
            worker_active: false,
            active_worker_pid: None,
            active_worker_trigger: None,
            active_worker_started_at_us: None,
            worker_timeout_seconds: None,
            pending_reconcile: false,
            last_worker_duration_ms: None,
            last_worker_cancelled: false,
        }
    }

    #[cfg(unix)]
    fn wait_for_test_pid(path: &Path) -> libc::pid_t {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(contents) = fs::read_to_string(path)
                && let Ok(pid) = contents.trim().parse::<libc::pid_t>()
            {
                return pid;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!(
            "test process PID file {} did not become readable and parseable within the bound",
            path.display()
        );
    }

    #[cfg(unix)]
    fn publish_failed_reconcile_report(root: &Path) -> ReconcileReport {
        let registry = read_automation_registry(root).unwrap();
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
            root,
            registry.revision,
            registry.digest_sha256.clone(),
            vec![source],
        )
        .unwrap();
        ReconcileReport {
            status: "failed".to_owned(),
            complete: false,
            membership_mode: "legacy_registration".to_owned(),
            automation_registry_revision: registry.revision,
            automation_registry_digest_sha256: registry.digest_sha256,
            project_registry_id: None,
            project_registry_revision: None,
            project_registry_digest: None,
            source_policy_revision: None,
            source_policy_digest_sha256: None,
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
        }
    }

    #[cfg(unix)]
    #[test]
    fn native_historical_identity_is_nested_and_linked_worktree_strict() {
        let temporary = TempDir::new().unwrap();
        let temporary_root = temporary.path().canonicalize().unwrap();

        let repository = temporary_root.join("repository");
        let nested = repository.join("src/nested");
        fs::create_dir_all(repository.join(".git")).unwrap();
        fs::create_dir_all(&nested).unwrap();
        let identity = resolve_project_identity_without_git_subprocess(&nested).unwrap();
        assert_eq!(identity.kind, "git-common-dir");
        assert_eq!(
            identity.root,
            fs::canonicalize(&repository).unwrap().display().to_string()
        );
        assert_eq!(
            identity.identity,
            fs::canonicalize(repository.join(".git"))
                .unwrap()
                .display()
                .to_string()
        );

        let common_repository = temporary_root.join("common-repository");
        let common_dir = common_repository.join(".git");
        let worktree = temporary_root.join("linked-worktree");
        let worktree_nested = worktree.join("nested");
        let worktree_admin = common_dir.join("worktrees/linked");
        fs::create_dir_all(&worktree_admin).unwrap();
        fs::create_dir_all(&worktree_nested).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", worktree_admin.display()),
        )
        .unwrap();
        fs::write(worktree_admin.join("commondir"), "../..\n").unwrap();
        fs::write(
            worktree_admin.join("gitdir"),
            format!("{}\n", worktree.join(".git").display()),
        )
        .unwrap();
        fs::write(worktree_admin.join("HEAD"), b"ref: refs/heads/linked\n").unwrap();
        let identity = resolve_project_identity_without_git_subprocess(&worktree_nested).unwrap();
        assert_eq!(identity.kind, "git-common-dir");
        assert_eq!(
            identity.identity,
            fs::canonicalize(&common_dir).unwrap().display().to_string()
        );
        assert_eq!(
            identity.root,
            fs::canonicalize(&worktree).unwrap().display().to_string()
        );
        let observed = observe_historical_owner(&worktree_nested).unwrap();
        fs::write(
            worktree_admin.join("HEAD"),
            b"ref: refs/heads/linked-changed\n",
        )
        .unwrap();
        let changed = observe_historical_owner(&worktree_nested).unwrap();
        assert_eq!(changed.identity, observed.identity);
        assert_ne!(changed.metadata_sha256, observed.metadata_sha256);
        let linked_aliases = historical_cwd_aliases_without_io(&worktree_nested).unwrap();
        assert!(
            historical_repository_boundaries(&common_dir)
                .iter()
                .any(|boundary| linked_aliases
                    .iter()
                    .any(|candidate| candidate.starts_with(boundary)))
        );
        let unrelated = temporary_root.join("unrelated-backlink-target");
        let unverified_admin = common_dir.join("worktrees/unverified");
        fs::create_dir(&unverified_admin).unwrap();
        fs::write(
            unverified_admin.join("gitdir"),
            format!("{}\n", unrelated.join(".git").display()),
        )
        .unwrap();
        assert!(
            historical_repository_boundaries(&common_dir)
                .iter()
                .any(|boundary| boundary == &unrelated)
        );

        let backlink_outside = temporary_root.join("backlink-outside");
        let backlink_worktree = backlink_outside.join("worktree");
        fs::create_dir_all(&backlink_worktree).unwrap();
        let backlink_link = temporary_root.join("backlink-link");
        std::os::unix::fs::symlink(&backlink_outside, &backlink_link).unwrap();
        let unsafe_root = backlink_link.join("worktree");
        let unsafe_admin = common_dir.join("worktrees/unsafe-link");
        fs::create_dir(&unsafe_admin).unwrap();
        fs::write(
            unsafe_admin.join("gitdir"),
            format!("{}\n", unsafe_root.join(".git").display()),
        )
        .unwrap();
        let boundaries = historical_repository_boundaries(&common_dir);
        let error = select_historical_probe_path(
            &unsafe_root,
            &boundaries,
            std::slice::from_ref(&unsafe_root),
        )
        .unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));
        assert!(error.to_string().contains("is a symlink"));

        let symlinked = temporary_root.join("symlinked-marker");
        fs::create_dir(&symlinked).unwrap();
        std::os::unix::fs::symlink(&common_dir, symlinked.join(".git")).unwrap();
        let error = resolve_project_identity_without_git_subprocess(&symlinked).unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));

        let separate = temporary_root.join("separate-git-dir");
        let separate_metadata = temporary_root.join("separate-metadata");
        fs::create_dir(&separate).unwrap();
        fs::create_dir(&separate_metadata).unwrap();
        fs::write(
            separate.join(".git"),
            format!("gitdir: {}\n", separate_metadata.display()),
        )
        .unwrap();
        let error = resolve_project_identity_without_git_subprocess(&separate).unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));
        assert!(error.to_string().contains("no common-dir pointer"));
        assert!(historical_repository_boundaries(&separate_metadata).is_empty());
    }

    #[test]
    fn historical_owner_cache_revalidates_local_identity_before_reuse_and_publication() {
        let temporary = TempDir::new().unwrap();
        let fixture = registry_derived_automation_fixture(&temporary);
        let membership = registry_derived_membership(Some(&fixture.registry_path)).unwrap();
        let source = fixture.rollout_root.join("thread.jsonl");
        let cwd = rollout_cwd_from_session_meta(&source).unwrap();
        let mut cache = HistoricalOwnerCache::new();

        let member =
            resolve_registry_derived_source_member(&source, &membership, &mut cache).unwrap();
        assert_eq!(member.project_ref_id, fixture.project_ref_id);

        #[cfg(unix)]
        {
            let moved_cwd = cwd.with_extension("moved-before-symlink-revalidation");
            let replacement = temporary.path().join("replacement-outside-active-boundary");
            fs::create_dir(&replacement).unwrap();
            fs::rename(&cwd, &moved_cwd).unwrap();
            std::os::unix::fs::symlink(&replacement, &cwd).unwrap();
            let error = validate_historical_owner_cache(&cache, &membership).unwrap_err();
            assert!(matches!(error, WorkVcsError::ControlPlaneInvalid(_)));
            assert!(
                error
                    .to_string()
                    .contains("historical identity boundary component"),
                "{error}"
            );
            assert!(error.to_string().contains("is a symlink"), "{error}");
            fs::remove_file(&cwd).unwrap();
            fs::rename(&moved_cwd, &cwd).unwrap();

            let outside = temporary.path().join("outside-active-boundary");
            fs::create_dir(&outside).unwrap();
            let escaped = cwd.join("escaped");
            std::os::unix::fs::symlink(&outside, &escaped).unwrap();
            let escaped_source = fixture.rollout_root.join("escaped.jsonl");
            write_registry_derived_test_rollout(
                &escaped_source,
                &escaped,
                "symlink escape must be rejected before canonicalization",
            );
            let error =
                resolve_registry_derived_source_member(&escaped_source, &membership, &mut cache)
                    .unwrap_err();
            assert_eq!(error.code, "integrity_invalid");
            assert!(error.error.contains("symlink that could escape"));
        }

        fs::create_dir(cwd.join(".git")).unwrap();
        fs::write(cwd.join(".git/HEAD"), b"ref: refs/heads/main\n").unwrap();
        let error =
            resolve_registry_derived_source_member(&source, &membership, &mut cache).unwrap_err();
        assert_eq!(error.code, "control_plane_invalid");
        assert!(
            error
                .error
                .contains("changed identity metadata during reconciliation")
        );

        let error = validate_historical_owner_cache(&cache, &membership).unwrap_err();
        assert!(matches!(error, WorkVcsError::ControlPlaneInvalid(_)));
        assert!(
            error
                .to_string()
                .contains("changed identity metadata during reconciliation")
        );

        let first = observe_historical_owner(&cwd).unwrap();
        fs::write(cwd.join(".git/HEAD"), b"ref: refs/heads/changed\n").unwrap();
        let changed = observe_historical_owner(&cwd).unwrap();
        assert_eq!(changed.identity, first.identity);
        assert_ne!(changed.metadata_sha256, first.metadata_sha256);
    }

    #[cfg(unix)]
    #[test]
    fn historical_owner_alias_is_exact_stable_and_fail_closed() {
        let temporary = TempDir::new().unwrap();
        let fixture = registry_derived_automation_fixture(&temporary);
        let fixture_root = temporary.path().canonicalize().unwrap();
        let membership = registry_derived_membership(Some(&fixture.registry_path)).unwrap();
        let physical_source = fixture.rollout_root.join("thread.jsonl");
        let physical_cwd = rollout_cwd_from_session_meta(&physical_source).unwrap();
        let alias_root = fixture_root.join("known-alias");
        let alias_target = physical_cwd.parent().unwrap().to_path_buf();
        std::os::unix::fs::symlink(&alias_target, &alias_root).unwrap();
        let alias_cwd = alias_root.join(physical_cwd.file_name().unwrap());
        let alias_source = fixture.rollout_root.join("alias.jsonl");
        write_registry_derived_test_rollout(
            &alias_source,
            &alias_cwd,
            "verified historical alias evidence",
        );
        let mut cache = HistoricalOwnerCache::new();
        let member =
            resolve_registry_derived_source_member(&alias_source, &membership, &mut cache).unwrap();
        assert_eq!(member.project_ref_id, fixture.project_ref_id);
        let cached = cache.get(&alias_cwd).unwrap();
        assert_eq!(cached.resolution.original_cwd, alias_cwd);
        assert_eq!(cached.resolution.probe_cwd, physical_cwd);
        let fingerprint = cached.resolution.alias.as_ref().unwrap();
        assert_eq!(fingerprint.link_path, alias_root);
        assert_eq!(fingerprint.link_target, alias_target);
        validate_historical_owner_cache(&cache, &membership).unwrap();

        let held_alias = fixture_root.join("held-original-alias");
        fs::rename(&alias_root, &held_alias).unwrap();
        std::os::unix::fs::symlink(&alias_target, &alias_root).unwrap();
        let error = resolve_registry_derived_source_member(&alias_source, &membership, &mut cache)
            .unwrap_err();
        assert_eq!(error.code, "control_plane_invalid");
        assert!(error.error.contains("changed local resolution"));

        let physical_a_parent = fixture_root.join("physical-a");
        let physical_b_parent = fixture_root.join("physical-b");
        let physical_a = physical_a_parent.join("project");
        let physical_b = physical_b_parent.join("project");
        fs::create_dir_all(&physical_a).unwrap();
        fs::create_dir_all(&physical_b).unwrap();
        let retarget_alias = fixture_root.join("retarget-alias");
        std::os::unix::fs::symlink(&physical_a_parent, &retarget_alias).unwrap();
        let retarget_cwd = retarget_alias.join("project");
        let mut retarget_membership = membership.clone();
        retarget_membership.historical_owner_boundaries =
            vec![physical_a.clone(), physical_b.clone()];
        let retarget_candidates = historical_cwd_aliases_without_io(&retarget_cwd).unwrap();
        let resolution = select_historical_probe_path(
            &retarget_cwd,
            &retarget_membership.historical_owner_boundaries,
            &retarget_candidates,
        )
        .unwrap()
        .unwrap();
        assert_eq!(resolution.probe_cwd, physical_a);
        let observation = observe_historical_owner(&resolution.probe_cwd).unwrap();
        let mut retarget_cache = HistoricalOwnerCache::new();
        retarget_cache.insert(
            retarget_cwd.clone(),
            HistoricalOwnerCacheEntry {
                resolution,
                observation,
                owner: Ok(retarget_membership.members[0].clone()),
            },
        );
        let held_retarget = fixture_root.join("held-retarget-alias");
        fs::rename(&retarget_alias, &held_retarget).unwrap();
        std::os::unix::fs::symlink(&physical_b_parent, &retarget_alias).unwrap();
        let error =
            validate_historical_owner_cache(&retarget_cache, &retarget_membership).unwrap_err();
        assert!(matches!(error, WorkVcsError::ControlPlaneInvalid(_)));
        assert!(error.to_string().contains("changed local resolution"));

        let relative_alias = fixture_root.join("relative-alias");
        std::os::unix::fs::symlink("physical-a", &relative_alias).unwrap();
        let relative_cwd = relative_alias.join("project");
        let error = select_historical_probe_path(
            &relative_cwd,
            std::slice::from_ref(&physical_a),
            std::slice::from_ref(&relative_cwd),
        )
        .unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));
        assert!(error.to_string().contains("target") && error.to_string().contains("relative"));

        let external_alias = fixture_root.join("external-alias");
        let nonexistent_external = fixture_root.join("external-target-does-not-exist");
        std::os::unix::fs::symlink(&nonexistent_external, &external_alias).unwrap();
        let external_cwd = external_alias.join("project");
        assert!(
            select_historical_probe_path(
                &external_cwd,
                std::slice::from_ref(&physical_a),
                std::slice::from_ref(&external_cwd),
            )
            .unwrap()
            .is_none()
        );
        assert!(!nonexistent_external.exists());

        let direct_external = fixture_root.join("direct-external");
        fs::create_dir(&direct_external).unwrap();
        let direct_suffix = physical_a.join("suffix-link");
        std::os::unix::fs::symlink(&direct_external, &direct_suffix).unwrap();
        let error = select_historical_probe_path(
            &direct_suffix,
            std::slice::from_ref(&physical_a),
            std::slice::from_ref(&direct_suffix),
        )
        .unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));
        assert!(error.to_string().contains("symlink that could escape"));
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

    #[cfg(unix)]
    #[test]
    fn daemon_client_times_out_when_listener_accepts_but_never_responds() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("automation");
        let runtime = root.join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = runtime.join("v1.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        let (release_sender, release_receiver) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            release_receiver
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        });

        let started = Instant::now();
        let error = daemon_request(&root, "status").unwrap_err();
        let elapsed = started.elapsed();
        release_sender.send(()).unwrap();
        server.join().unwrap();
        assert_eq!(error.code().to_string(), "storage_failure");
        assert!(elapsed >= Duration::from_millis(1_500));
        assert!(elapsed < Duration::from_secs(3));
    }

    #[cfg(unix)]
    #[test]
    fn daemon_protocol_deadline_rejects_slow_drip_frames() {
        fn assert_absolute_deadline(elapsed: Duration, error: &WorkVcsError) {
            assert_eq!(error.code().to_string(), "storage_failure");
            assert!(error.to_string().contains("protocol deadline"));
            assert!(elapsed >= Duration::from_millis(1_500));
            assert!(elapsed < Duration::from_secs(3));
        }

        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("automation");
        let runtime = root.join("runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = runtime.join("v1.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        let response_server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            for byte in b"{\"version\":1" {
                if stream.write_all(&[*byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        });
        let started = Instant::now();
        let error = daemon_request(&root, "status").unwrap_err();
        let elapsed = started.elapsed();
        response_server.join().unwrap();
        assert_absolute_deadline(elapsed, &error);

        let (mut daemon_stream, mut slow_client) = UnixStream::pair().unwrap();
        let request_client = std::thread::spawn(move || {
            for byte in b"{\"version\":1" {
                if slow_client.write_all(&[*byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        });
        let started = Instant::now();
        let error = read_daemon_request(
            &mut daemon_stream,
            Instant::now() + DAEMON_PROTOCOL_IO_TIMEOUT,
        )
        .unwrap_err();
        let elapsed = started.elapsed();
        drop(daemon_stream);
        request_client.join().unwrap();
        assert_absolute_deadline(elapsed, &error);
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
    fn registry_derived_reconcile_uses_project_membership_and_user_only_policy() {
        let temporary = TempDir::new().unwrap();
        let fixture = registry_derived_automation_fixture(&temporary);
        install_registry_derived_source_policy(&fixture, Vec::new());

        let first = reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(first.status, "completed", "{first:#?}");
        assert!(first.complete);
        assert_eq!(first.membership_mode, "registry_derived");
        assert_eq!(first.registrations_considered, 1);
        assert_eq!(first.canonical_generations_ready, 1);
        assert_eq!(first.rollout_archives_ready, 1);
        assert_eq!(first.rollout_archives_failed, 0);
        assert!(first.project_registry_id.is_some());
        assert!(first.project_registry_revision.is_some());
        assert!(first.project_registry_digest.is_some());
        assert!(first.source_policy_revision.is_some());
        assert!(first.source_policy_digest_sha256.is_some());

        let catalog = read_automation_catalog(&first.catalog_path).unwrap();
        assert_eq!(catalog.format_version, AUTOMATION_CATALOG_FORMAT_VERSION);
        assert_eq!(catalog.project_registry_id, first.project_registry_id);
        assert_eq!(
            catalog.project_registry_revision,
            first.project_registry_revision
        );
        assert_eq!(
            catalog.project_registry_digest,
            first.project_registry_digest
        );
        assert_eq!(catalog.source_policy_revision, first.source_policy_revision);
        assert_eq!(
            catalog.source_policy_digest_sha256,
            first.source_policy_digest_sha256
        );
        let archive = catalog
            .sources
            .iter()
            .find(|source| {
                source.lane == AutomationLane::HistoricalReference
                    && source.artifact_kind == AutomationArtifactKind::SourceArchive
                    && source.status == AutomationSourceStatus::Ready
            })
            .unwrap();
        assert_eq!(archive.project_ref_id, fixture.project_ref_id);
        assert!(
            archive.artifact_path.starts_with(
                fixture
                    .automation_root
                    .join("history")
                    .join("registry-derived")
            )
        );

        let user_query = query_catalog(
            &fixture.automation_root,
            Some(&fixture.registry_path),
            &first.catalog_path,
            "registry derived historical evidence",
            None,
        )
        .unwrap();
        assert_eq!(user_query.status, "completed");
        assert!(user_query.complete);
        assert!(user_query.matched_items >= 1);
        let assistant_query = query_catalog(
            &fixture.automation_root,
            Some(&fixture.registry_path),
            &first.catalog_path,
            "assistant text must be excluded",
            None,
        )
        .unwrap();
        assert_eq!(assistant_query.matched_items, 0);

        let second =
            reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(second.status, "completed");
        assert_eq!(second.rollout_files_unchanged, 1);
        assert_eq!(second.rollout_archives_ready, 1);
    }

    #[test]
    fn registry_derived_import_drift_with_prior_is_local_and_converges() {
        let temporary = TempDir::new().unwrap();
        let fixture = registry_derived_automation_fixture(&temporary);
        let raced_source = fixture.rollout_root.join("thread.jsonl");
        let stable_source = fixture.rollout_root.join("z-stable.jsonl");
        write_registry_derived_test_rollout(
            &stable_source,
            &fixture.project_roots[0],
            "stable source remains available",
        );
        install_registry_derived_source_policy(&fixture, Vec::new());

        let seeded =
            reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(seeded.status, "completed");
        assert_eq!(seeded.canonical_generations_ready, 1);
        let seeded_inventory = read_global_source_inventory(&fixture.automation_root).unwrap();
        let raced_key = raced_source.to_str().unwrap();
        let stable_key = stable_source.to_str().unwrap();
        let prior_raced_archive = seeded_inventory.files[raced_key].archive.clone().unwrap();
        let prior_stable_archive = seeded_inventory.files[stable_key].archive.clone().unwrap();

        append_registry_derived_test_message(
            &raced_source,
            "01900000-0000-7000-8000-000000000014",
            "visible before the coordinator fingerprint",
        );
        let pre_fingerprint = fingerprint_source(&raced_source).unwrap();
        let mut importer_calls = 0;
        let mut imported_after_drift = None;
        let raced = {
            let mut importer = |options| {
                importer_calls += 1;
                assert_eq!(
                    importer_calls, 1,
                    "stable source must reuse its prior archive"
                );
                append_registry_derived_test_message(
                    &raced_source,
                    "01900000-0000-7000-8000-000000000015",
                    "appended after fingerprint and before Engine import",
                );
                let result = Engine::import_source_archive(options)?;
                imported_after_drift = Some(result.clone());
                Ok(result)
            };
            reconcile_registry_derived_once_with_importer(
                &fixture.automation_root,
                Some(&fixture.registry_path),
                &mut importer,
            )
            .unwrap()
        };

        let imported_after_drift = imported_after_drift.unwrap();
        let imported_reference = archive_reference(
            imported_after_drift.archive_path.clone(),
            &imported_after_drift.manifest,
        )
        .unwrap();
        assert!(!archive_reference_matches_source(
            &imported_reference,
            &raced_source,
            &pre_fingerprint
        ));
        assert_eq!(raced.status, "partial_failure");
        assert!(!raced.complete);
        assert_eq!(raced.canonical_generations_ready, 1);
        assert_eq!(raced.rollout_archives_ready, 2);
        assert_eq!(raced.rollout_archives_failed, 1);
        let raced_catalog = read_automation_catalog(&raced.catalog_path).unwrap();
        assert!(raced_catalog.sources.iter().any(|source| {
            source.status == AutomationSourceStatus::Ready
                && source.artifact_path == prior_raced_archive.artifact_path
        }));
        assert!(raced_catalog.sources.iter().any(|source| {
            source.status == AutomationSourceStatus::Ready
                && source.artifact_path == prior_stable_archive.artifact_path
        }));
        let failure = raced_catalog
            .sources
            .iter()
            .find(|source| {
                source.status == AutomationSourceStatus::Failed
                    && source.artifact_path == raced_source
            })
            .unwrap();
        assert_eq!(failure.error_code.as_deref(), Some("integrity_invalid"));
        let error = failure.error.as_deref().unwrap();
        assert!(error.contains("source_changed_during_archive_import"));
        assert!(error.contains(&format!("path={}", raced_source.display())));
        assert!(error.contains(&format!("pre_bytes={}", pre_fingerprint.bytes)));
        assert!(error.contains(&format!("pre_sha256={}", pre_fingerprint.sha256)));
        assert!(error.contains(&format!("import_bytes={}", imported_reference.source_bytes)));
        assert!(error.contains(&format!(
            "import_sha256={}",
            imported_reference.source_sha256
        )));
        assert!(error.contains("outcome=imported"));
        assert!(error.contains("prior_archive=present"));
        assert!(raced_catalog.sources.iter().all(|source| {
            source.status != AutomationSourceStatus::Ready
                || source.artifact_path != imported_after_drift.archive_path
        }));
        let raced_inventory = read_global_source_inventory(&fixture.automation_root).unwrap();
        assert_eq!(raced_inventory.files.len(), 2);
        assert_eq!(
            raced_inventory.files[raced_key].fingerprint,
            pre_fingerprint
        );
        assert_eq!(
            raced_inventory.files[raced_key].archive.as_ref(),
            Some(&prior_raced_archive)
        );
        assert_eq!(
            raced_inventory.files[stable_key].archive.as_ref(),
            Some(&prior_stable_archive)
        );

        let converged =
            reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(converged.status, "completed", "{converged:#?}");
        assert!(converged.complete);
        assert_eq!(converged.canonical_generations_ready, 1);
        assert_eq!(converged.rollout_archives_failed, 0);
        let converged_inventory = read_global_source_inventory(&fixture.automation_root).unwrap();
        assert_eq!(
            converged_inventory.files[raced_key]
                .archive
                .as_ref()
                .unwrap()
                .artifact_path,
            imported_after_drift.archive_path
        );
    }

    #[test]
    fn registry_derived_import_drift_without_prior_keeps_later_source_available() {
        let temporary = TempDir::new().unwrap();
        let fixture = registry_derived_automation_fixture(&temporary);
        let raced_source = fixture.rollout_root.join("thread.jsonl");
        let stable_source = fixture.rollout_root.join("z-stable.jsonl");
        write_registry_derived_test_rollout(
            &stable_source,
            &fixture.project_roots[0],
            "later source imports despite earlier drift",
        );
        install_registry_derived_source_policy(&fixture, Vec::new());
        let pre_fingerprint = fingerprint_source(&raced_source).unwrap();

        let mut importer_calls = 0;
        let mut imported_after_drift = None;
        let raced = {
            let mut importer = |options| {
                importer_calls += 1;
                if importer_calls == 1 {
                    append_registry_derived_test_message(
                        &raced_source,
                        "01900000-0000-7000-8000-000000000016",
                        "first import changes only after fingerprint",
                    );
                }
                let result = Engine::import_source_archive(options)?;
                if importer_calls == 1 {
                    imported_after_drift = Some(result.clone());
                }
                Ok(result)
            };
            reconcile_registry_derived_once_with_importer(
                &fixture.automation_root,
                Some(&fixture.registry_path),
                &mut importer,
            )
            .unwrap()
        };

        assert_eq!(importer_calls, 2, "later source must still be imported");
        let imported_after_drift = imported_after_drift.unwrap();
        let imported_reference = archive_reference(
            imported_after_drift.archive_path.clone(),
            &imported_after_drift.manifest,
        )
        .unwrap();
        assert!(!archive_reference_matches_source(
            &imported_reference,
            &raced_source,
            &pre_fingerprint
        ));
        assert_eq!(raced.status, "partial_failure");
        assert!(!raced.complete);
        assert_eq!(raced.canonical_generations_ready, 1);
        assert_eq!(raced.rollout_archives_ready, 1);
        assert_eq!(raced.rollout_archives_failed, 1);
        let raced_catalog = read_automation_catalog(&raced.catalog_path).unwrap();
        assert!(raced_catalog.sources.iter().any(|source| {
            source.status == AutomationSourceStatus::Failed
                && source.artifact_path == raced_source
                && source
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("prior_archive=absent"))
        }));
        assert!(raced_catalog.sources.iter().any(|source| {
            source.status == AutomationSourceStatus::Ready
                && source
                    .source_anchor
                    .contains(stable_source.to_str().unwrap())
        }));
        assert!(raced_catalog.sources.iter().all(|source| {
            source.status != AutomationSourceStatus::Ready
                || source.artifact_path != imported_after_drift.archive_path
        }));
        let raced_inventory = read_global_source_inventory(&fixture.automation_root).unwrap();
        assert_eq!(raced_inventory.files.len(), 2);
        assert_eq!(
            raced_inventory.files[raced_source.to_str().unwrap()].fingerprint,
            pre_fingerprint
        );
        assert!(
            raced_inventory.files[raced_source.to_str().unwrap()]
                .archive
                .is_none()
        );
        assert!(
            raced_inventory.files[stable_source.to_str().unwrap()]
                .archive
                .is_some()
        );

        let converged =
            reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(converged.status, "completed", "{converged:#?}");
        assert_eq!(converged.canonical_generations_ready, 1);
        assert_eq!(converged.rollout_archives_ready, 2);
        assert_eq!(converged.rollout_archives_failed, 0);
    }

    #[test]
    fn registry_derived_import_owner_drift_is_not_adopted_by_prior_owner() {
        let temporary = TempDir::new().unwrap();
        let fixture = registry_derived_automation_fixture_with_project_count(&temporary, 2);
        let raced_source = fixture.rollout_root.join("thread.jsonl");
        let stable_source = fixture.rollout_root.join("z-stable.jsonl");
        write_registry_derived_test_rollout(
            &stable_source,
            &fixture.project_roots[0],
            "later source remains with the first owner",
        );
        install_registry_derived_source_policy(&fixture, Vec::new());
        let seeded =
            reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(seeded.status, "completed", "{seeded:#?}");
        assert_eq!(seeded.canonical_generations_ready, 2);
        let seeded_inventory = read_global_source_inventory(&fixture.automation_root).unwrap();
        let raced_key = raced_source.to_str().unwrap();
        let prior_entry = seeded_inventory.files[raced_key].clone();
        assert_eq!(
            prior_entry.member.project_ref_id,
            fixture.project_ref_ids[0]
        );

        append_registry_derived_test_message(
            &raced_source,
            "01900000-0000-7000-8000-000000000017",
            "pre-fingerprint update still belongs to the first owner",
        );
        let pre_fingerprint = fingerprint_source(&raced_source).unwrap();
        let mut importer_calls = 0;
        let mut imported_after_rewrite = None;
        let second_root = fixture.project_roots[1].clone();
        let raced = {
            let mut importer = |options| {
                importer_calls += 1;
                assert_eq!(
                    importer_calls, 1,
                    "stable source must reuse its prior archive"
                );
                write_registry_derived_test_rollout(
                    &raced_source,
                    &second_root,
                    "rewritten source belongs to the second owner",
                );
                let result = Engine::import_source_archive(options)?;
                imported_after_rewrite = Some(result.clone());
                Ok(result)
            };
            reconcile_registry_derived_once_with_importer(
                &fixture.automation_root,
                Some(&fixture.registry_path),
                &mut importer,
            )
            .unwrap()
        };

        let imported_after_rewrite = imported_after_rewrite.unwrap();
        assert_eq!(raced.status, "partial_failure");
        assert_eq!(raced.canonical_generations_ready, 2);
        assert_eq!(raced.rollout_archives_ready, 2);
        assert_eq!(raced.rollout_archives_failed, 1);
        let raced_catalog = read_automation_catalog(&raced.catalog_path).unwrap();
        assert!(raced_catalog.sources.iter().all(|source| {
            source.status != AutomationSourceStatus::Ready
                || source.artifact_path != imported_after_rewrite.archive_path
        }));
        assert!(raced_catalog.sources.iter().any(|source| {
            source.status == AutomationSourceStatus::Failed
                && source.project_ref_id == fixture.project_ref_ids[0]
                && source.artifact_path == raced_source
        }));
        let raced_inventory = read_global_source_inventory(&fixture.automation_root).unwrap();
        assert_eq!(
            raced_inventory.files[raced_key].fingerprint,
            pre_fingerprint
        );
        assert_eq!(
            raced_inventory.files[raced_key].member.project_ref_id,
            fixture.project_ref_ids[0]
        );
        assert_eq!(
            raced_inventory.files[raced_key].archive,
            prior_entry.archive
        );

        let converged =
            reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(converged.status, "completed", "{converged:#?}");
        assert!(converged.complete);
        assert_eq!(converged.canonical_generations_ready, 2);
        assert_eq!(converged.rollout_archives_ready, 2);
        assert_eq!(converged.rollout_archives_failed, 0);
        let converged_inventory = read_global_source_inventory(&fixture.automation_root).unwrap();
        assert_eq!(
            converged_inventory.files[raced_key].member.project_ref_id,
            fixture.project_ref_ids[1]
        );
        let converged_archive = converged_inventory.files[raced_key]
            .archive
            .as_ref()
            .unwrap();
        let imported_reference = archive_reference(
            imported_after_rewrite.archive_path.clone(),
            &imported_after_rewrite.manifest,
        )
        .unwrap();
        assert_ne!(
            converged_archive.artifact_path, imported_after_rewrite.archive_path,
            "the old-owner derived artifact must not be adopted by the new owner"
        );
        assert_eq!(
            converged_archive.source_sha256,
            imported_reference.source_sha256
        );
        assert_eq!(
            converged_archive.source_bytes,
            imported_reference.source_bytes
        );
        let converged_catalog = read_automation_catalog(&converged.catalog_path).unwrap();
        assert!(converged_catalog.sources.iter().any(|source| {
            source.status == AutomationSourceStatus::Ready
                && source.project_ref_id == fixture.project_ref_ids[1]
                && source.artifact_path == converged_archive.artifact_path
        }));
        assert!(converged_catalog.sources.iter().all(|source| {
            source.status != AutomationSourceStatus::Ready
                || source.artifact_path != imported_after_rewrite.archive_path
        }));
    }

    #[test]
    fn registry_derived_reconcile_quarantines_excluded_project_sources() {
        let temporary = TempDir::new().unwrap();
        let fixture = registry_derived_automation_fixture(&temporary);
        install_registry_derived_source_policy(&fixture, vec![fixture.project_ref_id]);

        let report =
            reconcile_once(&fixture.automation_root, Some(&fixture.registry_path)).unwrap();
        assert_eq!(report.status, "partial_failure", "{report:#?}");
        assert!(!report.complete);
        assert_eq!(report.canonical_generations_ready, 1);
        assert_eq!(report.rollout_archives_ready, 0);
        assert_eq!(report.rollout_archives_failed, 1);
        let catalog = read_automation_catalog(&report.catalog_path).unwrap();
        assert!(catalog.sources.iter().all(|source| {
            source.lane != AutomationLane::HistoricalReference
                || source.status != AutomationSourceStatus::Ready
        }));
        assert!(catalog.sources.iter().any(|source| {
            source.lane == AutomationLane::HistoricalReference
                && source.status == AutomationSourceStatus::Failed
                && source
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("excluded ProjectRef"))
        }));
        assert!(!fixture.automation_root.join("history").exists());
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
            let watcher = start_watcher(&root, None).unwrap();
            let mut ledger = test_ledger();
            project_watcher_health(&watcher.health, &mut ledger);
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

        let query = query_catalog(&root, None, &report.catalog_path, "unavailable", None).unwrap();
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

        let query = query_catalog(root, None, &report.catalog_path, "no sources", None).unwrap();
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
        let catalog_user =
            query_catalog(&root, None, &report.catalog_path, user_query, None).unwrap();
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

        let catalog_assistant =
            query_catalog(&root, None, &report.catalog_path, "answer", None).unwrap();
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
        let latest = latest_catalog_for_current_basis(&root, None).unwrap();
        assert_eq!(latest.catalog_path, published.catalog_path);
        assert_eq!(latest.catalog_manifest_sha256, published.manifest_sha256);
        assert!(!latest.manifest.complete);
        assert!(latest.manifest.sources.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn watcher_ignores_read_access_but_preserves_mutations_and_rescan_signals() {
        use notify::event::{
            AccessKind, AccessMode, CreateKind, DataChange, Flag, ModifyKind, RemoveKind,
        };
        use notify::{Event, EventKind};

        let read_open = Event::new(EventKind::Access(AccessKind::Open(AccessMode::Read)));
        let read_close = Event::new(EventKind::Access(AccessKind::Close(AccessMode::Read)));
        let writable_close = Event::new(EventKind::Access(AccessKind::Close(AccessMode::Write)));
        let created = Event::new(EventKind::Create(CreateKind::File));
        let modified = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Any)));
        let removed = Event::new(EventKind::Remove(RemoveKind::File));
        let unspecified = Event::new(EventKind::Any);
        let other = Event::new(EventKind::Other);
        let rescan = Event::new(EventKind::Access(AccessKind::Other)).set_flag(Flag::Rescan);

        assert!(!watcher_event_requires_reconcile(&read_open));
        assert!(!watcher_event_requires_reconcile(&read_close));
        assert!(!watcher_event_requires_reconcile(&writable_close));
        assert!(watcher_event_requires_reconcile(&created));
        assert!(watcher_event_requires_reconcile(&modified));
        assert!(watcher_event_requires_reconcile(&removed));
        assert!(watcher_event_requires_reconcile(&unspecified));
        assert!(watcher_event_requires_reconcile(&other));
        assert!(watcher_event_requires_reconcile(&rescan));
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
            .process_group(0)
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
                cancelled: false,
                duration_ms: 1,
                timeout_seconds: None,
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
                cancelled: false,
                duration_ms: 1,
                timeout_seconds: None,
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
    fn active_worker_timeout_and_shutdown_cancellation_are_explicit() {
        fn sleeping_worker(trigger: &str, timeout_seconds: Option<u64>) -> ActiveWorker {
            let mut child = ProcessCommand::new("/bin/sh")
                .arg("-c")
                .arg("sleep 10")
                .process_group(0)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let process_group_id = libc::pid_t::try_from(child.id()).unwrap();
            ActiveWorker {
                child: Some(child),
                process_group_id,
                stdout_reader: Some(std::thread::spawn(move || capture_worker_stream(stdout))),
                stderr_reader: Some(std::thread::spawn(move || capture_worker_stream(stderr))),
                trigger: trigger.to_owned(),
                started: Instant::now(),
                started_at_us: epoch_micros(),
                timeout_seconds,
            }
        }

        fn short_lived_barrier_worker(
            trigger: &str,
            timeout_seconds: Option<u64>,
            stdout_payload: &str,
            exit_code: i32,
        ) -> (ActiveWorker, std::process::ChildStdin) {
            let mut child = ProcessCommand::new("/bin/sh")
                .arg("-c")
                .arg("read ready; printf '%s' \"$PAYLOAD\"; exit \"$EXIT_CODE\"")
                .env("PAYLOAD", stdout_payload)
                .env("EXIT_CODE", exit_code.to_string())
                .process_group(0)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let stdin = child.stdin.take().unwrap();
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let process_group_id = libc::pid_t::try_from(child.id()).unwrap();
            (
                ActiveWorker {
                    child: Some(child),
                    process_group_id,
                    stdout_reader: Some(std::thread::spawn(move || capture_worker_stream(stdout))),
                    stderr_reader: Some(std::thread::spawn(move || capture_worker_stream(stderr))),
                    trigger: trigger.to_owned(),
                    started: Instant::now(),
                    started_at_us: epoch_micros(),
                    timeout_seconds,
                },
                stdin,
            )
        }

        fn release_short_lived_worker(mut stdin: std::process::ChildStdin) {
            stdin.write_all(b"ready\n").unwrap();
            drop(stdin);
            std::thread::sleep(Duration::from_millis(100));
        }

        let mut timed_out = Some(sleeping_worker("explicit-timeout", Some(1)));
        timed_out.as_mut().unwrap().started = Instant::now() - Duration::from_secs(2);
        let timed_out = poll_active_worker(&mut timed_out).unwrap().unwrap();
        assert!(timed_out.outcome.timed_out);
        assert!(!timed_out.outcome.cancelled);
        assert_eq!(timed_out.outcome.timeout_seconds, Some(1));

        let mut cancelled = Some(sleeping_worker("shutdown", None));
        let cancelled = cancel_active_worker(&mut cancelled).unwrap().unwrap();
        assert!(cancelled.outcome.cancelled);
        assert!(!cancelled.outcome.timed_out);
        assert_eq!(cancelled.outcome.timeout_seconds, None);
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("automation");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let mut cancelled_ledger = test_ledger();
        record_worker(
            &mut cancelled_ledger,
            &cancelled.trigger,
            cancelled.outcome,
            &root,
        )
        .unwrap();
        assert!(cancelled_ledger.last_worker_cancelled);
        assert!(cancelled_ledger.last_worker_evidence_path.is_some());
        assert!(
            cancelled_ledger
                .last_error
                .as_deref()
                .unwrap()
                .contains("cancelled")
        );

        let valid_report = serde_json::to_string(&publish_failed_reconcile_report(&root)).unwrap();
        let (mut natural_timeout_worker, timeout_release) =
            short_lived_barrier_worker("natural-timeout-race", Some(1), &valid_report, 0);
        natural_timeout_worker.started = Instant::now() - Duration::from_secs(2);
        let mut natural_timeout = Some(natural_timeout_worker);
        let natural_timeout = poll_active_worker_before_terminate(&mut natural_timeout, || {
            release_short_lived_worker(timeout_release);
        })
        .unwrap()
        .unwrap();
        assert_eq!(natural_timeout.outcome.exit_code, Some(0));
        assert!(!natural_timeout.outcome.timed_out);
        assert!(!natural_timeout.outcome.cancelled);
        let mut natural_timeout_ledger = test_ledger();
        record_worker(
            &mut natural_timeout_ledger,
            &natural_timeout.trigger,
            natural_timeout.outcome,
            &root,
        )
        .unwrap();
        assert_eq!(
            natural_timeout_ledger.last_worker_status.as_deref(),
            Some("failed")
        );
        assert!(natural_timeout_ledger.last_error.is_none());
        assert!(!natural_timeout_ledger.last_worker_timed_out);

        let (natural_shutdown_worker, shutdown_release) = short_lived_barrier_worker(
            "natural-shutdown-race",
            None,
            "natural shutdown completion",
            7,
        );
        let mut natural_shutdown = Some(natural_shutdown_worker);
        let natural_shutdown = cancel_active_worker_before_terminate(&mut natural_shutdown, || {
            release_short_lived_worker(shutdown_release);
        })
        .unwrap()
        .unwrap();
        assert_eq!(natural_shutdown.outcome.exit_code, Some(7));
        assert!(!natural_shutdown.outcome.timed_out);
        assert!(!natural_shutdown.outcome.cancelled);
        assert_eq!(
            natural_shutdown.outcome.stdout.bytes,
            b"natural shutdown completion"
        );
        let mut natural_shutdown_ledger = test_ledger();
        record_worker(
            &mut natural_shutdown_ledger,
            &natural_shutdown.trigger,
            natural_shutdown.outcome,
            &root,
        )
        .unwrap();
        assert!(!natural_shutdown_ledger.last_worker_cancelled);
        assert!(
            natural_shutdown_ledger
                .last_error
                .as_deref()
                .unwrap()
                .contains("exited with Some(7)")
        );

        let mut ledger = test_ledger();
        let active = Some(sleeping_worker("startup", None));
        project_active_worker(&active, &mut ledger);
        let mut response = DaemonResponse {
            version: AUTOMATION_PROTOCOL_VERSION,
            status: "completed".to_owned(),
            action: "status".to_owned(),
            message: "request completed".to_owned(),
            worker_exit_code: None,
        };
        project_worker_status(&ledger, &mut response);
        assert_eq!(response.status, "in_progress");
        let mut active = active;
        cancel_active_worker(&mut active).unwrap().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn active_worker_cancellation_terminates_and_reaps_its_process_group() {
        fn assert_process_absent(pid: libc::pid_t) {
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if unsafe { libc::kill(pid, 0) } == -1
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            panic!("process {pid} remained after worker process-group cancellation");
        }

        let temporary = TempDir::new().unwrap();
        let descendant_pid_path = temporary.path().join("descendant.pid");
        let mut child = ProcessCommand::new("/bin/sh")
            .arg("-c")
            .arg("/bin/sleep 30 & echo $! > \"$DESCENDANT_PID_PATH\"; wait")
            .env("DESCENDANT_PID_PATH", &descendant_pid_path)
            .process_group(0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let child_pid = libc::pid_t::try_from(child.id()).unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut active = Some(ActiveWorker {
            child: Some(child),
            process_group_id: child_pid,
            stdout_reader: Some(std::thread::spawn(move || capture_worker_stream(stdout))),
            stderr_reader: Some(std::thread::spawn(move || capture_worker_stream(stderr))),
            trigger: "process-group-cancel".to_owned(),
            started: Instant::now(),
            started_at_us: epoch_micros(),
            timeout_seconds: None,
        });
        let descendant_pid = wait_for_test_pid(&descendant_pid_path);

        let completed = cancel_active_worker(&mut active).unwrap().unwrap();
        assert!(completed.outcome.cancelled);
        assert_eq!(completed.outcome.exit_code, None);
        assert_process_absent(child_pid);
        assert_process_absent(descendant_pid);
    }

    #[cfg(unix)]
    #[test]
    fn completed_worker_cleans_pipe_holding_descendants_without_changing_leader_status() {
        fn assert_process_absent(pid: libc::pid_t) {
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if unsafe { libc::kill(pid, 0) } == -1
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            panic!("descendant process {pid} remained after worker leader completion");
        }

        for exit_code in [0, 7] {
            let temporary = TempDir::new().unwrap();
            let descendant_pid_path = temporary.path().join("descendant.pid");
            let mut child = ProcessCommand::new("/bin/sh")
                .arg("-c")
                .arg(
                    "/bin/sleep 30 & echo $! > \"$DESCENDANT_PID_PATH\"; printf 'leader-finished'; exit \"$EXIT_CODE\"",
                )
                .env("DESCENDANT_PID_PATH", &descendant_pid_path)
                .env("EXIT_CODE", exit_code.to_string())
                .process_group(0)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let process_group_id = libc::pid_t::try_from(child.id()).unwrap();
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let mut active = Some(ActiveWorker {
                child: Some(child),
                process_group_id,
                stdout_reader: Some(std::thread::spawn(move || capture_worker_stream(stdout))),
                stderr_reader: Some(std::thread::spawn(move || capture_worker_stream(stderr))),
                trigger: format!("leader-exit-{exit_code}"),
                started: Instant::now(),
                started_at_us: epoch_micros(),
                timeout_seconds: None,
            });
            let descendant_pid = wait_for_test_pid(&descendant_pid_path);

            let started = Instant::now();
            let completed = loop {
                if let Some(completed) = poll_active_worker(&mut active).unwrap() {
                    break completed;
                }
                assert!(started.elapsed() < Duration::from_secs(3));
                std::thread::sleep(Duration::from_millis(10));
            };
            assert!(started.elapsed() < Duration::from_secs(3));
            assert_eq!(completed.outcome.exit_code, Some(exit_code));
            assert!(!completed.outcome.timed_out);
            assert!(!completed.outcome.cancelled);
            assert_eq!(completed.outcome.stdout.bytes, b"leader-finished");
            assert_process_absent(descendant_pid);
        }
    }

    #[cfg(unix)]
    #[test]
    fn worker_report_projects_failed_catalog_without_false_success() {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("automation");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let report = publish_failed_reconcile_report(&root);
        let bytes = serde_json::to_vec(&report).unwrap();
        let mut ledger = test_ledger();
        record_worker(
            &mut ledger,
            "manual",
            WorkerOutcome {
                exit_code: Some(0),
                timed_out: false,
                cancelled: false,
                duration_ms: 1,
                timeout_seconds: None,
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
