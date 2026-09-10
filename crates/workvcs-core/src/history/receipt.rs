use super::admission::{
    BranchRow, PreparedEntity, PreparedRelation, load_active_branch, move_branch_head,
    write_entity, write_entity_change_operation, write_entity_transition_change_operation,
    write_entity_version, write_relation, write_relation_change_operation,
};
use super::entity::canonical_json_string;
use super::goal::GOAL_ENTITY_KIND;
use super::knowledge::KNOWLEDGE_ENTITY_KIND;
use super::plan::PLAN_ENTITY_KIND;
use super::record::{
    RECORD_ENTITY_KIND, RecordKind, RecordState, RecordStatus, parse_record_state,
};
use super::state_at;
use super::task::{
    ACCEPTANCE_CRITERION_ENTITY_KIND, TASK_ENTITY_KIND, VERIFICATION_ENTITY_KIND,
    VERIFICATION_REQUIREMENT_ENTITY_KIND,
};
use crate::canonical::{
    CanonicalValue, ImportDigestDomain, WorkState, canonical_bytes, entity_version_digest,
    parse_canonical_json, relation_version_digest, validate_import_fixed_point,
    work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, RelationId,
    RelationVersionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Deserialize;
use std::{collections::BTreeMap, fmt};

pub(crate) const AUTHORIZATION_RECEIPT_ISSUE_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const AUTHORIZATION_RECEIPT_ISSUE_OPERATION_TYPE: &str = "authorization_receipt.issue";
pub(crate) const AUTHORIZATION_RECEIPT_CONSUME_OPERATION_SCHEMA_VERSION: i64 = 1;
pub(crate) const AUTHORIZATION_RECEIPT_CONSUME_OPERATION_TYPE: &str =
    "authorization_receipt.consume";

const AUTHORIZATION_RECEIPT_ISSUED_EVENT_KIND: &str = "authorization_receipt.issued";
const AUTHORIZATION_RECEIPT_CONSUMED_EVENT_KIND: &str = "authorization_receipt.consumed";
const AUTHORIZATION_RECEIPT_RELATION_TYPE: &str = "authorizes";
const AUTHORIZATION_RECEIPT_RELATION_DISCRIMINATOR: &str = "authorization_receipt";
const ENTITY_OBJECT_KIND: &str = "entity";
const NORMAL_COMMIT_KIND: &str = "normal";
const PRIMARY_PARENT_ROLE: &str = "primary";
const STATE_SCHEMA_VERSION: i64 = 1;
const ISSUE_PAYLOAD_DIGEST_DOMAIN: &str = "workvcs.authorization-receipt.issue-manifest.v1";
const CONSUME_PAYLOAD_DIGEST_DOMAIN: &str = "workvcs.authorization-receipt.consume-manifest.v1";
const AUTHORITY_REF_DIGEST_DOMAIN: &str = "workvcs.authorization-receipt.authority-ref.v1";

#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationReceiptIssueOptions {
    branch_id: BranchId,
    manifest: AuthorizationReceiptIssueManifest,
}

impl fmt::Debug for AuthorizationReceiptIssueOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationReceiptIssueOptions")
            .field("branch_id", &self.branch_id)
            .field("manifest", &self.manifest)
            .finish()
    }
}

