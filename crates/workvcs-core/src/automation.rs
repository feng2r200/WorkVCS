use crate::error::{Result, WorkVcsError};
use crate::identity::{BranchId, ProjectRefId, RegistryId, StoreId, WorkspaceId};
use crate::store::current_epoch_micros;
use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::ffi::{CString, OsStr};
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};

pub const AUTOMATION_REGISTRY_FORMAT_VERSION: u32 = 1;
pub const AUTOMATION_SOURCE_POLICY_FORMAT_VERSION: u32 = 1;
pub const AUTOMATION_CATALOG_FORMAT_VERSION: u32 = 2;
const LEGACY_AUTOMATION_CATALOG_FORMAT_VERSION: u32 = 1;
const LEGACY_REGISTRY_DIGEST_SENTINEL: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
const REGISTRY_FILE: &str = "registrations.json";
const SOURCE_POLICY_FILE: &str = "source-policy.json";
const REGISTRY_LOCK_FILE: &str = ".registrations.lock";
const MAX_REGISTRY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SOURCE_POLICY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CATALOG_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRolePolicy {
    UserOnly,
    UserAndAssistant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationLane {
    CanonicalCurrent,
    HistoricalReference,
}

impl AutomationLane {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalCurrent => "canonical_current",
            Self::HistoricalReference => "historical_reference",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationArtifactKind {
    LexicalGeneration,
    SourceArchive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationSourceStatus {
    Ready,
    Failed,
    Quarantined,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationRegistration {
    pub registration_id: String,
    pub revision: u64,
    pub project_ref_id: ProjectRefId,
    pub store_path: PathBuf,
    pub store_id: StoreId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub project_root: PathBuf,
    pub rollout_roots: Vec<PathBuf>,
    pub role_policy: AutomationRolePolicy,
    pub registry_revision: u64,
    pub registry_digest: String,
    pub enabled: bool,
    pub revoked: bool,
    pub created_at_us: i64,
    pub updated_at_us: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomationRegistrationProposal {
    pub registration_id: String,
    pub project_ref_id: ProjectRefId,
    pub store_path: PathBuf,
    pub store_id: StoreId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub project_root: PathBuf,
    pub rollout_roots: Vec<PathBuf>,
    pub role_policy: AutomationRolePolicy,
    pub registry_revision: u64,
    pub registry_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AutomationRegistrySnapshot {
    pub format_version: u32,
    pub revision: u64,
    pub digest_sha256: String,
    pub registrations: Vec<AutomationRegistration>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AutomationRegistryBody {
    format_version: u32,
    revision: u64,
    registrations: Vec<AutomationRegistration>,
}

impl AutomationRegistryBody {
    fn empty() -> Self {
        Self {
            format_version: AUTOMATION_REGISTRY_FORMAT_VERSION,
            revision: 0,
            registrations: Vec::new(),
        }
    }

    fn snapshot(&self) -> Result<AutomationRegistrySnapshot> {
        Ok(AutomationRegistrySnapshot {
            format_version: self.format_version,
            revision: self.revision,
            digest_sha256: sha256_hex(&canonical_json(self)?),
            registrations: self.registrations.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AutomationRegistrationPreview {
    pub registry_revision: u64,
    pub registry_digest_sha256: String,
    pub registration: AutomationRegistration,
    pub would_write: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationSourcePolicyEntry {
    pub source_id: String,
    pub revision: u64,
    pub rollout_root: PathBuf,
    pub role_policy: AutomationRolePolicy,
    pub excluded_project_refs: Vec<ProjectRefId>,
    pub enabled: bool,
    pub revoked: bool,
    pub created_at_us: i64,
    pub updated_at_us: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomationSourcePolicyProposal {
    pub source_id: String,
    pub rollout_root: PathBuf,
    pub role_policy: AutomationRolePolicy,
    pub excluded_project_refs: Vec<ProjectRefId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AutomationSourcePolicySnapshot {
    pub format_version: u32,
    pub revision: u64,
    pub digest_sha256: String,
    pub sources: Vec<AutomationSourcePolicyEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AutomationSourcePolicyBody {
    format_version: u32,
    revision: u64,
    sources: Vec<AutomationSourcePolicyEntry>,
}

impl AutomationSourcePolicyBody {
    fn empty() -> Self {
        Self {
            format_version: AUTOMATION_SOURCE_POLICY_FORMAT_VERSION,
            revision: 0,
            sources: Vec::new(),
        }
    }

    fn snapshot(&self) -> Result<AutomationSourcePolicySnapshot> {
        Ok(AutomationSourcePolicySnapshot {
            format_version: self.format_version,
            revision: self.revision,
            digest_sha256: sha256_hex(&canonical_json(self)?),
            sources: self.sources.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AutomationSourcePolicyPreview {
    pub policy_revision: u64,
    pub policy_digest_sha256: String,
    pub source: AutomationSourcePolicyEntry,
    pub would_write: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationCatalogSource {
    pub registration_id: String,
    pub project_ref_id: ProjectRefId,
    pub lane: AutomationLane,
    pub artifact_kind: AutomationArtifactKind,
    pub artifact_path: PathBuf,
    pub artifact_manifest_sha256: String,
    pub source_anchor: String,
    pub status: AutomationSourceStatus,
    pub error_code: Option<String>,
    pub error: Option<String>,
}

/// The immutable control-plane basis for a catalog whose membership comes from
/// the active ProjectRef registry rather than the legacy registration list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryDerivedCatalogBasis {
    pub project_registry_id: String,
    pub project_registry_revision: u64,
    pub project_registry_digest: String,
    pub source_policy_revision: u64,
    pub source_policy_digest_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationCatalogManifest {
    pub format_version: u32,
    pub automation_registry_revision: u64,
    pub automation_registry_digest_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_registry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_registry_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_registry_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_policy_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_policy_digest_sha256: Option<String>,
    pub created_at_us: i64,
    pub complete: bool,
    pub sources: Vec<AutomationCatalogSource>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AutomationCatalogPublication {
    pub outcome: String,
    pub catalog_path: PathBuf,
    pub manifest_sha256: String,
    pub manifest: AutomationCatalogManifest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CatalogBasis {
    Legacy {
        automation_registry_revision: u64,
        automation_registry_digest_sha256: String,
    },
    RegistryDerived(RegistryDerivedCatalogBasis),
}

impl CatalogBasis {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Legacy {
                automation_registry_digest_sha256,
                ..
            } => validate_sha256(
                "automation registry digest",
                automation_registry_digest_sha256,
            ),
            Self::RegistryDerived(basis) => {
                RegistryId::parse_canonical(&basis.project_registry_id)?;
                validate_control_plane_digest(&basis.project_registry_digest)?;
                validate_sha256(
                    "automation source policy digest",
                    &basis.source_policy_digest_sha256,
                )
            }
        }
    }

    fn matches_manifest(&self, manifest: &AutomationCatalogManifest) -> bool {
        match self {
            Self::Legacy {
                automation_registry_revision,
                automation_registry_digest_sha256,
            } => {
                manifest.format_version == LEGACY_AUTOMATION_CATALOG_FORMAT_VERSION
                    && manifest.automation_registry_revision == *automation_registry_revision
                    && manifest.automation_registry_digest_sha256
                        == *automation_registry_digest_sha256
            }
            Self::RegistryDerived(basis) => {
                manifest.format_version == AUTOMATION_CATALOG_FORMAT_VERSION
                    && manifest.project_registry_id.as_deref()
                        == Some(basis.project_registry_id.as_str())
                    && manifest.project_registry_revision == Some(basis.project_registry_revision)
                    && manifest.project_registry_digest.as_deref()
                        == Some(basis.project_registry_digest.as_str())
                    && manifest.source_policy_revision == Some(basis.source_policy_revision)
                    && manifest.source_policy_digest_sha256.as_deref()
                        == Some(basis.source_policy_digest_sha256.as_str())
            }
        }
    }

    fn manifest(
        &self,
        created_at_us: i64,
        complete: bool,
        sources: Vec<AutomationCatalogSource>,
    ) -> AutomationCatalogManifest {
        match self {
            Self::Legacy {
                automation_registry_revision,
                automation_registry_digest_sha256,
            } => AutomationCatalogManifest {
                format_version: LEGACY_AUTOMATION_CATALOG_FORMAT_VERSION,
                automation_registry_revision: *automation_registry_revision,
                automation_registry_digest_sha256: automation_registry_digest_sha256.clone(),
                project_registry_id: None,
                project_registry_revision: None,
                project_registry_digest: None,
                source_policy_revision: None,
                source_policy_digest_sha256: None,
                created_at_us,
                complete,
                sources,
            },
            Self::RegistryDerived(basis) => AutomationCatalogManifest {
                format_version: AUTOMATION_CATALOG_FORMAT_VERSION,
                // The legacy registry is deliberately not a membership input in this mode.
                // A fixed sentinel makes that absence explicit while preserving v1 fields.
                automation_registry_revision: 0,
                automation_registry_digest_sha256: LEGACY_REGISTRY_DIGEST_SENTINEL.to_owned(),
                project_registry_id: Some(basis.project_registry_id.clone()),
                project_registry_revision: Some(basis.project_registry_revision),
                project_registry_digest: Some(basis.project_registry_digest.clone()),
                source_policy_revision: Some(basis.source_policy_revision),
                source_policy_digest_sha256: Some(basis.source_policy_digest_sha256.clone()),
                created_at_us,
                complete,
                sources,
            },
        }
    }
}

pub fn automation_registry_path(root: &Path) -> PathBuf {
    root.join(REGISTRY_FILE)
}

pub fn automation_source_policy_path(root: &Path) -> PathBuf {
    root.join(SOURCE_POLICY_FILE)
}

pub fn read_automation_registry(root: &Path) -> Result<AutomationRegistrySnapshot> {
    validate_derived_root(root, false)?;
    read_registry_body(root)?.snapshot()
}

pub fn read_automation_source_policy(root: &Path) -> Result<AutomationSourcePolicySnapshot> {
    validate_derived_root(root, false)?;
    read_source_policy_body(root)?.snapshot()
}

pub fn preview_automation_source_policy(
    root: &Path,
    proposal: AutomationSourcePolicyProposal,
) -> Result<AutomationSourcePolicyPreview> {
    validate_derived_root(root, false)?;
    reject_legacy_registration_mode(root)?;
    let body = read_source_policy_body(root)?;
    let snapshot = body.snapshot()?;
    let source = validate_source_policy_proposal(root, &body, proposal, 1)?;
    Ok(AutomationSourcePolicyPreview {
        policy_revision: snapshot.revision,
        policy_digest_sha256: snapshot.digest_sha256,
        source,
        would_write: true,
    })
}

pub fn apply_automation_source_policy(
    root: &Path,
    expected_revision: u64,
    expected_digest_sha256: &str,
    proposal: AutomationSourcePolicyProposal,
) -> Result<AutomationSourcePolicySnapshot> {
    validate_derived_root(root, true)?;
    let lock = open_registry_lock(root)?;
    fs4::FileExt::try_lock(&lock).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "automation policy is busy or cannot be exclusively locked: {error}"
        ))
    })?;
    reject_legacy_registration_mode(root)?;
    let mut body = read_source_policy_body(root)?;
    require_source_policy_basis(&body, expected_revision, expected_digest_sha256)?;
    if body
        .sources
        .iter()
        .any(|item| item.source_id == proposal.source_id)
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation source policy entry {:?} already exists",
            proposal.source_id
        )));
    }
    let source = validate_source_policy_proposal(root, &body, proposal, 1)?;
    body.sources.push(source);
    body.sources
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    body.revision = body.revision.checked_add(1).ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid("automation source policy revision overflow".to_owned())
    })?;
    write_source_policy_body(root, &body)?;
    body.snapshot()
}

pub fn revoke_automation_source_policy(
    root: &Path,
    expected_revision: u64,
    expected_digest_sha256: &str,
    source_id: &str,
) -> Result<AutomationSourcePolicySnapshot> {
    validate_derived_root(root, true)?;
    let lock = open_registry_lock(root)?;
    fs4::FileExt::try_lock(&lock).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "automation policy is busy or cannot be exclusively locked: {error}"
        ))
    })?;
    let mut body = read_source_policy_body(root)?;
    require_source_policy_basis(&body, expected_revision, expected_digest_sha256)?;
    let now = current_epoch_micros()?;
    let source = body
        .sources
        .iter_mut()
        .find(|item| item.source_id == source_id)
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "automation source policy entry {source_id:?} was not found"
            ))
        })?;
    if source.revoked {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation source policy entry {source_id:?} is already revoked"
        )));
    }
    source.revoked = true;
    source.enabled = false;
    source.revision = source.revision.checked_add(1).ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid("automation source policy revision overflow".to_owned())
    })?;
    source.updated_at_us = now;
    body.revision = body.revision.checked_add(1).ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid("automation source policy revision overflow".to_owned())
    })?;
    write_source_policy_body(root, &body)?;
    body.snapshot()
}

