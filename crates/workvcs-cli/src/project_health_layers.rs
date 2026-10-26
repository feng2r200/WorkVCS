use serde::Serialize;
use std::path::PathBuf;
use workvcs_core::{
    BranchId, CommitId, Digest, Engine, IntegrityReport, ProjectRefId, Result, SearchDerivedStatus,
    SearchQueryOptions, SearchSourceIdentity, SourceArchiveFileState, SourceArchiveInspectOptions,
    SourceArchiveQueryOptions, StoreId, WorkVcsError, WorkspaceId,
};

const REPORT_SCHEMA_VERSION: u32 = 1;
const HEALTH_SEARCH_PROBE: &str = "workvcs health availability probe";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LayerHealth {
    Healthy,
    Degraded,
    Blocked,
    NotChecked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct RecoveryAction {
    pub action: String,
    pub automatic: bool,
    pub requires_authorization: bool,
}

impl RecoveryAction {
    pub(crate) fn manual(action: impl Into<String>, requires_authorization: bool) -> Self {
        Self {
            action: action.into(),
            automatic: false,
            requires_authorization,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct HealthIssue {
    pub layer: String,
    pub code: String,
    pub message: String,
    pub affects_normal_work: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<RecoveryAction>,
}

impl HealthIssue {
    pub(crate) fn new(
        layer: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        affects_normal_work: bool,
        recovery_action: Option<RecoveryAction>,
    ) -> Self {
        Self {
            layer: layer.into(),
            code: code.into(),
            message: message.into(),
            affects_normal_work,
            recovery_action,
        }
    }

    fn from_error(
        layer: &'static str,
        error: &WorkVcsError,
        affects_normal_work: bool,
        recovery_action: RecoveryAction,
    ) -> Self {
        Self::new(
            layer,
            error.code().to_string(),
            error.to_string(),
            affects_normal_work,
            Some(recovery_action),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct RegistryHealthObservation {
    pub path: PathBuf,
    pub source: String,
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    pub digest: String,
    pub migration_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct BindingHealthObservation {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_ref_id: Option<String>,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_content_objects_verified: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ActivationHealthObservation {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct CapabilityHealthObservation {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ResolutionHealthObservation {
    pub requested: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_ref_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ControlPlaneHealthLayer {
    pub health: LayerHealth,
    pub checked: bool,
    pub required_for_normal_work: bool,
    pub registry: RegistryHealthObservation,
    pub bindings: Vec<BindingHealthObservation>,
    pub routing: ActivationHealthObservation,
    pub journal_admission: ActivationHealthObservation,
    pub capabilities: Vec<CapabilityHealthObservation>,
    pub resolution: ResolutionHealthObservation,
    pub issues: Vec<HealthIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct StoreIntegrityObservation {
    pub checked_branches: usize,
    pub checked_commits: usize,
    pub checked_changesets: usize,
    pub checked_change_operations: usize,
    pub checked_changeset_causal_anchors: usize,
    pub checked_events: usize,
    pub checked_checkpoints: usize,
    pub invalid_checkpoints: usize,
}

impl From<IntegrityReport> for StoreIntegrityObservation {
    fn from(value: IntegrityReport) -> Self {
        Self {
            checked_branches: value.checked_branches,
            checked_commits: value.checked_commits,
            checked_changesets: value.checked_changesets,
            checked_change_operations: value.checked_change_operations,
            checked_changeset_causal_anchors: value.checked_changeset_causal_anchors,
            checked_events: value.checked_events,
            checked_checkpoints: value.checked_checkpoints,
            invalid_checkpoints: value.invalid_checkpoints,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct CurrentStoreHealthLayer {
    pub health: LayerHealth,
    pub checked: bool,
    pub required_for_normal_work: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_ref_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_commit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_state_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<StoreIntegrityObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_content_objects_verified: Option<usize>,
    pub issues: Vec<HealthIssue>,
}

impl CurrentStoreHealthLayer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn verified(
        store_path: impl Into<PathBuf>,
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        head_commit_id: CommitId,
        head_state_digest: Digest,
        integrity: IntegrityReport,
        local_content_objects_verified: usize,
    ) -> Self {
        let integrity = StoreIntegrityObservation::from(integrity);
        let mut issues = Vec::new();
        let health = if integrity.invalid_checkpoints == 0 {
            LayerHealth::Healthy
        } else {
            issues.push(HealthIssue::new(
                "current_store",
                "invalid_checkpoint",
                format!(
                    "{} checkpoint(s) are retained but invalid",
                    integrity.invalid_checkpoints
                ),
                true,
                Some(RecoveryAction::manual(
                    "inspect the invalid checkpoint evidence before deciding whether to replace it",
                    true,
                )),
            ));
            LayerHealth::Degraded
        };
        Self {
            health,
            checked: true,
            required_for_normal_work: true,
            store_path: Some(store_path.into()),
            project_ref_id: Some(project_ref_id.to_string()),
            store_id: Some(store_id.to_string()),
            workspace_id: Some(workspace_id.to_string()),
            branch_id: Some(branch_id.to_string()),
            head_commit_id: Some(head_commit_id.to_string()),
            head_state_digest: Some(head_state_digest.to_string()),
            integrity: Some(integrity),
            local_content_objects_verified: Some(local_content_objects_verified),
            issues,
        }
    }

    pub(crate) fn not_checked(reason: impl Into<String>) -> Self {
        Self {
            health: LayerHealth::NotChecked,
            checked: false,
            required_for_normal_work: true,
            store_path: None,
            project_ref_id: None,
            store_id: None,
            workspace_id: None,
            branch_id: None,
            head_commit_id: None,
            head_state_digest: None,
            integrity: None,
            local_content_objects_verified: None,
            issues: vec![HealthIssue::new(
                "current_store",
                "store_not_selected",
                reason,
                true,
                Some(RecoveryAction::manual(
                    "select a CWD that resolves to one verified ProjectRef binding",
                    false,
                )),
            )],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn failed_selection(
        store_path: impl Into<PathBuf>,
        project_ref_id: ProjectRefId,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            health: LayerHealth::Blocked,
            checked: true,
            required_for_normal_work: true,
            store_path: Some(store_path.into()),
            project_ref_id: Some(project_ref_id.to_string()),
            store_id: Some(store_id.to_string()),
            workspace_id: Some(workspace_id.to_string()),
            branch_id: Some(branch_id.to_string()),
            head_commit_id: None,
            head_state_digest: None,
            integrity: None,
            local_content_objects_verified: None,
            issues: vec![HealthIssue::new(
                "current_store",
                code,
                message,
                true,
                Some(RecoveryAction::manual(
                    "inspect the exact selected binding and Store verification failure before changing either authority",
                    true,
                )),
            )],
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HistoricalArchiveRequest {
    pub identity: SearchSourceIdentity,
    pub archive_path: PathBuf,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct SourceObservationCounts {
    pub unchanged: usize,
    pub changed: usize,
    pub missing: usize,
    pub unreadable: usize,
}

impl SourceObservationCounts {
    fn observe(&mut self, state: SourceArchiveFileState) {
        match state {
            SourceArchiveFileState::Unchanged => self.unchanged += 1,
            SourceArchiveFileState::Changed => self.changed += 1,
            SourceArchiveFileState::Missing => self.missing += 1,
            SourceArchiveFileState::Unreadable => self.unreadable += 1,
        }
    }

    fn has_currentness_gap(&self) -> bool {
        self.changed != 0 || self.missing != 0 || self.unreadable != 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct HistoricalArchiveObservation {
    pub archive_path: PathBuf,
    pub health: LayerHealth,
    pub archive_readback_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_count: Option<usize>,
    pub source_files: SourceObservationCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<HealthIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct HistoricalSourcesHealthLayer {
    pub health: LayerHealth,
    pub checked: bool,
    pub required_for_normal_work: bool,
    pub requested_archives: usize,
    pub ready_archives: usize,
    pub failed_archives: usize,
    pub archives: Vec<HistoricalArchiveObservation>,
    pub issues: Vec<HealthIssue>,
}

#[derive(Clone, Debug)]
pub(crate) struct CurrentSearchRequest {
    pub store_path: PathBuf,
    pub identity: SearchSourceIdentity,
    pub index_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct RetrievalCapabilityHealth {
    pub health: LayerHealth,
    pub checked: bool,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub readback_verified: bool,
    pub issues: Vec<HealthIssue>,
}

impl RetrievalCapabilityHealth {
    fn not_checked(reason_code: &'static str, reason: &'static str) -> Self {
        Self {
            health: LayerHealth::NotChecked,
            checked: false,
            available: false,
            status: None,
            reason_code: Some(reason_code.to_owned()),
            reason: Some(reason.to_owned()),
            readback_verified: false,
            issues: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct RetrievalHealthLayer {
    pub health: LayerHealth,
    pub checked: bool,
    pub required_for_normal_work: bool,
    pub current_lexical: RetrievalCapabilityHealth,
    pub historical_lexical: RetrievalCapabilityHealth,
    pub semantic: RetrievalCapabilityHealth,
    pub issues: Vec<HealthIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct HealthWriteEvidence {
    pub store_written: bool,
    pub registry_written: bool,
    pub activation_written: bool,
    pub journal_written: bool,
    pub index_written: bool,
    pub archive_written: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct LayeredProjectHealthReport {
    pub schema_version: u32,
    pub health: LayerHealth,
    pub normal_work_ready: bool,
    pub read_only: bool,
    pub control_plane: ControlPlaneHealthLayer,
    pub current_store: CurrentStoreHealthLayer,
    pub historical_sources: HistoricalSourcesHealthLayer,
    pub retrieval: RetrievalHealthLayer,
    pub issues: Vec<HealthIssue>,
    pub writes: HealthWriteEvidence,
}

pub(crate) fn inspect_layered_project_health(
    control_plane: ControlPlaneHealthLayer,
    current_store: CurrentStoreHealthLayer,
    historical_requests: Vec<HistoricalArchiveRequest>,
    current_search: Option<CurrentSearchRequest>,
) -> LayeredProjectHealthReport {
    let historical_sources = inspect_historical_sources(&historical_requests);
    let retrieval = inspect_retrieval(current_search.as_ref(), &historical_requests);
    let health = aggregate_health(
        control_plane.health,
        current_store.health,
        historical_sources.health,
        retrieval.health,
    );
    let normal_work_ready = control_plane.health == LayerHealth::Healthy
        && current_store.health == LayerHealth::Healthy;
    let mut issues = Vec::new();
    issues.extend(control_plane.issues.iter().cloned());
    issues.extend(current_store.issues.iter().cloned());
    issues.extend(historical_sources.issues.iter().cloned());
    issues.extend(retrieval.issues.iter().cloned());
    LayeredProjectHealthReport {
        schema_version: REPORT_SCHEMA_VERSION,
        health,
        normal_work_ready,
        read_only: true,
        control_plane,
        current_store,
        historical_sources,
        retrieval,
        issues,
        writes: HealthWriteEvidence {
            store_written: false,
            registry_written: false,
            activation_written: false,
            journal_written: false,
            index_written: false,
            archive_written: false,
        },
    }
}

pub(crate) fn render_layered_project_health_json(
    report: &LayeredProjectHealthReport,
) -> Result<String> {
    let mut output = serde_json::to_string_pretty(report).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot serialize layered project health: {error}"
        ))
    })?;
    output.push('\n');
    Ok(output)
}

fn inspect_historical_sources(
    requests: &[HistoricalArchiveRequest],
) -> HistoricalSourcesHealthLayer {
    if requests.is_empty() {
        return HistoricalSourcesHealthLayer {
            health: LayerHealth::NotChecked,
            checked: false,
            required_for_normal_work: false,
            requested_archives: 0,
            ready_archives: 0,
            failed_archives: 0,
            archives: Vec::new(),
            issues: Vec::new(),
        };
    }
    let mut ready_archives = 0;
    let mut archives = Vec::with_capacity(requests.len());
    let mut issues = Vec::new();
    for request in requests {
        match Engine::inspect_source_archive(SourceArchiveInspectOptions::new(
            request.identity,
            &request.archive_path,
        )) {
            Ok(result) => {
                let mut source_files = SourceObservationCounts::default();
                for observation in result.file_observations {
                    source_files.observe(observation.state);
                }
                let issue = if !result.archive_readback_verified {
                    Some(HealthIssue::new(
                        "historical_sources",
                        "archive_readback_unverified",
                        format!(
                            "archive {} did not pass immutable archive readback verification",
                            request.archive_path.display()
                        ),
                        false,
                        Some(RecoveryAction::manual(
                            "inspect the immutable archive failure and republish only from an authorized verified source",
                            true,
                        )),
                    ))
                } else if source_files.has_currentness_gap() {
                    Some(HealthIssue::new(
                        "historical_sources",
                        "historical_source_currentness_gap",
                        format!(
                            "archive {} is readable but one or more original source files changed, moved, disappeared, or became unreadable",
                            request.archive_path.display()
                        ),
                        false,
                        Some(RecoveryAction::manual(
                            "run the authorized historical-source reconciliation and inspect its exact source classification",
                            true,
                        )),
                    ))
                } else {
                    None
                };
                if let Some(issue) = issue.as_ref() {
                    issues.push(issue.clone());
                }
                let health = if result.archive_readback_verified {
                    ready_archives += 1;
                    if source_files.has_currentness_gap() {
                        LayerHealth::Degraded
                    } else {
                        LayerHealth::Healthy
                    }
                } else {
                    LayerHealth::Blocked
                };
                archives.push(HistoricalArchiveObservation {
                    archive_path: result.archive_path,
                    health,
                    archive_readback_verified: result.archive_readback_verified,
                    manifest_sha256: Some(result.manifest.manifest_sha256),
                    format_version: Some(result.manifest.format_version),
                    document_count: Some(result.manifest.document_count),
                    source_files,
                    issue,
                });
            }
            Err(error) => {
                let issue = HealthIssue::from_error(
                    "historical_sources",
                    &error,
                    false,
                    RecoveryAction::manual(
                        "inspect the immutable archive failure and republish only from an authorized verified source",
                        true,
                    ),
                );
                issues.push(issue.clone());
                archives.push(HistoricalArchiveObservation {
                    archive_path: request.archive_path.clone(),
                    health: LayerHealth::Blocked,
                    archive_readback_verified: false,
                    manifest_sha256: None,
                    format_version: None,
                    document_count: None,
                    source_files: SourceObservationCounts::default(),
                    issue: Some(issue),
                });
            }
        }
    }
    let failed_archives = requests.len() - ready_archives;
    let health = if ready_archives == 0 {
        LayerHealth::Blocked
    } else if failed_archives != 0
        || archives
            .iter()
            .any(|archive| archive.health == LayerHealth::Degraded)
    {
        LayerHealth::Degraded
    } else {
        LayerHealth::Healthy
    };
    HistoricalSourcesHealthLayer {
        health,
        checked: true,
        required_for_normal_work: false,
        requested_archives: requests.len(),
        ready_archives,
        failed_archives,
        archives,
        issues,
    }
}

fn inspect_retrieval(
    current_search: Option<&CurrentSearchRequest>,
    historical_requests: &[HistoricalArchiveRequest],
) -> RetrievalHealthLayer {
    let current_lexical = current_search.map_or_else(
        || {
            RetrievalCapabilityHealth::not_checked(
                "current_search_not_selected",
                "no exact current Store search index was selected",
            )
        },
        inspect_current_search,
    );
    let historical_lexical = inspect_historical_search(historical_requests);
    let semantic = RetrievalCapabilityHealth::not_checked(
        "semantic_search_not_checked",
        "no pure read-only semantic generation inspection API was selected",
    );
    let checked = current_lexical.checked || historical_lexical.checked || semantic.checked;
    let health =
        aggregate_optional_capabilities([&current_lexical, &historical_lexical, &semantic]);
    let mut issues = Vec::new();
    issues.extend(current_lexical.issues.iter().cloned());
    issues.extend(historical_lexical.issues.iter().cloned());
    issues.extend(semantic.issues.iter().cloned());
    RetrievalHealthLayer {
        health,
        checked,
        required_for_normal_work: false,
        current_lexical,
        historical_lexical,
        semantic,
        issues,
    }
}

fn inspect_current_search(request: &CurrentSearchRequest) -> RetrievalCapabilityHealth {
    let engine = match Engine::open_readonly(&request.store_path) {
        Ok(engine) => engine,
        Err(error) => return blocked_search_capability("current_store_search", error),
    };
    let options =
        match SearchQueryOptions::new(request.identity, &request.index_path, HEALTH_SEARCH_PROBE)
            .and_then(|options| options.with_result_limit(1))
        {
            Ok(options) => options,
            Err(error) => return blocked_search_capability("current_store_search", error),
        };
    match engine.search(options) {
        Ok(result) => match result.status {
            SearchDerivedStatus::Ready if result.canonical_readback_verified => {
                RetrievalCapabilityHealth {
                    health: LayerHealth::Healthy,
                    checked: true,
                    available: true,
                    status: Some("ready".to_owned()),
                    reason_code: result.reason_code,
                    reason: result.reason,
                    readback_verified: true,
                    issues: Vec::new(),
                }
            }
            SearchDerivedStatus::Ready => {
                let issue = HealthIssue::new(
                    "retrieval.current_lexical",
                    "search_readback_unverified",
                    "current lexical search returned ready without canonical readback verification",
                    false,
                    Some(RecoveryAction::manual(
                        "inspect the exact derived index and refresh it only against the verified routed source",
                        true,
                    )),
                );
                RetrievalCapabilityHealth {
                    health: LayerHealth::Blocked,
                    checked: true,
                    available: false,
                    status: Some("blocked".to_owned()),
                    reason_code: Some("search_readback_unverified".to_owned()),
                    reason: Some(issue.message.clone()),
                    readback_verified: false,
                    issues: vec![issue],
                }
            }
            SearchDerivedStatus::RefreshRequired => {
                let reason_code = result
                    .reason_code
                    .unwrap_or_else(|| "refresh_required".to_owned());
                let reason = result
                    .reason
                    .unwrap_or_else(|| "derived search index requires refresh".to_owned());
                let issue = HealthIssue::new(
                    "retrieval.current_lexical",
                    &reason_code,
                    &reason,
                    false,
                    Some(RecoveryAction::manual(
                        "refresh the exact derived index only after confirming its Store, Workspace, Branch and source head",
                        true,
                    )),
                );
                RetrievalCapabilityHealth {
                    health: LayerHealth::Degraded,
                    checked: true,
                    available: false,
                    status: Some("refresh_required".to_owned()),
                    reason_code: Some(reason_code),
                    reason: Some(reason),
                    readback_verified: result.canonical_readback_verified,
                    issues: vec![issue],
                }
            }
        },
        Err(error) => blocked_search_capability("retrieval.current_lexical", error),
    }
}

fn inspect_historical_search(requests: &[HistoricalArchiveRequest]) -> RetrievalCapabilityHealth {
    if requests.is_empty() {
        return RetrievalCapabilityHealth::not_checked(
            "historical_search_not_selected",
            "no historical source archive was selected",
        );
    }
    let mut ready = 0;
    let mut issues = Vec::new();
    for request in requests {
        let options = match SourceArchiveQueryOptions::new(
            request.identity,
            &request.archive_path,
            HEALTH_SEARCH_PROBE,
        )
        .and_then(|options| options.with_result_limit(1))
        {
            Ok(options) => options,
            Err(error) => {
                issues.push(HealthIssue::from_error(
                    "retrieval.historical_lexical",
                    &error,
                    false,
                    RecoveryAction::manual(
                        "inspect the exact historical archive query failure before republishing",
                        true,
                    ),
                ));
                continue;
            }
        };
        match Engine::query_source_archive(options) {
            Ok(result) if result.status == "ready" && result.archive_readback_verified => {
                ready += 1;
            }
            Ok(result) => issues.push(HealthIssue::new(
                "retrieval.historical_lexical",
                "historical_search_not_ready",
                format!(
                    "archive {} returned status {:?} with readback_verified={}",
                    request.archive_path.display(),
                    result.status,
                    result.archive_readback_verified
                ),
                false,
                Some(RecoveryAction::manual(
                    "inspect the immutable archive readback before replacing it",
                    true,
                )),
            )),
            Err(error) => issues.push(HealthIssue::from_error(
                "retrieval.historical_lexical",
                &error,
                false,
                RecoveryAction::manual(
                    "inspect the exact historical archive query failure before republishing",
                    true,
                ),
            )),
        }
    }
    let health = if ready == requests.len() {
        LayerHealth::Healthy
    } else if ready == 0 {
        LayerHealth::Blocked
    } else {
        LayerHealth::Degraded
    };
    RetrievalCapabilityHealth {
        health,
        checked: true,
        available: ready != 0,
        status: Some(
            if ready == requests.len() {
                "ready"
            } else if ready == 0 {
                "unavailable"
            } else {
                "partially_ready"
            }
            .to_owned(),
        ),
        reason_code: (!issues.is_empty()).then(|| "historical_search_partial_failure".to_owned()),
        reason: (!issues.is_empty()).then(|| {
            format!(
                "{ready} of {} selected historical archives are searchable",
                requests.len()
            )
        }),
        readback_verified: ready == requests.len(),
        issues,
    }
}

fn blocked_search_capability(
    layer: &'static str,
    error: WorkVcsError,
) -> RetrievalCapabilityHealth {
    let issue = HealthIssue::from_error(
        layer,
        &error,
        false,
        RecoveryAction::manual(
            "inspect the exact derived search failure and refresh only against the verified routed source",
            true,
        ),
    );
    RetrievalCapabilityHealth {
        health: LayerHealth::Blocked,
        checked: true,
        available: false,
        status: Some("blocked".to_owned()),
        reason_code: Some(error.code().to_string()),
        reason: Some(error.to_string()),
        readback_verified: false,
        issues: vec![issue],
    }
}

fn aggregate_optional_capabilities<const N: usize>(
    capabilities: [&RetrievalCapabilityHealth; N],
) -> LayerHealth {
    let checked_count = capabilities
        .iter()
        .filter(|capability| capability.checked)
        .count();
    if checked_count == 0 {
        LayerHealth::NotChecked
    } else if capabilities
        .iter()
        .filter(|capability| capability.checked)
        .all(|capability| capability.health == LayerHealth::Healthy)
    {
        LayerHealth::Healthy
    } else if capabilities
        .iter()
        .filter(|capability| capability.checked)
        .all(|capability| capability.health == LayerHealth::Blocked)
    {
        LayerHealth::Blocked
    } else {
        LayerHealth::Degraded
    }
}

fn aggregate_health(
    control_plane: LayerHealth,
    current_store: LayerHealth,
    historical_sources: LayerHealth,
    retrieval: LayerHealth,
) -> LayerHealth {
    if control_plane == LayerHealth::Blocked || current_store == LayerHealth::Blocked {
        return LayerHealth::Blocked;
    }
    if matches!(
        control_plane,
        LayerHealth::Degraded | LayerHealth::NotChecked
    ) || matches!(
        current_store,
        LayerHealth::Degraded | LayerHealth::NotChecked
    ) || matches!(
        historical_sources,
        LayerHealth::Degraded | LayerHealth::Blocked
    ) || matches!(retrieval, LayerHealth::Degraded | LayerHealth::Blocked)
    {
        return LayerHealth::Degraded;
    }
    LayerHealth::Healthy
}