impl AuthorizationReceiptIssueOptions {
    pub fn new(branch_id: BranchId, manifest: AuthorizationReceiptIssueManifest) -> Self {
        Self {
            branch_id,
            manifest,
        }
    }
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationReceiptIssueManifest {
    pub schema_version: i64,
    pub idempotency_key: String,
    pub expected_head_commit_id: String,
    #[serde(default)]
    pub expected_state_digest: Option<String>,
    pub target: AuthorizationReceiptTargetManifest,
    pub action: String,
    pub contract_digest_domain: String,
    pub contract_digest: String,
    pub authority_ref: AuthorizationReceiptAuthorityRefManifest,
    #[serde(default)]
    pub expires_at_us: Option<i64>,
    pub rationale: CanonicalValue,
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationReceiptConsumeManifest {
    pub schema_version: i64,
    pub idempotency_key: String,
    pub expected_head_commit_id: String,
    #[serde(default)]
    pub expected_state_digest: Option<String>,
    pub receipt_record_entity_id: String,
    pub expected_receipt_entity_version_id: String,
    pub expected_receipt_state_digest: String,
    pub target: AuthorizationReceiptTargetManifest,
    pub action: String,
    pub contract_digest: String,
    pub contract_digest_domain: String,
    pub rationale: CanonicalValue,
}

impl AuthorizationReceiptConsumeManifest {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let value = parse_canonical_json(bytes).map_err(|error| {
            WorkVcsError::RecordInvalid(format!(
                "authorization receipt consume manifest is not valid JSON: {error}"
            ))
        })?;
        let canonical = canonical_bytes(&value).map_err(record_invalid_from)?;
        serde_json::from_slice(&canonical).map_err(|error| {
            WorkVcsError::RecordInvalid(format!(
                "authorization receipt consume manifest has invalid shape: {error}"
            ))
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationReceiptConsumeOptions {
    branch_id: BranchId,
    manifest: AuthorizationReceiptConsumeManifest,
}

impl AuthorizationReceiptConsumeOptions {
    pub fn new(branch_id: BranchId, manifest: AuthorizationReceiptConsumeManifest) -> Self {
        Self {
            branch_id,
            manifest,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationReceiptConsumeOutcome {
    Consumed,
    Reused,
}
impl AuthorizationReceiptConsumeOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Consumed => "consumed",
            Self::Reused => "reused",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptConsumeResult {
    pub outcome: AuthorizationReceiptConsumeOutcome,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub idempotency_key: String,
    pub payload_digest: Digest,
    pub work_state_digest: Digest,
    pub receipt: AuthorizationReceiptResult,
}

impl fmt::Debug for AuthorizationReceiptIssueManifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationReceiptIssueManifest")
            .field("schema_version", &self.schema_version)
            .field("idempotency_key", &self.idempotency_key)
            .field("expected_head_commit_id", &self.expected_head_commit_id)
            .field("expected_state_digest", &self.expected_state_digest)
            .field("target", &self.target)
            .field("action", &self.action)
            .field("contract_digest_domain", &self.contract_digest_domain)
            .field("contract_digest", &self.contract_digest)
            .field("authority_ref", &self.authority_ref)
            .field("expires_at_us", &self.expires_at_us)
            .field("rationale", &self.rationale)
            .finish()
    }
}

impl AuthorizationReceiptIssueManifest {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let value = parse_canonical_json(bytes).map_err(|error| {
            WorkVcsError::RecordInvalid(format!(
                "authorization receipt issue manifest is not valid JSON: {error}"
            ))
        })?;
        let canonical = canonical_bytes(&value).map_err(record_invalid_from)?;
        serde_json::from_slice(&canonical).map_err(|error| {
            WorkVcsError::RecordInvalid(format!(
                "authorization receipt issue manifest has invalid shape: {error}"
            ))
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationReceiptTargetManifest {
    pub entity_id: String,
    pub entity_kind: String,
    pub expected_version_id: String,
    pub expected_state_digest: String,
}

#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationReceiptAuthorityRefManifest {
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
    pub authority_digest: String,
}

impl fmt::Debug for AuthorizationReceiptAuthorityRefManifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationReceiptAuthorityRefManifest")
            .field("kind", &self.kind)
            .field("reference_redacted", &true)
            .field("authority_digest", &self.authority_digest)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationReceiptOutcome {
    Created,
    Reused,
}

impl AuthorizationReceiptOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Reused => "reused",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptIssueResult {
    pub outcome: AuthorizationReceiptOutcome,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub idempotency_key: String,
    pub payload_digest: Digest,
    pub work_state_digest: Digest,
    pub receipt: AuthorizationReceiptResult,
    pub relation: AuthorizationReceiptRelationResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptResult {
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub status: RecordStatus,
    pub binding: AuthorizationReceiptBinding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptRelationResult {
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub state_digest: Digest,
    pub receipt_entity_id: EntityId,
    pub target_entity_id: EntityId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptBinding {
    pub branch_id: BranchId,
    pub target_entity_id: EntityId,
    pub target_entity_kind: String,
    pub target_entity_version_id: EntityVersionId,
    pub target_state_digest: Digest,
    pub action: String,
    pub contract_digest_domain: String,
    pub contract_digest: Digest,
    pub authority_ref_kind: String,
    pub authority_ref_digest: Digest,
    pub authority_digest: Digest,
    pub expires_at_us: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub receipt_entity_id: EntityId,
    pub receipt_entity_version_id: EntityVersionId,
    pub state_digest: Digest,
    pub status: RecordStatus,
    pub statement: String,
    pub binding: AuthorizationReceiptBinding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptListOptions {
    commit_id: CommitId,
    receipt_entity_id: Option<EntityId>,
    target_entity_id: Option<EntityId>,
    action: Option<String>,
    status: Option<RecordStatus>,
}

impl AuthorizationReceiptListOptions {
    pub fn new(commit_id: CommitId) -> Self {
        Self {
            commit_id,
            receipt_entity_id: None,
            target_entity_id: None,
            action: None,
            status: None,
        }
    }

    pub fn with_receipt_entity_id(mut self, receipt_entity_id: EntityId) -> Self {
        self.receipt_entity_id = Some(receipt_entity_id);
        self
    }

    pub fn with_target_entity_id(mut self, target_entity_id: EntityId) -> Self {
        self.target_entity_id = Some(target_entity_id);
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Result<Self> {
        let action = action.into();
        validate_text_field("action", &action)?;
        self.action = Some(action);
        Ok(self)
    }

    pub fn with_status(mut self, status: RecordStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptListResult {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub receipts: Vec<AuthorizationReceiptSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceiptRelationSnapshot {
    pub workspace_id: WorkspaceId,
    pub commit_id: CommitId,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub receipt_entity_id: EntityId,
    pub target_entity_id: EntityId,
    pub target_entity_kind: String,
    pub state_digest: Digest,
}

#[derive(Clone, Debug)]
struct PreparedIssue {
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    expected_state_digest: Option<Digest>,
    idempotency_key: String,
    payload_digest: Digest,
    rationale: CanonicalValue,
    receipt: PreparedEntity,
    binding: AuthorizationReceiptBinding,
    relation: PreparedRelation,
}

#[derive(Clone, Debug)]
struct LoadedEntityVersion {
    entity_kind: String,
    state_digest: Digest,
    state: CanonicalValue,
}

pub(crate) fn issue_authorization_receipt(
    connection: &mut StoreConnection,
    options: &AuthorizationReceiptIssueOptions,
) -> Result<AuthorizationReceiptIssueResult> {
    connection.verify_foreign_keys()?;
    let prepared = prepare_issue(options)?;
    let now_us = current_epoch_micros()?;
    validate_expiry_at_issue(prepared.binding.expires_at_us, now_us)?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let parent = state_at(connection, prepared.previous_head_commit_id)?;

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, prepared.branch_id)?;
    if let Some(reused) = find_idempotent_issue(
        &transaction,
        branch.workspace_id,
        prepared.branch_id,
        &prepared.idempotency_key,
        prepared.payload_digest,
    )? {
        return Ok(reused);
    }
    validate_branch_and_head(
        &transaction,
        &branch,
        prepared.branch_id,
        prepared.previous_head_commit_id,
        &parent.state,
        parent.workspace_id,
        parent.state_digest,
        prepared.expected_state_digest,
    )?;
    validate_target_current_version(
        &transaction,
        branch.workspace_id,
        &parent.state,
        &prepared.binding,
    )?;

    let next_state = issue_work_state(&parent.state, &prepared)?;
    let work_state_digest = work_state_mapping_digest(&next_state);
    let operation_payload = issue_payload_value(
        branch.workspace_id,
        commit_id,
        changeset_id,
        work_state_digest,
        &prepared,
    )?;
    let operation_payload_json = canonical_json_string(&operation_payload)?;
    let rationale_json = canonical_json_string(&prepared.rationale)?;
    write_issue(
        &transaction,
        branch.workspace_id,
        changeset_id,
        commit_id,
        now_us,
        work_state_digest,
        &operation_payload_json,
        &rationale_json,
        &prepared,
    )?;
    move_branch_head(
        &transaction,
        prepared.branch_id,
        prepared.previous_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;

    let mut result = issue_result_from_payload(&operation_payload)?;
    result.outcome = AuthorizationReceiptOutcome::Created;
    Ok(result)
}

pub(crate) fn consume_authorization_receipt(
    connection: &mut StoreConnection,
    options: &AuthorizationReceiptConsumeOptions,
) -> Result<AuthorizationReceiptConsumeResult> {
    connection.verify_foreign_keys()?;
    let manifest = &options.manifest;
    require_schema_version(manifest.schema_version)?;
    validate_idempotency_key(&manifest.idempotency_key)?;
    validate_structured_rationale(&manifest.rationale)?;
    validate_text_field("action", &manifest.action)?;
    validate_text_field("contract_digest_domain", &manifest.contract_digest_domain)?;
    let previous_head_commit_id = CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?;
    let receipt_entity_id = EntityId::parse_canonical(&manifest.receipt_record_entity_id)?;
    let expected_receipt_entity_version_id =
        EntityVersionId::parse_canonical(&manifest.expected_receipt_entity_version_id)?;
    let expected_receipt_state_digest = Digest::from_hex(&manifest.expected_receipt_state_digest)?;
    let target_entity_id = EntityId::parse_canonical(&manifest.target.entity_id)?;
    let target_entity_version_id =
        EntityVersionId::parse_canonical(&manifest.target.expected_version_id)?;
    let target_state_digest = Digest::from_hex(&manifest.target.expected_state_digest)?;
    let contract_digest = Digest::from_hex(&manifest.contract_digest)?;
    let manifest_value = consume_manifest_value(manifest)?;
    let payload_digest = Digest::domain_separated(
        CONSUME_PAYLOAD_DIGEST_DOMAIN,
        &canonical_bytes(&manifest_value)?,
    );
    let now_us = current_epoch_micros()?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();
    let parent = state_at(connection, previous_head_commit_id)?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, options.branch_id)?;
    if let Some(reused) = find_idempotent_consume(
        &transaction,
        branch.workspace_id,
        options.branch_id,
        &manifest.idempotency_key,
        payload_digest,
    )? {
        return Ok(reused);
    }
    validate_branch_and_head(
        &transaction,
        &branch,
        options.branch_id,
        previous_head_commit_id,
        &parent.state,
        parent.workspace_id,
        parent.state_digest,
        expected_state_digest,
    )?;
    let receipt = load_current_receipt(
        &transaction,
        branch.workspace_id,
        &parent.state,
        receipt_entity_id,
        expected_receipt_entity_version_id,
        expected_receipt_state_digest,
    )?;
    if receipt.binding.branch_id != options.branch_id
        || receipt.binding.target_entity_id != target_entity_id
        || receipt.binding.target_entity_kind != manifest.target.entity_kind
        || receipt.binding.target_entity_version_id != target_entity_version_id
        || receipt.binding.target_state_digest != target_state_digest
        || receipt.binding.action != manifest.action
        || receipt.binding.contract_digest_domain != manifest.contract_digest_domain
        || receipt.binding.contract_digest != contract_digest
    {
        return Err(WorkVcsError::RecordInvalid(
            "authorization receipt consume manifest does not match receipt scope".to_owned(),
        ));
    }
    validate_expiry_at_issue(receipt.binding.expires_at_us, now_us)?;
    validate_target_current_version(
        &transaction,
        branch.workspace_id,
        &parent.state,
        &receipt.binding,
    )?;
    let consumed_state = RecordState {
        kind: RecordKind::AuthorizationReceipt,
        statement: receipt.statement,
        scope: receipt.scope,
        status: RecordStatus::Consumed,
    }
    .to_canonical_value()?;
    let consumed_entity = PreparedEntity {
        entity_id: receipt_entity_id,
        entity_version_id: EntityVersionId::new_v7(),
        state_digest: entity_version_digest(&consumed_state).map_err(record_invalid_from)?,
        entity_kind: RECORD_ENTITY_KIND,
        state: consumed_state,
    };
    let mut entities = parent
        .state
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    entities.insert(receipt_entity_id, consumed_entity.entity_version_id);
    let next_state = WorkState::new(
        entities,
        parent
            .state
            .relations()
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>(),
    )?;
    let work_state_digest = work_state_mapping_digest(&next_state);
    let result = AuthorizationReceiptConsumeResult {
        outcome: AuthorizationReceiptConsumeOutcome::Consumed,
        workspace_id: branch.workspace_id,
        branch_id: options.branch_id,
        previous_head_commit_id,
        commit_id,
        changeset_id,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest,
        work_state_digest,
        receipt: AuthorizationReceiptResult {
            entity_id: receipt_entity_id,
            entity_version_id: consumed_entity.entity_version_id,
            state_digest: consumed_entity.state_digest,
            status: RecordStatus::Consumed,
            binding: receipt.binding,
        },
    };
    let payload = consume_payload_value(&result)?;
    let payload_json = canonical_json_string(&payload)?;
    let rationale_json = canonical_json_string(&manifest.rationale)?;
    write_entity_version(&transaction, &consumed_entity)?;
    write_changeset(
        &transaction,
        branch.workspace_id,
        changeset_id,
        AUTHORIZATION_RECEIPT_CONSUME_OPERATION_TYPE,
        AUTHORIZATION_RECEIPT_CONSUME_OPERATION_SCHEMA_VERSION,
        &payload_json,
        &rationale_json,
        now_us,
    )?;
    write_entity_transition_change_operation(
        &transaction,
        changeset_id,
        0,
        receipt_entity_id,
        Some(expected_receipt_entity_version_id),
        Some(consumed_entity.entity_version_id),
    )?;
    write_commit_and_event(
        &transaction,
        branch.workspace_id,
        changeset_id,
        commit_id,
        previous_head_commit_id,
        work_state_digest,
        AUTHORIZATION_RECEIPT_CONSUMED_EVENT_KIND,
        &payload_json,
        now_us,
    )?;
    move_branch_head(
        &transaction,
        options.branch_id,
        previous_head_commit_id,
        commit_id,
        now_us,
    )?;
    transaction.commit().map_err(storage_error)?;
    Ok(result)
}

struct CurrentReceipt {
    statement: String,
    scope: CanonicalValue,
    binding: AuthorizationReceiptBinding,
}

fn load_current_receipt(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    state: &WorkState,
    entity_id: EntityId,
    version_id: EntityVersionId,
    state_digest: Digest,
) -> Result<CurrentReceipt> {
    let found = state
        .entities()
        .iter()
        .find_map(|(id, version)| (*id == entity_id).then_some(*version))
        .ok_or_else(|| {
            WorkVcsError::RecordNotFound(format!(
                "authorization receipt {entity_id} is not present at head"
            ))
        })?;
    if found != version_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt {entity_id} expected version {version_id}, found {found}"
        )));
    }
    let loaded = load_entity_version(transaction, workspace_id, entity_id, version_id)?;
    if loaded.entity_kind != RECORD_ENTITY_KIND || loaded.state_digest != state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "authorization receipt {entity_id} does not match expected state digest"
        )));
    }
    let record = parse_record_state(loaded.state)?;
    if record.kind != RecordKind::AuthorizationReceipt || record.status != RecordStatus::Active {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt {entity_id} is not active"
        )));
    }
    Ok(CurrentReceipt {
        statement: record.statement,
        scope: record.scope.clone(),
        binding: binding_from_scope(&record.scope)?,
    })
}

fn prepare_issue(options: &AuthorizationReceiptIssueOptions) -> Result<PreparedIssue> {
    let manifest = &options.manifest;
    require_schema_version(manifest.schema_version)?;
    validate_idempotency_key(&manifest.idempotency_key)?;
    validate_structured_rationale(&manifest.rationale)?;
    validate_text_field("action", &manifest.action)?;
    validate_text_field("contract_digest_domain", &manifest.contract_digest_domain)?;
    validate_text_field("authority_ref.kind", &manifest.authority_ref.kind)?;
    validate_authority_reference(&manifest.authority_ref.reference)?;
    validate_text_field(
        "authority_ref.authority_digest",
        &manifest.authority_ref.authority_digest,
    )?;
    let previous_head_commit_id = CommitId::parse_canonical(&manifest.expected_head_commit_id)?;
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?;
    let binding = AuthorizationReceiptBinding {
        branch_id: options.branch_id,
        target_entity_id: EntityId::parse_canonical(&manifest.target.entity_id)?,
        target_entity_kind: validate_supported_target_kind(&manifest.target.entity_kind)?,
        target_entity_version_id: EntityVersionId::parse_canonical(
            &manifest.target.expected_version_id,
        )?,
        target_state_digest: Digest::from_hex(&manifest.target.expected_state_digest)?,
        action: manifest.action.clone(),
        contract_digest_domain: manifest.contract_digest_domain.clone(),
        contract_digest: Digest::from_hex(&manifest.contract_digest)?,
        authority_ref_kind: manifest.authority_ref.kind.clone(),
        authority_ref_digest: Digest::domain_separated(
            AUTHORITY_REF_DIGEST_DOMAIN,
            manifest.authority_ref.reference.as_bytes(),
        ),
        authority_digest: Digest::from_hex(&manifest.authority_ref.authority_digest)?,
        expires_at_us: manifest.expires_at_us,
    };
    validate_expiry_value(binding.expires_at_us)?;
    let manifest_value = issue_manifest_value(manifest)?;
    let payload_digest = Digest::domain_separated(
        ISSUE_PAYLOAD_DIGEST_DOMAIN,
        &canonical_bytes(&manifest_value)?,
    );
    let receipt_entity_id = EntityId::new_v7();
    let receipt_entity_version_id = EntityVersionId::new_v7();
    let scope = binding_to_scope(&binding)?;
    let state = RecordState {
        kind: RecordKind::AuthorizationReceipt,
        statement: format!(
            "authorization receipt for {} {} on {}",
            binding.action, binding.target_entity_kind, binding.target_entity_id
        ),
        scope,
        status: RecordStatus::Active,
    }
    .to_canonical_value()?;
    let state_digest = entity_version_digest(&state).map_err(record_invalid_from)?;
    let receipt = PreparedEntity {
        entity_id: receipt_entity_id,
        entity_version_id: receipt_entity_version_id,
        entity_kind: RECORD_ENTITY_KIND,
        state,
        state_digest,
    };
    let relation_state = CanonicalValue::object(Vec::new())?;
    let relation = PreparedRelation {
        relation_id: RelationId::new_v7(),
        relation_version_id: RelationVersionId::new_v7(),
        relation_type: AUTHORIZATION_RECEIPT_RELATION_TYPE,
        relation_discriminator: AUTHORIZATION_RECEIPT_RELATION_DISCRIMINATOR,
        source_entity_id: receipt_entity_id,
        target_entity_id: binding.target_entity_id,
        state_json: canonical_json_string(&relation_state)?,
        state_digest: relation_version_digest(&relation_state).map_err(record_invalid_from)?,
    };
    Ok(PreparedIssue {
        branch_id: options.branch_id,
        previous_head_commit_id,
        expected_state_digest,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest,
        rationale: manifest.rationale.clone(),
        receipt,
        binding,
        relation,
    })
}

fn validate_branch_and_head(
    transaction: &Transaction<'_>,
    branch: &BranchRow,
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    parent_state: &WorkState,
    parent_workspace_id: WorkspaceId,
    parent_state_digest: Digest,
    expected_state_digest: Option<Digest>,
) -> Result<()> {
    if branch.head_commit_id != previous_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {branch_id} expected head {previous_head_commit_id}, found {}",
            branch.head_commit_id
        )));
    }
    if parent_workspace_id != branch.workspace_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "branch {branch_id} belongs to workspace {}, but expected head {previous_head_commit_id} belongs to workspace {parent_workspace_id}",
            branch.workspace_id
        )));
    }
    let stored_digest = load_commit_state_digest(transaction, previous_head_commit_id)?;
    if stored_digest != parent_state_digest {
        return Err(WorkVcsError::RecordInvalid(format!(
            "expected head {previous_head_commit_id} replayed state digest {parent_state_digest} does not match stored digest {stored_digest}"
        )));
    }
    if let Some(expected_state_digest) = expected_state_digest
        && parent_state_digest != expected_state_digest
    {
        return Err(WorkVcsError::DigestInvalid(format!(
            "expected head {previous_head_commit_id} state digest {parent_state_digest} does not match expected {expected_state_digest}"
        )));
    }
    if parent_state.entities().is_empty() && parent_state.relations().is_empty() {
        return Err(WorkVcsError::RecordInvalid(format!(
            "expected head {previous_head_commit_id} has an empty WorkState"
        )));
    }
    Ok(())
}