pub fn preview_automation_registration(
    root: &Path,
    proposal: AutomationRegistrationProposal,
) -> Result<AutomationRegistrationPreview> {
    validate_derived_root(root, false)?;
    reject_global_policy_mode(root)?;
    let body = read_registry_body(root)?;
    let snapshot = body.snapshot()?;
    let registration = validate_registration_proposal(root, &body, proposal, 1)?;
    Ok(AutomationRegistrationPreview {
        registry_revision: snapshot.revision,
        registry_digest_sha256: snapshot.digest_sha256,
        registration,
        would_write: true,
    })
}

pub fn apply_automation_registration(
    root: &Path,
    expected_revision: u64,
    expected_digest_sha256: &str,
    proposal: AutomationRegistrationProposal,
) -> Result<AutomationRegistrySnapshot> {
    validate_derived_root(root, true)?;
    let lock = open_registry_lock(root)?;
    fs4::FileExt::try_lock(&lock).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "automation registry is busy or cannot be exclusively locked: {error}"
        ))
    })?;
    reject_global_policy_mode(root)?;
    let mut body = read_registry_body(root)?;
    require_registry_basis(&body, expected_revision, expected_digest_sha256)?;
    if body
        .registrations
        .iter()
        .any(|item| item.registration_id == proposal.registration_id)
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation registration {:?} already exists",
            proposal.registration_id
        )));
    }
    if body
        .registrations
        .iter()
        .any(|item| !item.revoked && item.project_ref_id == proposal.project_ref_id)
    {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "ProjectRef {} already has an active automation registration",
            proposal.project_ref_id
        )));
    }
    let registration = validate_registration_proposal(root, &body, proposal, 1)?;
    body.registrations.push(registration);
    body.registrations
        .sort_by(|left, right| left.registration_id.cmp(&right.registration_id));
    body.revision = body.revision.checked_add(1).ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid("automation registry revision overflow".to_owned())
    })?;
    write_registry_body(root, &body)?;
    body.snapshot()
}

pub fn revoke_automation_registration(
    root: &Path,
    expected_revision: u64,
    expected_digest_sha256: &str,
    registration_id: &str,
) -> Result<AutomationRegistrySnapshot> {
    validate_derived_root(root, true)?;
    let lock = open_registry_lock(root)?;
    fs4::FileExt::try_lock(&lock).map_err(|error| {
        WorkVcsError::StorageFailure(format!(
            "automation registry is busy or cannot be exclusively locked: {error}"
        ))
    })?;
    let mut body = read_registry_body(root)?;
    require_registry_basis(&body, expected_revision, expected_digest_sha256)?;
    let now = current_epoch_micros()?;
    let registration = body
        .registrations
        .iter_mut()
        .find(|item| item.registration_id == registration_id)
        .ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "automation registration {registration_id:?} was not found"
            ))
        })?;
    if registration.revoked {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation registration {registration_id:?} is already revoked"
        )));
    }
    registration.revoked = true;
    registration.enabled = false;
    registration.revision = registration.revision.checked_add(1).ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid("automation registration revision overflow".to_owned())
    })?;
    registration.updated_at_us = now;
    body.revision = body.revision.checked_add(1).ok_or_else(|| {
        WorkVcsError::ControlPlaneInvalid("automation registry revision overflow".to_owned())
    })?;
    write_registry_body(root, &body)?;
    body.snapshot()
}

