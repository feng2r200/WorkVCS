use super::{knowledge_space, knowledge_version_state_digest, state_at};
use crate::canonical::{
    CanonicalValue, canonical_bytes, content_object_digest, parse_canonical_json,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    CommitId, Digest, EntityId, EntityVersionId, ExposureId, ExposureTransitionId,
    KnowledgeSpaceId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use std::fmt;

const KNOWLEDGE_EXPOSURE_OBJECT_KIND: &str = "knowledge_exposure";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeExposureLifecycleStatus {
    Active,
    Withdrawn,
}

impl KnowledgeExposureLifecycleStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "withdrawn" => Ok(Self::Withdrawn),
            other => Err(WorkVcsError::QueryInvalid(format!(
                "knowledge exposure lifecycle status {other:?} is not supported"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Withdrawn => "withdrawn",
        }
    }
}

impl fmt::Display for KnowledgeExposureLifecycleStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeExposureSourceStatus {
    Current,
    Stale,
    Unknown,
    Unresolved,
}

impl KnowledgeExposureSourceStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "current" => Ok(Self::Current),
            "stale" => Ok(Self::Stale),
            "unknown" => Ok(Self::Unknown),
            "unresolved" => Ok(Self::Unresolved),
            other => Err(WorkVcsError::QueryInvalid(format!(
                "knowledge exposure source status {other:?} is not supported"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
            Self::Unresolved => "unresolved",
        }
    }
}

impl fmt::Display for KnowledgeExposureSourceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureCreateLocalOptions {
    knowledge_space_id: KnowledgeSpaceId,
    workspace_id: WorkspaceId,
    knowledge_entity_id: EntityId,
    knowledge_entity_version_id: EntityVersionId,
    detail: CanonicalValue,
}

impl KnowledgeExposureCreateLocalOptions {
    pub fn new(
        knowledge_space_id: KnowledgeSpaceId,
        workspace_id: WorkspaceId,
        knowledge_entity_id: EntityId,
        knowledge_entity_version_id: EntityVersionId,
    ) -> Result<Self> {
        Ok(Self {
            knowledge_space_id,
            workspace_id,
            knowledge_entity_id,
            knowledge_entity_version_id,
            detail: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_detail(mut self, detail: CanonicalValue) -> Result<Self> {
        require_object_value("knowledge exposure transition detail", &detail)?;
        self.detail = detail;
        Ok(self)
    }

    fn knowledge_space_id(&self) -> KnowledgeSpaceId {
        self.knowledge_space_id
    }

    fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    fn knowledge_entity_id(&self) -> EntityId {
        self.knowledge_entity_id
    }

    fn knowledge_entity_version_id(&self) -> EntityVersionId {
        self.knowledge_entity_version_id
    }

    fn detail(&self) -> &CanonicalValue {
        &self.detail
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureWithdrawOptions {
    exposure_id: ExposureId,
    expected_transition_id: ExposureTransitionId,
    detail: CanonicalValue,
}

impl KnowledgeExposureWithdrawOptions {
    pub fn new(
        exposure_id: ExposureId,
        expected_transition_id: ExposureTransitionId,
    ) -> Result<Self> {
        Ok(Self {
            exposure_id,
            expected_transition_id,
            detail: CanonicalValue::object(Vec::new())?,
        })
    }

    pub fn with_detail(mut self, detail: CanonicalValue) -> Result<Self> {
        require_object_value("knowledge exposure withdrawal detail", &detail)?;
        self.detail = detail;
        Ok(self)
    }

    fn exposure_id(&self) -> ExposureId {
        self.exposure_id
    }

    fn expected_transition_id(&self) -> ExposureTransitionId {
        self.expected_transition_id
    }

    fn detail(&self) -> &CanonicalValue {
        &self.detail
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureRefreshSourceStatusOptions {
    exposure_id: ExposureId,
}

impl KnowledgeExposureRefreshSourceStatusOptions {
    pub fn new(exposure_id: ExposureId) -> Self {
        Self { exposure_id }
    }

    fn exposure_id(&self) -> ExposureId {
        self.exposure_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureCreateResult {
    pub exposure: KnowledgeExposureSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureWithdrawResult {
    pub exposure: KnowledgeExposureSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureRefreshSourceStatusResult {
    pub exposure: KnowledgeExposureSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureSnapshot {
    pub exposure_id: ExposureId,
    pub knowledge_space_id: KnowledgeSpaceId,
    pub created_at_us: i64,
    pub lifecycle_status: KnowledgeExposureLifecycleStatus,
    pub transition_id: ExposureTransitionId,
    pub previous_transition_id: Option<ExposureTransitionId>,
    pub changed_at_us: i64,
    pub transition_detail: CanonicalValue,
    pub transition_detail_digest: Digest,
    pub transition_detail_size_bytes: i64,
    pub source: KnowledgeExposureLocalSourceSnapshot,
    pub source_status: KnowledgeExposureSourceStatusSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureLocalSourceSnapshot {
    pub workspace_id: WorkspaceId,
    pub knowledge_entity_id: EntityId,
    pub knowledge_entity_version_id: EntityVersionId,
    pub knowledge_state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureSourceStatusSnapshot {
    pub source_status: KnowledgeExposureSourceStatus,
    pub checked_at_us: i64,
    pub detail: CanonicalValue,
    pub detail_digest: Digest,
    pub detail_size_bytes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureListOptions {
    limit: usize,
    knowledge_space_id: Option<KnowledgeSpaceId>,
    workspace_id: Option<WorkspaceId>,
    knowledge_entity_id: Option<EntityId>,
    lifecycle_status: Option<KnowledgeExposureLifecycleStatus>,
    source_status: Option<KnowledgeExposureSourceStatus>,
}

impl KnowledgeExposureListOptions {
    pub fn new() -> Self {
        Self {
            limit: 50,
            knowledge_space_id: None,
            workspace_id: None,
            knowledge_entity_id: None,
            lifecycle_status: None,
            source_status: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "knowledge exposure list limit must be positive".to_owned(),
            ));
        }
        self.limit = limit;
        Ok(self)
    }

    pub fn with_knowledge_space_id(mut self, knowledge_space_id: KnowledgeSpaceId) -> Self {
        self.knowledge_space_id = Some(knowledge_space_id);
        self
    }

    pub fn with_workspace_id(mut self, workspace_id: WorkspaceId) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    pub fn with_knowledge_entity_id(mut self, knowledge_entity_id: EntityId) -> Self {
        self.knowledge_entity_id = Some(knowledge_entity_id);
        self
    }

    pub fn with_lifecycle_status(
        mut self,
        lifecycle_status: KnowledgeExposureLifecycleStatus,
    ) -> Self {
        self.lifecycle_status = Some(lifecycle_status);
        self
    }

    pub fn with_source_status(mut self, source_status: KnowledgeExposureSourceStatus) -> Self {
        self.source_status = Some(source_status);
        self
    }

    fn limit(&self) -> usize {
        self.limit
    }

    fn knowledge_space_id(&self) -> Option<KnowledgeSpaceId> {
        self.knowledge_space_id
    }

    fn workspace_id(&self) -> Option<WorkspaceId> {
        self.workspace_id
    }

    fn knowledge_entity_id(&self) -> Option<EntityId> {
        self.knowledge_entity_id
    }

    fn lifecycle_status(&self) -> Option<KnowledgeExposureLifecycleStatus> {
        self.lifecycle_status
    }

    fn source_status(&self) -> Option<KnowledgeExposureSourceStatus> {
        self.source_status
    }
}

impl Default for KnowledgeExposureListOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSpaceAvailableExposuresOptions {
    knowledge_space_id: KnowledgeSpaceId,
    limit: usize,
}

impl KnowledgeSpaceAvailableExposuresOptions {
    pub fn new(knowledge_space_id: KnowledgeSpaceId) -> Self {
        Self {
            knowledge_space_id,
            limit: 50,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(WorkVcsError::QueryInvalid(
                "knowledge space available exposures limit must be greater than zero".to_owned(),
            ));
        }
        self.limit = limit;
        Ok(self)
    }

    fn knowledge_space_id(&self) -> KnowledgeSpaceId {
        self.knowledge_space_id
    }

    fn limit(&self) -> usize {
        self.limit
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeExposureListResult {
    pub exposures: Vec<KnowledgeExposureSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSpaceAvailableExposuresResult {
    pub knowledge_space_id: KnowledgeSpaceId,
    pub exposures: Vec<KnowledgeExposureSnapshot>,
}

pub(crate) fn create_local_knowledge_exposure(
    connection: &mut StoreConnection,
    options: KnowledgeExposureCreateLocalOptions,
) -> Result<KnowledgeExposureCreateResult> {
    connection.verify_foreign_keys()?;
    knowledge_space(connection, options.knowledge_space_id())?;
    let knowledge_state_digest = knowledge_version_state_digest(
        connection,
        options.workspace_id(),
        options.knowledge_entity_id(),
        options.knowledge_entity_version_id(),
    )?;

    let exposure_id = ExposureId::new_v7();
    let transition_id = ExposureTransitionId::new_v7();
    let created_at_us = current_epoch_micros()?;
    let detail_json =
        canonical_object_json("knowledge exposure transition detail", options.detail())?;
    let transition_detail_digest = content_object_digest(detail_json.as_bytes());
    let transition_detail_size_bytes = usize_to_i64(
        "knowledge exposure transition detail_json size",
        detail_json.len(),
    )?;
    let source_status_detail = CanonicalValue::object(Vec::new())?;
    let source_status_detail_json = canonical_object_json(
        "knowledge exposure source status detail",
        &source_status_detail,
    )?;
    let source_status_detail_digest = content_object_digest(source_status_detail_json.as_bytes());
    let source_status_detail_size_bytes = usize_to_i64(
        "knowledge exposure source status detail_json size",
        source_status_detail_json.len(),
    )?;

    let exposure_id_bytes = exposure_id.raw_bytes();
    let transition_id_bytes = transition_id.raw_bytes();
    let knowledge_space_id_bytes = options.knowledge_space_id().raw_bytes();
    let workspace_id_bytes = options.workspace_id().raw_bytes();
    let knowledge_entity_id_bytes = options.knowledge_entity_id().raw_bytes();
    let knowledge_entity_version_id_bytes = options.knowledge_entity_version_id().raw_bytes();
    let lifecycle_status = KnowledgeExposureLifecycleStatus::Active;
    let source_status = KnowledgeExposureSourceStatus::Current;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    ensure_local_source_available(&transaction, &options)?;
    transaction
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &exposure_id_bytes[..],
                KNOWLEDGE_EXPOSURE_OBJECT_KIND,
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO knowledge_exposure(exposure_id, knowledge_space_id, created_at_us)
             VALUES (?1, ?2, ?3)",
            params![
                &exposure_id_bytes[..],
                &knowledge_space_id_bytes[..],
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO knowledge_exposure_local_source(
                exposure_id,
                workspace_id,
                knowledge_entity_id,
                knowledge_entity_version_id
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &exposure_id_bytes[..],
                &workspace_id_bytes[..],
                &knowledge_entity_id_bytes[..],
                &knowledge_entity_version_id_bytes[..]
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO knowledge_exposure_transition(
                transition_id,
                exposure_id,
                previous_transition_id,
                lifecycle_status,
                changed_at_us,
                event_id,
                detail_json
             )
             VALUES (?1, ?2, NULL, ?3, ?4, NULL, ?5)",
            params![
                &transition_id_bytes[..],
                &exposure_id_bytes[..],
                lifecycle_status.as_str(),
                created_at_us,
                detail_json
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO knowledge_exposure_current(
                exposure_id,
                transition_id,
                lifecycle_status,
                updated_at_us
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &exposure_id_bytes[..],
                &transition_id_bytes[..],
                lifecycle_status.as_str(),
                created_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO knowledge_exposure_source_status(
                exposure_id,
                source_status,
                checked_at_us,
                detail_json
             )
             VALUES (?1, ?2, ?3, ?4)",
            params![
                &exposure_id_bytes[..],
                source_status.as_str(),
                created_at_us,
                source_status_detail_json
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(KnowledgeExposureCreateResult {
        exposure: KnowledgeExposureSnapshot {
            exposure_id,
            knowledge_space_id: options.knowledge_space_id(),
            created_at_us,
            lifecycle_status,
            transition_id,
            previous_transition_id: None,
            changed_at_us: created_at_us,
            transition_detail: options.detail().clone(),
            transition_detail_digest,
            transition_detail_size_bytes,
            source: KnowledgeExposureLocalSourceSnapshot {
                workspace_id: options.workspace_id(),
                knowledge_entity_id: options.knowledge_entity_id(),
                knowledge_entity_version_id: options.knowledge_entity_version_id(),
                knowledge_state_digest,
            },
            source_status: KnowledgeExposureSourceStatusSnapshot {
                source_status,
                checked_at_us: created_at_us,
                detail: source_status_detail,
                detail_digest: source_status_detail_digest,
                detail_size_bytes: source_status_detail_size_bytes,
            },
        },
    })
}

pub(crate) fn withdraw_knowledge_exposure(
    connection: &mut StoreConnection,
    options: KnowledgeExposureWithdrawOptions,
) -> Result<KnowledgeExposureWithdrawResult> {
    connection.verify_foreign_keys()?;
    let detail_json =
        canonical_object_json("knowledge exposure withdrawal detail", options.detail())?;
    let exposure_id_bytes = options.exposure_id().raw_bytes();
    let expected_transition_id_bytes = options.expected_transition_id().raw_bytes();
    let transition_id = ExposureTransitionId::new_v7();
    let transition_id_bytes = transition_id.raw_bytes();
    let changed_at_us = current_epoch_micros()?;
    let withdrawn = KnowledgeExposureLifecycleStatus::Withdrawn;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let (current_transition_id, current_status) =
        load_current_transition(&transaction, options.exposure_id())?.ok_or_else(|| {
            WorkVcsError::QueryInvalid(format!(
                "KnowledgeExposure {} has no current transition",
                options.exposure_id()
            ))
        })?;
    if current_transition_id != options.expected_transition_id() {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {} expected transition {}, found {}",
            options.exposure_id(),
            options.expected_transition_id(),
            current_transition_id
        )));
    }
    if current_status != KnowledgeExposureLifecycleStatus::Active {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {} is already {}",
            options.exposure_id(),
            current_status.as_str()
        )));
    }
    transaction
        .execute(
            "INSERT INTO knowledge_exposure_transition(
                transition_id,
                exposure_id,
                previous_transition_id,
                lifecycle_status,
                changed_at_us,
                event_id,
                detail_json
             )
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)",
            params![
                &transition_id_bytes[..],
                &exposure_id_bytes[..],
                &expected_transition_id_bytes[..],
                withdrawn.as_str(),
                changed_at_us,
                detail_json
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "UPDATE knowledge_exposure_current
             SET transition_id = ?2,
                 lifecycle_status = ?3,
                 updated_at_us = ?4
             WHERE exposure_id = ?1",
            params![
                &exposure_id_bytes[..],
                &transition_id_bytes[..],
                withdrawn.as_str(),
                changed_at_us
            ],
        )
        .map_err(storage_error)?;
    transaction.commit().map_err(storage_error)?;

    Ok(KnowledgeExposureWithdrawResult {
        exposure: knowledge_exposure(connection, options.exposure_id())?,
    })
}

pub(crate) fn refresh_knowledge_exposure_source_status(
    connection: &mut StoreConnection,
    options: KnowledgeExposureRefreshSourceStatusOptions,
) -> Result<KnowledgeExposureRefreshSourceStatusResult> {
    connection.verify_foreign_keys()?;
    let exposure = knowledge_exposure(connection, options.exposure_id())?;
    let assessment = assess_local_source_status(connection, &exposure.source)?;
    let detail = assessment.to_detail()?;
    let detail_json = canonical_object_json("knowledge exposure source status detail", &detail)?;
    let checked_at_us = current_epoch_micros()?;
    let exposure_id_bytes = options.exposure_id().raw_bytes();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let updated = transaction
        .execute(
            "UPDATE knowledge_exposure_source_status
             SET source_status = ?2,
                 checked_at_us = ?3,
                 detail_json = ?4
             WHERE exposure_id = ?1",
            params![
                &exposure_id_bytes[..],
                assessment.source_status.as_str(),
                checked_at_us,
                detail_json
            ],
        )
        .map_err(storage_error)?;
    if updated != 1 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {} has no source-status projection",
            options.exposure_id()
        )));
    }
    transaction.commit().map_err(storage_error)?;

    Ok(KnowledgeExposureRefreshSourceStatusResult {
        exposure: knowledge_exposure(connection, options.exposure_id())?,
    })
}

pub(crate) fn knowledge_exposure(
    connection: &StoreConnection,
    exposure_id: ExposureId,
) -> Result<KnowledgeExposureSnapshot> {
    connection.verify_foreign_keys()?;
    load_knowledge_exposure_snapshot(connection, exposure_id)?.ok_or_else(|| {
        WorkVcsError::QueryInvalid(format!("KnowledgeExposure {exposure_id} not found"))
    })
}

pub(crate) fn knowledge_exposures(
    connection: &StoreConnection,
    options: KnowledgeExposureListOptions,
) -> Result<KnowledgeExposureListResult> {
    connection.verify_foreign_keys()?;
    let limit = usize_to_i64("knowledge exposure list limit", options.limit())?;
    let knowledge_space_id_bytes = options.knowledge_space_id().map(|id| id.raw_bytes());
    let workspace_id_bytes = options.workspace_id().map(|id| id.raw_bytes());
    let knowledge_entity_id_bytes = options.knowledge_entity_id().map(|id| id.raw_bytes());
    let lifecycle_status = options
        .lifecycle_status()
        .map(KnowledgeExposureLifecycleStatus::as_str);
    let source_status = options
        .source_status()
        .map(KnowledgeExposureSourceStatus::as_str);
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT knowledge_exposure.exposure_id
             FROM knowledge_exposure
             JOIN knowledge_exposure_current
               ON knowledge_exposure_current.exposure_id = knowledge_exposure.exposure_id
             JOIN knowledge_exposure_local_source
               ON knowledge_exposure_local_source.exposure_id = knowledge_exposure.exposure_id
             JOIN knowledge_exposure_source_status
               ON knowledge_exposure_source_status.exposure_id = knowledge_exposure.exposure_id
             WHERE (?2 IS NULL OR knowledge_exposure.knowledge_space_id = ?2)
               AND (?3 IS NULL OR knowledge_exposure_local_source.workspace_id = ?3)
               AND (?4 IS NULL OR knowledge_exposure_local_source.knowledge_entity_id = ?4)
               AND (?5 IS NULL OR knowledge_exposure_current.lifecycle_status = ?5)
               AND (?6 IS NULL OR knowledge_exposure_source_status.source_status = ?6)
             ORDER BY knowledge_exposure.created_at_us DESC, knowledge_exposure.exposure_id DESC
             LIMIT ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                limit,
                knowledge_space_id_bytes.as_ref().map(|bytes| &bytes[..]),
                workspace_id_bytes.as_ref().map(|bytes| &bytes[..]),
                knowledge_entity_id_bytes.as_ref().map(|bytes| &bytes[..]),
                lifecycle_status,
                source_status
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .map_err(storage_error)?;

    let mut exposures = Vec::new();
    for row in rows {
        let exposure_id = decode_exposure_id(
            "knowledge_exposure.exposure_id",
            row.map_err(storage_error)?,
        )?;
        exposures.push(knowledge_exposure(connection, exposure_id)?);
    }
    Ok(KnowledgeExposureListResult { exposures })
}

pub(crate) fn knowledge_space_available_exposures(
    connection: &StoreConnection,
    options: KnowledgeSpaceAvailableExposuresOptions,
) -> Result<KnowledgeSpaceAvailableExposuresResult> {
    connection.verify_foreign_keys()?;
    knowledge_space(connection, options.knowledge_space_id())?;
    let exposures = knowledge_exposures(
        connection,
        KnowledgeExposureListOptions::new()
            .with_limit(options.limit())?
            .with_knowledge_space_id(options.knowledge_space_id())
            .with_lifecycle_status(KnowledgeExposureLifecycleStatus::Active)
            .with_source_status(KnowledgeExposureSourceStatus::Current),
    )?
    .exposures;
    Ok(KnowledgeSpaceAvailableExposuresResult {
        knowledge_space_id: options.knowledge_space_id(),
        exposures,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalSourceStatusAssessment {
    source_status: KnowledgeExposureSourceStatus,
    checked_branch_heads: usize,
    matching_branch_heads: usize,
    drifted_branch_heads: usize,
    missing_branch_heads: usize,
}

impl LocalSourceStatusAssessment {
    fn to_detail(&self) -> Result<CanonicalValue> {
        CanonicalValue::object(vec![
            (
                "checked_branch_heads".to_owned(),
                CanonicalValue::safe_integer(usize_to_i64(
                    "checked_branch_heads",
                    self.checked_branch_heads,
                )?)?,
            ),
            (
                "drifted_branch_heads".to_owned(),
                CanonicalValue::safe_integer(usize_to_i64(
                    "drifted_branch_heads",
                    self.drifted_branch_heads,
                )?)?,
            ),
            (
                "matching_branch_heads".to_owned(),
                CanonicalValue::safe_integer(usize_to_i64(
                    "matching_branch_heads",
                    self.matching_branch_heads,
                )?)?,
            ),
            (
                "missing_branch_heads".to_owned(),
                CanonicalValue::safe_integer(usize_to_i64(
                    "missing_branch_heads",
                    self.missing_branch_heads,
                )?)?,
            ),
        ])
    }
}

fn assess_local_source_status(
    connection: &StoreConnection,
    source: &KnowledgeExposureLocalSourceSnapshot,
) -> Result<LocalSourceStatusAssessment> {
    let head_commits = load_workspace_head_commits(connection, source.workspace_id)?;
    let mut matching_branch_heads = 0;
    let mut drifted_branch_heads = 0;
    let mut missing_branch_heads = 0;
    for head_commit_id in &head_commits {
        let replayed = state_at(connection, *head_commit_id)?;
        if replayed.workspace_id != source.workspace_id {
            return Err(WorkVcsError::QueryInvalid(format!(
                "branch head {head_commit_id} belongs to workspace {}, not {}",
                replayed.workspace_id, source.workspace_id
            )));
        }
        match replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == source.knowledge_entity_id).then_some(*entity_version_id)
            }) {
            Some(entity_version_id) if entity_version_id == source.knowledge_entity_version_id => {
                matching_branch_heads += 1;
            }
            Some(_) => drifted_branch_heads += 1,
            None => missing_branch_heads += 1,
        }
    }
    let source_status = if matching_branch_heads > 0 {
        KnowledgeExposureSourceStatus::Current
    } else if drifted_branch_heads > 0 || missing_branch_heads > 0 {
        KnowledgeExposureSourceStatus::Stale
    } else {
        KnowledgeExposureSourceStatus::Unknown
    };
    Ok(LocalSourceStatusAssessment {
        source_status,
        checked_branch_heads: head_commits.len(),
        matching_branch_heads,
        drifted_branch_heads,
        missing_branch_heads,
    })
}

fn load_workspace_head_commits(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
) -> Result<Vec<CommitId>> {
    let workspace_id_bytes = workspace_id.raw_bytes();
    let mut statement = connection
        .inner()
        .prepare(
            "SELECT head_commit_id
             FROM branch
             WHERE workspace_id = ?1
             ORDER BY branch_id ASC",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![&workspace_id_bytes[..]], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(storage_error)?;

    let mut head_commits = Vec::new();
    for row in rows {
        head_commits.push(decode_commit_id(
            "branch.head_commit_id",
            row.map_err(storage_error)?,
        )?);
    }
    Ok(head_commits)
}

fn ensure_local_source_available(
    transaction: &Transaction<'_>,
    options: &KnowledgeExposureCreateLocalOptions,
) -> Result<()> {
    let knowledge_space_id_bytes = options.knowledge_space_id().raw_bytes();
    let workspace_id_bytes = options.workspace_id().raw_bytes();
    let knowledge_entity_id_bytes = options.knowledge_entity_id().raw_bytes();
    let knowledge_entity_version_id_bytes = options.knowledge_entity_version_id().raw_bytes();
    let existing = transaction
        .query_row(
            "SELECT knowledge_exposure.exposure_id
             FROM knowledge_exposure
             JOIN knowledge_exposure_local_source
               ON knowledge_exposure_local_source.exposure_id = knowledge_exposure.exposure_id
             WHERE knowledge_exposure.knowledge_space_id = ?1
               AND knowledge_exposure_local_source.workspace_id = ?2
               AND knowledge_exposure_local_source.knowledge_entity_id = ?3
               AND knowledge_exposure_local_source.knowledge_entity_version_id = ?4",
            params![
                &knowledge_space_id_bytes[..],
                &workspace_id_bytes[..],
                &knowledge_entity_id_bytes[..],
                &knowledge_entity_version_id_bytes[..]
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    if existing.is_some() {
        Err(WorkVcsError::QueryInvalid(
            "knowledge exposure local source is already exposed in this knowledge space".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn load_current_transition(
    transaction: &Transaction<'_>,
    exposure_id: ExposureId,
) -> Result<Option<(ExposureTransitionId, KnowledgeExposureLifecycleStatus)>> {
    let row = transaction
        .query_row(
            "SELECT transition_id, lifecycle_status
             FROM knowledge_exposure_current
             WHERE exposure_id = ?1",
            params![&exposure_id.raw_bytes()[..]],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error)?;
    row.map(|(transition_id, lifecycle_status)| {
        Ok((
            decode_exposure_transition_id(
                "knowledge_exposure_current.transition_id",
                transition_id,
            )?,
            KnowledgeExposureLifecycleStatus::parse(&lifecycle_status)?,
        ))
    })
    .transpose()
}

fn load_knowledge_exposure_snapshot(
    connection: &StoreConnection,
    exposure_id: ExposureId,
) -> Result<Option<KnowledgeExposureSnapshot>> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    knowledge_exposure.knowledge_space_id,
                    knowledge_exposure.created_at_us,
                    knowledge_exposure_current.lifecycle_status,
                    knowledge_exposure_current.transition_id,
                    knowledge_exposure_transition.previous_transition_id,
                    knowledge_exposure_transition.changed_at_us,
                    knowledge_exposure_transition.detail_json,
                    knowledge_exposure_source_status.source_status,
                    knowledge_exposure_source_status.checked_at_us,
                    knowledge_exposure_source_status.detail_json
             FROM knowledge_exposure
             JOIN object_identity
               ON object_identity.object_id = knowledge_exposure.exposure_id
             JOIN knowledge_exposure_current
               ON knowledge_exposure_current.exposure_id = knowledge_exposure.exposure_id
             JOIN knowledge_exposure_transition
               ON knowledge_exposure_transition.transition_id =
                  knowledge_exposure_current.transition_id
             JOIN knowledge_exposure_source_status
               ON knowledge_exposure_source_status.exposure_id = knowledge_exposure.exposure_id
             WHERE knowledge_exposure.exposure_id = ?1",
            params![&exposure_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, Option<Vec<u8>>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        object_kind,
        knowledge_space_id,
        created_at_us,
        lifecycle_status,
        transition_id,
        previous_transition_id,
        changed_at_us,
        transition_detail_json,
        source_status,
        source_status_checked_at_us,
        source_status_detail_json,
    )) = row
    else {
        return Ok(None);
    };
    if object_kind != KNOWLEDGE_EXPOSURE_OBJECT_KIND {
        return Err(WorkVcsError::QueryInvalid(format!(
            "knowledge exposure {exposure_id} has object kind {object_kind:?}"
        )));
    }
    validate_positive_i64("knowledge_exposure.created_at_us", created_at_us)?;
    validate_positive_i64("knowledge_exposure_transition.changed_at_us", changed_at_us)?;
    validate_positive_i64(
        "knowledge_exposure_source_status.checked_at_us",
        source_status_checked_at_us,
    )?;

    let lifecycle_status = KnowledgeExposureLifecycleStatus::parse(&lifecycle_status)?;
    let source = load_local_source_snapshot(connection, exposure_id)?;
    let transition_detail = parse_canonical_object_json(
        "knowledge_exposure_transition.detail_json",
        &transition_detail_json,
    )?;
    let source_status_detail = parse_canonical_object_json(
        "knowledge_exposure_source_status.detail_json",
        &source_status_detail_json,
    )?;

    Ok(Some(KnowledgeExposureSnapshot {
        exposure_id,
        knowledge_space_id: decode_knowledge_space_id(
            "knowledge_exposure.knowledge_space_id",
            knowledge_space_id,
        )?,
        created_at_us,
        lifecycle_status,
        transition_id: decode_exposure_transition_id(
            "knowledge_exposure_current.transition_id",
            transition_id,
        )?,
        previous_transition_id: decode_optional_exposure_transition_id(
            "knowledge_exposure_transition.previous_transition_id",
            previous_transition_id,
        )?,
        changed_at_us,
        transition_detail,
        transition_detail_digest: content_object_digest(transition_detail_json.as_bytes()),
        transition_detail_size_bytes: usize_to_i64(
            "knowledge_exposure_transition.detail_json size",
            transition_detail_json.len(),
        )?,
        source,
        source_status: KnowledgeExposureSourceStatusSnapshot {
            source_status: KnowledgeExposureSourceStatus::parse(&source_status)?,
            checked_at_us: source_status_checked_at_us,
            detail: source_status_detail,
            detail_digest: content_object_digest(source_status_detail_json.as_bytes()),
            detail_size_bytes: usize_to_i64(
                "knowledge_exposure_source_status.detail_json size",
                source_status_detail_json.len(),
            )?,
        },
    }))
}

fn load_local_source_snapshot(
    connection: &StoreConnection,
    exposure_id: ExposureId,
) -> Result<KnowledgeExposureLocalSourceSnapshot> {
    let exposure_id_bytes = exposure_id.raw_bytes();
    let external_source = connection
        .inner()
        .query_row(
            "SELECT external_ref_id
             FROM knowledge_exposure_external_source
             WHERE exposure_id = ?1",
            params![&exposure_id_bytes[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let row = connection
        .inner()
        .query_row(
            "SELECT workspace_id,
                    knowledge_entity_id,
                    knowledge_entity_version_id
             FROM knowledge_exposure_local_source
             WHERE exposure_id = ?1",
            params![&exposure_id_bytes[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    match (row, external_source) {
        (Some((workspace_id, knowledge_entity_id, knowledge_entity_version_id)), None) => {
            let workspace_id =
                decode_workspace_id("knowledge_exposure_local_source.workspace_id", workspace_id)?;
            let knowledge_entity_id = decode_entity_id(
                "knowledge_exposure_local_source.knowledge_entity_id",
                knowledge_entity_id,
            )?;
            let knowledge_entity_version_id = decode_entity_version_id(
                "knowledge_exposure_local_source.knowledge_entity_version_id",
                knowledge_entity_version_id,
            )?;
            let knowledge_state_digest = knowledge_version_state_digest(
                connection,
                workspace_id,
                knowledge_entity_id,
                knowledge_entity_version_id,
            )?;
            Ok(KnowledgeExposureLocalSourceSnapshot {
                workspace_id,
                knowledge_entity_id,
                knowledge_entity_version_id,
                knowledge_state_digest,
            })
        }
        (None, None) => Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} has no source family"
        ))),
        (Some(_), Some(_)) => Err(WorkVcsError::QueryInvalid(format!(
            "KnowledgeExposure {exposure_id} has both local and external source families"
        ))),
        (None, Some(_)) => Err(WorkVcsError::QueryInvalid(
            "external KnowledgeExposure sources are outside this slice".to_owned(),
        )),
    }
}

fn canonical_object_json(label: &str, value: &CanonicalValue) -> Result<String> {
    require_object_value(label, value)?;
    let bytes = canonical_bytes(value)?;
    String::from_utf8(bytes).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON was not UTF-8: {error}"))
    })
}

fn parse_canonical_object_json(label: &str, input: &str) -> Result<CanonicalValue> {
    let value = parse_canonical_json(input.as_bytes())?;
    require_object_value(label, &value)?;
    let reencoded = canonical_object_json(label, &value)?;
    if reencoded != input {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} is not canonical JSON"
        )));
    }
    Ok(value)
}

fn require_object_value(label: &str, value: &CanonicalValue) -> Result<()> {
    match value {
        CanonicalValue::Object(_) => Ok(()),
        _ => Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be a canonical object"
        ))),
    }
}

fn validate_positive_i64(label: &str, value: i64) -> Result<()> {
    if value <= 0 {
        return Err(WorkVcsError::QueryInvalid(format!(
            "{label} must be positive"
        )));
    }
    Ok(())
}

fn usize_to_i64(label: &str, value: usize) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| WorkVcsError::QueryInvalid(format!("{label} does not fit i64")))
}

fn decode_exposure_id(column: &str, bytes: Vec<u8>) -> Result<ExposureId> {
    let bytes = decode_uuid_bytes(column, bytes)?;
    ExposureId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_exposure_transition_id(column: &str, bytes: Vec<u8>) -> Result<ExposureTransitionId> {
    let bytes = decode_uuid_bytes(column, bytes)?;
    ExposureTransitionId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_optional_exposure_transition_id(
    column: &str,
    bytes: Option<Vec<u8>>,
) -> Result<Option<ExposureTransitionId>> {
    bytes
        .map(|bytes| decode_exposure_transition_id(column, bytes))
        .transpose()
}

fn decode_knowledge_space_id(column: &str, bytes: Vec<u8>) -> Result<KnowledgeSpaceId> {
    let bytes = decode_uuid_bytes(column, bytes)?;
    KnowledgeSpaceId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_uuid_bytes(column, bytes)?;
    WorkspaceId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_entity_id(column: &str, bytes: Vec<u8>) -> Result<EntityId> {
    let bytes = decode_uuid_bytes(column, bytes)?;
    EntityId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_entity_version_id(column: &str, bytes: Vec<u8>) -> Result<EntityVersionId> {
    let bytes = decode_uuid_bytes(column, bytes)?;
    EntityVersionId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_commit_id(column: &str, bytes: Vec<u8>) -> Result<CommitId> {
    let bytes = decode_uuid_bytes(column, bytes)?;
    CommitId::from_bytes(bytes)
        .map_err(|error| WorkVcsError::QueryInvalid(format!("{column}: {error}")))
}

fn decode_uuid_bytes(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::QueryInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}