fn validate_target_current_version(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    parent_state: &WorkState,
    binding: &AuthorizationReceiptBinding,
) -> Result<()> {
    let Some(current_version_id) =
        parent_state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == binding.target_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt target entity {} is not present at head",
            binding.target_entity_id
        )));
    };
    if current_version_id != binding.target_entity_version_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt target entity {} expected current version {}, found {}",
            binding.target_entity_id, binding.target_entity_version_id, current_version_id
        )));
    }
    let loaded = load_entity_version(
        transaction,
        workspace_id,
        binding.target_entity_id,
        binding.target_entity_version_id,
    )?;
    if loaded.entity_kind != binding.target_entity_kind {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt target entity {} has kind {:?}, not {:?}",
            binding.target_entity_id, loaded.entity_kind, binding.target_entity_kind
        )));
    }
    if loaded.state_digest != binding.target_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "authorization receipt target entity {} state digest {} does not match expected {}",
            binding.target_entity_id, loaded.state_digest, binding.target_state_digest
        )));
    }
    Ok(())
}

fn issue_work_state(parent_state: &WorkState, prepared: &PreparedIssue) -> Result<WorkState> {
    let mut entities = parent_state
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    if entities
        .insert(
            prepared.receipt.entity_id,
            prepared.receipt.entity_version_id,
        )
        .is_some()
    {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt generated duplicate entity id {}",
            prepared.receipt.entity_id
        )));
    }
    let mut relations = parent_state
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    if relations
        .insert(
            prepared.relation.relation_id,
            prepared.relation.relation_version_id,
        )
        .is_some()
    {
        return Err(WorkVcsError::RelationInvalid(format!(
            "authorization receipt generated duplicate relation id {}",
            prepared.relation.relation_id
        )));
    }
    WorkState::new(entities, relations)
}