pub fn publish_automation_catalog(
    root: &Path,
    registry_revision: u64,
    registry_digest_sha256: String,
    sources: Vec<AutomationCatalogSource>,
) -> Result<AutomationCatalogPublication> {
    publish_catalog(
        root,
        CatalogBasis::Legacy {
            automation_registry_revision: registry_revision,
            automation_registry_digest_sha256: registry_digest_sha256,
        },
        sources,
    )
}

pub fn publish_registry_derived_automation_catalog(
    root: &Path,
    basis: RegistryDerivedCatalogBasis,
    sources: Vec<AutomationCatalogSource>,
) -> Result<AutomationCatalogPublication> {
    publish_catalog(root, CatalogBasis::RegistryDerived(basis), sources)
}

fn publish_catalog(
    root: &Path,
    basis: CatalogBasis,
    mut sources: Vec<AutomationCatalogSource>,
) -> Result<AutomationCatalogPublication> {
    validate_derived_root(root, true)?;
    basis.validate()?;
    for source in &sources {
        validate_catalog_source(source)?;
    }
    sources.sort_by(|left, right| {
        (
            left.lane,
            left.project_ref_id,
            &left.registration_id,
            left.artifact_kind,
            &left.source_anchor,
        )
            .cmp(&(
                right.lane,
                right.project_ref_id,
                &right.registration_id,
                right.artifact_kind,
                &right.source_anchor,
            ))
    });
    let complete = !sources.is_empty()
        && sources
            .iter()
            .all(|source| source.status == AutomationSourceStatus::Ready);
    let catalogs = root.join("catalogs");
    ensure_private_directory(&catalogs)?;
    let mut latest_for_basis: Option<(PathBuf, AutomationCatalogManifest)> = None;
    for entry in fs::read_dir(&catalogs).map_err(storage_error("read automation catalogs"))? {
        let entry = entry.map_err(storage_error("read automation catalog entry"))?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let catalog_path = entry.path();
        let existing = read_automation_catalog(&catalog_path)?;
        if !basis.matches_manifest(&existing) {
            continue;
        }
        let should_select = latest_for_basis
            .as_ref()
            .is_none_or(|(selected_path, selected)| {
                (existing.created_at_us, &catalog_path) > (selected.created_at_us, selected_path)
            });
        if should_select {
            latest_for_basis = Some((catalog_path, existing));
        }
    }
    let latest_created_at_us = latest_for_basis
        .as_ref()
        .map(|(_, manifest)| manifest.created_at_us);
    if let Some((catalog_path, existing)) = latest_for_basis
        && existing.complete == complete
        && existing.sources == sources
    {
        let manifest_sha256 = catalog_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                WorkVcsError::IntegrityInvalid(format!(
                    "automation catalog path {} has no UTF-8 digest name",
                    catalog_path.display()
                ))
            })?
            .to_owned();
        return Ok(AutomationCatalogPublication {
            outcome: "reused".to_owned(),
            catalog_path,
            manifest_sha256,
            manifest: existing,
        });
    }
    let manifest = basis.manifest(
        current_epoch_micros()?.max(latest_created_at_us.unwrap_or_default().saturating_add(1)),
        complete,
        sources,
    );
    let bytes = canonical_json(&manifest)?;
    let digest = sha256_hex(&bytes);
    let destination = catalogs.join(&digest);
    if destination.exists() {
        let existing = read_automation_catalog(&destination)?;
        if canonical_json(&existing)? != bytes {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "catalog path {} collides with different manifest bytes",
                destination.display()
            )));
        }
        return Ok(AutomationCatalogPublication {
            outcome: "reused".to_owned(),
            catalog_path: destination,
            manifest_sha256: digest,
            manifest,
        });
    }
    let temporary = catalogs.join(format!(
        ".{digest}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    create_private_directory_exclusive(&temporary)?;
    let publication_result = (|| {
        write_new_private_file(
            &temporary.join("manifest.json"),
            &[bytes.as_slice(), b"\n"].concat(),
        )?;
        sync_directory(&temporary)?;
        fs::rename(&temporary, &destination)
            .map_err(storage_error("publish automation catalog"))?;
        sync_directory(&catalogs)?;
        Ok(())
    })();
    if publication_result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    publication_result?;
    let readback = read_automation_catalog(&destination)?;
    if canonical_json(&readback)? != bytes {
        return Err(WorkVcsError::IntegrityInvalid(
            "published automation catalog failed exact readback".to_owned(),
        ));
    }
    Ok(AutomationCatalogPublication {
        outcome: "published".to_owned(),
        catalog_path: destination,
        manifest_sha256: digest,
        manifest,
    })
}

pub fn read_automation_catalog(path: &Path) -> Result<AutomationCatalogManifest> {
    validate_private_directory(path, "automation catalog")?;
    let bytes = read_private_file(&path.join("manifest.json"), MAX_CATALOG_BYTES)?;
    let manifest: AutomationCatalogManifest = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::IntegrityInvalid(format!(
            "automation catalog {} is invalid JSON: {error}",
            path.display()
        ))
    })?;
    match manifest.format_version {
        LEGACY_AUTOMATION_CATALOG_FORMAT_VERSION => {
            validate_sha256(
                "automation registry digest",
                &manifest.automation_registry_digest_sha256,
            )?;
            if manifest.project_registry_id.is_some()
                || manifest.project_registry_revision.is_some()
                || manifest.project_registry_digest.is_some()
                || manifest.source_policy_revision.is_some()
                || manifest.source_policy_digest_sha256.is_some()
            {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "legacy automation catalog {} carries registry-derived basis fields",
                    path.display()
                )));
            }
        }
        AUTOMATION_CATALOG_FORMAT_VERSION => {
            if manifest.automation_registry_revision != 0
                || manifest.automation_registry_digest_sha256 != LEGACY_REGISTRY_DIGEST_SENTINEL
            {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "registry-derived automation catalog {} has an invalid legacy-registry sentinel",
                    path.display()
                )));
            }
            let project_registry_id = manifest.project_registry_id.as_deref().ok_or_else(|| {
                WorkVcsError::IntegrityInvalid(format!(
                    "registry-derived automation catalog {} omits its project registry ID",
                    path.display()
                ))
            })?;
            let project_registry_digest =
                manifest.project_registry_digest.as_deref().ok_or_else(|| {
                    WorkVcsError::IntegrityInvalid(format!(
                        "registry-derived automation catalog {} omits its project registry digest",
                        path.display()
                    ))
                })?;
            let source_policy_digest =
                manifest
                    .source_policy_digest_sha256
                    .as_deref()
                    .ok_or_else(|| {
                        WorkVcsError::IntegrityInvalid(format!(
                            "registry-derived automation catalog {} omits its source policy digest",
                            path.display()
                        ))
                    })?;
            if manifest.project_registry_revision.is_none()
                || manifest.source_policy_revision.is_none()
            {
                return Err(WorkVcsError::IntegrityInvalid(format!(
                    "registry-derived automation catalog {} omits a required basis revision",
                    path.display()
                )));
            }
            RegistryId::parse_canonical(project_registry_id).map_err(|error| {
                WorkVcsError::IntegrityInvalid(format!(
                    "registry-derived automation catalog {} has an invalid project registry ID: {error}",
                    path.display()
                ))
            })?;
            validate_control_plane_digest(project_registry_digest)?;
            validate_sha256("automation source policy digest", source_policy_digest)?;
        }
        unsupported => {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "automation catalog {} has unsupported format version {unsupported}",
                path.display()
            )));
        }
    }
    for source in &manifest.sources {
        validate_catalog_source(source)?;
    }
    let expected_complete = !manifest.sources.is_empty()
        && manifest
            .sources
            .iter()
            .all(|source| source.status == AutomationSourceStatus::Ready);
    if manifest.complete != expected_complete {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "automation catalog {} has a forged completeness claim",
            path.display()
        )));
    }
    let expected_name = sha256_hex(&canonical_json(&manifest)?);
    if path.file_name().and_then(|value| value.to_str()) != Some(expected_name.as_str()) {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "automation catalog path {} does not match manifest digest {expected_name}",
            path.display()
        )));
    }
    Ok(manifest)
}

