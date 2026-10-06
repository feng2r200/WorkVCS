use crate::canonical::{canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError};
use crate::{
    BranchId, CaptureGroupId, Digest, ProjectLinkId, ProjectLocatorId, ProjectRefId, RegistryId,
    RegistryObservationId, StoreId, WorkspaceId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path};
use std::time::{SystemTime, UNIX_EPOCH};

const REGISTRY_VERSION: u64 = 2;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControlPlaneDigest(Digest);

impl ControlPlaneDigest {
    pub fn raw(bytes: &[u8]) -> Self {
        Self(Digest::raw(bytes))
    }

    pub fn from_text(value: &str) -> Result<Self> {
        let hex = value.strip_prefix("blake3-256:").ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(
                "control-plane digest must use the blake3-256 prefix".to_owned(),
            )
        })?;
        Digest::from_hex(hex).map(Self).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!("invalid control-plane digest: {error}"))
        })
    }

    pub fn digest(&self) -> Digest {
        self.0
    }
}

impl fmt::Display for ControlPlaneDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "blake3-256:{}", self.0)
    }
}

impl Serialize for ControlPlaneDigest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ControlPlaneDigest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_text(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct UtcTimestamp(String);

impl UtcTimestamp {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_utc_timestamp(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn now() -> Result<Self> {
        Self::from_system_time(SystemTime::now())
    }

    pub fn from_system_time(value: SystemTime) -> Result<Self> {
        let elapsed = value.duration_since(UNIX_EPOCH).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "timestamp is before the UNIX epoch: {error}"
            ))
        })?;
        let days = elapsed.as_secs() / 86_400;
        let seconds_in_day = elapsed.as_secs() % 86_400;
        let days = i64::try_from(days).map_err(|_| {
            WorkVcsError::ControlPlaneInvalid(
                "timestamp exceeds the supported calendar range".to_owned(),
            )
        })?;
        let (year, month, day) = civil_date_from_unix_days(days);
        if !(1..=9_999).contains(&year) {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "timestamp year is outside the supported four-digit range".to_owned(),
            ));
        }
        let hour = seconds_in_day / 3_600;
        let minute = (seconds_in_day % 3_600) / 60;
        let second = seconds_in_day % 60;
        let nanos = elapsed.subsec_nanos();
        let text = if nanos == 0 {
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
        } else {
            let fraction = format!("{nanos:09}");
            let fraction = fraction.trim_end_matches('0');
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction}Z")
        };
        Self::parse(text)
    }
}

fn civil_date_from_unix_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

impl<'de> Deserialize<'de> for UtcTimestamp {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

fn validate_utc_timestamp(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let valid_shape = bytes.len() >= 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes.last() == Some(&b'Z')
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && bytes[11..13].iter().all(u8::is_ascii_digit)
        && bytes[14..16].iter().all(u8::is_ascii_digit)
        && bytes[17..19].iter().all(u8::is_ascii_digit);
    if !valid_shape {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "timestamp must be RFC 3339 UTC with an explicit Z".to_owned(),
        ));
    }
    if bytes.len() > 20
        && (bytes[19] != b'.'
            || bytes[20..bytes.len() - 1].is_empty()
            || !bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit))
    {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "timestamp fractional seconds must contain only digits".to_owned(),
        ));
    }
    if bytes.len() > 20 && bytes[20..bytes.len() - 1].len() > 9 {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "timestamp fractional seconds must not exceed nanosecond precision".to_owned(),
        ));
    }
    let year = parse_decimal(&bytes[..4]);
    let month = parse_decimal(&bytes[5..7]);
    let day = parse_decimal(&bytes[8..10]);
    let hour = parse_decimal(&bytes[11..13]);
    let minute = parse_decimal(&bytes[14..16]);
    let second = parse_decimal(&bytes[17..19]);
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 60 {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "timestamp contains an out-of-range date or time component".to_owned(),
        ));
    }
    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err(WorkVcsError::ControlPlaneInvalid(
            "timestamp contains an invalid calendar date".to_owned(),
        ));
    }
    Ok(())
}

fn parse_decimal(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0_u32, |value, byte| value * 10 + u32::from(byte - b'0'))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct CanonicalPath(String);