fn write_issue(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    now_us: i64,
    work_state_digest: Digest,
    operation_payload_json: &str,
    rationale_json: &str,
    prepared: &PreparedIssue,
) -> Result<()> {
    write_entity(transaction, workspace_id, &prepared.receipt, now_us)?;
    write_relation(transaction, workspace_id, &prepared.relation, now_us)?;
    write_changeset(
        transaction,
        workspace_id,
        changeset_id,
        AUTHORIZATION_RECEIPT_ISSUE_OPERATION_TYPE,
        AUTHORIZATION_RECEIPT_ISSUE_OPERATION_SCHEMA_VERSION,
        operation_payload_json,
        rationale_json,
        now_us,
    )?;
    write_entity_change_operation(transaction, changeset_id, 0, &prepared.receipt)?;
    write_relation_change_operation(transaction, changeset_id, 1, &prepared.relation)?;
    write_commit_and_event(
        transaction,
        workspace_id,
        changeset_id,
        commit_id,
        prepared.previous_head_commit_id,
        work_state_digest,
        AUTHORIZATION_RECEIPT_ISSUED_EVENT_KIND,
        operation_payload_json,
        now_us,
    )
}

fn write_changeset(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    operation_type: &str,
    operation_schema_version: i64,
    operation_payload_json: &str,
    rationale_json: &str,
    now_us: i64,
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO changeset(
                changeset_id,
                workspace_id,
                operation_type,
                operation_schema_version,
                operation_payload_json,
                rationale_json,
                origin_session_id,
                created_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            params![
                &changeset_id.raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                operation_type,
                operation_schema_version,
                operation_payload_json,
                rationale_json,
                now_us
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn write_commit_and_event(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    changeset_id: ChangeSetId,
    commit_id: CommitId,
    parent_commit_id: CommitId,
    work_state_digest: Digest,
    event_kind: &str,
    event_payload_json: &str,
    now_us: i64,
) -> Result<()> {
    transaction
        .execute(
            "INSERT INTO workstate_commit(
                commit_id,
                workspace_id,
                changeset_id,
                commit_kind,
                state_digest,
                committed_at_us
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &commit_id.raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                &changeset_id.raw_bytes()[..],
                NORMAL_COMMIT_KIND,
                &work_state_digest.as_bytes()[..],
                now_us
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO commit_parent(
                commit_id,
                parent_ordinal,
                parent_role,
                parent_commit_id
             )
             VALUES (?1, 0, ?2, ?3)",
            params![
                &commit_id.raw_bytes()[..],
                PRIMARY_PARENT_ROLE,
                &parent_commit_id.raw_bytes()[..]
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO event(
                event_id,
                workspace_id,
                changeset_id,
                session_id,
                event_kind,
                occurred_at_us,
                payload_json
             )
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)",
            params![
                &EventId::new_v7().raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                &changeset_id.raw_bytes()[..],
                event_kind,
                now_us,
                event_payload_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn issue_payload_value(
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
    work_state_digest: Digest,
    prepared: &PreparedIssue,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "mode".to_owned(),
            CanonicalValue::String("issue".to_owned()),
        ),
        (
            "outcome".to_owned(),
            CanonicalValue::String(AuthorizationReceiptOutcome::Created.as_str().to_owned()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
        (
            "branch_id".to_owned(),
            CanonicalValue::String(prepared.branch_id.to_string()),
        ),
        (
            "previous_head_commit_id".to_owned(),
            CanonicalValue::String(prepared.previous_head_commit_id.to_string()),
        ),
        (
            "commit_id".to_owned(),
            CanonicalValue::String(commit_id.to_string()),
        ),
        (
            "changeset_id".to_owned(),
            CanonicalValue::String(changeset_id.to_string()),
        ),
        (
            "idempotency_key".to_owned(),
            CanonicalValue::String(prepared.idempotency_key.clone()),
        ),
        (
            "payload_digest".to_owned(),
            CanonicalValue::String(prepared.payload_digest.to_string()),
        ),
        (
            "work_state_digest".to_owned(),
            CanonicalValue::String(work_state_digest.to_string()),
        ),
        (
            "receipt".to_owned(),
            receipt_result_value(&AuthorizationReceiptResult {
                entity_id: prepared.receipt.entity_id,
                entity_version_id: prepared.receipt.entity_version_id,
                state_digest: prepared.receipt.state_digest,
                status: RecordStatus::Active,
                binding: prepared.binding.clone(),
            })?,
        ),
        (
            "relation".to_owned(),
            relation_result_value(&AuthorizationReceiptRelationResult {
                relation_id: prepared.relation.relation_id,
                relation_version_id: prepared.relation.relation_version_id,
                state_digest: prepared.relation.state_digest,
                receipt_entity_id: prepared.relation.source_entity_id,
                target_entity_id: prepared.relation.target_entity_id,
            })?,
        ),
    ])
}

fn receipt_result_value(result: &AuthorizationReceiptResult) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "entity_id".to_owned(),
            CanonicalValue::String(result.entity_id.to_string()),
        ),
        (
            "entity_version_id".to_owned(),
            CanonicalValue::String(result.entity_version_id.to_string()),
        ),
        (
            "state_digest".to_owned(),
            CanonicalValue::String(result.state_digest.to_string()),
        ),
        (
            "status".to_owned(),
            CanonicalValue::String(result.status.as_str().to_owned()),
        ),
        ("binding".to_owned(), binding_to_scope(&result.binding)?),
    ])
}

fn relation_result_value(result: &AuthorizationReceiptRelationResult) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "relation_id".to_owned(),
            CanonicalValue::String(result.relation_id.to_string()),
        ),
        (
            "relation_version_id".to_owned(),
            CanonicalValue::String(result.relation_version_id.to_string()),
        ),
        (
            "state_digest".to_owned(),
            CanonicalValue::String(result.state_digest.to_string()),
        ),
        (
            "receipt_entity_id".to_owned(),
            CanonicalValue::String(result.receipt_entity_id.to_string()),
        ),
        (
            "target_entity_id".to_owned(),
            CanonicalValue::String(result.target_entity_id.to_string()),
        ),
    ])
}