pub fn sha256_file(path: &Path, max_bytes: u64) -> Result<String> {
    Ok(sha256_hex(&read_regular_file(path, max_bytes)?))
}

fn validate_registration_proposal(
    root: &Path,
    body: &AutomationRegistryBody,
    proposal: AutomationRegistrationProposal,
    revision: u64,
) -> Result<AutomationRegistration> {
    if proposal.registration_id.is_empty()
        || proposal.registration_id.len() > 128
        || !proposal
            .registration_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_'))
    {
        return Err(WorkVcsError::QueryInvalid(
            "registration id must contain 1..=128 ASCII letters, digits, '-' or '_'".to_owned(),
        ));
    }
    validate_control_plane_digest(&proposal.registry_digest)?;
    let project_root = validate_source_root(&proposal.project_root, "project root")?;
    let store_path = validate_regular_path(&proposal.store_path, "Store path")?;
    let mut rollout_roots = proposal
        .rollout_roots
        .iter()
        .map(|path| validate_source_root(path, "rollout root"))
        .collect::<Result<Vec<_>>>()?;
    rollout_roots.sort();
    if rollout_roots.is_empty() {
        return Err(WorkVcsError::QueryInvalid(
            "automation registration requires at least one rollout root".to_owned(),
        ));
    }
    for (index, source) in rollout_roots.iter().enumerate() {
        for peer in rollout_roots.iter().skip(index + 1) {
            reject_overlap(source, peer, "rollout root", "rollout root")?;
        }
    }
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    for source in &rollout_roots {
        reject_overlap(source, &canonical_root, "rollout root", "automation root")?;
        reject_overlap(source, &store_path, "rollout root", "Store path")?;
        reject_overlap(source, &project_root, "rollout root", "project root")?;
        for existing in &body.registrations {
            if existing.revoked {
                continue;
            }
            for existing_root in &existing.rollout_roots {
                reject_overlap(
                    source,
                    existing_root,
                    "rollout root",
                    "registered rollout root",
                )?;
            }
        }
    }
    let now = current_epoch_micros()?;
    Ok(AutomationRegistration {
        registration_id: proposal.registration_id,
        revision,
        project_ref_id: proposal.project_ref_id,
        store_path,
        store_id: proposal.store_id,
        workspace_id: proposal.workspace_id,
        branch_id: proposal.branch_id,
        project_root,
        rollout_roots,
        role_policy: proposal.role_policy,
        registry_revision: proposal.registry_revision,
        registry_digest: proposal.registry_digest,
        enabled: true,
        revoked: false,
        created_at_us: now,
        updated_at_us: now,
    })
}

fn validate_source_policy_proposal(
    root: &Path,
    body: &AutomationSourcePolicyBody,
    proposal: AutomationSourcePolicyProposal,
    revision: u64,
) -> Result<AutomationSourcePolicyEntry> {
    validate_source_id(&proposal.source_id)?;
    let rollout_root = validate_source_root(&proposal.rollout_root, "global rollout root")?;
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    reject_overlap(
        &rollout_root,
        &canonical_root,
        "global rollout root",
        "automation root",
    )?;
    for existing in &body.sources {
        if existing.revoked {
            continue;
        }
        reject_overlap(
            &rollout_root,
            &existing.rollout_root,
            "global rollout root",
            "configured global rollout root",
        )?;
    }
    let mut excluded_project_refs = proposal.excluded_project_refs;
    excluded_project_refs.sort();
    if excluded_project_refs
        .windows(2)
        .any(|values| values[0] == values[1])
    {
        return Err(WorkVcsError::QueryInvalid(
            "global source policy exclusions must name each ProjectRef at most once".to_owned(),
        ));
    }
    let now = current_epoch_micros()?;
    Ok(AutomationSourcePolicyEntry {
        source_id: proposal.source_id,
        revision,
        rollout_root,
        role_policy: proposal.role_policy,
        excluded_project_refs,
        enabled: true,
        revoked: false,
        created_at_us: now,
        updated_at_us: now,
    })
}

fn validate_source_id(source_id: &str) -> Result<()> {
    if source_id.is_empty()
        || source_id.len() > 128
        || !source_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_'))
    {
        return Err(WorkVcsError::QueryInvalid(
            "global source policy id must contain 1..=128 ASCII letters, digits, '-' or '_'"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_source_policy_entry_shape(entry: &AutomationSourcePolicyEntry) -> Result<()> {
    validate_source_id(&entry.source_id)?;
    validate_absolute_normal_path(&entry.rollout_root, "global rollout root")?;
    let mut sorted = entry.excluded_project_refs.clone();
    sorted.sort();
    if sorted != entry.excluded_project_refs
        || sorted.windows(2).any(|values| values[0] == values[1])
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "global source policy exclusions are not canonically sorted and unique".to_owned(),
        ));
    }
    Ok(())
}

fn reject_legacy_registration_mode(root: &Path) -> Result<()> {
    let registrations = read_registry_body(root)?;
    if registrations
        .registrations
        .iter()
        .any(|entry| entry.enabled && !entry.revoked)
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "cannot configure a global source policy while active legacy registrations exist"
                .to_owned(),
        ));
    }
    Ok(())
}

fn reject_global_policy_mode(root: &Path) -> Result<()> {
    if automation_source_policy_path(root).exists() {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "cannot create a legacy registration after a global source policy is configured"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_catalog_source(source: &AutomationCatalogSource) -> Result<()> {
    if source.registration_id.is_empty() {
        return Err(WorkVcsError::QueryInvalid(
            "catalog source registration id cannot be empty".to_owned(),
        ));
    }
    validate_sha256("artifact manifest digest", &source.artifact_manifest_sha256)?;
    if !source.artifact_path.is_absolute() {
        return Err(WorkVcsError::QueryInvalid(
            "catalog artifact path must be absolute".to_owned(),
        ));
    }
    match source.status {
        AutomationSourceStatus::Ready => {
            if source.error_code.is_some() || source.error.is_some() {
                return Err(WorkVcsError::QueryInvalid(
                    "ready catalog source cannot carry an error".to_owned(),
                ));
            }
        }
        _ => {
            if source.error_code.as_deref().is_none_or(str::is_empty)
                || source.error.as_deref().is_none_or(str::is_empty)
            {
                return Err(WorkVcsError::QueryInvalid(
                    "non-ready catalog source requires an error code and message".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn require_registry_basis(
    body: &AutomationRegistryBody,
    expected_revision: u64,
    expected_digest_sha256: &str,
) -> Result<()> {
    validate_sha256(
        "expected automation registry digest",
        expected_digest_sha256,
    )?;
    let actual = body.snapshot()?;
    if actual.revision != expected_revision || actual.digest_sha256 != expected_digest_sha256 {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation registry basis changed: expected revision {expected_revision} digest {expected_digest_sha256}, actual revision {} digest {}",
            actual.revision, actual.digest_sha256
        )));
    }
    Ok(())
}

fn require_source_policy_basis(
    body: &AutomationSourcePolicyBody,
    expected_revision: u64,
    expected_digest_sha256: &str,
) -> Result<()> {
    validate_sha256(
        "expected automation source policy digest",
        expected_digest_sha256,
    )?;
    let actual = body.snapshot()?;
    if actual.revision != expected_revision || actual.digest_sha256 != expected_digest_sha256 {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation source policy basis changed: expected revision {expected_revision} digest {expected_digest_sha256}, actual revision {} digest {}",
            actual.revision, actual.digest_sha256
        )));
    }
    Ok(())
}

fn read_registry_body(root: &Path) -> Result<AutomationRegistryBody> {
    let path = automation_registry_path(root);
    if !path.exists() {
        return Ok(AutomationRegistryBody::empty());
    }
    let bytes = read_private_file(&path, MAX_REGISTRY_BYTES)?;
    let body: AutomationRegistryBody = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "automation registry {} is invalid JSON: {error}",
            path.display()
        ))
    })?;
    if body.format_version != AUTOMATION_REGISTRY_FORMAT_VERSION {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation registry {} has unsupported format version {}",
            path.display(),
            body.format_version
        )));
    }
    let mut sorted = body.registrations.clone();
    sorted.sort_by(|left, right| left.registration_id.cmp(&right.registration_id));
    if sorted != body.registrations {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "automation registry registrations are not canonically ordered".to_owned(),
        ));
    }
    Ok(body)
}