impl CanonicalPath {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let path = Path::new(&value);
        if value.is_empty() || !path.is_absolute() {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "canonical path must be absolute".to_owned(),
            ));
        }
        if value.contains('\0')
            || path
                .components()
                .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        {
            return Err(WorkVcsError::ControlPlaneInvalid(
                "canonical path must not contain NUL, dot, or parent components".to_owned(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl<'de> Deserialize<'de> for CanonicalPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub(crate) struct NonEmptyString(String);

impl NonEmptyString {
    pub(crate) fn new(value: impl Into<String>, label: &str) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(WorkVcsError::ControlPlaneInvalid(format!(
                "{label} must contain at least one non-whitespace character"
            )));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NonEmptyString {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value, "string").map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Nullable<T> {
    value: Option<T>,
    present: bool,
}

impl<T> Nullable<T> {
    pub(crate) fn present(value: Option<T>) -> Self {
        Self {
            value,
            present: true,
        }
    }

    pub(crate) fn as_ref(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub(crate) fn is_present(&self) -> bool {
        self.present
    }
}

impl<T: Copy> Nullable<T> {
    pub(crate) fn copied(&self) -> Option<T> {
        self.value
    }
}

impl<T> Default for Nullable<T> {
    fn default() -> Self {
        Self {
            value: None,
            present: false,
        }
    }
}

impl<T: Serialize> Serialize for Nullable<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Nullable<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| Self {
            value,
            present: true,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMaturity {
    Provisional,
    Established,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectCreatedBy {
    Explicit,
    Migration,
    SemanticLocator,
    RepositoryFirstWrite,
    CwdFirstWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocatorRole {
    Identity,
    Context,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocatorAuthority {
    SemanticProject,
    Repository,
    Cwd,
}

impl LocatorAuthority {
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::SemanticProject => 1,
            Self::Repository => 2,
            Self::Cwd => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocatorAssurance {
    Authoritative,
    VerifiedDerived,
    Observed,
}

impl LocatorAssurance {
    pub(crate) const fn priority(self) -> u8 {
        match self {
            Self::Authoritative => 2,
            Self::VerifiedDerived => 1,
            Self::Observed => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocatorState {
    Active,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingSource {
    Explicit,
    Migration,
    FirstWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLinkRelation {
    ArtifactRepository,
    ExecutionContext,
    RelatedWork,
    SameLogicalProject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLinkStatus {
    Candidate,
    Confirmed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLinkBasis {
    ExplicitUser,
    AdapterEvidence,
    CaptureGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryObservationKind {
    PossibleSharedTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryObservationSource {
    Migration,
    Audit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectRef {
    pub(crate) project_ref_id: ProjectRefId,
    pub(crate) maturity: ProjectMaturity,
    #[serde(default)]
    display_name: Nullable<NonEmptyString>,
    created_at: UtcTimestamp,
    pub(crate) created_by: ProjectCreatedBy,
}

impl ProjectRef {
    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn maturity(&self) -> ProjectMaturity {
        self.maturity
    }

    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_ref().map(NonEmptyString::as_str)
    }

    pub fn created_at(&self) -> &UtcTimestamp {
        &self.created_at
    }

    pub fn created_by(&self) -> ProjectCreatedBy {
        self.created_by
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LocatorKey {
    pub(crate) provider: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) normalized_value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectLocator {
    pub(crate) locator_id: ProjectLocatorId,
    pub(crate) project_ref_id: ProjectRefId,
    pub(crate) role: LocatorRole,
    pub(crate) authority: LocatorAuthority,
    provider: NonEmptyString,
    namespace: NonEmptyString,
    kind: NonEmptyString,
    normalized_value: NonEmptyString,
    pub(crate) assurance: LocatorAssurance,
    source_adapter: NonEmptyString,
    evidence_digest: ControlPlaneDigest,
    observed_at: UtcTimestamp,
    pub(crate) state: LocatorState,
}

impl ProjectLocator {
    pub fn locator_id(&self) -> ProjectLocatorId {
        self.locator_id
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn role(&self) -> LocatorRole {
        self.role
    }

    pub fn authority(&self) -> LocatorAuthority {
        self.authority
    }

    pub fn provider(&self) -> &str {
        self.provider.as_str()
    }

    pub fn namespace(&self) -> &str {
        self.namespace.as_str()
    }

    pub fn kind(&self) -> &str {
        self.kind.as_str()
    }

    pub fn normalized_value(&self) -> &str {
        self.normalized_value.as_str()
    }

    pub fn assurance(&self) -> LocatorAssurance {
        self.assurance
    }

    pub fn source_adapter(&self) -> &str {
        self.source_adapter.as_str()
    }

    pub fn evidence_digest(&self) -> &ControlPlaneDigest {
        &self.evidence_digest
    }

    pub fn observed_at(&self) -> &UtcTimestamp {
        &self.observed_at
    }

    pub fn state(&self) -> LocatorState {
        self.state
    }

    pub(crate) fn key(&self) -> LocatorKey {
        LocatorKey {
            provider: self.provider.0.clone(),
            namespace: self.namespace.0.clone(),
            kind: self.kind.0.clone(),
            normalized_value: self.normalized_value.0.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBinding {
    pub(crate) project_ref_id: ProjectRefId,
    store_path: CanonicalPath,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    bound_at: UtcTimestamp,
    binding_source: BindingSource,
}

impl ProjectBinding {
    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn store_path(&self) -> &CanonicalPath {
        &self.store_path
    }

    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    pub fn bound_at(&self) -> &UtcTimestamp {
        &self.bound_at
    }

    pub fn binding_source(&self) -> BindingSource {
        self.binding_source
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirstWriteProjectBinding {
    locator_evidence: LocatorEvidence,
    display_name: Nullable<NonEmptyString>,
    store_path: CanonicalPath,
    store_id: StoreId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    created_at: UtcTimestamp,
}

impl FirstWriteProjectBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        locator_evidence: LocatorEvidence,
        display_name: Option<String>,
        store_path: CanonicalPath,
        store_id: StoreId,
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        created_at: UtcTimestamp,
    ) -> Result<Self> {
        let candidate = Self {
            locator_evidence,
            display_name: Nullable::present(
                display_name
                    .map(|value| NonEmptyString::new(value, "ProjectRef display_name"))
                    .transpose()?,
            ),
            store_path,
            store_id,
            workspace_id,
            branch_id,
            created_at,
        };
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn locator_evidence(&self) -> &LocatorEvidence {
        &self.locator_evidence
    }

    pub fn store_path(&self) -> &CanonicalPath {
        &self.store_path
    }

    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn branch_id(&self) -> BranchId {
        self.branch_id
    }

    fn validate(&self) -> Result<()> {
        self.locator_evidence.validate()?;
        if self.locator_evidence.assurance == LocatorAssurance::Observed {
            return invalid("first-write ProjectRef binding requires eligible locator evidence");
        }
        if !self.display_name.is_present() {
            return invalid("first-write ProjectRef display_name is required even when null");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectBootstrapOutcome {
    Created,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectBootstrapResult {
    outcome: ProjectBootstrapOutcome,
    registry: ProjectRegistryV2,
    project_ref_id: ProjectRefId,
    locator_id: ProjectLocatorId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrongerLocatorAttachmentBasis {
    ExplicitProjectSelection,
    DeterministicAdapterProof(ControlPlaneDigest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrongerLocatorAttachment {
    project_ref_id: ProjectRefId,
    locator_evidence: LocatorEvidence,
    observed_at: UtcTimestamp,
    basis: StrongerLocatorAttachmentBasis,
}

impl StrongerLocatorAttachment {
    pub fn new(
        project_ref_id: ProjectRefId,
        locator_evidence: LocatorEvidence,
        observed_at: UtcTimestamp,
        basis: StrongerLocatorAttachmentBasis,
    ) -> Result<Self> {
        let attachment = Self {
            project_ref_id,
            locator_evidence,
            observed_at,
            basis,
        };
        attachment.validate()?;
        Ok(attachment)
    }

    fn validate(&self) -> Result<()> {
        self.locator_evidence.validate()?;
        if !matches!(
            self.locator_evidence.authority,
            LocatorAuthority::SemanticProject | LocatorAuthority::Repository
        ) || self.locator_evidence.assurance == LocatorAssurance::Observed
        {
            return invalid(
                "stronger-locator attachment requires eligible semantic or repository identity evidence",
            );
        }
        if let StrongerLocatorAttachmentBasis::DeterministicAdapterProof(proof_digest) = &self.basis
            && proof_digest != &self.locator_evidence.evidence_digest
        {
            return invalid(
                "deterministic adapter attachment proof must match the retained evidence digest",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrongerLocatorAttachmentOutcome {
    Attached,
    Reused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrongerLocatorAttachmentResult {
    outcome: StrongerLocatorAttachmentOutcome,
    registry: ProjectRegistryV2,
    project_ref_id: ProjectRefId,
    locator_id: ProjectLocatorId,
}

impl StrongerLocatorAttachmentResult {
    pub fn outcome(&self) -> StrongerLocatorAttachmentOutcome {
        self.outcome
    }

    pub fn registry(&self) -> &ProjectRegistryV2 {
        &self.registry
    }

    pub fn into_registry(self) -> ProjectRegistryV2 {
        self.registry
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn locator_id(&self) -> ProjectLocatorId {
        self.locator_id
    }
}

impl ProjectBootstrapResult {
    pub fn outcome(&self) -> ProjectBootstrapOutcome {
        self.outcome
    }

    pub fn registry(&self) -> &ProjectRegistryV2 {
        &self.registry
    }

    pub fn into_registry(self) -> ProjectRegistryV2 {
        self.registry
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn locator_id(&self) -> ProjectLocatorId {
        self.locator_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectLink {
    pub(crate) project_link_id: ProjectLinkId,
    pub(crate) from_project_ref: ProjectRefId,
    pub(crate) to_project_ref: ProjectRefId,
    relation: ProjectLinkRelation,
    status: ProjectLinkStatus,
    basis: ProjectLinkBasis,
    evidence_digest: ControlPlaneDigest,
    created_at: UtcTimestamp,
}

impl ProjectLink {
    pub fn project_link_id(&self) -> ProjectLinkId {
        self.project_link_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryObservation {
    pub(crate) observation_id: RegistryObservationId,
    kind: RegistryObservationKind,
    pub(crate) project_refs: Vec<ProjectRefId>,
    target_digest: ControlPlaneDigest,
    observed_at: UtcTimestamp,
    source: RegistryObservationSource,
}

impl RegistryObservation {
    pub fn observation_id(&self) -> RegistryObservationId {
        self.observation_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationMappingReceipt {
    v1_identity_kind: NonEmptyString,
    v1_identity: CanonicalPath,
    project_ref_id: ProjectRefId,
}

impl MigrationMappingReceipt {
    pub fn v1_identity_kind(&self) -> &str {
        self.v1_identity_kind.as_str()
    }

    pub fn v1_identity(&self) -> &CanonicalPath {
        &self.v1_identity
    }

    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationReceipt {
    from_version: u64,
    source_digest: ControlPlaneDigest,
    preview_digest: ControlPlaneDigest,
    applied_at: UtcTimestamp,
    backup_path: CanonicalPath,
    backup_digest: ControlPlaneDigest,
    mappings: Vec<MigrationMappingReceipt>,
}

impl MigrationReceipt {
    pub fn from_version(&self) -> u64 {
        self.from_version
    }

    pub fn source_digest(&self) -> &ControlPlaneDigest {
        &self.source_digest
    }

    pub fn preview_digest(&self) -> &ControlPlaneDigest {
        &self.preview_digest
    }

    pub fn applied_at(&self) -> &UtcTimestamp {
        &self.applied_at
    }

    pub fn backup_path(&self) -> &CanonicalPath {
        &self.backup_path
    }

    pub fn backup_digest(&self) -> &ControlPlaneDigest {
        &self.backup_digest
    }

    pub fn mappings(&self) -> &[MigrationMappingReceipt] {
        &self.mappings
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectRegistryV2 {
    version: u64,
    pub(crate) registry_id: RegistryId,
    revision: u64,
    pub(crate) projects: Vec<ProjectRef>,
    pub(crate) locators: Vec<ProjectLocator>,
    bindings: Vec<ProjectBinding>,
    links: Vec<ProjectLink>,
    observations: Vec<RegistryObservation>,
    #[serde(default)]
    migration: Nullable<MigrationReceipt>,
}

impl ProjectRegistryV2 {
    pub fn from_json_bytes(input: &[u8]) -> Result<Self> {
        parse_canonical_json(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "registry v2 is not strict canonical-domain JSON: {error}"
            ))
        })?;
        let registry: Self = serde_json::from_slice(input).map_err(|error| {
            WorkVcsError::ControlPlaneInvalid(format!("registry v2 has an invalid shape: {error}"))
        })?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != REGISTRY_VERSION {
            return invalid(format!(
                "registry version {} is not supported; expected {REGISTRY_VERSION}",
                self.version
            ));
        }
        if !self.migration.is_present() {
            return invalid("registry.migration is required even when its value is null");
        }
        require_sorted_unique(&self.projects, |project| project.project_ref_id, "projects")?;
        require_sorted_unique(&self.locators, |locator| locator.locator_id, "locators")?;
        require_sorted_unique(&self.bindings, |binding| binding.project_ref_id, "bindings")?;
        require_sorted_unique(&self.links, |link| link.project_link_id, "links")?;
        require_sorted_unique(
            &self.observations,
            |observation| observation.observation_id,
            "observations",
        )?;

        let projects = self
            .projects
            .iter()
            .map(|project| (project.project_ref_id, project))
            .collect::<BTreeMap<_, _>>();
        let expected_path_namespace = format!("registry:{}", self.registry_id);
        let mut active_identity_keys = BTreeMap::<LocatorKey, ProjectRefId>::new();
        for locator in &self.locators {
            if !projects.contains_key(&locator.project_ref_id) {
                return invalid(format!(
                    "locator {} refers to missing ProjectRef {}",
                    locator.locator_id, locator.project_ref_id
                ));
            }
            if locator.assurance == LocatorAssurance::Observed
                && locator.role != LocatorRole::Context
            {
                return invalid(format!(
                    "observed locator {} must have context role",
                    locator.locator_id
                ));
            }
            if locator.role == LocatorRole::Identity
                && locator.authority == LocatorAuthority::SemanticProject
                && !matches!(
                    locator.assurance,
                    LocatorAssurance::Authoritative | LocatorAssurance::VerifiedDerived
                )
            {
                return invalid(format!(
                    "semantic identity locator {} requires authoritative or verified-derived assurance",
                    locator.locator_id
                ));
            }
            validate_path_locator(locator, &expected_path_namespace)?;
            if locator.role == LocatorRole::Identity && locator.state == LocatorState::Active {
                let key = locator.key();
                if let Some(existing) = active_identity_keys.insert(key, locator.project_ref_id)
                    && existing != locator.project_ref_id
                {
                    return invalid(format!(
                        "active identity locator {} is claimed by ProjectRefs {} and {}",
                        locator.locator_id, existing, locator.project_ref_id
                    ));
                }
            }
        }

        for project in &self.projects {
            if !project.display_name.is_present() {
                return invalid(format!(
                    "ProjectRef {} display_name is required even when its value is null",
                    project.project_ref_id
                ));
            }
            let active_authorities = self
                .locators
                .iter()
                .filter(|locator| {
                    locator.project_ref_id == project.project_ref_id
                        && locator.role == LocatorRole::Identity
                        && locator.state == LocatorState::Active
                })
                .map(|locator| locator.authority)
                .collect::<Vec<_>>();
            match project.maturity {
                ProjectMaturity::Provisional => {
                    if active_authorities.is_empty()
                        || active_authorities
                            .iter()
                            .any(|authority| *authority != LocatorAuthority::Cwd)
                    {
                        return invalid(format!(
                            "provisional ProjectRef {} must have CWD as its strongest active identity locator",
                            project.project_ref_id
                        ));
                    }
                }
                ProjectMaturity::Established => {
                    let has_strong_identity = active_authorities.iter().any(|authority| {
                        matches!(
                            authority,
                            LocatorAuthority::SemanticProject | LocatorAuthority::Repository
                        )
                    });
                    if project.created_by != ProjectCreatedBy::Explicit && !has_strong_identity {
                        return invalid(format!(
                            "established ProjectRef {} requires explicit creation or a semantic/repository identity locator",
                            project.project_ref_id
                        ));
                    }
                }
            }
        }

        for binding in &self.bindings {
            if !projects.contains_key(&binding.project_ref_id) {
                return invalid(format!(
                    "binding refers to missing ProjectRef {}",
                    binding.project_ref_id
                ));
            }
        }
        for link in &self.links {
            if link.from_project_ref == link.to_project_ref {
                return invalid(format!(
                    "project link {} must connect distinct ProjectRefs",
                    link.project_link_id
                ));
            }
            if !projects.contains_key(&link.from_project_ref)
                || !projects.contains_key(&link.to_project_ref)
            {
                return invalid(format!(
                    "project link {} refers to a missing ProjectRef",
                    link.project_link_id
                ));
            }
        }
        for observation in &self.observations {
            if observation.project_refs.len() < 2 {
                return invalid(format!(
                    "registry observation {} requires at least two ProjectRefs",
                    observation.observation_id
                ));
            }
            require_sorted_unique_ids(
                &observation.project_refs,
                "registry observation project_refs",
            )?;
            if observation
                .project_refs
                .iter()
                .any(|project_ref| !projects.contains_key(project_ref))
            {
                return invalid(format!(
                    "registry observation {} refers to a missing ProjectRef",
                    observation.observation_id
                ));
            }
        }
        if let Some(migration) = self.migration.as_ref() {
            if migration.from_version != 1 {
                return invalid("migration receipt from_version must be 1".to_owned());
            }
            require_sorted_unique(
                &migration.mappings,
                |mapping| {
                    format!(
                        "{}\0{}",
                        mapping.v1_identity_kind.as_str(),
                        mapping.v1_identity.as_str()
                    )
                },
                "migration mappings",
            )?;
            let migrated_projects = self
                .projects
                .iter()
                .filter(|project| project.created_by == ProjectCreatedBy::Migration)
                .map(|project| project.project_ref_id)
                .collect::<BTreeSet<_>>();
            if migration.mappings.len() != migrated_projects.len() {
                return invalid(format!(
                    "migration receipt requires one mapping per migration-created ProjectRef; found {} mappings for {} migrated projects",
                    migration.mappings.len(),
                    migrated_projects.len()
                ));
            }
            let mut mapped_projects = BTreeSet::new();
            for mapping in &migration.mappings {
                if !matches!(mapping.v1_identity_kind.as_str(), "git-common-dir" | "cwd") {
                    return invalid(format!(
                        "migration mapping has unsupported v1 identity kind {:?}",
                        mapping.v1_identity_kind.as_str()
                    ));
                }
                if !projects.contains_key(&mapping.project_ref_id) {
                    return invalid(format!(
                        "migration mapping refers to missing ProjectRef {}",
                        mapping.project_ref_id
                    ));
                }
                if !migrated_projects.contains(&mapping.project_ref_id) {
                    return invalid(format!(
                        "migration mapping refers to ProjectRef {} that was not created by migration",
                        mapping.project_ref_id
                    ));
                }
                if !mapped_projects.insert(mapping.project_ref_id) {
                    return invalid(format!(
                        "migration receipt maps ProjectRef {} more than once",
                        mapping.project_ref_id
                    ));
                }
            }
            if mapped_projects != migrated_projects {
                return invalid(
                    "migration receipt mappings do not cover every migration-created ProjectRef",
                );
            }
        }
        Ok(())
    }

    pub fn canonical_json_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        canonicalize_serializable(self, "registry v2")
    }

    pub fn stored_json_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = self.canonical_json_bytes()?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn digest(&self) -> Result<ControlPlaneDigest> {
        Ok(ControlPlaneDigest::raw(&self.canonical_json_bytes()?))
    }

    pub fn registry_id(&self) -> RegistryId {
        self.registry_id
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn projects(&self) -> &[ProjectRef] {
        &self.projects
    }

    pub fn locators(&self) -> &[ProjectLocator] {
        &self.locators
    }

    pub fn bindings(&self) -> &[ProjectBinding] {
        &self.bindings
    }

    pub fn links(&self) -> &[ProjectLink] {
        &self.links
    }

    pub fn observations(&self) -> &[RegistryObservation] {
        &self.observations
    }

    pub fn migration(&self) -> Option<&MigrationReceipt> {
        self.migration.as_ref()
    }

    pub fn project(&self, project_ref_id: ProjectRefId) -> Option<&ProjectRef> {
        self.projects
            .binary_search_by_key(&project_ref_id, |project| project.project_ref_id)
            .ok()
            .map(|index| &self.projects[index])
    }

    pub fn binding(&self, project_ref_id: ProjectRefId) -> Option<&ProjectBinding> {
        self.bindings
            .binary_search_by_key(&project_ref_id, |binding| binding.project_ref_id)
            .ok()
            .map(|index| &self.bindings[index])
    }

    pub fn converge_first_write_binding(
        &self,
        candidate: FirstWriteProjectBinding,
    ) -> Result<ProjectBootstrapResult> {
        self.validate()?;
        candidate.validate()?;
        let key = candidate.locator_evidence.key();
        if let Some(project_ref_id) = self.active_identity_project(&key) {
            let locator = self
                .locators
                .iter()
                .find(|locator| {
                    locator.project_ref_id == project_ref_id
                        && locator.role == LocatorRole::Identity
                        && locator.state == LocatorState::Active
                        && locator.key() == key
                })
                .expect("active_identity_project returned an existing locator");
            let binding = self.binding(project_ref_id).ok_or_else(|| {
                WorkVcsError::ControlPlaneInvalid(format!(
                    "ProjectRef {project_ref_id} already claims the locator but has no binding"
                ))
            })?;
            if binding.store_path != candidate.store_path
                || binding.store_id != candidate.store_id
                || binding.workspace_id != candidate.workspace_id
                || binding.branch_id != candidate.branch_id
            {
                return invalid(format!(
                    "locator is already claimed by ProjectRef {project_ref_id} with a different target binding"
                ));
            }
            return Ok(ProjectBootstrapResult {
                outcome: ProjectBootstrapOutcome::Reused,
                registry: self.clone(),
                project_ref_id,
                locator_id: locator.locator_id,
            });
        }

        let project_ref_id = ProjectRefId::new_v7();
        let locator_id = ProjectLocatorId::new_v7();
        let (maturity, created_by) = match candidate.locator_evidence.authority {
            LocatorAuthority::SemanticProject => (
                ProjectMaturity::Established,
                ProjectCreatedBy::SemanticLocator,
            ),
            LocatorAuthority::Repository => (
                ProjectMaturity::Established,
                ProjectCreatedBy::RepositoryFirstWrite,
            ),
            LocatorAuthority::Cwd => (
                ProjectMaturity::Provisional,
                ProjectCreatedBy::CwdFirstWrite,
            ),
        };
        let project = ProjectRef {
            project_ref_id,
            maturity,
            display_name: candidate.display_name.clone(),
            created_at: candidate.created_at.clone(),
            created_by,
        };
        let locator = ProjectLocator {
            locator_id,
            project_ref_id,
            role: LocatorRole::Identity,
            authority: candidate.locator_evidence.authority,
            provider: candidate.locator_evidence.provider.clone(),
            namespace: candidate.locator_evidence.namespace.clone(),
            kind: candidate.locator_evidence.kind.clone(),
            normalized_value: candidate.locator_evidence.normalized_value.clone(),
            assurance: candidate.locator_evidence.assurance,
            source_adapter: candidate.locator_evidence.source_adapter.clone(),
            evidence_digest: candidate.locator_evidence.evidence_digest.clone(),
            observed_at: candidate.created_at.clone(),
            state: LocatorState::Active,
        };
        let binding = ProjectBinding {
            project_ref_id,
            store_path: candidate.store_path,
            store_id: candidate.store_id,
            workspace_id: candidate.workspace_id,
            branch_id: candidate.branch_id,
            bound_at: candidate.created_at,
            binding_source: BindingSource::FirstWrite,
        };
        let mut registry = self.clone();
        registry.revision = registry.revision.checked_add(1).ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid("registry revision overflow".to_owned())
        })?;
        registry.projects.push(project);
        registry
            .projects
            .sort_by_key(|project| project.project_ref_id);
        registry.locators.push(locator);
        registry.locators.sort_by_key(|locator| locator.locator_id);
        registry.bindings.push(binding);
        registry
            .bindings
            .sort_by_key(|binding| binding.project_ref_id);
        registry.validate()?;
        Ok(ProjectBootstrapResult {
            outcome: ProjectBootstrapOutcome::Created,
            registry,
            project_ref_id,
            locator_id,
        })
    }

    pub fn attach_stronger_identity_locator(
        &self,
        attachment: StrongerLocatorAttachment,
    ) -> Result<StrongerLocatorAttachmentResult> {
        self.validate()?;
        attachment.validate()?;
        let project = self.project(attachment.project_ref_id).ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid(format!(
                "stronger-locator attachment names missing ProjectRef {}",
                attachment.project_ref_id
            ))
        })?;
        if self.binding(attachment.project_ref_id).is_none() {
            return invalid(format!(
                "ProjectRef {} has no target binding to preserve during stronger-locator attachment",
                attachment.project_ref_id
            ));
        }

        let key = attachment.locator_evidence.key();
        if let Some(claimed_by) = self.active_identity_project(&key) {
            if claimed_by != attachment.project_ref_id {
                return Err(WorkVcsError::LocatorAlreadyClaimed(format!(
                    "locator key is owned by ProjectRef {claimed_by}, not selected ProjectRef {}",
                    attachment.project_ref_id
                )));
            }
            let locator = self
                .locators
                .iter()
                .find(|locator| {
                    locator.project_ref_id == claimed_by
                        && locator.role == LocatorRole::Identity
                        && locator.state == LocatorState::Active
                        && locator.key() == key
                })
                .expect("active_identity_project returned an existing locator");
            return Ok(StrongerLocatorAttachmentResult {
                outcome: StrongerLocatorAttachmentOutcome::Reused,
                registry: self.clone(),
                project_ref_id: claimed_by,
                locator_id: locator.locator_id,
            });
        }

        if project.maturity != ProjectMaturity::Provisional {
            return invalid(format!(
                "ProjectRef {} is not provisional; attaching another identity requires a separately accepted lifecycle operation",
                attachment.project_ref_id
            ));
        }

        let locator_id = ProjectLocatorId::new_v7();
        let locator = ProjectLocator {
            locator_id,
            project_ref_id: attachment.project_ref_id,
            role: LocatorRole::Identity,
            authority: attachment.locator_evidence.authority,
            provider: attachment.locator_evidence.provider,
            namespace: attachment.locator_evidence.namespace,
            kind: attachment.locator_evidence.kind,
            normalized_value: attachment.locator_evidence.normalized_value,
            assurance: attachment.locator_evidence.assurance,
            source_adapter: attachment.locator_evidence.source_adapter,
            evidence_digest: attachment.locator_evidence.evidence_digest,
            observed_at: attachment.observed_at,
            state: LocatorState::Active,
        };
        let mut registry = self.clone();
        registry.revision = registry.revision.checked_add(1).ok_or_else(|| {
            WorkVcsError::ControlPlaneInvalid("registry revision overflow".to_owned())
        })?;
        let project = registry
            .projects
            .iter_mut()
            .find(|project| project.project_ref_id == attachment.project_ref_id)
            .expect("validated ProjectRef exists");
        project.maturity = ProjectMaturity::Established;
        registry.locators.push(locator);
        registry.locators.sort_by_key(|locator| locator.locator_id);
        registry.validate()?;
        Ok(StrongerLocatorAttachmentResult {
            outcome: StrongerLocatorAttachmentOutcome::Attached,
            registry,
            project_ref_id: attachment.project_ref_id,
            locator_id,
        })
    }

    pub(crate) fn contains_project(&self, project_ref_id: ProjectRefId) -> bool {
        self.projects
            .binary_search_by_key(&project_ref_id, |project| project.project_ref_id)
            .is_ok()
    }

    pub(crate) fn active_identity_project(&self, key: &LocatorKey) -> Option<ProjectRefId> {
        self.locators
            .iter()
            .find(|locator| {
                locator.role == LocatorRole::Identity
                    && locator.state == LocatorState::Active
                    && locator.key() == *key
            })
            .map(|locator| locator.project_ref_id)
    }
}

fn validate_path_locator(locator: &ProjectLocator, expected_namespace: &str) -> Result<()> {
    let expected = match locator.authority {
        LocatorAuthority::SemanticProject => return Ok(()),
        LocatorAuthority::Repository => ("git", "git_common_dir"),
        LocatorAuthority::Cwd => ("filesystem", "canonical_directory"),
    };
    if locator.provider.as_str() != expected.0
        || locator.kind.as_str() != expected.1
        || locator.namespace.as_str() != expected_namespace
    {
        return invalid(format!(
            "path locator {} must use provider={}, namespace={}, kind={}",
            locator.locator_id, expected.0, expected_namespace, expected.1
        ));
    }
    CanonicalPath::parse(locator.normalized_value.as_str())?;
    Ok(())
}

fn require_sorted_unique<T, K: Ord + fmt::Display>(
    values: &[T],
    key: impl Fn(&T) -> K,
    label: &str,
) -> Result<()> {
    for pair in values.windows(2) {
        let left = key(&pair[0]);
        let right = key(&pair[1]);
        if left >= right {
            return invalid(format!(
                "{label} must be strictly sorted by stable ID; found {left} before {right}"
            ));
        }
    }
    Ok(())
}

fn require_sorted_unique_ids<T: Ord + fmt::Display>(values: &[T], label: &str) -> Result<()> {
    for pair in values.windows(2) {
        if pair[0] >= pair[1] {
            return invalid(format!(
                "{label} must be strictly sorted; found {} before {}",
                pair[0], pair[1]
            ));
        }
    }
    Ok(())
}

pub(crate) fn canonicalize_serializable(value: &impl Serialize, label: &str) -> Result<Vec<u8>> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("cannot serialize {label}: {error}"))
    })?;
    let canonical = parse_canonical_json(&encoded).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("cannot canonicalize {label}: {error}"))
    })?;
    canonical_bytes(&canonical).map_err(|error| {
        WorkVcsError::ControlPlaneInvalid(format!("cannot encode canonical {label}: {error}"))
    })
}

pub(crate) fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(WorkVcsError::ControlPlaneInvalid(message.into()))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocatorEvidence {
    pub(crate) authority: LocatorAuthority,
    provider: NonEmptyString,
    namespace: NonEmptyString,
    kind: NonEmptyString,
    normalized_value: NonEmptyString,
    pub(crate) assurance: LocatorAssurance,
    source_adapter: NonEmptyString,
    evidence_digest: ControlPlaneDigest,
}

impl LocatorEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authority: LocatorAuthority,
        provider: impl Into<String>,
        namespace: impl Into<String>,
        kind: impl Into<String>,
        normalized_value: impl Into<String>,
        assurance: LocatorAssurance,
        source_adapter: impl Into<String>,
        evidence_digest: ControlPlaneDigest,
    ) -> Result<Self> {
        let evidence = Self {
            authority,
            provider: NonEmptyString::new(provider, "locator provider")?,
            namespace: NonEmptyString::new(namespace, "locator namespace")?,
            kind: NonEmptyString::new(kind, "locator kind")?,
            normalized_value: NonEmptyString::new(normalized_value, "locator normalized value")?,
            assurance,
            source_adapter: NonEmptyString::new(source_adapter, "source adapter")?,
            evidence_digest,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    pub fn authority(&self) -> LocatorAuthority {
        self.authority
    }

    pub fn provider(&self) -> &str {
        self.provider.as_str()
    }

    pub fn namespace(&self) -> &str {
        self.namespace.as_str()
    }

    pub fn kind(&self) -> &str {
        self.kind.as_str()
    }

    pub fn normalized_value(&self) -> &str {
        self.normalized_value.as_str()
    }

    pub fn assurance(&self) -> LocatorAssurance {
        self.assurance
    }

    pub fn source_adapter(&self) -> &str {
        self.source_adapter.as_str()
    }

    pub fn evidence_digest(&self) -> &ControlPlaneDigest {
        &self.evidence_digest
    }

    pub(crate) fn key(&self) -> LocatorKey {
        LocatorKey {
            provider: self.provider.0.clone(),
            namespace: self.namespace.0.clone(),
            kind: self.kind.0.clone(),
            normalized_value: self.normalized_value.0.clone(),
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.assurance == LocatorAssurance::Observed {
            return Ok(());
        }
        if self.authority == LocatorAuthority::SemanticProject
            && !matches!(
                self.assurance,
                LocatorAssurance::Authoritative | LocatorAssurance::VerifiedDerived
            )
        {
            return invalid(
                "semantic locator evidence requires authoritative or verified-derived assurance",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathLocatorEvidence {
    canonical_path: CanonicalPath,
    source_adapter: NonEmptyString,
    evidence_digest: ControlPlaneDigest,
}

impl PathLocatorEvidence {
    pub fn new(
        canonical_path: CanonicalPath,
        source_adapter: impl Into<String>,
        evidence_digest: ControlPlaneDigest,
    ) -> Result<Self> {
        Ok(Self {
            canonical_path,
            source_adapter: NonEmptyString::new(source_adapter, "source adapter")?,
            evidence_digest,
        })
    }

    pub fn canonical_path(&self) -> &CanonicalPath {
        &self.canonical_path
    }

    pub(crate) fn as_locator_in_namespace(
        &self,
        authority: LocatorAuthority,
        namespace: impl Into<String>,
    ) -> Result<LocatorEvidence> {
        let (provider, kind) = match authority {
            LocatorAuthority::Repository => ("git", "git_common_dir"),
            LocatorAuthority::Cwd => ("filesystem", "canonical_directory"),
            LocatorAuthority::SemanticProject => {
                return invalid("path evidence cannot be a semantic Project locator");
            }
        };
        LocatorEvidence::new(
            authority,
            provider,
            namespace,
            kind,
            self.canonical_path.as_str(),
            LocatorAssurance::VerifiedDerived,
            self.source_adapter.as_str(),
            self.evidence_digest.clone(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionMode {
    ReadOnly,
    DurableWrite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionContext {
    pub(crate) project_ref_id: Option<ProjectRefId>,
    #[serde(default)]
    pub(crate) locator_evidence: Vec<LocatorEvidence>,
    pub(crate) git_common_dir: Option<PathLocatorEvidence>,
    pub(crate) cwd: Option<PathLocatorEvidence>,
    pub(crate) operation_mode: ResolutionMode,
}

impl ResolutionContext {
    pub fn new(operation_mode: ResolutionMode) -> Self {
        Self {
            project_ref_id: None,
            locator_evidence: Vec::new(),
            git_common_dir: None,
            cwd: None,
            operation_mode,
        }
    }

    pub fn with_project_ref(mut self, project_ref_id: ProjectRefId) -> Self {
        self.project_ref_id = Some(project_ref_id);
        self
    }

    pub fn with_locator_evidence(mut self, evidence: LocatorEvidence) -> Self {
        self.locator_evidence.push(evidence);
        self
    }

    pub fn with_git_common_dir(mut self, evidence: PathLocatorEvidence) -> Self {
        self.git_common_dir = Some(evidence);
        self
    }

    pub fn with_cwd(mut self, evidence: PathLocatorEvidence) -> Self {
        self.cwd = Some(evidence);
        self
    }

    pub fn operation_mode(&self) -> ResolutionMode {
        self.operation_mode
    }

    pub fn project_ref_id(&self) -> Option<ProjectRefId> {
        self.project_ref_id
    }

    pub fn locator_evidence(&self) -> &[LocatorEvidence] {
        &self.locator_evidence
    }

    pub fn git_common_dir(&self) -> Option<&PathLocatorEvidence> {
        self.git_common_dir.as_ref()
    }

    pub fn cwd(&self) -> Option<&PathLocatorEvidence> {
        self.cwd.as_ref()
    }

    pub(crate) fn all_evidence(&self, registry_id: RegistryId) -> Result<Vec<LocatorEvidence>> {
        self.all_evidence_in_namespace(format!("registry:{registry_id}"))
    }

    pub(crate) fn all_evidence_in_namespace(
        &self,
        path_namespace: impl Into<String>,
    ) -> Result<Vec<LocatorEvidence>> {
        let path_namespace = path_namespace.into();
        let mut evidence = self.locator_evidence.clone();
        for item in &evidence {
            item.validate()?;
        }
        if let Some(git) = &self.git_common_dir {
            evidence.push(
                git.as_locator_in_namespace(LocatorAuthority::Repository, path_namespace.clone())?,
            );
        }
        if let Some(cwd) = &self.cwd {
            evidence.push(cwd.as_locator_in_namespace(LocatorAuthority::Cwd, path_namespace)?);
        }
        Ok(evidence)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureGroupMemberRole {
    Primary,
    ArtifactRepository,
    ExecutionContext,
    Related,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureGroupMemberDelivery {
    Canonical,
    ImmutableReference,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureGroupMember {
    pub(crate) project_ref_id: ProjectRefId,
    pub(crate) role: CaptureGroupMemberRole,
    relation: NonEmptyString,
    pub(crate) delivery_mode: CaptureGroupMemberDelivery,
}

impl CaptureGroupMember {
    pub fn project_ref_id(&self) -> ProjectRefId {
        self.project_ref_id
    }

    pub fn role(&self) -> CaptureGroupMemberRole {
        self.role
    }

    pub fn relation(&self) -> &str {
        self.relation.as_str()
    }

    pub fn delivery_mode(&self) -> CaptureGroupMemberDelivery {
        self.delivery_mode
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureGroupIntent {
    pub(crate) capture_group_id: CaptureGroupId,
    #[serde(default)]
    pub(crate) primary_project_ref: Nullable<ProjectRefId>,
    #[serde(default)]
    pub(crate) primary_locator_evidence_digest: Nullable<ControlPlaneDigest>,
    canonical_record_local_id: NonEmptyString,
    pub(crate) members: Vec<CaptureGroupMember>,
}

impl CaptureGroupIntent {
    pub fn capture_group_id(&self) -> CaptureGroupId {
        self.capture_group_id
    }

    pub fn primary_project_ref(&self) -> Option<ProjectRefId> {
        self.primary_project_ref.copied()
    }

    pub fn primary_locator_evidence_digest(&self) -> Option<&ControlPlaneDigest> {
        self.primary_locator_evidence_digest.as_ref()
    }

    pub fn canonical_record_local_id(&self) -> &str {
        self.canonical_record_local_id.as_str()
    }

    pub fn members(&self) -> &[CaptureGroupMember] {
        &self.members
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if !self.primary_project_ref.is_present()
            || !self.primary_locator_evidence_digest.is_present()
        {
            return invalid(
                "capture group primary_project_ref and primary_locator_evidence_digest are required even when null",
            );
        }
        if self.primary_project_ref.as_ref().is_some()
            == self.primary_locator_evidence_digest.as_ref().is_some()
        {
            return invalid(
                "capture group requires exactly one of primary_project_ref and primary_locator_evidence_digest",
            );
        }
        let mut members = BTreeSet::new();
        let mut canonical_members = Vec::new();
        let mut primary_role_members = Vec::new();
        for member in &self.members {
            if !members.insert(member.project_ref_id) {
                return invalid(format!(
                    "capture group contains duplicate ProjectRef {}",
                    member.project_ref_id
                ));
            }
            if member.delivery_mode == CaptureGroupMemberDelivery::Canonical {
                canonical_members.push(member);
            }
            if member.role == CaptureGroupMemberRole::Primary {
                primary_role_members.push(member);
            }
        }
        match self.primary_project_ref.copied() {
            Some(primary) => {
                if canonical_members.len() != 1
                    || canonical_members[0].project_ref_id != primary
                    || canonical_members[0].role != CaptureGroupMemberRole::Primary
                    || primary_role_members.len() != 1
                    || primary_role_members[0].project_ref_id != primary
                {
                    return invalid(
                        "resolved capture group requires exactly one canonical primary member and exactly one primary role",
                    );
                }
            }
            None if !canonical_members.is_empty() || !primary_role_members.is_empty() => {
                return invalid(
                    "unresolved capture group must not declare a primary role or canonical delivery before resolution",
                );
            }
            None => {}
        }
        Ok(())
    }
}