fn binding_to_scope(binding: &AuthorizationReceiptBinding) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "receipt_schema_version".to_owned(),
            CanonicalValue::safe_integer(1)?,
        ),
        (
            "branch_id".to_owned(),
            CanonicalValue::String(binding.branch_id.to_string()),
        ),
        (
            "target".to_owned(),
            CanonicalValue::object(vec![
                (
                    "entity_id".to_owned(),
                    CanonicalValue::String(binding.target_entity_id.to_string()),
                ),
                (
                    "entity_kind".to_owned(),
                    CanonicalValue::String(binding.target_entity_kind.clone()),
                ),
                (
                    "entity_version_id".to_owned(),
                    CanonicalValue::String(binding.target_entity_version_id.to_string()),
                ),
                (
                    "state_digest".to_owned(),
                    CanonicalValue::String(binding.target_state_digest.to_string()),
                ),
            ])?,
        ),
        (
            "action".to_owned(),
            CanonicalValue::String(binding.action.clone()),
        ),
        (
            "contract_digest_domain".to_owned(),
            CanonicalValue::String(binding.contract_digest_domain.clone()),
        ),
        (
            "contract_digest".to_owned(),
            CanonicalValue::String(binding.contract_digest.to_string()),
        ),
        (
            "authority_ref_kind".to_owned(),
            CanonicalValue::String(binding.authority_ref_kind.clone()),
        ),
        (
            "authority_ref_digest".to_owned(),
            CanonicalValue::String(binding.authority_ref_digest.to_string()),
        ),
        (
            "authority_digest".to_owned(),
            CanonicalValue::String(binding.authority_digest.to_string()),
        ),
        (
            "expires_at_us".to_owned(),
            optional_i64_value(binding.expires_at_us)?,
        ),
    ])
}

fn binding_from_scope(scope: &CanonicalValue) -> Result<AuthorizationReceiptBinding> {
    let entries = canonical_object_entries("authorization receipt scope", scope)?;
    let schema_version = required_i64(entries, "receipt_schema_version")?;
    if schema_version != 1 {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt scope schema version {schema_version} is not supported"
        )));
    }
    let branch_id = BranchId::parse_canonical(required_string(entries, "branch_id")?)?;
    let target_entries = canonical_object_entries(
        "authorization receipt target",
        required_value(entries, "target")?,
    )?;
    Ok(AuthorizationReceiptBinding {
        branch_id,
        target_entity_id: EntityId::parse_canonical(required_string(target_entries, "entity_id")?)?,
        target_entity_kind: validate_supported_target_kind(required_string(
            target_entries,
            "entity_kind",
        )?)?,
        target_entity_version_id: EntityVersionId::parse_canonical(required_string(
            target_entries,
            "entity_version_id",
        )?)?,
        target_state_digest: Digest::from_hex(required_string(target_entries, "state_digest")?)?,
        action: required_string(entries, "action")?.to_owned(),
        contract_digest_domain: required_string(entries, "contract_digest_domain")?.to_owned(),
        contract_digest: Digest::from_hex(required_string(entries, "contract_digest")?)?,
        authority_ref_kind: required_string(entries, "authority_ref_kind")?.to_owned(),
        authority_ref_digest: Digest::from_hex(required_string(entries, "authority_ref_digest")?)?,
        authority_digest: Digest::from_hex(required_string(entries, "authority_digest")?)?,
        expires_at_us: optional_i64_from_entries(entries, "expires_at_us")?,
    })
}

pub(crate) fn authorization_receipt_at(
    connection: &StoreConnection,
    commit_id: CommitId,
    receipt_entity_id: EntityId,
) -> Result<AuthorizationReceiptSnapshot> {
    let replayed = state_at(connection, commit_id)?;
    let Some(receipt_entity_version_id) =
        replayed
            .state
            .entities()
            .iter()
            .find_map(|(entity_id, entity_version_id)| {
                (*entity_id == receipt_entity_id).then_some(*entity_version_id)
            })
    else {
        return Err(WorkVcsError::RecordNotFound(format!(
            "authorization receipt {receipt_entity_id} is not present at commit {commit_id}"
        )));
    };
    let loaded = load_entity_version_readonly(
        connection,
        replayed.workspace_id,
        receipt_entity_id,
        receipt_entity_version_id,
    )?;
    if loaded.entity_kind != RECORD_ENTITY_KIND {
        return Err(WorkVcsError::RecordNotFound(format!(
            "entity {receipt_entity_id} has kind {:?}, not authorization_receipt record",
            loaded.entity_kind
        )));
    }
    let record = parse_record_state(loaded.state)?;
    if record.kind != RecordKind::AuthorizationReceipt {
        return Err(WorkVcsError::RecordNotFound(format!(
            "record {receipt_entity_id} has kind {}, not authorization_receipt",
            record.kind
        )));
    }
    Ok(AuthorizationReceiptSnapshot {
        workspace_id: replayed.workspace_id,
        commit_id,
        receipt_entity_id,
        receipt_entity_version_id,
        state_digest: loaded.state_digest,
        status: record.status,
        statement: record.statement,
        binding: binding_from_scope(&record.scope)?,
    })
}

pub(crate) fn authorization_receipts_at(
    connection: &StoreConnection,
    options: &AuthorizationReceiptListOptions,
) -> Result<AuthorizationReceiptListResult> {
    let replayed = state_at(connection, options.commit_id)?;
    let mut receipts = Vec::new();
    for (entity_id, entity_version_id) in replayed.state.entities() {
        if let Some(expected) = options.receipt_entity_id
            && *entity_id != expected
        {
            continue;
        }
        let Some(entity_kind) = load_entity_kind(connection, *entity_id)? else {
            return Err(WorkVcsError::RecordInvalid(format!(
                "WorkState at commit {} references missing entity {entity_id}",
                options.commit_id
            )));
        };
        if entity_kind != RECORD_ENTITY_KIND {
            continue;
        }
        let loaded = load_entity_version_readonly(
            connection,
            replayed.workspace_id,
            *entity_id,
            *entity_version_id,
        )?;
        let record = parse_record_state(loaded.state)?;
        if record.kind != RecordKind::AuthorizationReceipt {
            continue;
        }
        let binding = binding_from_scope(&record.scope)?;
        if options.status.is_some_and(|status| record.status != status) {
            continue;
        }
        if options
            .target_entity_id
            .is_some_and(|target_entity_id| binding.target_entity_id != target_entity_id)
        {
            continue;
        }
        if options
            .action
            .as_ref()
            .is_some_and(|action| binding.action != *action)
        {
            continue;
        }
        receipts.push(AuthorizationReceiptSnapshot {
            workspace_id: replayed.workspace_id,
            commit_id: options.commit_id,
            receipt_entity_id: *entity_id,
            receipt_entity_version_id: *entity_version_id,
            state_digest: loaded.state_digest,
            status: record.status,
            statement: record.statement,
            binding,
        });
    }
    receipts.sort_by_key(|receipt| receipt.receipt_entity_id);
    Ok(AuthorizationReceiptListResult {
        workspace_id: replayed.workspace_id,
        commit_id: options.commit_id,
        receipts,
    })
}