fn read_source_policy_body(root: &Path) -> Result<AutomationSourcePolicyBody> {
    let path = automation_source_policy_path(root);
    if !path.exists() {
        return Ok(AutomationSourcePolicyBody::empty());
    }
    let bytes = read_private_file(&path, MAX_SOURCE_POLICY_BYTES)?;
    let body: AutomationSourcePolicyBody = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!(
            "automation source policy {} is invalid JSON: {error}",
            path.display()
        ))
    })?;
    if body.format_version != AUTOMATION_SOURCE_POLICY_FORMAT_VERSION {
        return Err(WorkVcsError::ControlPlaneInvalid(format!(
            "automation source policy {} has unsupported format version {}",
            path.display(),
            body.format_version
        )));
    }
    let mut sorted = body.sources.clone();
    sorted.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    if sorted != body.sources {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "automation source policy entries are not canonically ordered".to_owned(),
        ));
    }
    for source in &body.sources {
        validate_source_policy_entry_shape(source)?;
    }
    Ok(body)
}

fn write_registry_body(root: &Path, body: &AutomationRegistryBody) -> Result<()> {
    let bytes = [canonical_json(body)?.as_slice(), b"\n"].concat();
    let destination = automation_registry_path(root);
    let temporary = root.join(format!(
        ".{REGISTRY_FILE}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    write_new_private_file(&temporary, &bytes)?;
    fs::rename(&temporary, &destination).map_err(storage_error("install automation registry"))?;
    sync_directory(root)
}

fn write_source_policy_body(root: &Path, body: &AutomationSourcePolicyBody) -> Result<()> {
    let bytes = [canonical_json(body)?.as_slice(), b"\n"].concat();
    let destination = automation_source_policy_path(root);
    let temporary = root.join(format!(
        ".{SOURCE_POLICY_FILE}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    write_new_private_file(&temporary, &bytes)?;
    fs::rename(&temporary, &destination)
        .map_err(storage_error("install automation source policy"))?;
    sync_directory(root)
}

fn validate_derived_root(root: &Path, create: bool) -> Result<()> {
    validate_absolute_normal_path(root, "automation root")?;
    if create {
        ensure_private_directory(root)
    } else if root.exists() {
        validate_private_directory(root, "automation root")
    } else {
        Ok(())
    }
}

fn open_registry_lock(root: &Path) -> Result<File> {
    let path = root.join(REGISTRY_LOCK_FILE);
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options
        .open(&path)
        .map_err(storage_error("open automation registry lock"))?;
    validate_private_metadata(
        &file
            .metadata()
            .map_err(storage_error("stat automation registry lock"))?,
        false,
        "automation registry lock",
    )?;
    Ok(file)
}

fn validate_source_root(path: &Path, label: &str) -> Result<PathBuf> {
    validate_absolute_normal_path(path, label)?;
    let metadata = fs::symlink_metadata(path).map_err(storage_error("inspect source root"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} {} must be a real directory",
            path.display()
        )));
    }
    validate_owner_and_writes(&metadata, label)?;
    path.canonicalize()
        .map_err(storage_error("canonicalize source root"))
}

fn validate_regular_path(path: &Path, label: &str) -> Result<PathBuf> {
    validate_absolute_normal_path(path, label)?;
    let metadata = fs::symlink_metadata(path).map_err(storage_error("inspect regular path"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} {} must be a real regular file",
            path.display()
        )));
    }
    validate_owner_and_writes(&metadata, label)?;
    path.canonicalize()
        .map_err(storage_error("canonicalize regular path"))
}

fn reject_overlap(left: &Path, right: &Path, left_label: &str, right_label: &str) -> Result<()> {
    if left == right || left.starts_with(right) || right.starts_with(left) {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{left_label} {} overlaps {right_label} {}",
            left.display(),
            right.display()
        )));
    }
    Ok(())
}

fn validate_absolute_normal_path(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::RootDir
            ) && !matches!(component, Component::RootDir)
        })
    {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be an absolute normalized path: {}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_private_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        builder
            .recursive(true)
            .create(path)
            .map_err(storage_error("create private derived directory"))?;
    }
    validate_private_directory(path, "private derived directory")
}

fn create_private_directory_exclusive(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder
        .create(path)
        .map_err(storage_error("create private temporary directory"))?;
    Ok(())
}

fn validate_private_directory(path: &Path, label: &str) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(storage_error("inspect private directory"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} {} is not a real directory",
            path.display()
        )));
    }
    validate_private_metadata(&metadata, true, label)
}

fn validate_private_metadata(metadata: &fs::Metadata, directory: bool, label: &str) -> Result<()> {
    let expected_type = if directory {
        metadata.is_dir()
    } else {
        metadata.is_file()
    };
    if !expected_type {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "{label} has an unexpected filesystem type"
        )));
    }
    #[cfg(unix)]
    {
        if metadata.uid() != unsafe { libc::geteuid() } {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "{label} is not owned by the current user"
            )));
        }
        if metadata.mode() & 0o077 != 0 {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "{label} is not owner-private"
            )));
        }
    }
    Ok(())
}

fn validate_owner_and_writes(metadata: &fs::Metadata, label: &str) -> Result<()> {
    #[cfg(unix)]
    {
        if metadata.uid() != unsafe { libc::geteuid() } {
            return Err(WorkVcsError::QueryInvalid(format!(
                "{label} is not owned by the current user"
            )));
        }
        if metadata.mode() & 0o022 != 0 {
            return Err(WorkVcsError::QueryInvalid(format!(
                "{label} is group/world writable"
            )));
        }
    }
    Ok(())
}

fn read_private_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    #[cfg(unix)]
    {
        read_private_file_unix(path, max_bytes)
    }
    #[cfg(not(unix))]
    {
        let mut options = OpenOptions::new();
        options.read(true);
        let mut file = options
            .open(path)
            .map_err(storage_error("open private file"))?;
        let metadata = file
            .metadata()
            .map_err(storage_error("inspect private file"))?;
        validate_private_metadata(&metadata, false, "private file")?;
        if metadata.len() > max_bytes {
            return Err(WorkVcsError::IntegrityInvalid(format!(
                "private file {} exceeds its byte bound",
                path.display()
            )));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut bytes)
            .map_err(storage_error("read private file"))?;
        Ok(bytes)
    }
}