fn find_idempotent_issue(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    idempotency_key: &str,
    payload_digest: Digest,
) -> Result<Option<AuthorizationReceiptIssueResult>> {
    let mut statement = transaction
        .prepare(
            "SELECT changeset.operation_payload_json
             FROM changeset
             JOIN workstate_commit
               ON workstate_commit.changeset_id = changeset.changeset_id
             WHERE changeset.workspace_id = ?1
               AND changeset.operation_type = ?2
             ORDER BY changeset.created_at_us, changeset.changeset_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                &workspace_id.raw_bytes()[..],
                AUTHORIZATION_RECEIPT_ISSUE_OPERATION_TYPE
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage_error)?;
    let mut matched = None;
    for row in rows {
        let payload_json = row.map_err(storage_error)?;
        let value: serde_json::Value = serde_json::from_str(&payload_json).map_err(|error| {
            WorkVcsError::RecordInvalid(format!(
                "stored authorization receipt issue payload is invalid: {error}"
            ))
        })?;
        if value
            .get("idempotency_key")
            .and_then(|value| value.as_str())
            != Some(idempotency_key)
        {
            continue;
        }
        validate_idempotent_match(&value, branch_id, idempotency_key, payload_digest)?;
        let result = issue_result_from_json_value(&value, AuthorizationReceiptOutcome::Reused)?;
        if matched.replace(result).is_some() {
            return Err(WorkVcsError::RecordInvalid(format!(
                "idempotency key {idempotency_key:?} has multiple matching authorization receipt issue payloads"
            )));
        }
    }
    Ok(matched)
}

fn validate_idempotent_match(
    value: &serde_json::Value,
    branch_id: BranchId,
    idempotency_key: &str,
    payload_digest: Digest,
) -> Result<()> {
    let stored_branch = required_json_str(value, "branch_id")?;
    if stored_branch != branch_id.to_string() {
        return Err(WorkVcsError::RecordInvalid(format!(
            "idempotency key {idempotency_key:?} already belongs to branch {stored_branch}, not {branch_id}"
        )));
    }
    let stored_digest = Digest::from_hex(required_json_str(value, "payload_digest")?)?;
    if stored_digest != payload_digest {
        return Err(WorkVcsError::RecordInvalid(format!(
            "idempotency key {idempotency_key:?} was already used with payload digest {stored_digest}, not {payload_digest}"
        )));
    }
    Ok(())
}

fn issue_result_from_payload(value: &CanonicalValue) -> Result<AuthorizationReceiptIssueResult> {
    let json = canonical_json_string(value)?;
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
        WorkVcsError::RecordInvalid(format!(
            "authorization receipt issue payload is invalid: {error}"
        ))
    })?;
    issue_result_from_json_value(&value, AuthorizationReceiptOutcome::Created)
}

fn issue_result_from_json_value(
    value: &serde_json::Value,
    outcome: AuthorizationReceiptOutcome,
) -> Result<AuthorizationReceiptIssueResult> {
    Ok(AuthorizationReceiptIssueResult {
        outcome,
        workspace_id: WorkspaceId::parse_canonical(required_json_str(value, "workspace_id")?)?,
        branch_id: BranchId::parse_canonical(required_json_str(value, "branch_id")?)?,
        previous_head_commit_id: CommitId::parse_canonical(required_json_str(
            value,
            "previous_head_commit_id",
        )?)?,
        commit_id: CommitId::parse_canonical(required_json_str(value, "commit_id")?)?,
        changeset_id: ChangeSetId::parse_canonical(required_json_str(value, "changeset_id")?)?,
        idempotency_key: required_json_str(value, "idempotency_key")?.to_owned(),
        payload_digest: Digest::from_hex(required_json_str(value, "payload_digest")?)?,
        work_state_digest: Digest::from_hex(required_json_str(value, "work_state_digest")?)?,
        receipt: parse_receipt_result(required_json_object(value, "receipt")?)?,
        relation: parse_relation_result(required_json_object(value, "relation")?)?,
    })
}

fn parse_receipt_result(value: &serde_json::Value) -> Result<AuthorizationReceiptResult> {
    Ok(AuthorizationReceiptResult {
        entity_id: EntityId::parse_canonical(required_json_str(value, "entity_id")?)?,
        entity_version_id: EntityVersionId::parse_canonical(required_json_str(
            value,
            "entity_version_id",
        )?)?,
        state_digest: Digest::from_hex(required_json_str(value, "state_digest")?)?,
        status: RecordStatus::parse(required_json_str(value, "status")?)?,
        binding: binding_from_scope(&canonical_value_from_json_field(value, "binding")?)?,
    })
}

fn parse_relation_result(value: &serde_json::Value) -> Result<AuthorizationReceiptRelationResult> {
    Ok(AuthorizationReceiptRelationResult {
        relation_id: RelationId::parse_canonical(required_json_str(value, "relation_id")?)?,
        relation_version_id: RelationVersionId::parse_canonical(required_json_str(
            value,
            "relation_version_id",
        )?)?,
        state_digest: Digest::from_hex(required_json_str(value, "state_digest")?)?,
        receipt_entity_id: EntityId::parse_canonical(required_json_str(
            value,
            "receipt_entity_id",
        )?)?,
        target_entity_id: EntityId::parse_canonical(required_json_str(value, "target_entity_id")?)?,
    })
}

fn canonical_value_from_json_field(
    value: &serde_json::Value,
    field: &str,
) -> Result<CanonicalValue> {
    let field_value = value.get(field).ok_or_else(|| {
        WorkVcsError::RecordInvalid(format!(
            "authorization receipt payload missing field {field}"
        ))
    })?;
    let bytes = serde_json::to_vec(field_value).map_err(|error| {
        WorkVcsError::RecordInvalid(format!(
            "authorization receipt payload field {field} cannot be encoded: {error}"
        ))
    })?;
    parse_canonical_json(&bytes).map_err(record_invalid_from)
}

fn load_entity_version(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
) -> Result<LoadedEntityVersion> {
    let row = transaction
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &entity_id.raw_bytes()[..],
                &entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((object_kind, entity_workspace_id, entity_kind, schema_version, state_json, digest)) =
        row
    else {
        return Err(WorkVcsError::EntityNotFound(format!(
            "entity {entity_id} version {entity_version_id} does not exist"
        )));
    };
    validate_loaded_entity_version(
        workspace_id,
        entity_id,
        entity_version_id,
        object_kind,
        entity_workspace_id,
        entity_kind,
        schema_version,
        state_json,
        digest,
    )
}