#[cfg(unix)]
fn read_private_file_unix(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    validate_absolute_normal_path(path, "private file")?;
    let parent = path.parent().ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!(
            "private file {} has no parent directory",
            path.display()
        ))
    })?;
    let name = path.file_name().ok_or_else(|| {
        WorkVcsError::IntegrityInvalid(format!("private file {} has no name", path.display()))
    })?;
    let directory = open_private_directory_without_symlinks(parent)?;
    let expected = fstatat_identity(directory.as_raw_fd(), name, path)?;
    let name = CString::new(name.as_bytes()).map_err(|_| {
        WorkVcsError::IntegrityInvalid(format!(
            "private file {} contains a NUL byte",
            path.display()
        ))
    })?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(storage_error("open private file")(
            std::io::Error::last_os_error(),
        ));
    }
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    let metadata = file
        .metadata()
        .map_err(storage_error("inspect private file"))?;
    validate_private_metadata(&metadata, false, "private file")?;
    let opened = file_identity(&metadata);
    if opened != expected || metadata.len() > max_bytes {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "private file {} has an unsafe identity or exceeds its byte bound",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .map_err(storage_error("read private file"))?;
    let after = file
        .metadata()
        .map_err(storage_error("reinspect private file"))?;
    if file_identity(&after) != opened || bytes.len() as u64 != metadata.len() {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "private file {} changed while it was read",
            path.display()
        )));
    }
    Ok(bytes)
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PrivateFileIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
fn file_identity(metadata: &fs::Metadata) -> PrivateFileIdentity {
    PrivateFileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        uid: metadata.uid(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(unix)]
fn fstatat_identity(directory: i32, name: &OsStr, path: &Path) -> Result<PrivateFileIdentity> {
    let name = CString::new(name.as_bytes()).map_err(|_| {
        WorkVcsError::IntegrityInvalid(format!(
            "private file {} contains a NUL byte",
            path.display()
        ))
    })?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(storage_error("inspect private file")(
            std::io::Error::last_os_error(),
        ));
    }
    let stat = unsafe { stat.assume_init() };
    // Darwin dev_t/mode_t and Linux dev_t/mode_t have different integer widths.
    #[allow(clippy::unnecessary_cast)]
    let device = stat.st_dev as u64;
    #[allow(clippy::useless_conversion)]
    let mode = u32::from(stat.st_mode);
    Ok(PrivateFileIdentity {
        device,
        inode: stat.st_ino,
        mode,
        uid: stat.st_uid,
        length: stat.st_size as u64,
        modified_seconds: stat_modified_seconds(&stat),
        modified_nanoseconds: stat_modified_nanoseconds(&stat),
        changed_seconds: stat_changed_seconds(&stat),
        changed_nanoseconds: stat_changed_nanoseconds(&stat),
    })
}

#[cfg(unix)]
fn stat_modified_seconds(stat: &libc::stat) -> i64 {
    stat.st_mtime
}

#[cfg(unix)]
fn stat_modified_nanoseconds(stat: &libc::stat) -> i64 {
    stat.st_mtime_nsec
}

#[cfg(unix)]
fn stat_changed_seconds(stat: &libc::stat) -> i64 {
    stat.st_ctime
}

#[cfg(unix)]
fn stat_changed_nanoseconds(stat: &libc::stat) -> i64 {
    stat.st_ctime_nsec
}

#[cfg(unix)]
fn open_private_directory_without_symlinks(path: &Path) -> Result<File> {
    validate_absolute_normal_path(path, "private directory")?;
    let root = CString::new("/").expect("root path contains no NUL");
    let mut descriptor = unsafe {
        libc::open(
            root.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(storage_error("open filesystem root")(
            std::io::Error::last_os_error(),
        ));
    }
    for component in path.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        let name = CString::new(name.as_bytes()).map_err(|_| {
            WorkVcsError::IntegrityInvalid(format!(
                "private directory {} contains a NUL byte",
                path.display()
            ))
        })?;
        let next = unsafe {
            libc::openat(
                descriptor,
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
            )
        };
        if next < 0 {
            let error = std::io::Error::last_os_error();
            unsafe { libc::close(descriptor) };
            return Err(storage_error("open private directory component")(error));
        }
        unsafe { libc::close(descriptor) };
        descriptor = next;
    }
    let directory = unsafe { File::from_raw_fd(descriptor) };
    validate_private_metadata(
        &directory
            .metadata()
            .map_err(storage_error("inspect private directory"))?,
        true,
        "private directory",
    )?;
    Ok(directory)
}

fn read_regular_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(storage_error("inspect file"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > max_bytes {
        return Err(WorkVcsError::IntegrityInvalid(format!(
            "file {} has an unsafe type or exceeds its byte bound",
            path.display()
        )));
    }
    fs::read(path).map_err(storage_error("read file"))
}

fn write_new_private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(path)
        .map_err(storage_error("create private file"))?;
    file.write_all(bytes)
        .map_err(storage_error("write private file"))?;
    file.sync_all().map_err(storage_error("sync private file"))
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(storage_error("sync directory"))
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!(
            "cannot encode automation value canonically: {error}"
        ))
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn validate_sha256(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be lowercase SHA-256"
        )));
    }
    Ok(())
}

fn validate_control_plane_digest(value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("blake3-256:") else {
        return Err(WorkVcsError::QueryInvalid(
            "ProjectRef registry digest must use the blake3-256 prefix".to_owned(),
        ));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        return Err(WorkVcsError::QueryInvalid(
            "ProjectRef registry digest must contain lowercase BLAKE3-256 hex".to_owned(),
        ));
    }
    Ok(())
}