fn load_entity_version_readonly(
    connection: &StoreConnection,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
) -> Result<LoadedEntityVersion> {
    let row = connection
        .inner()
        .query_row(
            "SELECT object_identity.object_kind,
                    entity.workspace_id,
                    entity.entity_kind,
                    entity_version.state_schema_version,
                    entity_version.state_json,
                    entity_version.state_digest
             FROM entity
             JOIN object_identity
               ON object_identity.object_id = entity.object_id
             JOIN entity_version
               ON entity_version.entity_id = entity.object_id
             WHERE entity.object_id = ?1
               AND entity_version.entity_version_id = ?2",
            params![
                &entity_id.raw_bytes()[..],
                &entity_version_id.raw_bytes()[..]
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((object_kind, entity_workspace_id, entity_kind, schema_version, state_json, digest)) =
        row
    else {
        return Err(WorkVcsError::EntityNotFound(format!(
            "entity {entity_id} version {entity_version_id} does not exist"
        )));
    };
    validate_loaded_entity_version(
        workspace_id,
        entity_id,
        entity_version_id,
        object_kind,
        entity_workspace_id,
        entity_kind,
        schema_version,
        state_json,
        digest,
    )
}

fn validate_loaded_entity_version(
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    entity_version_id: EntityVersionId,
    object_kind: String,
    entity_workspace_id: Vec<u8>,
    entity_kind: String,
    schema_version: i64,
    state_json: String,
    digest: Vec<u8>,
) -> Result<LoadedEntityVersion> {
    if object_kind != ENTITY_OBJECT_KIND {
        return Err(WorkVcsError::RecordInvalid(format!(
            "entity {entity_id} has object kind {object_kind:?}"
        )));
    }
    let entity_workspace_id = decode_workspace_id("entity.workspace_id", entity_workspace_id)?;
    if entity_workspace_id != workspace_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "entity {entity_id} belongs to workspace {entity_workspace_id}, not {workspace_id}"
        )));
    }
    if schema_version != STATE_SCHEMA_VERSION {
        return Err(WorkVcsError::RecordInvalid(format!(
            "entity {entity_id} version {entity_version_id} has state schema version {schema_version}"
        )));
    }
    let state_digest = decode_digest("entity_version.state_digest", digest)?;
    validate_import_fixed_point(
        state_json.as_bytes(),
        state_digest,
        ImportDigestDomain::EntityVersion,
    )
    .map_err(record_invalid_from)?;
    let state = parse_canonical_json(state_json.as_bytes()).map_err(record_invalid_from)?;
    let actual = entity_version_digest(&state).map_err(record_invalid_from)?;
    if actual != state_digest {
        return Err(WorkVcsError::RecordInvalid(format!(
            "entity {entity_id} version {entity_version_id} digest does not match state JSON"
        )));
    }
    Ok(LoadedEntityVersion {
        entity_kind,
        state_digest,
        state,
    })
}

fn load_entity_kind(connection: &StoreConnection, entity_id: EntityId) -> Result<Option<String>> {
    connection
        .inner()
        .query_row(
            "SELECT entity_kind
             FROM entity
             WHERE object_id = ?1",
            params![&entity_id.raw_bytes()[..]],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)
}

fn load_commit_state_digest(transaction: &Transaction<'_>, commit_id: CommitId) -> Result<Digest> {
    let row = transaction
        .query_row(
            "SELECT state_digest
             FROM workstate_commit
             WHERE commit_id = ?1",
            params![&commit_id.raw_bytes()[..]],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    let Some(state_digest) = row else {
        return Err(WorkVcsError::CommitNotFound(format!(
            "commit {commit_id} does not exist"
        )));
    };
    decode_digest("workstate_commit.state_digest", state_digest)
}

fn issue_manifest_value(manifest: &AuthorizationReceiptIssueManifest) -> Result<CanonicalValue> {
    let mut entries = vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(manifest.schema_version)?,
        ),
        (
            "idempotency_key".to_owned(),
            CanonicalValue::String(manifest.idempotency_key.clone()),
        ),
        (
            "expected_head_commit_id".to_owned(),
            CanonicalValue::String(manifest.expected_head_commit_id.clone()),
        ),
        (
            "target".to_owned(),
            target_manifest_value(&manifest.target)?,
        ),
        (
            "action".to_owned(),
            CanonicalValue::String(manifest.action.clone()),
        ),
        (
            "contract_digest_domain".to_owned(),
            CanonicalValue::String(manifest.contract_digest_domain.clone()),
        ),
        (
            "contract_digest".to_owned(),
            CanonicalValue::String(manifest.contract_digest.clone()),
        ),
        (
            "authority_ref".to_owned(),
            CanonicalValue::object(vec![
                (
                    "kind".to_owned(),
                    CanonicalValue::String(manifest.authority_ref.kind.clone()),
                ),
                (
                    "ref_digest".to_owned(),
                    CanonicalValue::String(
                        Digest::domain_separated(
                            AUTHORITY_REF_DIGEST_DOMAIN,
                            manifest.authority_ref.reference.as_bytes(),
                        )
                        .to_string(),
                    ),
                ),
                (
                    "authority_digest".to_owned(),
                    CanonicalValue::String(manifest.authority_ref.authority_digest.clone()),
                ),
            ])?,
        ),
        (
            "expires_at_us".to_owned(),
            optional_i64_value(manifest.expires_at_us)?,
        ),
        ("rationale".to_owned(), manifest.rationale.clone()),
    ];
    if let Some(expected_state_digest) = &manifest.expected_state_digest {
        entries.push((
            "expected_state_digest".to_owned(),
            CanonicalValue::String(expected_state_digest.clone()),
        ));
    }
    CanonicalValue::object(entries)
}

fn consume_manifest_value(
    manifest: &AuthorizationReceiptConsumeManifest,
) -> Result<CanonicalValue> {
    let mut entries = vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(manifest.schema_version)?,
        ),
        (
            "idempotency_key".to_owned(),
            CanonicalValue::String(manifest.idempotency_key.clone()),
        ),
        (
            "expected_head_commit_id".to_owned(),
            CanonicalValue::String(manifest.expected_head_commit_id.clone()),
        ),
        (
            "receipt_record_entity_id".to_owned(),
            CanonicalValue::String(manifest.receipt_record_entity_id.clone()),
        ),
        (
            "expected_receipt_entity_version_id".to_owned(),
            CanonicalValue::String(manifest.expected_receipt_entity_version_id.clone()),
        ),
        (
            "expected_receipt_state_digest".to_owned(),
            CanonicalValue::String(manifest.expected_receipt_state_digest.clone()),
        ),
        (
            "target".to_owned(),
            target_manifest_value(&manifest.target)?,
        ),
        (
            "action".to_owned(),
            CanonicalValue::String(manifest.action.clone()),
        ),
        (
            "contract_digest".to_owned(),
            CanonicalValue::String(manifest.contract_digest.clone()),
        ),
        (
            "contract_digest_domain".to_owned(),
            CanonicalValue::String(manifest.contract_digest_domain.clone()),
        ),
        ("rationale".to_owned(), manifest.rationale.clone()),
    ];
    if let Some(value) = &manifest.expected_state_digest {
        entries.push((
            "expected_state_digest".to_owned(),
            CanonicalValue::String(value.clone()),
        ));
    }
    CanonicalValue::object(entries)
}

fn consume_payload_value(result: &AuthorizationReceiptConsumeResult) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "mode".to_owned(),
            CanonicalValue::String("consume".to_owned()),
        ),
        (
            "outcome".to_owned(),
            CanonicalValue::String(result.outcome.as_str().to_owned()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(result.workspace_id.to_string()),
        ),
        (
            "branch_id".to_owned(),
            CanonicalValue::String(result.branch_id.to_string()),
        ),
        (
            "previous_head_commit_id".to_owned(),
            CanonicalValue::String(result.previous_head_commit_id.to_string()),
        ),
        (
            "commit_id".to_owned(),
            CanonicalValue::String(result.commit_id.to_string()),
        ),
        (
            "changeset_id".to_owned(),
            CanonicalValue::String(result.changeset_id.to_string()),
        ),
        (
            "idempotency_key".to_owned(),
            CanonicalValue::String(result.idempotency_key.clone()),
        ),
        (
            "payload_digest".to_owned(),
            CanonicalValue::String(result.payload_digest.to_string()),
        ),
        (
            "work_state_digest".to_owned(),
            CanonicalValue::String(result.work_state_digest.to_string()),
        ),
        ("receipt".to_owned(), receipt_result_value(&result.receipt)?),
    ])
}