fn storage_error(context: &'static str) -> impl Fn(std::io::Error) -> WorkVcsError {
    move |error| WorkVcsError::StorageFailure(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{BranchId, ProjectRefId, StoreId, WorkspaceId};
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf, AutomationRegistrationProposal) {
        let temporary = TempDir::new().unwrap();
        let temporary_root = temporary.path().canonicalize().unwrap();
        let root = temporary_root.join("automation");
        let project = temporary_root.join("project");
        let rollout = temporary_root.join("rollouts");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&rollout).unwrap();
        let store = temporary_root.join("store.sqlite");
        fs::write(&store, b"store").unwrap();
        let proposal = AutomationRegistrationProposal {
            registration_id: "registration-1".to_owned(),
            project_ref_id: ProjectRefId::new_v7(),
            store_path: store,
            store_id: StoreId::new_v7(),
            workspace_id: WorkspaceId::new_v7(),
            branch_id: BranchId::new_v7(),
            project_root: project,
            rollout_roots: vec![rollout],
            role_policy: AutomationRolePolicy::UserOnly,
            registry_revision: 7,
            registry_digest: format!("blake3-256:{}", "1".repeat(64)),
        };
        (temporary, root, proposal)
    }

    #[test]
    fn preview_is_read_only_and_apply_requires_exact_basis() {
        let (_temporary, root, proposal) = fixture();
        let preview = preview_automation_registration(&root, proposal.clone()).unwrap();
        assert_eq!(preview.registry_revision, 0);
        assert!(!root.exists());
        let applied = apply_automation_registration(
            &root,
            preview.registry_revision,
            &preview.registry_digest_sha256,
            proposal,
        )
        .unwrap();
        assert_eq!(applied.revision, 1);
        assert_eq!(applied.registrations.len(), 1);
        let error = revoke_automation_registration(
            &root,
            0,
            &preview.registry_digest_sha256,
            "registration-1",
        )
        .unwrap_err();
        assert!(matches!(error, WorkVcsError::ControlPlaneInvalid(_)));
    }

    #[test]
    fn revoke_preserves_registration_history() {
        let (_temporary, root, proposal) = fixture();
        let preview = preview_automation_registration(&root, proposal.clone()).unwrap();
        let applied = apply_automation_registration(
            &root,
            preview.registry_revision,
            &preview.registry_digest_sha256,
            proposal,
        )
        .unwrap();
        let revoked = revoke_automation_registration(
            &root,
            applied.revision,
            &applied.digest_sha256,
            "registration-1",
        )
        .unwrap();
        assert_eq!(revoked.revision, 2);
        assert!(revoked.registrations[0].revoked);
        assert!(!revoked.registrations[0].enabled);
    }

    #[test]
    fn registration_rejects_overlapping_rollout_and_project_roots() {
        let (_temporary, root, mut proposal) = fixture();
        proposal.rollout_roots = vec![proposal.project_root.join("rollouts")];
        fs::create_dir(&proposal.rollout_roots[0]).unwrap();
        let error = preview_automation_registration(&root, proposal).unwrap_err();
        assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
    }

    #[test]
    fn registration_rejects_parent_and_child_rollout_roots() {
        let (_temporary, root, mut proposal) = fixture();
        let child = proposal.rollout_roots[0].join("nested");
        fs::create_dir(&child).unwrap();
        proposal.rollout_roots.push(child);
        let error = preview_automation_registration(&root, proposal).unwrap_err();
        assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
        assert!(error.to_string().contains("rollout root"));
        assert!(error.to_string().contains("overlaps"));
    }

    #[test]
    fn registration_rejects_duplicate_rollout_roots() {
        let (_temporary, root, mut proposal) = fixture();
        proposal
            .rollout_roots
            .push(proposal.rollout_roots[0].clone());
        let error = preview_automation_registration(&root, proposal).unwrap_err();
        assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
        assert!(error.to_string().contains("overlaps"));
    }

    #[test]
    fn registration_accepts_distinct_sibling_rollout_roots() {
        let (temporary, root, mut proposal) = fixture();
        let sibling = temporary.path().join("other-rollouts");
        fs::create_dir(&sibling).unwrap();
        proposal.rollout_roots.push(sibling.canonicalize().unwrap());
        let preview = preview_automation_registration(&root, proposal).unwrap();
        assert_eq!(preview.registration.rollout_roots.len(), 2);
    }

    #[test]
    fn global_source_policy_is_cas_guarded_and_excludes_legacy_registration_mode() {
        let (_temporary, root, registration) = fixture();
        let source = AutomationSourcePolicyProposal {
            source_id: "codex-sessions".to_owned(),
            rollout_root: registration.rollout_roots[0].clone(),
            role_policy: AutomationRolePolicy::UserOnly,
            excluded_project_refs: vec![ProjectRefId::new_v7()],
        };
        let preview = preview_automation_source_policy(&root, source.clone()).unwrap();
        assert_eq!(preview.policy_revision, 0);
        assert!(!root.exists());
        let applied = apply_automation_source_policy(
            &root,
            preview.policy_revision,
            &preview.policy_digest_sha256,
            source,
        )
        .unwrap();
        assert_eq!(applied.revision, 1);
        assert_eq!(applied.sources.len(), 1);
        assert_eq!(
            read_automation_source_policy(&root).unwrap(),
            applied,
            "source policy readback must preserve its exact CAS result"
        );
        let error = preview_automation_registration(&root, registration).unwrap_err();
        assert!(matches!(error, WorkVcsError::ControlPlaneInvalid(_)));
        assert!(error.to_string().contains("global source policy"));
    }

    #[test]
    fn global_source_policy_rejects_duplicate_exclusions_and_overlapping_roots() {
        let (_temporary, root, registration) = fixture();
        let duplicate = ProjectRefId::new_v7();
        let invalid = AutomationSourcePolicyProposal {
            source_id: "source-a".to_owned(),
            rollout_root: registration.rollout_roots[0].clone(),
            role_policy: AutomationRolePolicy::UserOnly,
            excluded_project_refs: vec![duplicate, duplicate],
        };
        let error = preview_automation_source_policy(&root, invalid).unwrap_err();
        assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
        assert!(error.to_string().contains("at most once"));

        let first = AutomationSourcePolicyProposal {
            source_id: "source-a".to_owned(),
            rollout_root: registration.rollout_roots[0].clone(),
            role_policy: AutomationRolePolicy::UserOnly,
            excluded_project_refs: Vec::new(),
        };
        let preview = preview_automation_source_policy(&root, first.clone()).unwrap();
        let applied = apply_automation_source_policy(
            &root,
            preview.policy_revision,
            &preview.policy_digest_sha256,
            first,
        )
        .unwrap();
        let nested = registration.rollout_roots[0].join("nested");
        fs::create_dir(&nested).unwrap();
        let overlap = AutomationSourcePolicyProposal {
            source_id: "source-b".to_owned(),
            rollout_root: nested,
            role_policy: AutomationRolePolicy::UserAndAssistant,
            excluded_project_refs: Vec::new(),
        };
        let error = apply_automation_source_policy(
            &root,
            applied.revision,
            &applied.digest_sha256,
            overlap,
        )
        .unwrap_err();
        assert!(matches!(error, WorkVcsError::QueryInvalid(_)));
        assert!(error.to_string().contains("overlaps"));
    }

    #[cfg(unix)]
    #[test]
    fn source_policy_reader_rejects_fifo_leaf_and_intermediate_symlinks() {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::{PermissionsExt, symlink};

        let temporary = TempDir::new().unwrap();
        let temporary_root = temporary.path().canonicalize().unwrap();
        let root = temporary_root.join("automation");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let policy = automation_source_policy_path(&root);
        let fifo = CString::new(policy.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        let started = std::time::Instant::now();
        let error = read_automation_source_policy(&root).unwrap_err();
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert!(
            matches!(error, WorkVcsError::IntegrityInvalid(_)),
            "{error:?}"
        );

        fs::remove_file(&policy).unwrap();
        let outside = temporary_root.join("outside-policy.json");
        fs::write(
            &outside,
            br#"{"format_version":1,"revision":0,"sources":[]}"#,
        )
        .unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
        symlink(&outside, &policy).unwrap();
        assert!(read_automation_source_policy(&root).is_err());

        fs::remove_file(&policy).unwrap();
        let real_parent = temporary_root.join("real-parent");
        let real_root = real_parent.join("automation");
        fs::create_dir_all(&real_root).unwrap();
        fs::set_permissions(&real_parent, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&real_root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(
            automation_source_policy_path(&real_root),
            fs::read(&outside).unwrap(),
        )
        .unwrap();
        fs::set_permissions(
            automation_source_policy_path(&real_root),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let alias_parent = temporary_root.join("alias-parent");
        symlink(&real_parent, &alias_parent).unwrap();
        assert!(read_automation_source_policy(&alias_parent.join("automation")).is_err());
    }

    #[test]
    fn global_source_policy_rejects_active_legacy_registration() {
        let (_temporary, root, registration) = fixture();
        let preview = preview_automation_registration(&root, registration.clone()).unwrap();
        apply_automation_registration(
            &root,
            preview.registry_revision,
            &preview.registry_digest_sha256,
            registration.clone(),
        )
        .unwrap();
        let source = AutomationSourcePolicyProposal {
            source_id: "codex-sessions".to_owned(),
            rollout_root: registration.rollout_roots[0].clone(),
            role_policy: AutomationRolePolicy::UserOnly,
            excluded_project_refs: Vec::new(),
        };
        let error = preview_automation_source_policy(&root, source).unwrap_err();
        assert!(matches!(error, WorkVcsError::ControlPlaneInvalid(_)));
        assert!(error.to_string().contains("active legacy registrations"));
    }

    #[test]
    fn catalog_is_immutable_and_partial_failure_is_visible() {
        let (_temporary, root, proposal) = fixture();
        let preview = preview_automation_registration(&root, proposal.clone()).unwrap();
        let registry = apply_automation_registration(
            &root,
            preview.registry_revision,
            &preview.registry_digest_sha256,
            proposal.clone(),
        )
        .unwrap();
        let source = AutomationCatalogSource {
            registration_id: proposal.registration_id,
            project_ref_id: proposal.project_ref_id,
            lane: AutomationLane::HistoricalReference,
            artifact_kind: AutomationArtifactKind::SourceArchive,
            artifact_path: root.join("missing-source"),
            artifact_manifest_sha256: "2".repeat(64),
            source_anchor: "rollout:missing".to_owned(),
            status: AutomationSourceStatus::Failed,
            error_code: Some("source_missing".to_owned()),
            error: Some("registered source is missing".to_owned()),
        };
        let first = publish_automation_catalog(
            &root,
            registry.revision,
            registry.digest_sha256.clone(),
            vec![source],
        )
        .unwrap();
        assert_eq!(first.outcome, "published");
        assert!(!first.manifest.complete);
        let readback = read_automation_catalog(&first.catalog_path).unwrap();
        assert_eq!(readback, first.manifest);

        let reused = publish_automation_catalog(
            &root,
            registry.revision,
            registry.digest_sha256.clone(),
            first.manifest.sources.clone(),
        )
        .unwrap();
        assert_eq!(reused.outcome, "reused");
        assert_eq!(reused.catalog_path, first.catalog_path);
        assert_eq!(reused.manifest, first.manifest);

        let mut ready_source = first.manifest.sources[0].clone();
        ready_source.status = AutomationSourceStatus::Ready;
        ready_source.error_code = None;
        ready_source.error = None;
        let changed = publish_automation_catalog(
            &root,
            registry.revision,
            registry.digest_sha256.clone(),
            vec![ready_source],
        )
        .unwrap();
        assert_eq!(changed.outcome, "published");
        assert!(changed.manifest.complete);

        let returned = publish_automation_catalog(
            &root,
            registry.revision,
            registry.digest_sha256.clone(),
            first.manifest.sources.clone(),
        )
        .unwrap();
        assert_eq!(returned.outcome, "published");
        assert_ne!(returned.catalog_path, first.catalog_path);
        let stable = publish_automation_catalog(
            &root,
            registry.revision,
            registry.digest_sha256,
            first.manifest.sources.clone(),
        )
        .unwrap();
        assert_eq!(stable.outcome, "reused");
        assert_eq!(stable.catalog_path, returned.catalog_path);
    }

    #[test]
    fn registry_derived_catalog_records_both_immutable_control_plane_bases() {
        let (_temporary, root, proposal) = fixture();
        let source = AutomationCatalogSource {
            registration_id: format!("project-ref:{}", proposal.project_ref_id),
            project_ref_id: proposal.project_ref_id,
            lane: AutomationLane::HistoricalReference,
            artifact_kind: AutomationArtifactKind::SourceArchive,
            artifact_path: root.join("missing-source"),
            artifact_manifest_sha256: "2".repeat(64),
            source_anchor: "rollout:missing".to_owned(),
            status: AutomationSourceStatus::Failed,
            error_code: Some("source_missing".to_owned()),
            error: Some("configured source is missing".to_owned()),
        };
        let basis = RegistryDerivedCatalogBasis {
            project_registry_id: RegistryId::new_v7().to_string(),
            project_registry_revision: 7,
            project_registry_digest: format!("blake3-256:{}", "3".repeat(64)),
            source_policy_revision: 11,
            source_policy_digest_sha256: "4".repeat(64),
        };
        let first =
            publish_registry_derived_automation_catalog(&root, basis.clone(), vec![source.clone()])
                .unwrap();
        assert_eq!(
            first.manifest.format_version,
            AUTOMATION_CATALOG_FORMAT_VERSION
        );
        assert_eq!(
            first.manifest.project_registry_id.as_deref(),
            Some(basis.project_registry_id.as_str())
        );
        assert_eq!(first.manifest.project_registry_revision, Some(7));
        assert_eq!(
            first.manifest.project_registry_digest.as_deref(),
            Some(basis.project_registry_digest.as_str())
        );
        assert_eq!(first.manifest.source_policy_revision, Some(11));
        assert_eq!(
            first.manifest.source_policy_digest_sha256.as_deref(),
            Some(basis.source_policy_digest_sha256.as_str())
        );
        let reused =
            publish_registry_derived_automation_catalog(&root, basis.clone(), vec![source])
                .unwrap();
        assert_eq!(reused.outcome, "reused");
        assert_eq!(reused.catalog_path, first.catalog_path);

        let changed_basis = RegistryDerivedCatalogBasis {
            source_policy_revision: 12,
            ..basis
        };
        let changed = publish_registry_derived_automation_catalog(
            &root,
            changed_basis,
            first.manifest.sources.clone(),
        )
        .unwrap();
        assert_eq!(changed.outcome, "published");
        assert_ne!(changed.catalog_path, first.catalog_path);

        let mut missing_registry_id = first.manifest.clone();
        missing_registry_id.project_registry_id = None;
        let bytes = canonical_json(&missing_registry_id).unwrap();
        let destination = root.join("catalogs").join(sha256_hex(&bytes));
        create_private_directory_exclusive(&destination).unwrap();
        write_new_private_file(
            &destination.join("manifest.json"),
            &[bytes.as_slice(), b"\n"].concat(),
        )
        .unwrap();
        let error = read_automation_catalog(&destination).unwrap_err();
        assert!(error.to_string().contains("omits its project registry ID"));
    }

    #[test]
    fn empty_catalog_is_incomplete_and_rejects_vacuous_completion() {
        let (_temporary, root, _proposal) = fixture();
        let published = publish_automation_catalog(&root, 0, "1".repeat(64), Vec::new()).unwrap();
        assert!(!published.manifest.complete);
        assert!(published.manifest.sources.is_empty());
        assert_eq!(
            read_automation_catalog(&published.catalog_path).unwrap(),
            published.manifest
        );

        let mut forged = published.manifest;
        forged.complete = true;
        forged.created_at_us += 1;
        let bytes = canonical_json(&forged).unwrap();
        let destination = root.join("catalogs").join(sha256_hex(&bytes));
        create_private_directory_exclusive(&destination).unwrap();
        write_new_private_file(
            &destination.join("manifest.json"),
            &[bytes.as_slice(), b"\n"].concat(),
        )
        .unwrap();
        let error = read_automation_catalog(&destination).unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));
        assert!(error.to_string().contains("forged completeness"));
    }

    #[test]
    fn catalog_readback_rejects_forged_complete_flag() {
        let (_temporary, root, proposal) = fixture();
        let source = AutomationCatalogSource {
            registration_id: proposal.registration_id,
            project_ref_id: proposal.project_ref_id,
            lane: AutomationLane::HistoricalReference,
            artifact_kind: AutomationArtifactKind::SourceArchive,
            artifact_path: root.join("missing-source"),
            artifact_manifest_sha256: "2".repeat(64),
            source_anchor: "rollout:missing".to_owned(),
            status: AutomationSourceStatus::Failed,
            error_code: Some("source_missing".to_owned()),
            error: Some("registered source is missing".to_owned()),
        };
        let forged = AutomationCatalogManifest {
            format_version: LEGACY_AUTOMATION_CATALOG_FORMAT_VERSION,
            automation_registry_revision: 0,
            automation_registry_digest_sha256: "1".repeat(64),
            project_registry_id: None,
            project_registry_revision: None,
            project_registry_digest: None,
            source_policy_revision: None,
            source_policy_digest_sha256: None,
            created_at_us: 1,
            complete: true,
            sources: vec![source],
        };
        let bytes = canonical_json(&forged).unwrap();
        let catalogs = root.join("catalogs");
        ensure_private_directory(&catalogs).unwrap();
        let destination = catalogs.join(sha256_hex(&bytes));
        create_private_directory_exclusive(&destination).unwrap();
        write_new_private_file(
            &destination.join("manifest.json"),
            &[bytes.as_slice(), b"\n"].concat(),
        )
        .unwrap();
        let error = read_automation_catalog(&destination).unwrap_err();
        assert!(matches!(error, WorkVcsError::IntegrityInvalid(_)));
        assert!(error.to_string().contains("forged completeness"));
    }
}