fn find_idempotent_consume(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    idempotency_key: &str,
    payload_digest: Digest,
) -> Result<Option<AuthorizationReceiptConsumeResult>> {
    let mut statement = transaction
        .prepare(
            "SELECT changeset.operation_payload_json
             FROM changeset
             JOIN workstate_commit
               ON workstate_commit.changeset_id = changeset.changeset_id
              AND workstate_commit.workspace_id = changeset.workspace_id
             WHERE changeset.workspace_id = ?1
               AND changeset.operation_type = ?2
             ORDER BY changeset.created_at_us, changeset.changeset_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![
                &workspace_id.raw_bytes()[..],
                AUTHORIZATION_RECEIPT_CONSUME_OPERATION_TYPE
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage_error)?;
    let mut matched = None;
    for row in rows {
        let value: serde_json::Value =
            serde_json::from_str(&row.map_err(storage_error)?).map_err(|error| {
                WorkVcsError::RecordInvalid(format!(
                    "stored authorization receipt consume payload is invalid: {error}"
                ))
            })?;
        if value
            .get("idempotency_key")
            .and_then(|value| value.as_str())
            != Some(idempotency_key)
        {
            continue;
        }
        validate_idempotent_match(&value, branch_id, idempotency_key, payload_digest)?;
        let result =
            consume_result_from_json_value(&value, AuthorizationReceiptConsumeOutcome::Reused)?;
        if matched.replace(result).is_some() {
            return Err(WorkVcsError::RecordInvalid(format!(
                "idempotency key {idempotency_key:?} has multiple matching authorization receipt consume payloads"
            )));
        }
    }
    Ok(matched)
}

fn consume_result_from_json_value(
    value: &serde_json::Value,
    outcome: AuthorizationReceiptConsumeOutcome,
) -> Result<AuthorizationReceiptConsumeResult> {
    Ok(AuthorizationReceiptConsumeResult {
        outcome,
        workspace_id: WorkspaceId::parse_canonical(required_json_str(value, "workspace_id")?)?,
        branch_id: BranchId::parse_canonical(required_json_str(value, "branch_id")?)?,
        previous_head_commit_id: CommitId::parse_canonical(required_json_str(
            value,
            "previous_head_commit_id",
        )?)?,
        commit_id: CommitId::parse_canonical(required_json_str(value, "commit_id")?)?,
        changeset_id: ChangeSetId::parse_canonical(required_json_str(value, "changeset_id")?)?,
        idempotency_key: required_json_str(value, "idempotency_key")?.to_owned(),
        payload_digest: Digest::from_hex(required_json_str(value, "payload_digest")?)?,
        work_state_digest: Digest::from_hex(required_json_str(value, "work_state_digest")?)?,
        receipt: parse_receipt_result(required_json_object(value, "receipt")?)?,
    })
}

fn target_manifest_value(manifest: &AuthorizationReceiptTargetManifest) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "entity_id".to_owned(),
            CanonicalValue::String(manifest.entity_id.clone()),
        ),
        (
            "entity_kind".to_owned(),
            CanonicalValue::String(manifest.entity_kind.clone()),
        ),
        (
            "expected_version_id".to_owned(),
            CanonicalValue::String(manifest.expected_version_id.clone()),
        ),
        (
            "expected_state_digest".to_owned(),
            CanonicalValue::String(manifest.expected_state_digest.clone()),
        ),
    ])
}

fn required_json_object<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a serde_json::Value> {
    value
        .get(field)
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            WorkVcsError::RecordInvalid(format!(
                "authorization receipt payload missing object {field}"
            ))
        })
}

fn required_json_str<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            WorkVcsError::RecordInvalid(format!(
                "authorization receipt payload missing string {field}"
            ))
        })
}

fn canonical_object_entries<'a>(
    label: &str,
    value: &'a CanonicalValue,
) -> Result<&'a [(String, CanonicalValue)]> {
    match value {
        CanonicalValue::Object(entries) => Ok(entries),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "{label} must be a canonical object, found {other:?}"
        ))),
    }
}

fn required_value<'a>(
    entries: &'a [(String, CanonicalValue)],
    field: &str,
) -> Result<&'a CanonicalValue> {
    entries
        .iter()
        .find_map(|(key, value)| (key == field).then_some(value))
        .ok_or_else(|| WorkVcsError::RecordInvalid(format!("missing field {field}")))
}

fn required_string<'a>(entries: &'a [(String, CanonicalValue)], field: &str) -> Result<&'a str> {
    match required_value(entries, field)? {
        CanonicalValue::String(value) => Ok(value.as_str()),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "field {field} must be a string, found {other:?}"
        ))),
    }
}

fn required_i64(entries: &[(String, CanonicalValue)], field: &str) -> Result<i64> {
    match required_value(entries, field)? {
        CanonicalValue::Integer(value) => Ok(value.get()),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "field {field} must be an integer, found {other:?}"
        ))),
    }
}

fn optional_i64_from_entries(
    entries: &[(String, CanonicalValue)],
    field: &str,
) -> Result<Option<i64>> {
    match required_value(entries, field)? {
        CanonicalValue::Null => Ok(None),
        CanonicalValue::Integer(value) => Ok(Some(value.get())),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "field {field} must be null or an integer, found {other:?}"
        ))),
    }
}

fn optional_i64_value(value: Option<i64>) -> Result<CanonicalValue> {
    value
        .map(CanonicalValue::safe_integer)
        .transpose()
        .map(|value| value.unwrap_or(CanonicalValue::Null))
}

fn require_schema_version(schema_version: i64) -> Result<()> {
    if schema_version == 1 {
        Ok(())
    } else {
        Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt manifest schema_version {schema_version} is not supported"
        )))
    }
}

fn validate_idempotency_key(value: &str) -> Result<()> {
    validate_text_field("idempotency_key", value)
}

fn validate_text_field(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt {field} must not be empty"
        )));
    }
    if value.trim() != value {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt {field} must not have leading or trailing whitespace"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt {field} must not contain control characters"
        )));
    }
    Ok(())
}

fn validate_authority_reference(value: &str) -> Result<()> {
    validate_text_field("authority_ref.ref", value)?;
    let lower = value.to_ascii_lowercase();
    for marker in [
        "token=",
        "password=",
        "secret=",
        "credential=",
        "api_key=",
        "apikey=",
        "private_key",
        "bearer ",
    ] {
        if lower.contains(marker) {
            return Err(WorkVcsError::RecordInvalid(format!(
                "authorization receipt authority_ref.ref appears to contain sensitive material marker {marker:?}"
            )));
        }
    }
    Ok(())
}

fn validate_structured_rationale(value: &CanonicalValue) -> Result<()> {
    let entries = canonical_object_entries("authorization receipt rationale", value)?;
    if entries.is_empty() {
        return Err(WorkVcsError::RecordInvalid(
            "authorization receipt rationale must be a non-empty object".to_owned(),
        ));
    }
    Ok(())
}

fn validate_expiry_value(value: Option<i64>) -> Result<()> {
    if matches!(value, Some(value) if value <= 0) {
        return Err(WorkVcsError::RecordInvalid(
            "authorization receipt expires_at_us must be positive when provided".to_owned(),
        ));
    }
    Ok(())
}

fn validate_expiry_at_issue(expires_at_us: Option<i64>, now_us: i64) -> Result<()> {
    if let Some(expires_at_us) = expires_at_us
        && expires_at_us <= now_us
    {
        return Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt expires_at_us {expires_at_us} is not after issue time {now_us}"
        )));
    }
    Ok(())
}

fn validate_supported_target_kind(value: &str) -> Result<String> {
    validate_text_field("target.entity_kind", value)?;
    match value {
        GOAL_ENTITY_KIND
        | PLAN_ENTITY_KIND
        | TASK_ENTITY_KIND
        | ACCEPTANCE_CRITERION_ENTITY_KIND
        | VERIFICATION_REQUIREMENT_ENTITY_KIND
        | VERIFICATION_ENTITY_KIND
        | RECORD_ENTITY_KIND
        | KNOWLEDGE_ENTITY_KIND => Ok(value.to_owned()),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "authorization receipt target entity kind {other:?} is not supported"
        ))),
    }
}

fn decode_workspace_id(column: &str, bytes: Vec<u8>) -> Result<WorkspaceId> {
    let bytes = decode_16(column, bytes)?;
    WorkspaceId::from_bytes(bytes).map_err(record_invalid_from)
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RecordInvalid(format!("{column} must be 32 bytes, found {}", bytes.len()))
    })?;
    Ok(Digest::from_bytes(bytes))
}

fn decode_16(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::RecordInvalid(format!("{column} must be 16 bytes, found {}", bytes.len()))
    })
}

fn record_invalid_from(error: WorkVcsError) -> WorkVcsError {
    WorkVcsError::RecordInvalid(error.to_string())
}
