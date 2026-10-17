use super::{claim, session};
use crate::canonical::{CanonicalValue, canonical_bytes, parse_canonical_json};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::history::{self, BranchProjectionRefreshOptions};
use crate::identity::{
    BranchId, ClaimId, ContinuationRotationId, ContinuationScopeId, Digest, EntityId,
    EntityVersionId, EventId, SessionId, WorkspaceId,
};
use crate::runtime::SessionLifecycleState;
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest as ShaDigest, Sha256};
use std::collections::BTreeSet;

const SESSION_ROLE_METADATA_KEY: &str = "workvcs_session_role";
const THREAD_ID_METADATA_KEY: &str = "thread_id";
const WORK_SESSION_ROLE: &str = "work";
const LISTENER_SESSION_ROLE: &str = "listener";
const WORK_CANDIDATE_SESSION_ROLE: &str = "work_candidate";
const SUBAGENT_SESSION_ROLE: &str = "subagent";
const SIDE_CHAT_SESSION_ROLE: &str = "side_chat";
const PENDING_CREATION_STATUS: &str = "pending_creation";
const CREATION_DISPATCHED_STATUS: &str = "creation_dispatched";
const CREATION_UNKNOWN_STATUS: &str = "creation_unknown";
const CREATION_SUCCEEDED_STATUS: &str = "creation_succeeded";
const CANDIDATE_ATTACHED_STATUS: &str = "candidate_attached";
const CANDIDATE_ACCEPTED_STATUS: &str = "candidate_accepted";
const COMMITTED_STATUS: &str = "committed";
const ABORTED_STATUS: &str = "aborted";
const CURRENTNESS_DIGEST_DOMAIN: &str = "workvcs.continuation.currentness.v1";
const POLICY_DECISION_DIGEST_DOMAIN: &str = "workvcs.continuation.policy-decision.v1";
const CREATION_REQUEST_DIGEST_DOMAIN: &str = "workvcs.continuation.creation-request.v1";
const DISPATCH_DIGEST_DOMAIN: &str = "workvcs.continuation.dispatch.v1";
const CREATION_RECEIPT_DIGEST_DOMAIN: &str = "workvcs.continuation.creation-receipt.v1";
const RECOVERY_AUTHORITY_DIGEST_DOMAIN: &str = "workvcs.continuation.abort-only-authority.v1";
const RECOVERY_RECEIPT_DIGEST_DOMAIN: &str = "workvcs.continuation.abort-only-recovery.v1";
const RECOVERY_HISTORY_CHAIN_DIGEST_DOMAIN: &str =
    "workvcs.continuation.abort-only-recovery-history.v1";
const MAX_CONTINUATION_DOCUMENT_BYTES: usize = 65_536;
const MAX_CURRENTNESS_ENTITY_IDS: usize = 64;
const MAX_CONTINUATION_PACKET_BYTES: i64 = 8_192;
const MAX_CONTINUATION_PACKET_ITEMS: i64 = 64;
const MAX_CONTINUATION_PACKET_TOKEN_BUDGET: i64 = 4_096;
const MAX_CONTINUATION_EVIDENCE_REFERENCES: usize = 32;
const CONTINUATION_EVALUATOR_VERSION: &str = "wg-continuation-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuationSessionRole {
    Work,
    Listener,
    WorkCandidate,
    Subagent,
    SideChat,
}

impl ContinuationSessionRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => WORK_SESSION_ROLE,
            Self::Listener => LISTENER_SESSION_ROLE,
            Self::WorkCandidate => WORK_CANDIDATE_SESSION_ROLE,
            Self::Subagent => SUBAGENT_SESSION_ROLE,
            Self::SideChat => SIDE_CHAT_SESSION_ROLE,
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            WORK_SESSION_ROLE => Some(Self::Work),
            LISTENER_SESSION_ROLE => Some(Self::Listener),
            WORK_CANDIDATE_SESSION_ROLE => Some(Self::WorkCandidate),
            SUBAGENT_SESSION_ROLE => Some(Self::Subagent),
            SIDE_CHAT_SESSION_ROLE => Some(Self::SideChat),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuationRotationState {
    PendingCreation,
    CreationDispatched,
    CreationUnknown,
    CreationSucceeded,
    CandidateAttached,
    CandidateAccepted,
    Committed,
    Aborted,
}

impl ContinuationRotationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingCreation => PENDING_CREATION_STATUS,
            Self::CreationDispatched => CREATION_DISPATCHED_STATUS,
            Self::CreationUnknown => CREATION_UNKNOWN_STATUS,
            Self::CreationSucceeded => CREATION_SUCCEEDED_STATUS,
            Self::CandidateAttached => CANDIDATE_ATTACHED_STATUS,
            Self::CandidateAccepted => CANDIDATE_ACCEPTED_STATUS,
            Self::Committed => COMMITTED_STATUS,
            Self::Aborted => ABORTED_STATUS,
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            PENDING_CREATION_STATUS => Some(Self::PendingCreation),
            CREATION_DISPATCHED_STATUS => Some(Self::CreationDispatched),
            CREATION_UNKNOWN_STATUS => Some(Self::CreationUnknown),
            CREATION_SUCCEEDED_STATUS => Some(Self::CreationSucceeded),
            CANDIDATE_ATTACHED_STATUS => Some(Self::CandidateAttached),
            CANDIDATE_ACCEPTED_STATUS => Some(Self::CandidateAccepted),
            COMMITTED_STATUS => Some(Self::Committed),
            ABORTED_STATUS => Some(Self::Aborted),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationProjectPolicySnapshot {
    pub project_id: String,
    pub enabled: bool,
    pub revision: i64,
    pub actor_session_id: SessionId,
    pub authority: CanonicalValue,
    pub receipt_digest: Digest,
    pub updated_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationProjectPolicySetOptions {
    actor_session_id: SessionId,
    project_id: String,
    expected_revision: i64,
    enabled: bool,
    authority: CanonicalValue,
}

impl ContinuationProjectPolicySetOptions {
    pub fn new(
        actor_session_id: SessionId,
        project_id: impl Into<String>,
        expected_revision: i64,
        enabled: bool,
        authority: CanonicalValue,
    ) -> Result<Self> {
        let project_id =
            validate_embedded_identifier("continuation project_id", &project_id.into())?;
        if expected_revision < 0 {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation project policy expected revision must be non-negative".to_owned(),
            ));
        }
        validate_document("continuation project policy authority", &authority)?;
        Ok(Self {
            actor_session_id,
            project_id,
            expected_revision,
            enabled,
            authority,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationScopeSnapshot {
    pub scope_id: ContinuationScopeId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub task_entity_id: EntityId,
    pub listener_session_id: SessionId,
    pub listener_revision: i64,
    pub carrier_session_id: SessionId,
    pub carrier_claim_id: ClaimId,
    pub owner_epoch: i64,
    pub updated_at_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationRotationSnapshot {
    pub rotation_id: ContinuationRotationId,
    pub scope_id: ContinuationScopeId,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub task_entity_id: EntityId,
    pub initiating_listener_session_id: SessionId,
    pub initial_listener_revision: i64,
    pub listener_session_id: SessionId,
    pub listener_revision: i64,
    pub source_session_id: SessionId,
    pub source_claim_id: ClaimId,
    pub source_owner_epoch: i64,
    pub idempotency_key: String,
    pub creation_operation_id: String,
    pub policy_decision_digest: Digest,
    pub policy_decision: CanonicalValue,
    pub creation_request_digest: Digest,
    pub creation_request: CanonicalValue,
    pub handoff_digest: Digest,
    pub scope_currentness_digest: Digest,
    pub basis: CanonicalValue,
    pub state: ContinuationRotationState,
    pub abort_only: bool,
    pub recovery_revision: i64,
    pub recovery: Option<CanonicalValue>,
    pub recovery_receipt_digest: Option<Digest>,
    pub recovery_history_count: i64,
    pub recovery_history_chain_digest: Digest,
    pub dispatch: Option<CanonicalValue>,
    pub creation_receipt: Option<CanonicalValue>,
    pub initial_unknown_receipt: Option<CanonicalValue>,
    pub candidate_session_id: Option<SessionId>,
    pub candidate_binding: Option<CanonicalValue>,
    pub candidate_acceptance: Option<CanonicalValue>,
    pub candidate_acceptance_history: CanonicalValue,
    pub resulting_claim_id: Option<ClaimId>,
    pub result: Option<CanonicalValue>,
    pub created_at_us: i64,
    pub last_activity_at_us: i64,
    pub ended_at_us: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationAbortOnlyRecoveryOptions {
    rotation_id: ContinuationRotationId,
    expected_scope_id: ContinuationScopeId,
    actor_session_id: SessionId,
    expected_listener_session_id: SessionId,
    replacement_listener_session_id: SessionId,
    expected_listener_revision: i64,
    expected_recovery_revision: i64,
    expected_owner_epoch: i64,
    authority: CanonicalValue,
}

impl ContinuationAbortOnlyRecoveryOptions {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rotation_id: ContinuationRotationId,
        expected_scope_id: ContinuationScopeId,
        actor_session_id: SessionId,
        expected_listener_session_id: SessionId,
        replacement_listener_session_id: SessionId,
        expected_listener_revision: i64,
        expected_recovery_revision: i64,
        expected_owner_epoch: i64,
        authority: CanonicalValue,
    ) -> Result<Self> {
        validate_document("continuation abort-only recovery authority", &authority)?;
        require_positive_revision_epoch(expected_listener_revision, expected_owner_epoch)?;
        if expected_recovery_revision < 0 {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation expected recovery revision must be non-negative".to_owned(),
            ));
        }
        Ok(Self {
            rotation_id,
            expected_scope_id,
            actor_session_id,
            expected_listener_session_id,
            replacement_listener_session_id,
            expected_listener_revision,
            expected_recovery_revision,
            expected_owner_epoch,
            authority,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationAbortOnlyRecoveryPreviewOptions {
    rotation_id: ContinuationRotationId,
    expected_scope_id: ContinuationScopeId,
    expected_listener_session_id: SessionId,
    expected_listener_revision: i64,
    expected_recovery_revision: i64,
    expected_owner_epoch: i64,
}

impl ContinuationAbortOnlyRecoveryPreviewOptions {
    pub fn new(
        rotation_id: ContinuationRotationId,
        expected_scope_id: ContinuationScopeId,
        expected_listener_session_id: SessionId,
        expected_listener_revision: i64,
        expected_recovery_revision: i64,
        expected_owner_epoch: i64,
    ) -> Result<Self> {
        require_positive_revision_epoch(expected_listener_revision, expected_owner_epoch)?;
        if expected_recovery_revision < 0 {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation expected recovery revision must be non-negative".to_owned(),
            ));
        }
        Ok(Self {
            rotation_id,
            expected_scope_id,
            expected_listener_session_id,
            expected_listener_revision,
            expected_recovery_revision,
            expected_owner_epoch,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationAbortOnlyRecoveryPreview {
    pub rotation_id: ContinuationRotationId,
    pub scope_id: ContinuationScopeId,
    pub listener_session_id: SessionId,
    pub listener_revision: i64,
    pub listener_lifecycle_state: SessionLifecycleState,
    pub recovery_revision: i64,
    pub owner_epoch: i64,
    pub creation_operation_id: String,
    pub original_scope_currentness_digest: Digest,
    pub observed_scope_currentness_digest: Digest,
    pub currentness_drifted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationScopeCurrentnessOptions {
    source_session_id: SessionId,
    source_claim_id: ClaimId,
    expected_scope_id: ContinuationScopeId,
    expected_listener_session_id: SessionId,
    expected_listener_revision: i64,
    expected_owner_epoch: i64,
    handoff_digest: Digest,
    basis: CanonicalValue,
}

impl ContinuationScopeCurrentnessOptions {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_session_id: SessionId,
        source_claim_id: ClaimId,
        expected_scope_id: ContinuationScopeId,
        expected_listener_session_id: SessionId,
        expected_listener_revision: i64,
        expected_owner_epoch: i64,
        handoff_digest: Digest,
        basis: CanonicalValue,
    ) -> Result<Self> {
        validate_document("continuation basis", &basis)?;
        parse_basis(&basis)?;
        require_positive_revision_epoch(expected_listener_revision, expected_owner_epoch)?;
        Ok(Self {
            source_session_id,
            source_claim_id,
            expected_scope_id,
            expected_listener_session_id,
            expected_listener_revision,
            expected_owner_epoch,
            handoff_digest,
            basis,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationScopeCurrentnessSnapshot {
    pub scope: ContinuationScopeSnapshot,
    pub scope_currentness_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationScopeBindOptions {
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
    listener_session_id: SessionId,
    carrier_session_id: SessionId,
    carrier_claim_id: ClaimId,
}

impl ContinuationScopeBindOptions {
    pub fn new(
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        task_entity_id: EntityId,
        listener_session_id: SessionId,
        carrier_session_id: SessionId,
        carrier_claim_id: ClaimId,
    ) -> Self {
        Self {
            workspace_id,
            branch_id,
            task_entity_id,
            listener_session_id,
            carrier_session_id,
            carrier_claim_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationListenerReplaceOptions {
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
    expected_scope_id: ContinuationScopeId,
    actor_session_id: SessionId,
    expected_listener_session_id: SessionId,
    replacement_listener_session_id: SessionId,
    expected_listener_revision: i64,
    expected_owner_epoch: i64,
    authority: CanonicalValue,
}

impl ContinuationListenerReplaceOptions {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: WorkspaceId,
        branch_id: BranchId,
        task_entity_id: EntityId,
        expected_scope_id: ContinuationScopeId,
        actor_session_id: SessionId,
        expected_listener_session_id: SessionId,
        replacement_listener_session_id: SessionId,
        expected_listener_revision: i64,
        expected_owner_epoch: i64,
        authority: CanonicalValue,
    ) -> Result<Self> {
        validate_document("continuation listener replacement authority", &authority)?;
        require_positive_revision_epoch(expected_listener_revision, expected_owner_epoch)?;
        Ok(Self {
            workspace_id,
            branch_id,
            task_entity_id,
            expected_scope_id,
            actor_session_id,
            expected_listener_session_id,
            replacement_listener_session_id,
            expected_listener_revision,
            expected_owner_epoch,
            authority,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationRotationBeginOptions {
    listener_session_id: SessionId,
    source_session_id: SessionId,
    source_claim_id: ClaimId,
    expected_scope_id: ContinuationScopeId,
    expected_listener_revision: i64,
    expected_owner_epoch: i64,
    idempotency_key: String,
    creation_operation_id: String,
    handoff_digest: Digest,
    basis: CanonicalValue,
    policy_decision: CanonicalValue,
    creation_request: CanonicalValue,
}

impl ContinuationRotationBeginOptions {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        listener_session_id: SessionId,
        source_session_id: SessionId,
        source_claim_id: ClaimId,
        expected_scope_id: ContinuationScopeId,
        expected_listener_revision: i64,
        expected_owner_epoch: i64,
        idempotency_key: impl Into<String>,
        creation_operation_id: impl Into<String>,
        handoff_digest: Digest,
        basis: CanonicalValue,
        policy_decision: CanonicalValue,
        creation_request: CanonicalValue,
    ) -> Result<Self> {
        let idempotency_key =
            validate_bounded_text("continuation idempotency key", idempotency_key.into(), 512)?;
        let creation_operation_id = validate_bounded_text(
            "continuation creation operation id",
            creation_operation_id.into(),
            512,
        )?;
        validate_document("continuation basis", &basis)?;
        validate_document("continuation policy decision", &policy_decision)?;
        validate_document("continuation creation request", &creation_request)?;
        parse_basis(&basis)?;
        require_positive_revision_epoch(expected_listener_revision, expected_owner_epoch)?;
        Ok(Self {
            listener_session_id,
            source_session_id,
            source_claim_id,
            expected_scope_id,
            expected_listener_revision,
            expected_owner_epoch,
            idempotency_key,
            creation_operation_id,
            handoff_digest,
            basis,
            policy_decision,
            creation_request,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationCreationDispatchOptions {
    rotation_id: ContinuationRotationId,
    listener_session_id: SessionId,
    expected_listener_revision: i64,
    dispatch: CanonicalValue,
}

impl ContinuationCreationDispatchOptions {
    pub fn new(
        rotation_id: ContinuationRotationId,
        listener_session_id: SessionId,
        expected_listener_revision: i64,
        dispatch: CanonicalValue,
    ) -> Result<Self> {
        validate_document("continuation creation dispatch", &dispatch)?;
        Ok(Self {
            rotation_id,
            listener_session_id,
            expected_listener_revision,
            dispatch,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationCreationResultOptions {
    rotation_id: ContinuationRotationId,
    listener_session_id: SessionId,
    expected_listener_revision: i64,
    receipt: CanonicalValue,
}

impl ContinuationCreationResultOptions {
    pub fn new(
        rotation_id: ContinuationRotationId,
        listener_session_id: SessionId,
        expected_listener_revision: i64,
        receipt: CanonicalValue,
    ) -> Result<Self> {
        validate_document("continuation creation result receipt", &receipt)?;
        Ok(Self {
            rotation_id,
            listener_session_id,
            expected_listener_revision,
            receipt,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationCandidateAttachOptions {
    rotation_id: ContinuationRotationId,
    listener_session_id: SessionId,
    expected_listener_revision: i64,
    candidate_session_id: SessionId,
    candidate_binding: CanonicalValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationCandidateAcceptOptions {
    rotation_id: ContinuationRotationId,
    candidate_session_id: SessionId,
    acceptance: CanonicalValue,
}

impl ContinuationCandidateAcceptOptions {
    pub fn new(
        rotation_id: ContinuationRotationId,
        candidate_session_id: SessionId,
        acceptance: CanonicalValue,
    ) -> Result<Self> {
        validate_document("continuation candidate acceptance", &acceptance)?;
        Ok(Self {
            rotation_id,
            candidate_session_id,
            acceptance,
        })
    }
}

impl ContinuationCandidateAttachOptions {
    pub fn new(
        rotation_id: ContinuationRotationId,
        listener_session_id: SessionId,
        expected_listener_revision: i64,
        candidate_session_id: SessionId,
        candidate_binding: CanonicalValue,
    ) -> Result<Self> {
        require_object("continuation candidate binding", &candidate_binding)?;
        Ok(Self {
            rotation_id,
            listener_session_id,
            expected_listener_revision,
            candidate_session_id,
            candidate_binding,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationRotationAbortOptions {
    rotation_id: ContinuationRotationId,
    listener_session_id: SessionId,
    expected_listener_revision: i64,
    rationale: CanonicalValue,
}

impl ContinuationRotationAbortOptions {
    pub fn new(
        rotation_id: ContinuationRotationId,
        listener_session_id: SessionId,
        expected_listener_revision: i64,
        rationale: CanonicalValue,
    ) -> Result<Self> {
        validate_document("continuation abort rationale", &rationale)?;
        Ok(Self {
            rotation_id,
            listener_session_id,
            expected_listener_revision,
            rationale,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationRotationCommitOptions {
    rotation_id: ContinuationRotationId,
    listener_session_id: SessionId,
    expected_listener_revision: i64,
    expected_owner_epoch: i64,
    expected_handoff_digest: Digest,
}

impl ContinuationRotationCommitOptions {
    pub fn new(
        rotation_id: ContinuationRotationId,
        listener_session_id: SessionId,
        expected_listener_revision: i64,
        expected_owner_epoch: i64,
        expected_handoff_digest: Digest,
    ) -> Self {
        Self {
            rotation_id,
            listener_session_id,
            expected_listener_revision,
            expected_owner_epoch,
            expected_handoff_digest,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuationRotationCommitResult {
    pub rotation: ContinuationRotationSnapshot,
    pub scope: ContinuationScopeSnapshot,
    pub previous_claim_id: ClaimId,
    pub resulting_claim_id: ClaimId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RotationOccurrence {
    rotation_id: ContinuationRotationId,
    scope_id: ContinuationScopeId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
    initial_listener_session_id: SessionId,
    initial_listener_revision: i64,
    source_session_id: SessionId,
    source_claim_id: ClaimId,
    source_owner_epoch: i64,
    idempotency_key: String,
    creation_operation_id: String,
    policy_decision_digest: Digest,
    policy_decision: CanonicalValue,
    policy_decision_json: String,
    creation_request_digest: Digest,
    creation_request: CanonicalValue,
    creation_request_json: String,
    handoff_digest: Digest,
    basis: CanonicalValue,
    basis_json: String,
    created_at_us: i64,
}

#[derive(Clone, Debug)]
struct ActiveRotationProjection {
    state: ContinuationRotationState,
    listener_session_id: SessionId,
    listener_revision: i64,
    scope_currentness_digest: Digest,
    abort_only: bool,
    recovery_revision: i64,
    recovery: Option<CanonicalValue>,
    dispatch: Option<CanonicalValue>,
    creation_receipt: Option<CanonicalValue>,
    initial_unknown_receipt: Option<CanonicalValue>,
    candidate_session_id: Option<SessionId>,
    candidate_binding: Option<CanonicalValue>,
    candidate_acceptance: Option<CanonicalValue>,
    candidate_acceptance_history: CanonicalValue,
}

#[derive(Clone, Debug)]
struct ContinuationBasis {
    project_id: String,
    source_thread_id: String,
    handoff_record_id: EntityId,
    currentness_entity_ids: Vec<EntityId>,
}

#[derive(Clone, Debug)]
struct ProjectedEntity {
    entity_version_id: EntityVersionId,
    state_digest: Digest,
    entity_kind: String,
    record_kind: Option<String>,
}

pub(crate) fn set_continuation_project_policy(
    connection: &mut StoreConnection,
    options: &ContinuationProjectPolicySetOptions,
) -> Result<ContinuationProjectPolicySnapshot> {
    connection.verify_foreign_keys()?;
    let authority_json =
        canonical_json("continuation project policy authority", &options.authority)?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let actor =
        session::load_active_session_runtime_for_update(&transaction, options.actor_session_id)?;
    validate_project_policy_authority(options)?;
    if let Some(existing) = load_project_policy_optional(&transaction, &options.project_id)? {
        if existing.revision == options.expected_revision + 1
            && existing.enabled == options.enabled
            && existing.actor_session_id == options.actor_session_id
            && canonical_json(
                "stored continuation project policy authority",
                &existing.authority,
            )? == authority_json
        {
            return Ok(existing);
        }
        if existing.revision != options.expected_revision {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "continuation project policy revision is {}, expected {}",
                existing.revision, options.expected_revision
            )));
        }
    } else if options.expected_revision != 0 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation project policy does not exist at the expected revision".to_owned(),
        ));
    }
    let revision = options.expected_revision.checked_add(1).ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation project policy revision overflow".to_owned(),
        )
    })?;
    let now_us = current_epoch_micros()?;
    let authority_digest = Digest::domain_separated(
        "workvcs.continuation.project-policy-authority.v1",
        authority_json.as_bytes(),
    );
    let receipt_document = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1)?,
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(options.project_id.clone()),
        ),
        ("enabled".to_owned(), CanonicalValue::Bool(options.enabled)),
        (
            "revision".to_owned(),
            CanonicalValue::safe_integer(revision)?,
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(options.actor_session_id.to_string()),
        ),
        (
            "authority_digest".to_owned(),
            CanonicalValue::String(authority_digest.to_string()),
        ),
        (
            "updated_at_us".to_owned(),
            CanonicalValue::safe_integer(now_us)?,
        ),
    ])?;
    let receipt_digest = document_digest(
        "workvcs.continuation.project-policy-receipt.v1",
        "continuation project policy receipt",
        &receipt_document,
    )?;
    let changed = transaction
        .execute(
            "INSERT INTO continuation_project_policy(
                project_id, enabled, revision, actor_session_id,
                authority_json, receipt_digest, updated_at_us
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(project_id) DO UPDATE SET
                enabled = excluded.enabled,
                revision = excluded.revision,
                actor_session_id = excluded.actor_session_id,
                authority_json = excluded.authority_json,
                receipt_digest = excluded.receipt_digest,
                updated_at_us = excluded.updated_at_us
             WHERE continuation_project_policy.revision = ?8",
            params![
                options.project_id,
                i64::from(options.enabled),
                revision,
                &options.actor_session_id.raw_bytes()[..],
                authority_json,
                &receipt_digest.as_bytes()[..],
                now_us,
                options.expected_revision,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation project policy compare-and-swap failed".to_owned(),
        ));
    }
    insert_event(
        &transaction,
        actor.active_workspace_id,
        options.actor_session_id,
        "continuation.project_policy_changed",
        now_us,
        receipt_document,
    )?;
    transaction.commit().map_err(storage_error)?;
    continuation_project_policy(connection.inner(), &options.project_id)
}

pub(crate) fn continuation_project_policy(
    connection: &Connection,
    project_id: &str,
) -> Result<ContinuationProjectPolicySnapshot> {
    load_project_policy_optional(connection, project_id)?.ok_or_else(|| {
        WorkVcsError::ContinuationNotFound(format!(
            "continuation project policy {project_id:?} does not exist"
        ))
    })
}

pub(crate) fn preview_continuation_scope_currentness(
    connection: &mut StoreConnection,
    options: &ContinuationScopeCurrentnessOptions,
) -> Result<ContinuationScopeCurrentnessSnapshot> {
    connection.verify_foreign_keys()?;
    let initial_scope = load_scope_by_carrier(
        connection.inner(),
        options.source_session_id,
        options.source_claim_id,
    )?;
    refresh_projection_for_branch(connection, initial_scope.branch_id)?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let scope = load_scope_by_carrier(
        &transaction,
        options.source_session_id,
        options.source_claim_id,
    )?;
    if scope.scope_id != options.expected_scope_id
        || scope.listener_session_id != options.expected_listener_session_id
        || scope.listener_revision != options.expected_listener_revision
        || scope.owner_epoch != options.expected_owner_epoch
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation currentness preview compare-and-swap failed".to_owned(),
        ));
    }
    require_source_role_and_provenance(
        &transaction,
        options.source_session_id,
        options.source_claim_id,
        &scope,
    )?;
    require_active_exclusive_claim(
        &transaction,
        options.source_claim_id,
        options.source_session_id,
        scope.workspace_id,
        scope.branch_id,
        scope.task_entity_id,
    )?;
    let basis = parse_basis(&options.basis)?;
    require_session_thread_id(
        &transaction,
        options.source_session_id,
        &basis.source_thread_id,
        "source",
    )?;
    let scope_currentness_digest = compute_scope_currentness(
        &transaction,
        &scope,
        options.source_session_id,
        options.source_claim_id,
        options.handoff_digest,
        &options.basis,
    )?;
    transaction.commit().map_err(storage_error)?;
    Ok(ContinuationScopeCurrentnessSnapshot {
        scope,
        scope_currentness_digest,
    })
}

pub(crate) fn preview_abort_only_recovery(
    connection: &mut StoreConnection,
    options: &ContinuationAbortOnlyRecoveryPreviewOptions,
) -> Result<ContinuationAbortOnlyRecoveryPreview> {
    connection.verify_foreign_keys()?;
    let initial = load_rotation_snapshot(connection.inner(), options.rotation_id)?;
    if matches!(
        initial.state,
        ContinuationRotationState::Committed | ContinuationRotationState::Aborted
    ) {
        return Err(WorkVcsError::ContinuationInvalid(
            "a terminal continuation rotation cannot enter abort-only recovery".to_owned(),
        ));
    }
    refresh_projection_for_branch(connection, initial.branch_id)?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let runtime = load_active_rotation(&transaction, options.rotation_id)?;
    let scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    if scope.scope_id != options.expected_scope_id
        || scope.listener_session_id != options.expected_listener_session_id
        || scope.listener_revision != options.expected_listener_revision
        || scope.owner_epoch != options.expected_owner_epoch
        || occurrence.scope_id != options.expected_scope_id
        || occurrence.source_owner_epoch != options.expected_owner_epoch
        || runtime.listener_session_id != options.expected_listener_session_id
        || runtime.listener_revision != options.expected_listener_revision
        || runtime.recovery_revision != options.expected_recovery_revision
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery preview compare-and-swap failed".to_owned(),
        ));
    }
    let listener_lifecycle_state = session::session_lifecycle_state_for_connection(
        &transaction,
        options.expected_listener_session_id,
    )?;
    if listener_lifecycle_state == SessionLifecycleState::Active {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery preview requires the prior listener to be non-active".to_owned(),
        ));
    }
    let observed_scope_currentness_digest = observe_scope_currentness_for_recovery(
        &transaction,
        &scope,
        occurrence.source_session_id,
        occurrence.source_claim_id,
        &occurrence.basis,
    )?;
    transaction.commit().map_err(storage_error)?;
    Ok(ContinuationAbortOnlyRecoveryPreview {
        rotation_id: occurrence.rotation_id,
        scope_id: occurrence.scope_id,
        listener_session_id: runtime.listener_session_id,
        listener_revision: runtime.listener_revision,
        listener_lifecycle_state,
        recovery_revision: runtime.recovery_revision,
        owner_epoch: occurrence.source_owner_epoch,
        creation_operation_id: occurrence.creation_operation_id,
        original_scope_currentness_digest: runtime.scope_currentness_digest,
        observed_scope_currentness_digest,
        currentness_drifted: observed_scope_currentness_digest != runtime.scope_currentness_digest,
    })
}

pub(crate) fn bind_continuation_scope(
    connection: &mut StoreConnection,
    options: &ContinuationScopeBindOptions,
) -> Result<ContinuationScopeSnapshot> {
    connection.verify_foreign_keys()?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let listener =
        session::load_active_session_runtime_for_update(&transaction, options.listener_session_id)?;
    let carrier =
        session::load_active_session_runtime_for_update(&transaction, options.carrier_session_id)?;
    require_session_role(
        &transaction,
        options.listener_session_id,
        &[ContinuationSessionRole::Listener],
    )?;
    require_session_role(
        &transaction,
        options.carrier_session_id,
        &[ContinuationSessionRole::Work],
    )?;
    require_active_session_target(
        "listener",
        &listener,
        options.workspace_id,
        options.branch_id,
    )?;
    require_active_session_target("carrier", &carrier, options.workspace_id, options.branch_id)?;
    if options.listener_session_id == options.carrier_session_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation listener and carrier sessions must differ".to_owned(),
        ));
    }
    let listener_thread_id =
        require_unique_session_thread(&transaction, options.listener_session_id, "listener")?;
    let carrier_thread_id =
        require_unique_session_thread(&transaction, options.carrier_session_id, "carrier")?;
    ensure_listener_thread_not_created_target(
        &transaction,
        &listener_thread_id,
        "continuation listener",
    )?;
    ensure_thread_not_historical_created_target(
        &transaction,
        &carrier_thread_id,
        "continuation work carrier",
    )?;
    if listener_thread_id == carrier_thread_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation listener and carrier must not alias the same product thread".to_owned(),
        ));
    }
    require_active_exclusive_claim(
        &transaction,
        options.carrier_claim_id,
        options.carrier_session_id,
        options.workspace_id,
        options.branch_id,
        options.task_entity_id,
    )?;
    if let Some(existing) = load_scope_optional(
        &transaction,
        options.workspace_id,
        options.branch_id,
        options.task_entity_id,
    )? {
        if existing.listener_session_id == options.listener_session_id
            && existing.carrier_session_id == options.carrier_session_id
            && existing.carrier_claim_id == options.carrier_claim_id
        {
            return Ok(existing);
        }
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation scope ({}, {}, {}) is already bound",
            options.workspace_id, options.branch_id, options.task_entity_id
        )));
    }
    let scope_id = ContinuationScopeId::new_v7();
    transaction
        .execute(
            "INSERT INTO continuation_scope_runtime(
                scope_id, workspace_id, branch_id, task_entity_id, listener_session_id,
                listener_revision, carrier_session_id, carrier_claim_id,
                owner_epoch, updated_at_us
             ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, 1, ?8)",
            params![
                &scope_id.raw_bytes()[..],
                &options.workspace_id.raw_bytes()[..],
                &options.branch_id.raw_bytes()[..],
                &options.task_entity_id.raw_bytes()[..],
                &options.listener_session_id.raw_bytes()[..],
                &options.carrier_session_id.raw_bytes()[..],
                &options.carrier_claim_id.raw_bytes()[..],
                now_us,
            ],
        )
        .map_err(storage_error)?;
    insert_event(
        &transaction,
        options.workspace_id,
        options.listener_session_id,
        "continuation.scope_bound",
        now_us,
        scope_event_payload(options, scope_id, 1, 1)?,
    )?;
    session::update_session_activity(&transaction, options.listener_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_scope(
        connection.inner(),
        options.workspace_id,
        options.branch_id,
        options.task_entity_id,
    )
}

pub(crate) fn replace_continuation_listener(
    connection: &mut StoreConnection,
    options: &ContinuationListenerReplaceOptions,
) -> Result<ContinuationScopeSnapshot> {
    connection.verify_foreign_keys()?;
    let authority_json = canonical_json(
        "continuation listener replacement authority",
        &options.authority,
    )?;
    let authority_digest = Digest::domain_separated(
        "workvcs.continuation.listener-authority.v1",
        authority_json.as_bytes(),
    );
    if options.expected_listener_session_id == options.replacement_listener_session_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "replacement listener must differ from current listener".to_owned(),
        ));
    }
    refresh_projection_for_branch(connection, options.branch_id)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let scope = load_scope(
        &transaction,
        options.workspace_id,
        options.branch_id,
        options.task_entity_id,
    )?;
    let next_revision = options
        .expected_listener_revision
        .checked_add(1)
        .ok_or_else(|| {
            WorkVcsError::ContinuationInvalid("continuation listener revision overflow".to_owned())
        })?;
    if scope.listener_session_id == options.replacement_listener_session_id
        && scope.listener_revision == next_revision
        && scope.owner_epoch == options.expected_owner_epoch
        && scope.scope_id == options.expected_scope_id
    {
        let listener_thread_id = require_unique_session_thread(
            &transaction,
            options.replacement_listener_session_id,
            "replacement listener",
        )?;
        let carrier_thread_id =
            require_unique_session_thread(&transaction, scope.carrier_session_id, "carrier")?;
        if listener_thread_id == carrier_thread_id {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation replacement listener and carrier must not alias the same product thread"
                    .to_owned(),
            ));
        }
        ensure_listener_thread_not_created_target(
            &transaction,
            &listener_thread_id,
            "continuation replacement listener",
        )?;
        if listener_replacement_event_matches(
            &transaction,
            options,
            next_revision,
            &authority_json,
        )? {
            return Ok(scope);
        }
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation listener replacement replay does not match recorded provenance"
                .to_owned(),
        ));
    }
    if scope.scope_id != options.expected_scope_id
        || scope.listener_session_id != options.expected_listener_session_id
        || scope.listener_revision != options.expected_listener_revision
        || scope.owner_epoch != options.expected_owner_epoch
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation listener replacement compare-and-swap failed".to_owned(),
        ));
    }
    validate_listener_replacement_authority(&transaction, options, &scope)?;
    let replacement = session::load_active_session_runtime_for_update(
        &transaction,
        options.replacement_listener_session_id,
    )?;
    require_active_session_target(
        "replacement listener",
        &replacement,
        options.workspace_id,
        options.branch_id,
    )?;
    require_session_role(
        &transaction,
        options.replacement_listener_session_id,
        &[ContinuationSessionRole::Listener],
    )?;
    let replacement_thread_id = require_unique_session_thread(
        &transaction,
        options.replacement_listener_session_id,
        "replacement listener",
    )?;
    ensure_listener_thread_not_created_target(
        &transaction,
        &replacement_thread_id,
        "continuation replacement listener",
    )?;
    let carrier_thread_id =
        require_unique_session_thread(&transaction, scope.carrier_session_id, "carrier")?;
    if replacement_thread_id == carrier_thread_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation replacement listener and carrier must not alias the same product thread"
                .to_owned(),
        ));
    }
    let active_rotation = active_rotation_for_scope(
        &transaction,
        options.workspace_id,
        options.branch_id,
        options.task_entity_id,
    )?;
    if let Some(rotation_id) = active_rotation {
        let occurrence = load_rotation_occurrence(&transaction, rotation_id)?;
        let runtime = load_active_rotation(&transaction, rotation_id)?;
        if runtime.abort_only {
            return Err(WorkVcsError::ContinuationInvalid(
                "an abort-only continuation rotation may replace its listener only through abort-only recovery"
                    .to_owned(),
            ));
        }
        let currentness_before_replacement = compute_scope_currentness(
            &transaction,
            &scope,
            occurrence.source_session_id,
            occurrence.source_claim_id,
            occurrence.handoff_digest,
            &occurrence.basis,
        )?;
        if currentness_before_replacement != runtime.scope_currentness_digest {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation scope changed before listener replacement; recover or abort the attempt without rebasing its decision basis"
                    .to_owned(),
            ));
        }
    }
    let changed = transaction
        .execute(
            "UPDATE continuation_scope_runtime
             SET listener_session_id = ?1, listener_revision = ?2, updated_at_us = ?3
             WHERE workspace_id = ?4 AND branch_id = ?5 AND task_entity_id = ?6
               AND scope_id = ?7 AND listener_session_id = ?8
               AND listener_revision = ?9 AND owner_epoch = ?10",
            params![
                &options.replacement_listener_session_id.raw_bytes()[..],
                next_revision,
                now_us,
                &options.workspace_id.raw_bytes()[..],
                &options.branch_id.raw_bytes()[..],
                &options.task_entity_id.raw_bytes()[..],
                &options.expected_scope_id.raw_bytes()[..],
                &options.expected_listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
                options.expected_owner_epoch,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation listener replacement compare-and-swap failed".to_owned(),
        ));
    }
    let mut updated_scope = scope.clone();
    updated_scope.listener_session_id = options.replacement_listener_session_id;
    updated_scope.listener_revision = next_revision;
    updated_scope.updated_at_us = now_us;
    if let Some(rotation_id) = active_rotation {
        let occurrence = load_rotation_occurrence(&transaction, rotation_id)?;
        let runtime = load_active_rotation(&transaction, rotation_id)?;
        let currentness = compute_scope_currentness(
            &transaction,
            &updated_scope,
            occurrence.source_session_id,
            occurrence.source_claim_id,
            occurrence.handoff_digest,
            &occurrence.basis,
        )?;
        let updated = if runtime.state == ContinuationRotationState::CandidateAccepted {
            let prior_acceptance = runtime.candidate_acceptance.as_ref().ok_or_else(|| {
                WorkVcsError::ContinuationInvalid(
                    "accepted continuation rotation is missing candidate acceptance".to_owned(),
                )
            })?;
            let acceptance_history =
                append_acceptance_history(&runtime.candidate_acceptance_history, prior_acceptance)?;
            transaction
                .execute(
                    "UPDATE continuation_rotation_runtime
                     SET listener_session_id = ?1, listener_revision = ?2,
                         scope_currentness_digest = ?3, status = ?4,
                         candidate_acceptance_json = NULL,
                         candidate_acceptance_history_json = ?5,
                         last_activity_at_us = ?6
                     WHERE rotation_id = ?7 AND listener_session_id = ?8
                       AND listener_revision = ?9 AND status = ?10",
                    params![
                        &options.replacement_listener_session_id.raw_bytes()[..],
                        next_revision,
                        &currentness.as_bytes()[..],
                        CANDIDATE_ATTACHED_STATUS,
                        canonical_json(
                            "continuation candidate acceptance history",
                            &acceptance_history,
                        )?,
                        now_us,
                        &rotation_id.raw_bytes()[..],
                        &options.expected_listener_session_id.raw_bytes()[..],
                        options.expected_listener_revision,
                        CANDIDATE_ACCEPTED_STATUS,
                    ],
                )
                .map_err(storage_error)?
        } else {
            transaction
                .execute(
                    "UPDATE continuation_rotation_runtime
                     SET listener_session_id = ?1, listener_revision = ?2,
                         scope_currentness_digest = ?3, last_activity_at_us = ?4
                     WHERE rotation_id = ?5 AND listener_session_id = ?6
                       AND listener_revision = ?7 AND status = ?8",
                    params![
                        &options.replacement_listener_session_id.raw_bytes()[..],
                        next_revision,
                        &currentness.as_bytes()[..],
                        now_us,
                        &rotation_id.raw_bytes()[..],
                        &options.expected_listener_session_id.raw_bytes()[..],
                        options.expected_listener_revision,
                        runtime.state.as_str(),
                    ],
                )
                .map_err(storage_error)?
        };
        if updated != 1 {
            return Err(WorkVcsError::ContinuationInvalid(
                "active continuation rotation listener replacement compare-and-swap failed"
                    .to_owned(),
            ));
        }
    }
    insert_event(
        &transaction,
        options.workspace_id,
        options.actor_session_id,
        "continuation.listener_replaced",
        now_us,
        CanonicalValue::object(vec![
            ("authority".to_owned(), options.authority.clone()),
            (
                "authority_digest".to_owned(),
                CanonicalValue::String(authority_digest.to_string()),
            ),
            (
                "active_rotation_id".to_owned(),
                active_rotation
                    .map(|id| CanonicalValue::String(id.to_string()))
                    .unwrap_or(CanonicalValue::Null),
            ),
            (
                "branch_id".to_owned(),
                CanonicalValue::String(options.branch_id.to_string()),
            ),
            (
                "new_listener_revision".to_owned(),
                CanonicalValue::safe_integer(next_revision)?,
            ),
            (
                "new_listener_session_id".to_owned(),
                CanonicalValue::String(options.replacement_listener_session_id.to_string()),
            ),
            (
                "old_listener_revision".to_owned(),
                CanonicalValue::safe_integer(options.expected_listener_revision)?,
            ),
            (
                "old_listener_session_id".to_owned(),
                CanonicalValue::String(options.expected_listener_session_id.to_string()),
            ),
            (
                "owner_epoch".to_owned(),
                CanonicalValue::safe_integer(options.expected_owner_epoch)?,
            ),
            (
                "scope_id".to_owned(),
                CanonicalValue::String(options.expected_scope_id.to_string()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(options.task_entity_id.to_string()),
            ),
        ])?,
    )?;
    session::update_session_activity(&transaction, options.actor_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_scope(
        connection.inner(),
        options.workspace_id,
        options.branch_id,
        options.task_entity_id,
    )
}

pub(crate) fn recover_continuation_listener_abort_only(
    connection: &mut StoreConnection,
    options: &ContinuationAbortOnlyRecoveryOptions,
) -> Result<ContinuationRotationSnapshot> {
    connection.verify_foreign_keys()?;
    let authority_json = canonical_json(
        "continuation abort-only recovery authority",
        &options.authority,
    )?;
    let authority_digest =
        Digest::domain_separated(RECOVERY_AUTHORITY_DIGEST_DOMAIN, authority_json.as_bytes());
    if options.actor_session_id != options.replacement_listener_session_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery must be authored by the replacement listener".to_owned(),
        ));
    }
    if options.expected_listener_session_id == options.replacement_listener_session_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery listener must differ from the lost listener".to_owned(),
        ));
    }
    let initial = load_rotation_snapshot(connection.inner(), options.rotation_id)?;
    if matches!(
        initial.state,
        ContinuationRotationState::Committed | ContinuationRotationState::Aborted
    ) {
        return Err(WorkVcsError::ContinuationInvalid(
            "a terminal continuation rotation cannot enter abort-only recovery".to_owned(),
        ));
    }
    refresh_projection_for_branch(connection, initial.branch_id)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let runtime = load_active_rotation(&transaction, options.rotation_id)?;
    let scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    let next_listener_revision = options
        .expected_listener_revision
        .checked_add(1)
        .ok_or_else(|| {
            WorkVcsError::ContinuationInvalid("continuation listener revision overflow".to_owned())
        })?;
    let next_recovery_revision = options
        .expected_recovery_revision
        .checked_add(1)
        .ok_or_else(|| {
            WorkVcsError::ContinuationInvalid("continuation recovery revision overflow".to_owned())
        })?;
    if scope.scope_id == options.expected_scope_id
        && scope.listener_session_id == options.replacement_listener_session_id
        && scope.listener_revision == next_listener_revision
        && scope.owner_epoch == options.expected_owner_epoch
        && runtime.listener_session_id == options.replacement_listener_session_id
        && runtime.listener_revision == next_listener_revision
        && runtime.abort_only
        && runtime.recovery_revision == next_recovery_revision
    {
        if abort_only_recovery_matches(
            &runtime,
            options,
            next_listener_revision,
            next_recovery_revision,
            authority_digest,
        )? {
            return load_rotation_snapshot(&transaction, options.rotation_id);
        }
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery replay does not match recorded provenance".to_owned(),
        ));
    }
    if scope.scope_id != options.expected_scope_id
        || scope.listener_session_id != options.expected_listener_session_id
        || scope.listener_revision != options.expected_listener_revision
        || scope.owner_epoch != options.expected_owner_epoch
        || occurrence.scope_id != options.expected_scope_id
        || occurrence.source_owner_epoch != options.expected_owner_epoch
        || runtime.listener_session_id != options.expected_listener_session_id
        || runtime.listener_revision != options.expected_listener_revision
        || runtime.recovery_revision != options.expected_recovery_revision
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery compare-and-swap failed".to_owned(),
        ));
    }
    let prior_listener_state = session::session_lifecycle_state_for_connection(
        &transaction,
        options.expected_listener_session_id,
    )?;
    if prior_listener_state == SessionLifecycleState::Active {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery requires the prior listener to be non-active".to_owned(),
        ));
    }
    let replacement = session::load_active_session_runtime_for_update(
        &transaction,
        options.replacement_listener_session_id,
    )?;
    require_active_session_target(
        "abort-only recovery listener",
        &replacement,
        occurrence.workspace_id,
        occurrence.branch_id,
    )?;
    require_session_role(
        &transaction,
        options.replacement_listener_session_id,
        &[ContinuationSessionRole::Listener],
    )?;
    let replacement_thread_id = require_unique_session_thread(
        &transaction,
        options.replacement_listener_session_id,
        "abort-only recovery listener",
    )?;
    let source_thread_id =
        require_unique_session_thread(&transaction, occurrence.source_session_id, "source")?;
    if replacement_thread_id == source_thread_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery listener and source must not alias the same product thread"
                .to_owned(),
        ));
    }
    ensure_listener_thread_not_created_target(
        &transaction,
        &replacement_thread_id,
        "abort-only recovery listener",
    )?;
    if let Some(candidate_session_id) = runtime.candidate_session_id {
        let candidate_thread_id =
            require_unique_session_thread(&transaction, candidate_session_id, "candidate")?;
        if candidate_thread_id == replacement_thread_id {
            return Err(WorkVcsError::ContinuationInvalid(
                "abort-only recovery listener and candidate must not alias the same product thread"
                    .to_owned(),
            ));
        }
    }
    let observed_currentness = observe_scope_currentness_for_recovery(
        &transaction,
        &scope,
        occurrence.source_session_id,
        occurrence.source_claim_id,
        &occurrence.basis,
    )?;
    validate_abort_only_recovery_authority(
        options,
        &occurrence,
        &runtime,
        prior_listener_state,
        observed_currentness,
    )?;
    let prior_recovery_receipt_digest = runtime
        .recovery
        .as_ref()
        .map(|value| {
            document_digest(
                RECOVERY_RECEIPT_DIGEST_DOMAIN,
                "continuation abort-only recovery",
                value,
            )
        })
        .transpose()?;
    let prior_recovery_history = load_abort_only_recovery_summary(
        &transaction,
        occurrence.workspace_id,
        options.rotation_id,
        runtime.recovery_revision,
        runtime.recovery.as_ref(),
        prior_recovery_receipt_digest,
    )?;
    let recovery = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1)?,
        ),
        (
            "mode".to_owned(),
            CanonicalValue::String("abort_only".to_owned()),
        ),
        (
            "rotation_id".to_owned(),
            CanonicalValue::String(options.rotation_id.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(options.expected_scope_id.to_string()),
        ),
        (
            "old_listener_session_id".to_owned(),
            CanonicalValue::String(options.expected_listener_session_id.to_string()),
        ),
        (
            "old_listener_revision".to_owned(),
            CanonicalValue::safe_integer(options.expected_listener_revision)?,
        ),
        (
            "old_listener_lifecycle_state".to_owned(),
            CanonicalValue::String(session_lifecycle_state_label(prior_listener_state).to_owned()),
        ),
        (
            "new_listener_session_id".to_owned(),
            CanonicalValue::String(options.replacement_listener_session_id.to_string()),
        ),
        (
            "new_listener_revision".to_owned(),
            CanonicalValue::safe_integer(next_listener_revision)?,
        ),
        (
            "recovery_revision".to_owned(),
            CanonicalValue::safe_integer(next_recovery_revision)?,
        ),
        (
            "original_scope_currentness_digest".to_owned(),
            CanonicalValue::String(runtime.scope_currentness_digest.to_string()),
        ),
        (
            "observed_scope_currentness_digest".to_owned(),
            CanonicalValue::String(observed_currentness.to_string()),
        ),
        (
            "currentness_drifted".to_owned(),
            CanonicalValue::Bool(observed_currentness != runtime.scope_currentness_digest),
        ),
        ("authority".to_owned(), options.authority.clone()),
        (
            "authority_digest".to_owned(),
            CanonicalValue::String(authority_digest.to_string()),
        ),
        (
            "recovered_at_us".to_owned(),
            CanonicalValue::safe_integer(now_us)?,
        ),
    ])?;
    let recovery_json = canonical_json("continuation abort-only recovery", &recovery)?;
    let recovery_receipt_digest = document_digest(
        RECOVERY_RECEIPT_DIGEST_DOMAIN,
        "continuation abort-only recovery",
        &recovery,
    )?;
    let recovery_history_chain_digest = next_recovery_history_chain_digest(
        prior_recovery_history.chain_digest,
        next_recovery_revision,
        recovery_receipt_digest,
    )?;
    let scope_changed = transaction
        .execute(
            "UPDATE continuation_scope_runtime
             SET listener_session_id = ?1, listener_revision = ?2, updated_at_us = ?3
             WHERE workspace_id = ?4 AND branch_id = ?5 AND task_entity_id = ?6
               AND scope_id = ?7 AND listener_session_id = ?8
               AND listener_revision = ?9 AND owner_epoch = ?10",
            params![
                &options.replacement_listener_session_id.raw_bytes()[..],
                next_listener_revision,
                now_us,
                &occurrence.workspace_id.raw_bytes()[..],
                &occurrence.branch_id.raw_bytes()[..],
                &occurrence.task_entity_id.raw_bytes()[..],
                &options.expected_scope_id.raw_bytes()[..],
                &options.expected_listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
                options.expected_owner_epoch,
            ],
        )
        .map_err(storage_error)?;
    if scope_changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery scope compare-and-swap failed".to_owned(),
        ));
    }
    let rotation_changed = transaction
        .execute(
            "UPDATE continuation_rotation_runtime
             SET listener_session_id = ?1, listener_revision = ?2,
                 abort_only = 1, recovery_revision = ?3, recovery_json = ?4,
                 last_activity_at_us = ?5
             WHERE rotation_id = ?6 AND listener_session_id = ?7
               AND listener_revision = ?8 AND recovery_revision = ?9",
            params![
                &options.replacement_listener_session_id.raw_bytes()[..],
                next_listener_revision,
                next_recovery_revision,
                recovery_json,
                now_us,
                &options.rotation_id.raw_bytes()[..],
                &options.expected_listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
                options.expected_recovery_revision,
            ],
        )
        .map_err(storage_error)?;
    if rotation_changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery rotation compare-and-swap failed".to_owned(),
        ));
    }
    let updated_scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    let recovery_event = AbortOnlyRecoveryEvent {
        recovery: &recovery,
        recovery_receipt_digest,
        previous_chain_digest: prior_recovery_history.chain_digest,
        history: AbortOnlyRecoveryHistorySummary {
            count: next_recovery_revision,
            chain_digest: recovery_history_chain_digest,
        },
    };
    insert_abort_only_recovery_event(
        &transaction,
        &updated_scope,
        options.rotation_id,
        options.actor_session_id,
        now_us,
        &recovery_event,
    )?;
    session::update_session_activity(&transaction, options.actor_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_rotation(connection.inner(), options.rotation_id)
}

pub(crate) fn begin_continuation_rotation(
    connection: &mut StoreConnection,
    options: &ContinuationRotationBeginOptions,
) -> Result<ContinuationRotationSnapshot> {
    connection.verify_foreign_keys()?;
    let basis_json = canonical_json("continuation basis", &options.basis)?;
    let policy_decision_json =
        canonical_json("continuation policy decision", &options.policy_decision)?;
    let creation_request_json =
        canonical_json("continuation creation request", &options.creation_request)?;
    let policy_decision_digest = Digest::domain_separated(
        POLICY_DECISION_DIGEST_DOMAIN,
        policy_decision_json.as_bytes(),
    );
    let creation_request_digest = Digest::domain_separated(
        CREATION_REQUEST_DIGEST_DOMAIN,
        creation_request_json.as_bytes(),
    );
    if let Some(existing) = load_rotation_by_idempotency(
        connection.inner(),
        options.source_claim_id,
        &options.idempotency_key,
    )? {
        ensure_idempotent_begin_matches(
            &existing,
            options,
            &basis_json,
            &policy_decision_json,
            &creation_request_json,
        )?;
        return load_rotation_snapshot(connection.inner(), existing.rotation_id);
    }
    let initial_scope = load_scope_by_carrier(
        connection.inner(),
        options.source_session_id,
        options.source_claim_id,
    )?;
    refresh_projection_for_branch(connection, initial_scope.branch_id)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    if let Some(existing) = load_rotation_by_idempotency(
        &transaction,
        options.source_claim_id,
        &options.idempotency_key,
    )? {
        ensure_idempotent_begin_matches(
            &existing,
            options,
            &basis_json,
            &policy_decision_json,
            &creation_request_json,
        )?;
        return load_rotation_snapshot(&transaction, existing.rotation_id);
    }
    if creation_operation_exists(&transaction, &options.creation_operation_id)? {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation creation operation {:?} is already bound to another rotation",
            options.creation_operation_id
        )));
    }
    let scope = load_scope_by_carrier(
        &transaction,
        options.source_session_id,
        options.source_claim_id,
    )?;
    if scope.scope_id != options.expected_scope_id
        || scope.listener_session_id != options.listener_session_id
        || scope.listener_revision != options.expected_listener_revision
        || scope.carrier_session_id != options.source_session_id
        || scope.carrier_claim_id != options.source_claim_id
        || scope.owner_epoch != options.expected_owner_epoch
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation rotation admission compare-and-swap failed".to_owned(),
        ));
    }
    let listener =
        session::load_active_session_runtime_for_update(&transaction, options.listener_session_id)?;
    require_active_session_target("listener", &listener, scope.workspace_id, scope.branch_id)?;
    require_session_role(
        &transaction,
        options.listener_session_id,
        &[ContinuationSessionRole::Listener],
    )?;
    let listener_thread_id =
        require_unique_session_thread(&transaction, options.listener_session_id, "listener")?;
    ensure_listener_thread_not_created_target(
        &transaction,
        &listener_thread_id,
        "continuation listener",
    )?;
    require_source_role_and_provenance(
        &transaction,
        options.source_session_id,
        options.source_claim_id,
        &scope,
    )?;
    require_active_exclusive_claim(
        &transaction,
        options.source_claim_id,
        options.source_session_id,
        scope.workspace_id,
        scope.branch_id,
        scope.task_entity_id,
    )?;
    if active_rotation_for_scope(
        &transaction,
        scope.workspace_id,
        scope.branch_id,
        scope.task_entity_id,
    )?
    .is_some()
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation scope already has an active rotation".to_owned(),
        ));
    }
    let parsed_basis = parse_basis(&options.basis)?;
    require_session_thread_id(
        &transaction,
        options.source_session_id,
        &parsed_basis.source_thread_id,
        "source",
    )?;
    let currentness = compute_scope_currentness(
        &transaction,
        &scope,
        options.source_session_id,
        options.source_claim_id,
        options.handoff_digest,
        &options.basis,
    )?;
    validate_policy_decision(
        &transaction,
        &scope,
        options,
        &parsed_basis,
        currentness,
        &options.policy_decision,
    )?;
    validate_creation_request(
        &transaction,
        &scope,
        options,
        &parsed_basis,
        currentness,
        &options.creation_request,
        &options.policy_decision,
    )?;
    let rotation_id = ContinuationRotationId::new_v7();
    transaction
        .execute(
            "INSERT INTO continuation_rotation(
                rotation_id, scope_id, workspace_id, branch_id, task_entity_id,
                initial_listener_session_id, initial_listener_revision,
                source_session_id, source_claim_id, source_owner_epoch,
                idempotency_key, creation_operation_id,
                policy_decision_digest, policy_decision_json,
                creation_request_digest, creation_request_json,
                handoff_digest, basis_json, created_at_us
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                &rotation_id.raw_bytes()[..],
                &scope.scope_id.raw_bytes()[..],
                &scope.workspace_id.raw_bytes()[..],
                &scope.branch_id.raw_bytes()[..],
                &scope.task_entity_id.raw_bytes()[..],
                &options.listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
                &options.source_session_id.raw_bytes()[..],
                &options.source_claim_id.raw_bytes()[..],
                options.expected_owner_epoch,
                options.idempotency_key,
                options.creation_operation_id,
                &policy_decision_digest.as_bytes()[..],
                policy_decision_json,
                &creation_request_digest.as_bytes()[..],
                creation_request_json,
                &options.handoff_digest.as_bytes()[..],
                basis_json,
                now_us,
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO continuation_rotation_runtime(
                rotation_id, workspace_id, branch_id, task_entity_id,
                listener_session_id, listener_revision, scope_currentness_digest,
                status, dispatch_json, candidate_session_id, candidate_binding_json,
                creation_receipt_json, candidate_acceptance_json, last_activity_at_us
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL, NULL, NULL, ?9)",
            params![
                &rotation_id.raw_bytes()[..],
                &scope.workspace_id.raw_bytes()[..],
                &scope.branch_id.raw_bytes()[..],
                &scope.task_entity_id.raw_bytes()[..],
                &options.listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
                &currentness.as_bytes()[..],
                PENDING_CREATION_STATUS,
                now_us,
            ],
        )
        .map_err(storage_error)?;
    insert_rotation_event(
        &transaction,
        &scope,
        rotation_id,
        options.listener_session_id,
        "continuation.rotation_began",
        now_us,
    )?;
    session::update_session_activity(&transaction, options.listener_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_rotation(connection.inner(), rotation_id)
}

pub(crate) fn dispatch_continuation_creation(
    connection: &mut StoreConnection,
    options: &ContinuationCreationDispatchOptions,
) -> Result<ContinuationRotationSnapshot> {
    connection.verify_foreign_keys()?;
    let dispatch_json = canonical_json("continuation creation dispatch", &options.dispatch)?;
    let initial = load_rotation_snapshot(connection.inner(), options.rotation_id)?;
    refresh_projection_for_branch(connection, initial.branch_id)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let runtime = load_active_rotation(&transaction, options.rotation_id)?;
    require_normal_rotation(&runtime, "dispatch creation")?;
    let snapshot = load_rotation_snapshot(&transaction, options.rotation_id)?;
    if snapshot.state != ContinuationRotationState::PendingCreation {
        let dispatch_matches = snapshot
            .dispatch
            .as_ref()
            .map(|value| canonical_json("continuation creation dispatch", value))
            .transpose()?
            .as_deref()
            == Some(dispatch_json.as_str());
        if dispatch_matches {
            require_snapshot_actor(
                &snapshot,
                options.listener_session_id,
                options.expected_listener_revision,
            )?;
            return Ok(snapshot);
        }
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation rotation {} is not pending creation or has a different dispatch record",
            options.rotation_id
        )));
    }
    let scope = require_active_rotation_actor(
        &transaction,
        &occurrence,
        &runtime,
        options.listener_session_id,
        options.expected_listener_revision,
    )?;
    require_rotation_participant_threads_unique(&transaction, &occurrence, &runtime, None)?;
    validate_creation_dispatch(&transaction, &occurrence, &runtime, &options.dispatch)?;
    let currentness = compute_scope_currentness(
        &transaction,
        &scope,
        occurrence.source_session_id,
        occurrence.source_claim_id,
        occurrence.handoff_digest,
        &occurrence.basis,
    )?;
    if currentness != runtime.scope_currentness_digest {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation scope currentness changed from {} to {} before creation dispatch",
            runtime.scope_currentness_digest, currentness
        )));
    }
    let changed = transaction
        .execute(
            "UPDATE continuation_rotation_runtime
             SET status = ?1, dispatch_json = ?2, last_activity_at_us = ?3
             WHERE rotation_id = ?4 AND status = ?5
               AND listener_session_id = ?6 AND listener_revision = ?7",
            params![
                CREATION_DISPATCHED_STATUS,
                dispatch_json,
                now_us,
                &options.rotation_id.raw_bytes()[..],
                PENDING_CREATION_STATUS,
                &options.listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation creation dispatch compare-and-swap failed".to_owned(),
        ));
    }
    insert_rotation_event(
        &transaction,
        &scope,
        options.rotation_id,
        options.listener_session_id,
        "continuation.creation_dispatched",
        now_us,
    )?;
    session::update_session_activity(&transaction, options.listener_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_rotation(connection.inner(), options.rotation_id)
}

pub(crate) fn record_continuation_creation_result(
    connection: &mut StoreConnection,
    options: &ContinuationCreationResultOptions,
) -> Result<ContinuationRotationSnapshot> {
    connection.verify_foreign_keys()?;
    let receipt_json = canonical_json("continuation creation result receipt", &options.receipt)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let snapshot = load_rotation_snapshot(&transaction, options.rotation_id)?;
    let runtime = match load_active_rotation(&transaction, options.rotation_id) {
        Ok(runtime) => runtime,
        Err(_) => {
            let final_receipt_matches = snapshot
                .creation_receipt
                .as_ref()
                .map(|value| canonical_json("continuation creation result receipt", value))
                .transpose()?
                .as_deref()
                == Some(receipt_json.as_str());
            let initial_unknown_matches = snapshot
                .initial_unknown_receipt
                .as_ref()
                .map(|value| canonical_json("initial unknown creation receipt", value))
                .transpose()?
                .as_deref()
                == Some(receipt_json.as_str());
            if final_receipt_matches || initial_unknown_matches {
                require_snapshot_actor(
                    &snapshot,
                    options.listener_session_id,
                    options.expected_listener_revision,
                )?;
                return Ok(snapshot);
            }
            return Err(WorkVcsError::ContinuationInvalid(
                "terminal continuation rotation has a different creation receipt".to_owned(),
            ));
        }
    };
    require_active_rotation_actor(
        &transaction,
        &occurrence,
        &runtime,
        options.listener_session_id,
        options.expected_listener_revision,
    )?;
    require_rotation_participant_threads_unique(&transaction, &occurrence, &runtime, None)?;
    let initial_unknown_matches = runtime
        .initial_unknown_receipt
        .as_ref()
        .map(|value| canonical_json("initial unknown creation receipt", value))
        .transpose()?
        .as_deref()
        == Some(receipt_json.as_str());
    if initial_unknown_matches {
        return Ok(snapshot);
    }
    if let Some(stored) = runtime.creation_receipt.as_ref() {
        if canonical_json("stored continuation creation result", stored)? == receipt_json {
            return Ok(snapshot);
        }
        if runtime.state != ContinuationRotationState::CreationUnknown {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation creation result replay used a conflicting receipt".to_owned(),
            ));
        }
    }
    if !matches!(
        runtime.state,
        ContinuationRotationState::CreationDispatched | ContinuationRotationState::CreationUnknown
    ) {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation creation result cannot be recorded from state {}",
            runtime.state.as_str()
        )));
    }
    let next_state =
        validate_creation_result_receipt(&transaction, &occurrence, &runtime, &options.receipt)?;
    if runtime.state == ContinuationRotationState::CreationUnknown
        && next_state != ContinuationRotationState::CreationSucceeded
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "an unknown creation result may only reconcile to a created target".to_owned(),
        ));
    }
    let changed = transaction
        .execute(
            "UPDATE continuation_rotation_runtime
             SET status = ?1,
                 creation_receipt_json = ?2,
                 initial_unknown_receipt_json = CASE
                    WHEN ?1 = 'creation_unknown' THEN ?2
                    ELSE initial_unknown_receipt_json
                 END,
                 last_activity_at_us = ?3
             WHERE rotation_id = ?4 AND status = ?5
               AND listener_session_id = ?6 AND listener_revision = ?7",
            params![
                next_state.as_str(),
                receipt_json,
                now_us,
                &options.rotation_id.raw_bytes()[..],
                runtime.state.as_str(),
                &options.listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation creation result compare-and-swap failed".to_owned(),
        ));
    }
    let scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    let event_kind = if next_state == ContinuationRotationState::CreationUnknown {
        "continuation.creation_unknown"
    } else {
        "continuation.creation_succeeded"
    };
    insert_rotation_event(
        &transaction,
        &scope,
        options.rotation_id,
        options.listener_session_id,
        event_kind,
        now_us,
    )?;
    session::update_session_activity(&transaction, options.listener_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_rotation(connection.inner(), options.rotation_id)
}

pub(crate) fn attach_continuation_candidate(
    connection: &mut StoreConnection,
    options: &ContinuationCandidateAttachOptions,
) -> Result<ContinuationRotationSnapshot> {
    connection.verify_foreign_keys()?;
    let binding_json =
        canonical_json("continuation candidate binding", &options.candidate_binding)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let snapshot = load_rotation_snapshot(&transaction, options.rotation_id)?;
    if snapshot.abort_only {
        return Err(WorkVcsError::ContinuationInvalid(
            "an abort-only continuation rotation cannot attach a candidate".to_owned(),
        ));
    }
    if matches!(
        snapshot.state,
        ContinuationRotationState::CandidateAttached
            | ContinuationRotationState::CandidateAccepted
            | ContinuationRotationState::Committed
            | ContinuationRotationState::Aborted
    ) {
        let binding_matches = snapshot
            .candidate_binding
            .as_ref()
            .map(|value| canonical_json("continuation candidate binding", value))
            .transpose()?
            .as_deref()
            == Some(binding_json.as_str());
        if snapshot.candidate_session_id == Some(options.candidate_session_id) && binding_matches {
            require_snapshot_actor(
                &snapshot,
                options.listener_session_id,
                options.expected_listener_revision,
            )?;
            return Ok(snapshot);
        }
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation rotation {} already has a different candidate attachment or terminal result",
            options.rotation_id
        )));
    }
    let runtime = load_active_rotation(&transaction, options.rotation_id)?;
    if runtime.state != ContinuationRotationState::CreationSucceeded {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation candidate cannot be attached from state {}",
            runtime.state.as_str()
        )));
    }
    require_active_rotation_actor(
        &transaction,
        &occurrence,
        &runtime,
        options.listener_session_id,
        options.expected_listener_revision,
    )?;
    require_rotation_participant_threads_unique(&transaction, &occurrence, &runtime, None)?;
    if options.candidate_session_id == occurrence.initial_listener_session_id
        || options.candidate_session_id == occurrence.source_session_id
        || options.candidate_session_id == runtime.listener_session_id
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate must differ from listener and source sessions".to_owned(),
        ));
    }
    let candidate = session::load_active_session_runtime_for_update(
        &transaction,
        options.candidate_session_id,
    )?;
    require_active_session_target(
        "candidate",
        &candidate,
        occurrence.workspace_id,
        occurrence.branch_id,
    )?;
    require_session_role(
        &transaction,
        options.candidate_session_id,
        &[ContinuationSessionRole::WorkCandidate],
    )?;
    require_unique_session_thread(&transaction, options.candidate_session_id, "candidate")?;
    validate_candidate_binding(
        &transaction,
        &occurrence,
        &runtime,
        options.candidate_session_id,
        &options.candidate_binding,
    )?;
    let changed = transaction
        .execute(
            "UPDATE continuation_rotation_runtime
             SET status = ?1, candidate_session_id = ?2,
                 candidate_binding_json = ?3, last_activity_at_us = ?4
             WHERE rotation_id = ?5 AND status = ?6
               AND listener_session_id = ?7 AND listener_revision = ?8",
            params![
                CANDIDATE_ATTACHED_STATUS,
                &options.candidate_session_id.raw_bytes()[..],
                binding_json,
                now_us,
                &options.rotation_id.raw_bytes()[..],
                CREATION_SUCCEEDED_STATUS,
                &options.listener_session_id.raw_bytes()[..],
                options.expected_listener_revision,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate attachment compare-and-swap failed".to_owned(),
        ));
    }
    let scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    insert_rotation_event(
        &transaction,
        &scope,
        options.rotation_id,
        options.listener_session_id,
        "continuation.candidate_attached",
        now_us,
    )?;
    session::update_session_activity(&transaction, options.listener_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_rotation(connection.inner(), options.rotation_id)
}

pub(crate) fn accept_continuation_candidate(
    connection: &mut StoreConnection,
    options: &ContinuationCandidateAcceptOptions,
) -> Result<ContinuationRotationSnapshot> {
    connection.verify_foreign_keys()?;
    let acceptance_json = canonical_json("continuation candidate acceptance", &options.acceptance)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let snapshot = load_rotation_snapshot(&transaction, options.rotation_id)?;
    if snapshot.abort_only {
        return Err(WorkVcsError::ContinuationInvalid(
            "an abort-only continuation rotation cannot accept a candidate".to_owned(),
        ));
    }
    if matches!(
        snapshot.state,
        ContinuationRotationState::CandidateAccepted | ContinuationRotationState::Committed
    ) {
        let matches = snapshot.candidate_session_id == Some(options.candidate_session_id)
            && snapshot
                .candidate_acceptance
                .as_ref()
                .map(|value| canonical_json("stored continuation candidate acceptance", value))
                .transpose()?
                .as_deref()
                == Some(acceptance_json.as_str());
        if matches {
            return Ok(snapshot);
        }
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate acceptance replay used conflicting inputs".to_owned(),
        ));
    }
    if snapshot.state == ContinuationRotationState::Aborted {
        return Err(WorkVcsError::ContinuationInvalid(
            "an aborted continuation rotation cannot be accepted".to_owned(),
        ));
    }
    let runtime = load_active_rotation(&transaction, options.rotation_id)?;
    if runtime.state != ContinuationRotationState::CandidateAttached
        || runtime.candidate_session_id != Some(options.candidate_session_id)
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate acceptance compare-and-swap failed".to_owned(),
        ));
    }
    let candidate = session::load_active_session_runtime_for_update(
        &transaction,
        options.candidate_session_id,
    )?;
    require_active_session_target(
        "candidate",
        &candidate,
        occurrence.workspace_id,
        occurrence.branch_id,
    )?;
    require_session_role(
        &transaction,
        options.candidate_session_id,
        &[ContinuationSessionRole::WorkCandidate],
    )?;
    require_rotation_participant_threads_unique(
        &transaction,
        &occurrence,
        &runtime,
        Some(options.candidate_session_id),
    )?;
    validate_candidate_acceptance(
        &transaction,
        &occurrence,
        &runtime,
        options.candidate_session_id,
        &options.acceptance,
    )?;
    let changed = transaction
        .execute(
            "UPDATE continuation_rotation_runtime
             SET status = ?1, candidate_acceptance_json = ?2, last_activity_at_us = ?3
             WHERE rotation_id = ?4 AND status = ?5 AND candidate_session_id = ?6",
            params![
                CANDIDATE_ACCEPTED_STATUS,
                acceptance_json,
                now_us,
                &options.rotation_id.raw_bytes()[..],
                CANDIDATE_ATTACHED_STATUS,
                &options.candidate_session_id.raw_bytes()[..],
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate acceptance compare-and-swap failed".to_owned(),
        ));
    }
    let scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    insert_rotation_event(
        &transaction,
        &scope,
        options.rotation_id,
        options.candidate_session_id,
        "continuation.candidate_accepted",
        now_us,
    )?;
    session::update_session_activity(&transaction, options.candidate_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_rotation(connection.inner(), options.rotation_id)
}

pub(crate) fn abort_continuation_rotation(
    connection: &mut StoreConnection,
    options: &ContinuationRotationAbortOptions,
) -> Result<ContinuationRotationSnapshot> {
    connection.verify_foreign_keys()?;
    let rationale_json = canonical_json("continuation abort rationale", &options.rationale)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let snapshot = load_rotation_snapshot(&transaction, options.rotation_id)?;
    match snapshot.state {
        ContinuationRotationState::Aborted => {
            require_snapshot_actor(
                &snapshot,
                options.listener_session_id,
                options.expected_listener_revision,
            )?;
            let stored = snapshot
                .result
                .as_ref()
                .and_then(|value| optional_object_field(value, "rationale"))
                .ok_or_else(|| {
                    WorkVcsError::ContinuationInvalid(
                        "aborted continuation result is missing rationale".to_owned(),
                    )
                })?;
            if canonical_json("stored continuation abort rationale", stored)? == rationale_json {
                return Ok(snapshot);
            }
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation abort replay used a conflicting rationale".to_owned(),
            ));
        }
        ContinuationRotationState::Committed => {
            return Err(WorkVcsError::ContinuationInvalid(
                "a committed continuation rotation cannot be aborted".to_owned(),
            ));
        }
        _ => {}
    }
    let runtime = load_active_rotation(&transaction, options.rotation_id)?;
    require_active_rotation_actor(
        &transaction,
        &occurrence,
        &runtime,
        options.listener_session_id,
        options.expected_listener_revision,
    )?;
    validate_abort_rationale(&occurrence, &runtime, &options.rationale)?;
    let result_value = CanonicalValue::object(vec![
        (
            "candidate_acceptance".to_owned(),
            runtime
                .candidate_acceptance
                .clone()
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "candidate_binding".to_owned(),
            runtime
                .candidate_binding
                .clone()
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "dispatch".to_owned(),
            runtime.dispatch.clone().unwrap_or(CanonicalValue::Null),
        ),
        (
            "creation_receipt".to_owned(),
            runtime
                .creation_receipt
                .clone()
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "initial_unknown_receipt".to_owned(),
            runtime
                .initial_unknown_receipt
                .clone()
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "candidate_acceptance_history".to_owned(),
            runtime.candidate_acceptance_history.clone(),
        ),
        (
            "abort_only".to_owned(),
            CanonicalValue::Bool(runtime.abort_only),
        ),
        (
            "recovery_revision".to_owned(),
            CanonicalValue::safe_integer(runtime.recovery_revision)?,
        ),
        (
            "recovery".to_owned(),
            runtime.recovery.clone().unwrap_or(CanonicalValue::Null),
        ),
        (
            "recovery_history_count".to_owned(),
            CanonicalValue::safe_integer(snapshot.recovery_history_count)?,
        ),
        (
            "recovery_history_chain_digest".to_owned(),
            CanonicalValue::String(snapshot.recovery_history_chain_digest.to_string()),
        ),
        ("rationale".to_owned(), options.rationale.clone()),
    ])?;
    transaction
        .execute(
            "DELETE FROM continuation_rotation_runtime WHERE rotation_id = ?1",
            params![&options.rotation_id.raw_bytes()[..]],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO continuation_rotation_result(
                rotation_id, status, candidate_session_id, resulting_claim_id,
                listener_session_id, listener_revision, scope_currentness_digest,
                abort_only, recovery_revision, recovery_json,
                dispatch_json, creation_receipt_json, initial_unknown_receipt_json,
                candidate_binding_json, candidate_acceptance_json,
                candidate_acceptance_history_json, ended_at_us, result_json
             ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                &options.rotation_id.raw_bytes()[..],
                ABORTED_STATUS,
                runtime
                    .candidate_session_id
                    .map(|session_id| session_id.raw_bytes().to_vec()),
                &runtime.listener_session_id.raw_bytes()[..],
                runtime.listener_revision,
                &runtime.scope_currentness_digest.as_bytes()[..],
                if runtime.abort_only { 1_i64 } else { 0_i64 },
                runtime.recovery_revision,
                runtime
                    .recovery
                    .as_ref()
                    .map(|value| canonical_json("continuation abort-only recovery", value))
                    .transpose()?,
                runtime
                    .dispatch
                    .as_ref()
                    .map(|value| canonical_json("continuation dispatch", value))
                    .transpose()?,
                runtime
                    .creation_receipt
                    .as_ref()
                    .map(|value| canonical_json("continuation creation receipt", value))
                    .transpose()?,
                runtime
                    .initial_unknown_receipt
                    .as_ref()
                    .map(|value| canonical_json("initial unknown creation receipt", value))
                    .transpose()?,
                runtime
                    .candidate_binding
                    .as_ref()
                    .map(|value| canonical_json("continuation candidate binding", value))
                    .transpose()?,
                runtime
                    .candidate_acceptance
                    .as_ref()
                    .map(|value| canonical_json("continuation candidate acceptance", value))
                    .transpose()?,
                canonical_json(
                    "continuation candidate acceptance history",
                    &runtime.candidate_acceptance_history,
                )?,
                now_us,
                canonical_json("continuation abort result", &result_value)?,
            ],
        )
        .map_err(storage_error)?;
    let scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    insert_rotation_event(
        &transaction,
        &scope,
        options.rotation_id,
        options.listener_session_id,
        "continuation.rotation_aborted",
        now_us,
    )?;
    session::update_session_activity(&transaction, options.listener_session_id, now_us)?;
    transaction.commit().map_err(storage_error)?;
    continuation_rotation(connection.inner(), options.rotation_id)
}

pub(crate) fn commit_continuation_rotation(
    connection: &mut StoreConnection,
    options: &ContinuationRotationCommitOptions,
) -> Result<ContinuationRotationCommitResult> {
    connection.verify_foreign_keys()?;
    let initial = load_rotation_snapshot(connection.inner(), options.rotation_id)?;
    if initial.abort_only {
        return Err(WorkVcsError::ContinuationInvalid(
            "an abort-only continuation rotation cannot be committed".to_owned(),
        ));
    }
    if initial.state == ContinuationRotationState::Committed {
        return committed_replay_result(&initial, options);
    }
    if initial.state == ContinuationRotationState::Aborted {
        return Err(WorkVcsError::ContinuationInvalid(
            "an aborted continuation rotation cannot be committed".to_owned(),
        ));
    }
    refresh_projection_for_branch(connection, initial.branch_id)?;
    let now_us = current_epoch_micros()?;
    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let occurrence = load_rotation_occurrence(&transaction, options.rotation_id)?;
    let existing = load_rotation_snapshot(&transaction, options.rotation_id)?;
    if existing.state == ContinuationRotationState::Committed {
        return committed_replay_result(&existing, options);
    }
    if existing.state == ContinuationRotationState::Aborted {
        return Err(WorkVcsError::ContinuationInvalid(
            "an aborted continuation rotation cannot be committed".to_owned(),
        ));
    }
    let runtime = load_active_rotation(&transaction, options.rotation_id)?;
    require_active_rotation_actor(
        &transaction,
        &occurrence,
        &runtime,
        options.listener_session_id,
        options.expected_listener_revision,
    )?;
    if runtime.state != ContinuationRotationState::CandidateAccepted
        || occurrence.source_owner_epoch != options.expected_owner_epoch
        || occurrence.handoff_digest != options.expected_handoff_digest
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation rotation commit compare-and-swap failed".to_owned(),
        ));
    }
    let candidate_session_id = runtime.candidate_session_id.ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation rotation must have an attached candidate before commit".to_owned(),
        )
    })?;
    require_rotation_participant_threads_unique(
        &transaction,
        &occurrence,
        &runtime,
        Some(candidate_session_id),
    )?;
    let candidate_binding = runtime.candidate_binding.as_ref().ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation rotation candidate binding is missing".to_owned(),
        )
    })?;
    let candidate_acceptance = runtime.candidate_acceptance.as_ref().ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation rotation candidate acceptance is missing".to_owned(),
        )
    })?;
    validate_candidate_binding(
        &transaction,
        &occurrence,
        &runtime,
        candidate_session_id,
        candidate_binding,
    )?;
    validate_candidate_acceptance(
        &transaction,
        &occurrence,
        &runtime,
        candidate_session_id,
        candidate_acceptance,
    )?;
    require_session_role(
        &transaction,
        candidate_session_id,
        &[ContinuationSessionRole::WorkCandidate],
    )?;
    let scope = load_scope(
        &transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    if scope.scope_id != occurrence.scope_id
        || scope.listener_session_id != runtime.listener_session_id
        || scope.listener_revision != runtime.listener_revision
        || scope.carrier_session_id != occurrence.source_session_id
        || scope.carrier_claim_id != occurrence.source_claim_id
        || scope.owner_epoch != occurrence.source_owner_epoch
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation scope changed before rotation commit".to_owned(),
        ));
    }
    let currentness = compute_scope_currentness(
        &transaction,
        &scope,
        occurrence.source_session_id,
        occurrence.source_claim_id,
        occurrence.handoff_digest,
        &occurrence.basis,
    )?;
    if currentness != runtime.scope_currentness_digest {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation scope currentness changed from {} to {} before commit",
            runtime.scope_currentness_digest, currentness
        )));
    }
    let transferred = claim::transfer_claim_in_transaction(
        &transaction,
        &crate::ClaimTransferOptions::new(
            occurrence.source_session_id,
            candidate_session_id,
            occurrence.source_claim_id,
        ),
        now_us,
    )?;
    let next_epoch = scope.owner_epoch.checked_add(1).ok_or_else(|| {
        WorkVcsError::ContinuationInvalid("continuation owner epoch overflow".to_owned())
    })?;
    let changed = transaction
        .execute(
            "UPDATE continuation_scope_runtime
             SET carrier_session_id = ?1, carrier_claim_id = ?2,
                 owner_epoch = ?3, updated_at_us = ?4
             WHERE workspace_id = ?5 AND branch_id = ?6 AND task_entity_id = ?7
               AND scope_id = ?8 AND listener_session_id = ?9 AND listener_revision = ?10
               AND carrier_session_id = ?11 AND carrier_claim_id = ?12 AND owner_epoch = ?13",
            params![
                &candidate_session_id.raw_bytes()[..],
                &transferred.claim_id.raw_bytes()[..],
                next_epoch,
                now_us,
                &scope.workspace_id.raw_bytes()[..],
                &scope.branch_id.raw_bytes()[..],
                &scope.task_entity_id.raw_bytes()[..],
                &scope.scope_id.raw_bytes()[..],
                &scope.listener_session_id.raw_bytes()[..],
                scope.listener_revision,
                &scope.carrier_session_id.raw_bytes()[..],
                &scope.carrier_claim_id.raw_bytes()[..],
                scope.owner_epoch,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation scope changed during rotation commit".to_owned(),
        ));
    }
    transaction
        .execute(
            "DELETE FROM continuation_rotation_runtime WHERE rotation_id = ?1",
            params![&options.rotation_id.raw_bytes()[..]],
        )
        .map_err(storage_error)?;
    let result_value = CanonicalValue::object(vec![
        (
            "candidate_acceptance".to_owned(),
            candidate_acceptance.clone(),
        ),
        ("candidate_binding".to_owned(), candidate_binding.clone()),
        (
            "creation_receipt".to_owned(),
            runtime.creation_receipt.clone().ok_or_else(|| {
                WorkVcsError::ContinuationInvalid(
                    "continuation rotation creation receipt is missing".to_owned(),
                )
            })?,
        ),
        (
            "initial_unknown_receipt".to_owned(),
            runtime
                .initial_unknown_receipt
                .clone()
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "candidate_acceptance_history".to_owned(),
            runtime.candidate_acceptance_history.clone(),
        ),
        (
            "dispatch".to_owned(),
            runtime.dispatch.clone().ok_or_else(|| {
                WorkVcsError::ContinuationInvalid(
                    "continuation rotation dispatch record is missing".to_owned(),
                )
            })?,
        ),
        (
            "owner_epoch".to_owned(),
            CanonicalValue::safe_integer(next_epoch)?,
        ),
        (
            "recovery_history_count".to_owned(),
            CanonicalValue::safe_integer(existing.recovery_history_count)?,
        ),
        (
            "recovery_history_chain_digest".to_owned(),
            CanonicalValue::String(existing.recovery_history_chain_digest.to_string()),
        ),
    ])?;
    transaction
        .execute(
            "INSERT INTO continuation_rotation_result(
                rotation_id, status, candidate_session_id, resulting_claim_id,
                listener_session_id, listener_revision, scope_currentness_digest,
                abort_only, recovery_revision, recovery_json,
                dispatch_json, creation_receipt_json, initial_unknown_receipt_json,
                candidate_binding_json, candidate_acceptance_json,
                candidate_acceptance_history_json, ended_at_us, result_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                &options.rotation_id.raw_bytes()[..],
                COMMITTED_STATUS,
                &candidate_session_id.raw_bytes()[..],
                &transferred.claim_id.raw_bytes()[..],
                &runtime.listener_session_id.raw_bytes()[..],
                runtime.listener_revision,
                &runtime.scope_currentness_digest.as_bytes()[..],
                if runtime.abort_only { 1_i64 } else { 0_i64 },
                runtime.recovery_revision,
                runtime
                    .recovery
                    .as_ref()
                    .map(|value| canonical_json("continuation abort-only recovery", value))
                    .transpose()?,
                runtime
                    .dispatch
                    .as_ref()
                    .map(|value| canonical_json("continuation dispatch", value))
                    .transpose()?,
                runtime
                    .creation_receipt
                    .as_ref()
                    .map(|value| canonical_json("continuation creation receipt", value))
                    .transpose()?,
                runtime
                    .initial_unknown_receipt
                    .as_ref()
                    .map(|value| canonical_json("initial unknown creation receipt", value))
                    .transpose()?,
                canonical_json("continuation candidate binding", candidate_binding)?,
                canonical_json("continuation candidate acceptance", candidate_acceptance)?,
                canonical_json(
                    "continuation candidate acceptance history",
                    &runtime.candidate_acceptance_history,
                )?,
                now_us,
                canonical_json("continuation commit result", &result_value)?,
            ],
        )
        .map_err(storage_error)?;
    let committed_scope = ContinuationScopeSnapshot {
        carrier_session_id: candidate_session_id,
        carrier_claim_id: transferred.claim_id,
        owner_epoch: next_epoch,
        updated_at_us: now_us,
        ..scope
    };
    insert_rotation_event(
        &transaction,
        &committed_scope,
        options.rotation_id,
        options.listener_session_id,
        "continuation.rotation_committed",
        now_us,
    )?;
    let committed_rotation = load_rotation_snapshot(&transaction, options.rotation_id)?;
    transaction.commit().map_err(storage_error)?;
    Ok(ContinuationRotationCommitResult {
        rotation: committed_rotation,
        scope: committed_scope,
        previous_claim_id: transferred.previous_claim_id,
        resulting_claim_id: transferred.claim_id,
    })
}

pub(crate) fn continuation_scope(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
) -> Result<ContinuationScopeSnapshot> {
    load_scope(connection, workspace_id, branch_id, task_entity_id)
}

pub(crate) fn continuation_rotation(
    connection: &Connection,
    rotation_id: ContinuationRotationId,
) -> Result<ContinuationRotationSnapshot> {
    load_rotation_snapshot(connection, rotation_id)
}

pub(super) fn retire_scope_for_claim_change(
    transaction: &Transaction<'_>,
    claim_id: ClaimId,
    actor_session_id: SessionId,
    now_us: i64,
) -> Result<()> {
    let Some(scope) = load_scope_by_claim_optional(transaction, claim_id)? else {
        return Ok(());
    };
    if active_rotation_for_scope(
        transaction,
        scope.workspace_id,
        scope.branch_id,
        scope.task_entity_id,
    )?
    .is_some()
    {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "claim {claim_id} carries a continuation scope with an active rotation"
        )));
    }
    delete_scope(transaction, &scope, actor_session_id, now_us)
}

pub(super) fn prepare_session_departure(
    transaction: &Transaction<'_>,
    session_id: SessionId,
    target: Option<(WorkspaceId, BranchId)>,
    now_us: i64,
) -> Result<()> {
    let active_count = match target {
        Some((workspace_id, branch_id)) => transaction
            .query_row(
                "SELECT count(*)
                 FROM continuation_rotation_runtime AS runtime
                 INNER JOIN continuation_rotation AS occurrence
                   ON occurrence.rotation_id = runtime.rotation_id
                 WHERE (occurrence.source_session_id = ?1 OR runtime.candidate_session_id = ?1)
                   AND occurrence.workspace_id = ?2 AND occurrence.branch_id = ?3",
                params![
                    &session_id.raw_bytes()[..],
                    &workspace_id.raw_bytes()[..],
                    &branch_id.raw_bytes()[..]
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)?,
        None => transaction
            .query_row(
                "SELECT count(*)
                 FROM continuation_rotation_runtime AS runtime
                 INNER JOIN continuation_rotation AS occurrence
                   ON occurrence.rotation_id = runtime.rotation_id
                 WHERE occurrence.source_session_id = ?1 OR runtime.candidate_session_id = ?1",
                params![&session_id.raw_bytes()[..]],
                |row| row.get::<_, i64>(0),
            )
            .map_err(storage_error)?,
    };
    if active_count != 0 {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "session {session_id} is a source or attached candidate in an active continuation rotation"
        )));
    }

    let rows = {
        let mut statement = match target {
            Some(_) => transaction
                .prepare(
                    "SELECT workspace_id, branch_id, task_entity_id
                     FROM continuation_scope_runtime
                     WHERE carrier_session_id = ?1 AND workspace_id = ?2 AND branch_id = ?3",
                )
                .map_err(storage_error)?,
            None => transaction
                .prepare(
                    "SELECT workspace_id, branch_id, task_entity_id
                     FROM continuation_scope_runtime WHERE carrier_session_id = ?1",
                )
                .map_err(storage_error)?,
        };
        match target {
            Some((workspace_id, branch_id)) => statement
                .query_map(
                    params![
                        &session_id.raw_bytes()[..],
                        &workspace_id.raw_bytes()[..],
                        &branch_id.raw_bytes()[..]
                    ],
                    |row| {
                        Ok((
                            row.get::<_, Vec<u8>>(0)?,
                            row.get::<_, Vec<u8>>(1)?,
                            row.get::<_, Vec<u8>>(2)?,
                        ))
                    },
                )
                .map_err(storage_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage_error)?,
            None => statement
                .query_map(params![&session_id.raw_bytes()[..]], |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                })
                .map_err(storage_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage_error)?,
        }
    };
    for (workspace, branch, task) in rows {
        let scope = load_scope(
            transaction,
            decode_workspace_id("workspace_id", workspace)?,
            decode_branch_id("branch_id", branch)?,
            decode_entity_id("task_entity_id", task)?,
        )?;
        if active_rotation_for_scope(
            transaction,
            scope.workspace_id,
            scope.branch_id,
            scope.task_entity_id,
        )?
        .is_some()
        {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "session {session_id} carries a continuation scope with an active rotation"
            )));
        }
        delete_scope(transaction, &scope, session_id, now_us)?;
    }
    Ok(())
}

fn committed_replay_result(
    existing: &ContinuationRotationSnapshot,
    options: &ContinuationRotationCommitOptions,
) -> Result<ContinuationRotationCommitResult> {
    if existing.listener_session_id != options.listener_session_id
        || existing.listener_revision != options.expected_listener_revision
        || existing.source_owner_epoch != options.expected_owner_epoch
        || existing.handoff_digest != options.expected_handoff_digest
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation rotation commit replay compare-and-swap failed".to_owned(),
        ));
    }
    let candidate_session_id = existing.candidate_session_id.ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "committed continuation rotation is missing its candidate session".to_owned(),
        )
    })?;
    let resulting_claim_id = existing.resulting_claim_id.ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "committed continuation rotation is missing its resulting claim".to_owned(),
        )
    })?;
    let owner_epoch = existing.source_owner_epoch.checked_add(1).ok_or_else(|| {
        WorkVcsError::ContinuationInvalid("continuation owner epoch overflow".to_owned())
    })?;
    let scope = ContinuationScopeSnapshot {
        scope_id: existing.scope_id,
        workspace_id: existing.workspace_id,
        branch_id: existing.branch_id,
        task_entity_id: existing.task_entity_id,
        listener_session_id: existing.listener_session_id,
        listener_revision: existing.listener_revision,
        carrier_session_id: candidate_session_id,
        carrier_claim_id: resulting_claim_id,
        owner_epoch,
        updated_at_us: existing.ended_at_us.unwrap_or(existing.last_activity_at_us),
    };
    Ok(ContinuationRotationCommitResult {
        previous_claim_id: existing.source_claim_id,
        resulting_claim_id,
        rotation: existing.clone(),
        scope,
    })
}

fn refresh_projection_for_branch(
    connection: &mut StoreConnection,
    branch_id: BranchId,
) -> Result<()> {
    history::refresh_branch_projection(connection, BranchProjectionRefreshOptions::new(branch_id))?;
    Ok(())
}

fn compute_scope_currentness(
    connection: &Connection,
    scope: &ContinuationScopeSnapshot,
    source_session_id: SessionId,
    source_claim_id: ClaimId,
    handoff_digest: Digest,
    basis_value: &CanonicalValue,
) -> Result<Digest> {
    ensure_projection_current(connection, scope.workspace_id, scope.branch_id)?;
    let basis = parse_basis(basis_value)?;
    let task = load_projected_entity(connection, scope.branch_id, scope.task_entity_id)?;
    if task.entity_kind != "task" {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation scope entity {} is not a Task",
            scope.task_entity_id
        )));
    }
    let handoff = load_projected_entity(connection, scope.branch_id, basis.handoff_record_id)?;
    if handoff.entity_kind != "record" || handoff.record_kind.as_deref() != Some("handoff") {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation handoff entity {} is not a Handoff Record",
            basis.handoff_record_id
        )));
    }
    if handoff.state_digest != handoff_digest {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation handoff digest {} does not match current Handoff Record digest {}",
            handoff_digest, handoff.state_digest
        )));
    }
    let source = session::load_active_session_runtime_for_update(connection, source_session_id)?;
    require_active_session_target("source", &source, scope.workspace_id, scope.branch_id)?;
    require_active_exclusive_claim(
        connection,
        source_claim_id,
        source_session_id,
        scope.workspace_id,
        scope.branch_id,
        scope.task_entity_id,
    )?;
    let claim_activity = connection
        .query_row(
            "SELECT last_activity_at_us FROM claim_runtime WHERE claim_id = ?1",
            params![&source_claim_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    let dependencies = basis
        .currentness_entity_ids
        .iter()
        .map(|entity_id| {
            let projected = load_projected_entity(connection, scope.branch_id, *entity_id)?;
            projected_entity_value(*entity_id, &projected)
        })
        .collect::<Result<Vec<_>>>()?;
    let value = CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(scope.branch_id.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(scope.scope_id.to_string()),
        ),
        (
            "currentness_entities".to_owned(),
            CanonicalValue::Array(dependencies),
        ),
        (
            "handoff".to_owned(),
            projected_entity_value(basis.handoff_record_id, &handoff)?,
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(scope.listener_revision)?,
        ),
        (
            "listener_session_id".to_owned(),
            CanonicalValue::String(scope.listener_session_id.to_string()),
        ),
        (
            "owner_epoch".to_owned(),
            CanonicalValue::safe_integer(scope.owner_epoch)?,
        ),
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1)?,
        ),
        (
            "source_claim".to_owned(),
            CanonicalValue::object(vec![
                (
                    "claim_id".to_owned(),
                    CanonicalValue::String(source_claim_id.to_string()),
                ),
                (
                    "last_activity_at_us".to_owned(),
                    CanonicalValue::safe_integer(claim_activity)?,
                ),
            ])?,
        ),
        (
            "source_session".to_owned(),
            CanonicalValue::object(vec![
                (
                    "last_activity_at_us".to_owned(),
                    CanonicalValue::safe_integer(source.last_activity_at_us)?,
                ),
                (
                    "session_id".to_owned(),
                    CanonicalValue::String(source_session_id.to_string()),
                ),
            ])?,
        ),
        (
            "task".to_owned(),
            projected_entity_value(scope.task_entity_id, &task)?,
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(scope.workspace_id.to_string()),
        ),
    ])?;
    Ok(Digest::domain_separated(
        CURRENTNESS_DIGEST_DOMAIN,
        &canonical_bytes(&value)?,
    ))
}

fn observe_scope_currentness_for_recovery(
    connection: &Connection,
    scope: &ContinuationScopeSnapshot,
    source_session_id: SessionId,
    source_claim_id: ClaimId,
    basis_value: &CanonicalValue,
) -> Result<Digest> {
    ensure_projection_current(connection, scope.workspace_id, scope.branch_id)?;
    let basis = parse_basis(basis_value)?;
    let task = load_projected_entity_optional(connection, scope.branch_id, scope.task_entity_id)?;
    let handoff =
        load_projected_entity_optional(connection, scope.branch_id, basis.handoff_record_id)?;
    let source = session::load_active_session_runtime_for_update(connection, source_session_id)?;
    require_active_session_target("source", &source, scope.workspace_id, scope.branch_id)?;
    require_active_exclusive_claim(
        connection,
        source_claim_id,
        source_session_id,
        scope.workspace_id,
        scope.branch_id,
        scope.task_entity_id,
    )?;
    let claim_activity = connection
        .query_row(
            "SELECT last_activity_at_us FROM claim_runtime WHERE claim_id = ?1",
            params![&source_claim_id.raw_bytes()[..]],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    let dependencies = basis
        .currentness_entity_ids
        .iter()
        .map(|entity_id| {
            let projected =
                load_projected_entity_optional(connection, scope.branch_id, *entity_id)?;
            observed_projected_entity_value(*entity_id, projected.as_ref())
        })
        .collect::<Result<Vec<_>>>()?;
    let value = CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(scope.branch_id.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(scope.scope_id.to_string()),
        ),
        (
            "currentness_entities".to_owned(),
            CanonicalValue::Array(dependencies),
        ),
        (
            "handoff".to_owned(),
            observed_projected_entity_value(basis.handoff_record_id, handoff.as_ref())?,
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(scope.listener_revision)?,
        ),
        (
            "listener_session_id".to_owned(),
            CanonicalValue::String(scope.listener_session_id.to_string()),
        ),
        (
            "owner_epoch".to_owned(),
            CanonicalValue::safe_integer(scope.owner_epoch)?,
        ),
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1)?,
        ),
        (
            "source_claim".to_owned(),
            CanonicalValue::object(vec![
                (
                    "claim_id".to_owned(),
                    CanonicalValue::String(source_claim_id.to_string()),
                ),
                (
                    "last_activity_at_us".to_owned(),
                    CanonicalValue::safe_integer(claim_activity)?,
                ),
            ])?,
        ),
        (
            "source_session".to_owned(),
            CanonicalValue::object(vec![
                (
                    "last_activity_at_us".to_owned(),
                    CanonicalValue::safe_integer(source.last_activity_at_us)?,
                ),
                (
                    "session_id".to_owned(),
                    CanonicalValue::String(source_session_id.to_string()),
                ),
            ])?,
        ),
        (
            "task".to_owned(),
            observed_projected_entity_value(scope.task_entity_id, task.as_ref())?,
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(scope.workspace_id.to_string()),
        ),
    ])?;
    Ok(Digest::domain_separated(
        CURRENTNESS_DIGEST_DOMAIN,
        &canonical_bytes(&value)?,
    ))
}

fn ensure_projection_current(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
) -> Result<()> {
    let row = connection
        .query_row(
            "SELECT branch.workspace_id, branch.head_commit_id,
                    workstate_commit.state_digest,
                    branch_projection_state.projection_status,
                    branch_projection_state.projected_commit_id,
                    branch_projection_state.projection_state_digest
             FROM branch
             INNER JOIN workstate_commit
               ON workstate_commit.commit_id = branch.head_commit_id
             LEFT JOIN branch_projection_state
               ON branch_projection_state.branch_id = branch.branch_id
             WHERE branch.branch_id = ?1",
            params![&branch_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<Vec<u8>>>(4)?,
                    row.get::<_, Option<Vec<u8>>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((stored_workspace, head, state_digest, status, projected, projected_digest)) = row
    else {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation branch {branch_id} does not exist"
        )));
    };
    if decode_workspace_id("branch.workspace_id", stored_workspace)? != workspace_id
        || status.as_deref() != Some("complete")
        || projected.as_deref() != Some(head.as_slice())
        || projected_digest.as_deref() != Some(state_digest.as_slice())
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation branch projection is not current inside the operation transaction"
                .to_owned(),
        ));
    }
    Ok(())
}

fn load_projected_entity(
    connection: &Connection,
    branch_id: BranchId,
    entity_id: EntityId,
) -> Result<ProjectedEntity> {
    let row = connection
        .query_row(
            "SELECT current.entity_version_id, version.state_digest,
                    entity.entity_kind,
                    CASE WHEN entity.entity_kind = 'record'
                         THEN json_extract(version.state_json, '$.kind')
                         ELSE NULL END
             FROM branch_entity_current AS current
             INNER JOIN entity_version AS version
               ON version.entity_id = current.entity_id
              AND version.entity_version_id = current.entity_version_id
             INNER JOIN entity ON entity.object_id = current.entity_id
             WHERE current.branch_id = ?1 AND current.entity_id = ?2",
            params![&branch_id.raw_bytes()[..], &entity_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((version, digest, entity_kind, record_kind)) = row else {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation currentness entity {entity_id} is not current on branch {branch_id}"
        )));
    };
    Ok(ProjectedEntity {
        entity_version_id: decode_entity_version_id("entity_version_id", version)?,
        state_digest: decode_digest("state_digest", digest)?,
        entity_kind,
        record_kind,
    })
}

fn load_projected_entity_optional(
    connection: &Connection,
    branch_id: BranchId,
    entity_id: EntityId,
) -> Result<Option<ProjectedEntity>> {
    let row = connection
        .query_row(
            "SELECT current.entity_version_id, version.state_digest,
                    entity.entity_kind,
                    CASE WHEN entity.entity_kind = 'record'
                         THEN json_extract(version.state_json, '$.kind')
                         ELSE NULL END
             FROM branch_entity_current AS current
             INNER JOIN entity_version AS version
               ON version.entity_id = current.entity_id
              AND version.entity_version_id = current.entity_version_id
             INNER JOIN entity ON entity.object_id = current.entity_id
             WHERE current.branch_id = ?1 AND current.entity_id = ?2",
            params![&branch_id.raw_bytes()[..], &entity_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    row.map(|(version, digest, entity_kind, record_kind)| {
        Ok(ProjectedEntity {
            entity_version_id: decode_entity_version_id("entity_version_id", version)?,
            state_digest: decode_digest("state_digest", digest)?,
            entity_kind,
            record_kind,
        })
    })
    .transpose()
}

fn observed_projected_entity_value(
    entity_id: EntityId,
    projected: Option<&ProjectedEntity>,
) -> Result<CanonicalValue> {
    match projected {
        Some(projected) => projected_entity_value(entity_id, projected),
        None => CanonicalValue::object(vec![
            (
                "entity_id".to_owned(),
                CanonicalValue::String(entity_id.to_string()),
            ),
            (
                "observation".to_owned(),
                CanonicalValue::String("missing_from_branch".to_owned()),
            ),
        ]),
    }
}

fn projected_entity_value(
    entity_id: EntityId,
    projected: &ProjectedEntity,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "entity_id".to_owned(),
            CanonicalValue::String(entity_id.to_string()),
        ),
        (
            "entity_kind".to_owned(),
            CanonicalValue::String(projected.entity_kind.clone()),
        ),
        (
            "entity_version_id".to_owned(),
            CanonicalValue::String(projected.entity_version_id.to_string()),
        ),
        (
            "record_kind".to_owned(),
            projected
                .record_kind
                .as_ref()
                .map(|value| CanonicalValue::String(value.clone()))
                .unwrap_or(CanonicalValue::Null),
        ),
        (
            "state_digest".to_owned(),
            CanonicalValue::String(projected.state_digest.to_string()),
        ),
    ])
}

fn require_active_rotation_actor(
    transaction: &Transaction<'_>,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    listener_session_id: SessionId,
    expected_listener_revision: i64,
) -> Result<ContinuationScopeSnapshot> {
    if runtime.listener_session_id != listener_session_id
        || runtime.listener_revision != expected_listener_revision
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation listener compare-and-swap failed".to_owned(),
        ));
    }
    let scope = load_scope(
        transaction,
        occurrence.workspace_id,
        occurrence.branch_id,
        occurrence.task_entity_id,
    )?;
    if scope.scope_id != occurrence.scope_id
        || scope.listener_session_id != listener_session_id
        || scope.listener_revision != expected_listener_revision
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation scope listener changed before the operation".to_owned(),
        ));
    }
    let listener =
        session::load_active_session_runtime_for_update(transaction, listener_session_id)?;
    require_active_session_target(
        "listener",
        &listener,
        occurrence.workspace_id,
        occurrence.branch_id,
    )?;
    require_session_role(
        transaction,
        listener_session_id,
        &[ContinuationSessionRole::Listener],
    )?;
    Ok(scope)
}

fn require_normal_rotation(runtime: &ActiveRotationProjection, operation: &str) -> Result<()> {
    if runtime.abort_only {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "an abort-only continuation rotation cannot {operation}"
        )))
    } else {
        Ok(())
    }
}

fn require_snapshot_actor(
    snapshot: &ContinuationRotationSnapshot,
    listener_session_id: SessionId,
    expected_listener_revision: i64,
) -> Result<()> {
    if snapshot.listener_session_id == listener_session_id
        && snapshot.listener_revision == expected_listener_revision
    {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(
            "continuation terminal or replay actor does not match the recorded listener revision"
                .to_owned(),
        ))
    }
}

fn validate_listener_replacement_authority(
    connection: &Connection,
    options: &ContinuationListenerReplaceOptions,
    scope: &ContinuationScopeSnapshot,
) -> Result<()> {
    let label = "continuation listener replacement authority";
    require_schema_version(&options.authority, label)?;
    require_matching_string(
        &options.authority,
        "scope_id",
        &scope.scope_id.to_string(),
        label,
    )?;
    require_matching_string(
        &options.authority,
        "actor_session_id",
        &options.actor_session_id.to_string(),
        label,
    )?;
    require_matching_string(
        &options.authority,
        "expected_listener_session_id",
        &options.expected_listener_session_id.to_string(),
        label,
    )?;
    require_matching_string(
        &options.authority,
        "replacement_listener_session_id",
        &options.replacement_listener_session_id.to_string(),
        label,
    )?;
    require_matching_integer(
        &options.authority,
        "expected_listener_revision",
        options.expected_listener_revision,
        label,
    )?;
    require_matching_integer(
        &options.authority,
        "expected_owner_epoch",
        options.expected_owner_epoch,
        label,
    )?;
    let mode = require_nonempty_string(&options.authority, "mode", label)?;
    let actor =
        session::load_active_session_runtime_for_update(connection, options.actor_session_id)?;
    require_active_session_target(
        "listener replacement actor",
        &actor,
        options.workspace_id,
        options.branch_id,
    )?;
    require_session_role(
        connection,
        options.actor_session_id,
        &[ContinuationSessionRole::Listener],
    )?;
    match mode {
        "current_listener_delegated" => {
            if options.actor_session_id != options.expected_listener_session_id {
                return Err(WorkVcsError::ContinuationInvalid(
                    "delegated listener replacement must be authored by the current listener"
                        .to_owned(),
                ));
            }
            require_digest_string(&options.authority, "delegation_digest", label)?;
        }
        "unavailable_listener_recovery" => {
            if options.actor_session_id != options.replacement_listener_session_id {
                return Err(WorkVcsError::ContinuationInvalid(
                    "unavailable-listener recovery must be authored by the replacement listener"
                        .to_owned(),
                ));
            }
            if session::session_lifecycle_state_for_connection(
                connection,
                options.expected_listener_session_id,
            )? == SessionLifecycleState::Active
            {
                return Err(WorkVcsError::ContinuationInvalid(
                    "unavailable-listener recovery requires the prior listener to be non-active"
                        .to_owned(),
                ));
            }
            require_digest_string(&options.authority, "evidence_digest", label)?;
        }
        _ => {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "{label} mode {mode:?} is unsupported"
            )));
        }
    }
    Ok(())
}

fn validate_abort_only_recovery_authority(
    options: &ContinuationAbortOnlyRecoveryOptions,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    prior_listener_state: SessionLifecycleState,
    observed_currentness: Digest,
) -> Result<()> {
    let label = "continuation abort-only recovery authority";
    require_schema_version(&options.authority, label)?;
    for (field, expected) in [
        ("mode", "abort_only".to_owned()),
        ("rotation_id", options.rotation_id.to_string()),
        ("scope_id", options.expected_scope_id.to_string()),
        ("actor_session_id", options.actor_session_id.to_string()),
        (
            "expected_listener_session_id",
            options.expected_listener_session_id.to_string(),
        ),
        (
            "replacement_listener_session_id",
            options.replacement_listener_session_id.to_string(),
        ),
        (
            "expected_listener_lifecycle_state",
            session_lifecycle_state_label(prior_listener_state).to_owned(),
        ),
        (
            "original_scope_currentness_digest",
            runtime.scope_currentness_digest.to_string(),
        ),
        (
            "observed_scope_currentness_digest",
            observed_currentness.to_string(),
        ),
        (
            "creation_operation_id",
            occurrence.creation_operation_id.clone(),
        ),
    ] {
        require_matching_string(&options.authority, field, &expected, label)?;
    }
    for (field, expected) in [
        (
            "expected_listener_revision",
            options.expected_listener_revision,
        ),
        (
            "expected_recovery_revision",
            options.expected_recovery_revision,
        ),
        ("expected_owner_epoch", options.expected_owner_epoch),
    ] {
        require_matching_integer(&options.authority, field, expected, label)?;
    }
    let expected_drift = observed_currentness != runtime.scope_currentness_digest;
    if require_bool(&options.authority, "currentness_drifted", label)? != expected_drift {
        return Err(WorkVcsError::ContinuationInvalid(
            "abort-only recovery authority currentness_drifted does not match observed state"
                .to_owned(),
        ));
    }
    require_digest_string(&options.authority, "recovery_evidence_digest", label)?;
    require_digest_string(&options.authority, "authorization_receipt_digest", label)?;
    require_positive_integer(&options.authority, "authorized_at_us", label)?;
    Ok(())
}

fn abort_only_recovery_matches(
    runtime: &ActiveRotationProjection,
    options: &ContinuationAbortOnlyRecoveryOptions,
    next_listener_revision: i64,
    next_recovery_revision: i64,
    authority_digest: Digest,
) -> Result<bool> {
    let Some(recovery) = runtime.recovery.as_ref() else {
        return Ok(false);
    };
    Ok(
        require_nonempty_string(recovery, "mode", "continuation abort-only recovery")?
            == "abort_only"
            && require_nonempty_string(
                recovery,
                "rotation_id",
                "continuation abort-only recovery",
            )? == options.rotation_id.to_string()
            && require_nonempty_string(recovery, "scope_id", "continuation abort-only recovery")?
                == options.expected_scope_id.to_string()
            && require_nonempty_string(
                recovery,
                "old_listener_session_id",
                "continuation abort-only recovery",
            )? == options.expected_listener_session_id.to_string()
            && require_integer(
                recovery,
                "old_listener_revision",
                "continuation abort-only recovery",
            )? == options.expected_listener_revision
            && require_nonempty_string(
                recovery,
                "new_listener_session_id",
                "continuation abort-only recovery",
            )? == options.replacement_listener_session_id.to_string()
            && require_integer(
                recovery,
                "new_listener_revision",
                "continuation abort-only recovery",
            )? == next_listener_revision
            && require_integer(
                recovery,
                "recovery_revision",
                "continuation abort-only recovery",
            )? == next_recovery_revision
            && require_nonempty_string(
                recovery,
                "authority_digest",
                "continuation abort-only recovery",
            )? == authority_digest.to_string(),
    )
}

fn listener_replacement_event_matches(
    connection: &Connection,
    options: &ContinuationListenerReplaceOptions,
    next_revision: i64,
    authority_json: &str,
) -> Result<bool> {
    let authority_digest = Digest::domain_separated(
        "workvcs.continuation.listener-authority.v1",
        authority_json.as_bytes(),
    );
    let count = connection
        .query_row(
            "SELECT count(*)
             FROM event
             WHERE workspace_id = ?1
               AND session_id = ?2
               AND event_kind = 'continuation.listener_replaced'
               AND json_extract(payload_json, '$.branch_id') = ?3
               AND json_extract(payload_json, '$.scope_id') = ?4
               AND json_extract(payload_json, '$.task_entity_id') = ?5
               AND json_extract(payload_json, '$.old_listener_session_id') = ?6
               AND json_extract(payload_json, '$.old_listener_revision') = ?7
               AND json_extract(payload_json, '$.new_listener_session_id') = ?8
               AND json_extract(payload_json, '$.new_listener_revision') = ?9
               AND json_extract(payload_json, '$.owner_epoch') = ?10
               AND json_extract(payload_json, '$.authority_digest') = ?11",
            params![
                &options.workspace_id.raw_bytes()[..],
                &options.actor_session_id.raw_bytes()[..],
                options.branch_id.to_string(),
                options.expected_scope_id.to_string(),
                options.task_entity_id.to_string(),
                options.expected_listener_session_id.to_string(),
                options.expected_listener_revision,
                options.replacement_listener_session_id.to_string(),
                next_revision,
                options.expected_owner_epoch,
                authority_digest.to_string(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    Ok(count == 1)
}

fn validate_project_policy_authority(options: &ContinuationProjectPolicySetOptions) -> Result<()> {
    let label = "continuation project policy authority";
    require_schema_version(&options.authority, label)?;
    require_matching_string(
        &options.authority,
        "action",
        if options.enabled { "enable" } else { "disable" },
        label,
    )?;
    require_matching_string(&options.authority, "project_id", &options.project_id, label)?;
    require_matching_string(
        &options.authority,
        "actor_session_id",
        &options.actor_session_id.to_string(),
        label,
    )?;
    require_matching_integer(
        &options.authority,
        "expected_revision",
        options.expected_revision,
        label,
    )?;
    require_matching_string(
        &options.authority,
        "authorization_envelope_kind",
        "trusted_product_control_plane",
        label,
    )?;
    require_nonempty_string(&options.authority, "product_actor_id", label)?;
    require_digest_string(
        &options.authority,
        "user_authorization_receipt_digest",
        label,
    )?;
    require_digest_string(
        &options.authority,
        "project_membership_receipt_digest",
        label,
    )?;
    require_positive_integer(&options.authority, "authorized_at_us", label)?;
    Ok(())
}

fn load_project_policy_optional(
    connection: &Connection,
    project_id: &str,
) -> Result<Option<ContinuationProjectPolicySnapshot>> {
    let row = connection
        .query_row(
            "SELECT enabled, revision, actor_session_id, authority_json,
                    receipt_digest, updated_at_us
             FROM continuation_project_policy WHERE project_id = ?1",
            params![project_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    row.map(
        |(enabled, revision, actor, authority, receipt_digest, updated_at_us)| {
            Ok(ContinuationProjectPolicySnapshot {
                project_id: project_id.to_owned(),
                enabled: match enabled {
                    0 => false,
                    1 => true,
                    other => {
                        return Err(WorkVcsError::ContinuationInvalid(format!(
                            "stored continuation project policy enabled flag {other} is invalid"
                        )));
                    }
                },
                revision,
                actor_session_id: decode_session_id("actor_session_id", actor)?,
                authority: parse_continuation_json("project policy authority_json", &authority)?,
                receipt_digest: decode_digest("receipt_digest", receipt_digest)?,
                updated_at_us,
            })
        },
    )
    .transpose()
}

fn require_source_role_and_provenance(
    transaction: &Transaction<'_>,
    session_id: SessionId,
    claim_id: ClaimId,
    scope: &ContinuationScopeSnapshot,
) -> Result<()> {
    let role = require_session_role(
        transaction,
        session_id,
        &[
            ContinuationSessionRole::Work,
            ContinuationSessionRole::WorkCandidate,
        ],
    )?;
    let source_thread_id = require_unique_session_thread(transaction, session_id, "source")?;
    if role == ContinuationSessionRole::Work {
        return ensure_thread_not_historical_created_target(
            transaction,
            &source_thread_id,
            "continuation work source",
        );
    }
    let count = transaction
        .query_row(
            "SELECT count(*)
             FROM continuation_rotation_result AS result
             INNER JOIN continuation_rotation AS occurrence
               ON occurrence.rotation_id = result.rotation_id
             WHERE result.status = ?1
               AND result.candidate_session_id = ?2
               AND result.resulting_claim_id = ?3
               AND occurrence.workspace_id = ?4
               AND occurrence.branch_id = ?5
               AND occurrence.task_entity_id = ?6
               AND json_extract(result.creation_receipt_json, '$.outcome') = 'created'
               AND json_extract(result.creation_receipt_json, '$.thread_id') = ?7",
            params![
                COMMITTED_STATUS,
                &session_id.raw_bytes()[..],
                &claim_id.raw_bytes()[..],
                &scope.workspace_id.raw_bytes()[..],
                &scope.branch_id.raw_bytes()[..],
                &scope.task_entity_id.raw_bytes()[..],
                source_thread_id,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    let created_occurrences = historical_created_thread_count(transaction, &source_thread_id)?;
    if count == 1 && created_occurrences == 1 {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "work_candidate session {session_id} is not the sole historical created target and committed carrier for this exact continuation scope and Claim"
        )))
    }
}

fn validate_policy_decision(
    connection: &Connection,
    scope: &ContinuationScopeSnapshot,
    options: &ContinuationRotationBeginOptions,
    basis: &ContinuationBasis,
    currentness: Digest,
    decision: &CanonicalValue,
) -> Result<()> {
    let label = "continuation policy decision";
    require_schema_version(decision, label)?;
    require_matching_string(decision, "action", "prepare_rotation", label)?;
    if !require_bool(decision, "continuation_enabled", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation policy decision requires stable Project opt-in".to_owned(),
        ));
    }
    if !require_bool(decision, "durable_provider_available", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation policy decision requires an available durable provider".to_owned(),
        ));
    }
    require_matching_string(
        decision,
        "project_opt_in_receipt_kind",
        "workvcs_project_policy",
        label,
    )?;
    validate_project_policy_reference(connection, basis, decision)?;
    require_digest_string(decision, "continuation_packet_digest", label)?;
    require_digest_string(decision, "initial_message_digest", label)?;
    require_bounded_positive_integer(
        decision,
        "continuation_packet_bytes",
        MAX_CONTINUATION_PACKET_BYTES,
        label,
    )?;
    require_bounded_positive_integer(
        decision,
        "continuation_packet_items",
        MAX_CONTINUATION_PACKET_ITEMS,
        label,
    )?;
    require_bounded_positive_integer(
        decision,
        "continuation_packet_token_budget",
        MAX_CONTINUATION_PACKET_TOKEN_BUDGET,
        label,
    )?;
    for (field, expected) in [
        ("project_id", basis.project_id.as_str()),
        ("scope_id", &scope.scope_id.to_string()),
        ("workspace_id", &scope.workspace_id.to_string()),
        ("branch_id", &scope.branch_id.to_string()),
        ("task_entity_id", &scope.task_entity_id.to_string()),
        (
            "listener_session_id",
            &options.listener_session_id.to_string(),
        ),
        ("source_session_id", &options.source_session_id.to_string()),
        ("source_claim_id", &options.source_claim_id.to_string()),
        ("handoff_digest", &options.handoff_digest.to_string()),
        ("scope_currentness_digest", &currentness.to_string()),
    ] {
        require_matching_string(decision, field, expected, label)?;
    }
    require_matching_integer(
        decision,
        "listener_revision",
        options.expected_listener_revision,
        label,
    )?;
    require_matching_integer(decision, "owner_epoch", options.expected_owner_epoch, label)?;
    for field in ["adapter_kind", "adapter_version"] {
        require_nonempty_string(decision, field, label)?;
    }
    require_positive_integer(decision, "evaluated_at_us", label)?;
    validate_evaluator_receipt(decision)?;
    Ok(())
}

fn validate_evaluator_receipt(decision: &CanonicalValue) -> Result<()> {
    let label = "continuation policy decision";
    require_matching_string(
        decision,
        "evaluator_version",
        CONTINUATION_EVALUATOR_VERSION,
        label,
    )?;
    let evaluator_result_digest =
        require_nonempty_string(decision, "evaluator_result_digest", label)?;
    require_digest_string(decision, "evaluator_result_digest", label)?;
    let expected_receipt_digest =
        require_nonempty_string(decision, "evaluator_receipt_digest", label)?;
    require_digest_string(decision, "evaluator_receipt_digest", label)?;
    let receipt = object_field(decision, "evaluator_receipt", label)?;
    let receipt_label = "continuation evaluator receipt";
    require_exact_object_fields(
        receipt,
        &[
            "schema_version",
            "evaluator_version",
            "result_digest",
            "result",
            "action",
            "gate_summary",
            "evidence_references",
        ],
        receipt_label,
    )?;
    require_schema_version(receipt, receipt_label)?;
    require_matching_string(
        receipt,
        "evaluator_version",
        CONTINUATION_EVALUATOR_VERSION,
        receipt_label,
    )?;
    require_matching_string(
        receipt,
        "result_digest",
        evaluator_result_digest,
        receipt_label,
    )?;
    require_matching_string(receipt, "action", "prepare_rotation", receipt_label)?;
    let result = object_field(receipt, "result", receipt_label)?;
    let result_label = "continuation evaluator result";
    require_exact_object_fields(
        result,
        &[
            "schema_version",
            "evaluator_version",
            "action",
            "reasons",
            "pressure_signal_count",
            "decision_identity",
            "listener_revision",
            "owner_epoch",
            "evaluated_at_us",
            "gate_summary",
            "evidence_references",
        ],
        result_label,
    )?;
    require_schema_version(result, result_label)?;
    require_matching_string(
        result,
        "evaluator_version",
        CONTINUATION_EVALUATOR_VERSION,
        result_label,
    )?;
    require_matching_string(result, "action", "prepare_rotation", result_label)?;
    match object_field(result, "reasons", result_label)? {
        CanonicalValue::Array(values)
            if values.as_slice() == [CanonicalValue::String("all_gates_satisfied".to_owned())] => {}
        _ => {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation evaluator result must contain only the all_gates_satisfied reason"
                    .to_owned(),
            ));
        }
    }
    let pressure_signal_count = require_integer(result, "pressure_signal_count", result_label)?;
    if !(2..=4).contains(&pressure_signal_count) {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation evaluator result requires 2 to 4 independent pressure signals".to_owned(),
        ));
    }
    for field in ["listener_revision", "owner_epoch", "evaluated_at_us"] {
        require_matching_integer(
            result,
            field,
            require_integer(decision, field, label)?,
            result_label,
        )?;
    }
    validate_evaluator_decision_identity(
        object_field(result, "decision_identity", result_label)?,
        decision,
    )?;
    let result_gate_summary = object_field(result, "gate_summary", result_label)?;
    validate_evaluator_gate_summary(result_gate_summary, pressure_signal_count)?;
    require_canonical_match(
        object_field(receipt, "gate_summary", receipt_label)?,
        result_gate_summary,
        "continuation evaluator receipt gate_summary",
    )?;
    let result_evidence = object_field(result, "evidence_references", result_label)?;
    validate_evaluator_evidence_references(result_evidence)?;
    require_canonical_match(
        object_field(receipt, "evidence_references", receipt_label)?,
        result_evidence,
        "continuation evaluator receipt evidence_references",
    )?;
    let actual_result_digest = hex::encode(Sha256::digest(canonical_bytes(result)?));
    if actual_result_digest != evaluator_result_digest {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation evaluator result digest does not match its embedded canonical result"
                .to_owned(),
        ));
    }
    let actual_receipt_digest = hex::encode(Sha256::digest(canonical_bytes(receipt)?));
    if actual_receipt_digest != expected_receipt_digest {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation evaluator receipt digest does not match its canonical receipt".to_owned(),
        ));
    }
    Ok(())
}

fn validate_evaluator_evidence_references(value: &CanonicalValue) -> Result<()> {
    let evidence_references = match value {
        CanonicalValue::Array(values)
            if !values.is_empty() && values.len() <= MAX_CONTINUATION_EVIDENCE_REFERENCES =>
        {
            values
        }
        CanonicalValue::Array(values) => {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "continuation evaluator evidence references contain {} entries; expected 1 to {}",
                values.len(),
                MAX_CONTINUATION_EVIDENCE_REFERENCES
            )));
        }
        _ => {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation evaluator evidence references must be an array".to_owned(),
            ));
        }
    };
    let mut seen = BTreeSet::new();
    for (index, reference) in evidence_references.iter().enumerate() {
        let reference_label = format!("continuation evaluator evidence reference {index}");
        require_exact_object_fields(
            reference,
            &["kind", "reference", "digest"],
            &reference_label,
        )?;
        let kind = validate_embedded_identifier(
            &format!("{reference_label} kind"),
            require_nonempty_string(reference, "kind", &reference_label)?,
        )?;
        let identity = validate_embedded_identifier(
            &format!("{reference_label} identity"),
            require_nonempty_string(reference, "reference", &reference_label)?,
        )?;
        let digest = require_nonempty_string(reference, "digest", &reference_label)?.to_owned();
        require_digest_string(reference, "digest", &reference_label)?;
        if !seen.insert((kind, identity, digest)) {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation evaluator evidence references must not contain duplicates".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_evaluator_decision_identity(
    identity: &CanonicalValue,
    decision: &CanonicalValue,
) -> Result<()> {
    let identity_label = "continuation evaluator decision identity";
    require_exact_object_fields(
        identity,
        &[
            "project_opt_in_receipt_kind",
            "project_opt_in_receipt_digest",
            "project_opt_in_revision",
            "project_opt_in_verified_at_us",
            "project_id",
            "scope_id",
            "workspace_id",
            "branch_id",
            "task_entity_id",
            "listener_session_id",
            "source_session_id",
            "source_claim_id",
            "handoff_digest",
            "scope_currentness_digest",
            "continuation_packet_digest",
            "continuation_packet_bytes",
            "continuation_packet_items",
            "continuation_packet_token_budget",
            "initial_message_digest",
            "adapter_kind",
            "adapter_version",
        ],
        identity_label,
    )?;
    for field in [
        "project_opt_in_receipt_kind",
        "project_opt_in_receipt_digest",
        "project_id",
        "scope_id",
        "workspace_id",
        "branch_id",
        "task_entity_id",
        "listener_session_id",
        "source_session_id",
        "source_claim_id",
        "handoff_digest",
        "scope_currentness_digest",
        "continuation_packet_digest",
        "initial_message_digest",
        "adapter_kind",
        "adapter_version",
    ] {
        require_matching_string(
            identity,
            field,
            require_nonempty_string(decision, field, "continuation policy decision")?,
            identity_label,
        )?;
    }
    for field in [
        "project_opt_in_revision",
        "project_opt_in_verified_at_us",
        "continuation_packet_bytes",
        "continuation_packet_items",
        "continuation_packet_token_budget",
    ] {
        require_matching_integer(
            identity,
            field,
            require_integer(decision, field, "continuation policy decision")?,
            identity_label,
        )?;
    }
    Ok(())
}

fn validate_evaluator_gate_summary(
    gate_summary: &CanonicalValue,
    pressure_signal_count: i64,
) -> Result<()> {
    let gate_label = "continuation evaluator gate summary";
    require_exact_object_fields(
        gate_summary,
        &[
            "continuation_enabled",
            "durable_provider_available",
            "actor_role",
            "source_role",
            "target_role",
            "source_committed",
            "source_is_current_carrier",
            "source_has_exclusive_claim",
            "listener_matches",
            "listener_revision_matches",
            "scope_currentness_matches",
            "project_binding_matches",
            "active_rotation",
            "prior_rotation_terminal",
            "cooldown_satisfied",
            "safe_semantic_point",
            "handoff_fresh",
            "progress_since_last_rotation",
            "expected_benefit",
            "continuation_risk",
            "signals",
        ],
        gate_label,
    )?;
    for field in [
        "continuation_enabled",
        "durable_provider_available",
        "source_committed",
        "source_is_current_carrier",
        "source_has_exclusive_claim",
        "listener_matches",
        "listener_revision_matches",
        "scope_currentness_matches",
        "project_binding_matches",
        "prior_rotation_terminal",
        "cooldown_satisfied",
        "safe_semantic_point",
        "handoff_fresh",
        "progress_since_last_rotation",
    ] {
        if !require_bool(gate_summary, field, gate_label)? {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "{gate_label} requires {field}=true"
            )));
        }
    }
    if require_bool(gate_summary, "active_rotation", gate_label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation evaluator gate summary requires active_rotation=false".to_owned(),
        ));
    }
    require_matching_string(gate_summary, "actor_role", "listener", gate_label)?;
    require_matching_string(gate_summary, "target_role", "work_candidate", gate_label)?;
    let source_role = require_nonempty_string(gate_summary, "source_role", gate_label)?;
    if !matches!(source_role, "work" | "work_candidate") {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation evaluator gate summary source_role must be work or work_candidate"
                .to_owned(),
        ));
    }
    require_matching_string(gate_summary, "expected_benefit", "high", gate_label)?;
    let continuation_risk = require_nonempty_string(gate_summary, "continuation_risk", gate_label)?;
    if !matches!(continuation_risk, "low" | "medium") {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation evaluator gate summary continuation_risk must be low or medium"
                .to_owned(),
        ));
    }
    let signals = object_field(gate_summary, "signals", gate_label)?;
    let signals_label = "continuation evaluator pressure signals";
    require_exact_object_fields(
        signals,
        &[
            "context_pressure",
            "repeated_compaction",
            "unchanged_context_replay",
            "coordination_drag",
        ],
        signals_label,
    )?;
    let observed_count = [
        "context_pressure",
        "repeated_compaction",
        "unchanged_context_replay",
        "coordination_drag",
    ]
    .into_iter()
    .map(|field| require_bool(signals, field, signals_label).map(i64::from))
    .collect::<Result<Vec<_>>>()?
    .into_iter()
    .sum::<i64>();
    if observed_count != pressure_signal_count || observed_count < 2 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation evaluator pressure signal count does not match its gate summary"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_project_policy_reference(
    connection: &Connection,
    basis: &ContinuationBasis,
    decision: &CanonicalValue,
) -> Result<()> {
    let label = "continuation policy decision";
    let policy = continuation_project_policy(connection, &basis.project_id).map_err(|_| {
        WorkVcsError::ContinuationInvalid(format!(
            "continuation Project {:?} has no durable policy receipt",
            basis.project_id
        ))
    })?;
    if !policy.enabled {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation Project {:?} is not opted in",
            basis.project_id
        )));
    }
    require_matching_integer(decision, "project_opt_in_revision", policy.revision, label)?;
    require_matching_integer(
        decision,
        "project_opt_in_verified_at_us",
        policy.updated_at_us,
        label,
    )?;
    require_matching_string(
        decision,
        "project_opt_in_receipt_digest",
        &policy.receipt_digest.to_string(),
        label,
    )?;
    Ok(())
}

fn validate_creation_request(
    connection: &Connection,
    scope: &ContinuationScopeSnapshot,
    options: &ContinuationRotationBeginOptions,
    basis: &ContinuationBasis,
    currentness: Digest,
    request: &CanonicalValue,
    policy_decision: &CanonicalValue,
) -> Result<()> {
    let label = "continuation creation request";
    require_schema_version(request, label)?;
    for (field, expected) in [
        (
            "creation_operation_id",
            options.creation_operation_id.as_str(),
        ),
        ("project_id", basis.project_id.as_str()),
        ("project_binding_kind", "project_id"),
        ("project_binding_id", basis.project_id.as_str()),
        ("source_thread_id", basis.source_thread_id.as_str()),
        ("scope_id", &scope.scope_id.to_string()),
        ("handoff_record_id", &basis.handoff_record_id.to_string()),
        ("handoff_digest", &options.handoff_digest.to_string()),
        ("scope_currentness_digest", &currentness.to_string()),
        ("actor_session_id", &options.listener_session_id.to_string()),
        ("actor_role", LISTENER_SESSION_ROLE),
        ("creation_method", "create_thread"),
        ("target_conversation_kind", "work"),
        ("continuation_packet_kind", "bounded_reference_packet"),
        ("project_opt_in_receipt_kind", "workvcs_project_policy"),
    ] {
        require_matching_string(request, field, expected, label)?;
    }
    require_matching_integer(
        request,
        "listener_revision",
        options.expected_listener_revision,
        label,
    )?;
    for field in ["adapter_kind", "adapter_version"] {
        require_matching_string(
            request,
            field,
            require_nonempty_string(policy_decision, field, "continuation policy decision")?,
            label,
        )?;
    }
    for field in [
        "project_opt_in_receipt_digest",
        "continuation_packet_digest",
        "initial_message_digest",
    ] {
        require_matching_string(
            request,
            field,
            require_nonempty_string(policy_decision, field, "continuation policy decision")?,
            label,
        )?;
    }
    for field in [
        "project_opt_in_revision",
        "continuation_packet_bytes",
        "continuation_packet_items",
        "continuation_packet_token_budget",
    ] {
        require_matching_integer(
            request,
            field,
            require_integer(policy_decision, field, "continuation policy decision")?,
            label,
        )?;
    }
    let actor_thread_id = session_thread_id(connection, options.listener_session_id)?;
    require_matching_string(request, "actor_thread_id", &actor_thread_id, label)?;
    if require_bool(request, "forked", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation creation request must not fork a conversation".to_owned(),
        ));
    }
    if require_bool(request, "source_transcript_included", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation creation request must not inject the source transcript".to_owned(),
        ));
    }
    for field in ["user_visible", "auditable", "interactive"] {
        if !require_bool(request, field, label)? {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "continuation creation request requires {field}=true"
            )));
        }
    }
    require_positive_integer(request, "prepared_at_us", label)?;
    Ok(())
}

fn validate_creation_dispatch(
    connection: &Connection,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    dispatch: &CanonicalValue,
) -> Result<()> {
    let label = "continuation creation dispatch";
    require_schema_version(dispatch, label)?;
    let basis = parse_basis(&occurrence.basis)?;
    validate_project_policy_reference(connection, &basis, &occurrence.policy_decision)?;
    for (field, expected) in [
        (
            "creation_operation_id",
            occurrence.creation_operation_id.as_str(),
        ),
        (
            "creation_request_digest",
            &occurrence.creation_request_digest.to_string(),
        ),
        ("scope_id", &occurrence.scope_id.to_string()),
        ("actor_session_id", &runtime.listener_session_id.to_string()),
        ("actor_role", LISTENER_SESSION_ROLE),
    ] {
        require_matching_string(dispatch, field, expected, label)?;
    }
    require_matching_integer(
        dispatch,
        "listener_revision",
        runtime.listener_revision,
        label,
    )?;
    let actor_thread_id = session_thread_id(connection, runtime.listener_session_id)?;
    require_matching_string(dispatch, "actor_thread_id", &actor_thread_id, label)?;
    for field in ["adapter_kind", "adapter_version"] {
        require_matching_string(
            dispatch,
            field,
            require_nonempty_string(
                &occurrence.creation_request,
                field,
                "continuation creation request",
            )?,
            label,
        )?;
    }
    for field in ["continuation_packet_digest", "initial_message_digest"] {
        require_matching_string(
            dispatch,
            field,
            require_nonempty_string(
                &occurrence.creation_request,
                field,
                "continuation creation request",
            )?,
            label,
        )?;
    }
    if require_bool(dispatch, "source_transcript_included", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation dispatch must not include the source transcript".to_owned(),
        ));
    }
    if !require_bool(dispatch, "request_will_be_sent", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation dispatch must cross the durable request-sent boundary".to_owned(),
        ));
    }
    require_positive_integer(dispatch, "dispatched_at_us", label)?;
    Ok(())
}

fn validate_creation_result_receipt(
    connection: &Connection,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    receipt: &CanonicalValue,
) -> Result<ContinuationRotationState> {
    let label = "continuation creation result receipt";
    require_schema_version(receipt, label)?;
    let dispatch = runtime.dispatch.as_ref().ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation creation result requires a durable dispatch record".to_owned(),
        )
    })?;
    let dispatch_digest = document_digest(DISPATCH_DIGEST_DOMAIN, label, dispatch)?;
    for (field, expected) in [
        (
            "creation_operation_id",
            occurrence.creation_operation_id.as_str(),
        ),
        (
            "creation_request_digest",
            &occurrence.creation_request_digest.to_string(),
        ),
        ("dispatch_digest", &dispatch_digest.to_string()),
        ("scope_id", &occurrence.scope_id.to_string()),
        (
            "actor_session_id",
            require_nonempty_string(
                dispatch,
                "actor_session_id",
                "continuation creation dispatch",
            )?,
        ),
        (
            "actor_thread_id",
            require_nonempty_string(
                dispatch,
                "actor_thread_id",
                "continuation creation dispatch",
            )?,
        ),
        ("actor_role", LISTENER_SESSION_ROLE),
        ("creation_method", "create_thread"),
    ] {
        require_matching_string(receipt, field, expected, label)?;
    }
    require_matching_integer(
        receipt,
        "listener_revision",
        require_integer(
            dispatch,
            "listener_revision",
            "continuation creation dispatch",
        )?,
        label,
    )?;
    for field in ["adapter_kind", "adapter_version"] {
        require_matching_string(
            receipt,
            field,
            require_nonempty_string(dispatch, field, "continuation creation dispatch")?,
            label,
        )?;
    }
    for field in ["continuation_packet_digest", "initial_message_digest"] {
        require_matching_string(
            receipt,
            field,
            require_nonempty_string(
                &occurrence.creation_request,
                field,
                "continuation creation request",
            )?,
            label,
        )?;
    }
    if require_bool(receipt, "source_transcript_included", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation creation receipt must attest that the source transcript was excluded"
                .to_owned(),
        ));
    }
    if !require_bool(receipt, "request_sent", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation creation receipt requires request_sent=true".to_owned(),
        ));
    }
    if require_bool(receipt, "forked", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation creation receipt must not describe a fork".to_owned(),
        ));
    }
    require_positive_integer(receipt, "observed_at_us", label)?;
    require_digest_string(receipt, "product_receipt_digest", label)?;
    match require_nonempty_string(receipt, "outcome", label)? {
        "unknown" => {
            for field in ["client_identity_kind", "client_identity"] {
                require_nonempty_string(receipt, field, label)?;
            }
            Ok(ContinuationRotationState::CreationUnknown)
        }
        "created" => {
            if runtime.state == ContinuationRotationState::CreationUnknown {
                let previous = runtime.initial_unknown_receipt.as_ref().ok_or_else(|| {
                    WorkVcsError::ContinuationInvalid(
                        "unknown creation reconciliation is missing its immutable initial receipt"
                            .to_owned(),
                    )
                })?;
                let previous_digest = document_digest(
                    CREATION_RECEIPT_DIGEST_DOMAIN,
                    "initial unknown creation receipt",
                    previous,
                )?;
                for (field, expected) in [
                    (
                        "previous_unknown_receipt_digest",
                        previous_digest.to_string(),
                    ),
                    (
                        "resolved_from_client_identity_kind",
                        require_nonempty_string(
                            previous,
                            "client_identity_kind",
                            "initial unknown creation receipt",
                        )?
                        .to_owned(),
                    ),
                    (
                        "resolved_from_client_identity",
                        require_nonempty_string(
                            previous,
                            "client_identity",
                            "initial unknown creation receipt",
                        )?
                        .to_owned(),
                    ),
                ] {
                    require_matching_string(receipt, field, &expected, label)?;
                }
            }
            let basis = parse_basis(&occurrence.basis)?;
            for (field, expected) in [
                ("conversation_kind", "work"),
                ("project_id", basis.project_id.as_str()),
                ("project_binding_kind", "project_id"),
                ("project_binding_id", basis.project_id.as_str()),
            ] {
                require_matching_string(receipt, field, expected, label)?;
            }
            for field in ["user_visible", "auditable", "interactive"] {
                if !require_bool(receipt, field, label)? {
                    return Err(WorkVcsError::ContinuationInvalid(format!(
                        "continuation creation receipt requires {field}=true"
                    )));
                }
            }
            let target_thread_id = require_nonempty_string(receipt, "thread_id", label)?;
            ensure_created_thread_identity_eligible(
                connection,
                occurrence,
                runtime,
                target_thread_id,
            )?;
            Ok(ContinuationRotationState::CreationSucceeded)
        }
        outcome => Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation creation result outcome {outcome:?} is unsupported"
        ))),
    }
}

fn validate_candidate_binding(
    transaction: &Transaction<'_>,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    candidate_session_id: SessionId,
    binding: &CanonicalValue,
) -> Result<()> {
    let label = "continuation candidate binding";
    require_schema_version(binding, label)?;
    let receipt = runtime.creation_receipt.as_ref().ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation candidate attachment requires a created product receipt".to_owned(),
        )
    })?;
    require_matching_string(
        receipt,
        "outcome",
        "created",
        "continuation creation receipt",
    )?;
    let dispatch = runtime.dispatch.as_ref().ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation candidate attachment requires a durable dispatch record".to_owned(),
        )
    })?;
    let dispatch_digest = document_digest(DISPATCH_DIGEST_DOMAIN, label, dispatch)?;
    let basis = parse_basis(&occurrence.basis)?;
    for (field, expected) in [
        ("conversation_kind", "work"),
        ("creation_method", "create_thread"),
        ("identity_kind", "thread_id"),
        ("resolution_status", "resolved"),
        (
            "creation_operation_id",
            occurrence.creation_operation_id.as_str(),
        ),
        (
            "creation_request_digest",
            &occurrence.creation_request_digest.to_string(),
        ),
        ("dispatch_digest", &dispatch_digest.to_string()),
        ("scope_id", &occurrence.scope_id.to_string()),
        ("source_thread_id", basis.source_thread_id.as_str()),
        ("project_id", basis.project_id.as_str()),
        ("project_binding_kind", "project_id"),
        ("project_binding_id", basis.project_id.as_str()),
        (
            "thread_id",
            require_nonempty_string(receipt, "thread_id", "continuation creation receipt")?,
        ),
        ("resolved_from_identity_kind", "thread_id"),
        (
            "resolved_from_identity",
            require_nonempty_string(receipt, "thread_id", "continuation creation receipt")?,
        ),
        (
            "product_receipt_digest",
            require_nonempty_string(
                receipt,
                "product_receipt_digest",
                "continuation creation receipt",
            )?,
        ),
        (
            "creation_actor_session_id",
            require_nonempty_string(receipt, "actor_session_id", "continuation creation receipt")?,
        ),
        (
            "creation_actor_thread_id",
            require_nonempty_string(receipt, "actor_thread_id", "continuation creation receipt")?,
        ),
    ] {
        require_matching_string(binding, field, expected, label)?;
    }
    require_matching_integer(
        binding,
        "creation_listener_revision",
        require_integer(
            receipt,
            "listener_revision",
            "continuation creation receipt",
        )?,
        label,
    )?;
    if require_bool(binding, "forked", label)? {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate must not be created by forking".to_owned(),
        ));
    }
    let thread_id = require_nonempty_string(binding, "thread_id", label)?;
    if thread_id == basis.source_thread_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate target thread must differ from the source thread".to_owned(),
        ));
    }
    require_session_thread_id(transaction, candidate_session_id, thread_id, "candidate")?;
    ensure_candidate_thread_not_reused(transaction, occurrence.rotation_id, thread_id)?;
    for field in ["user_visible", "auditable", "interactive"] {
        if !require_bool(binding, field, label)?
            || !require_bool(receipt, field, "continuation creation receipt")?
        {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "continuation candidate binding requires {field}=true"
            )));
        }
    }
    require_digest_string(binding, "product_receipt_digest", label)?;
    Ok(())
}

fn validate_candidate_acceptance(
    connection: &Connection,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    candidate_session_id: SessionId,
    acceptance: &CanonicalValue,
) -> Result<()> {
    let label = "continuation candidate acceptance";
    require_schema_version(acceptance, label)?;
    let binding = runtime.candidate_binding.as_ref().ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation candidate acceptance requires an attached candidate".to_owned(),
        )
    })?;
    let basis = parse_basis(&occurrence.basis)?;
    let rotation_id = occurrence.rotation_id.to_string();
    let scope_id = occurrence.scope_id.to_string();
    let candidate_session = candidate_session_id.to_string();
    let handoff_record_id = basis.handoff_record_id.to_string();
    let handoff_digest = occurrence.handoff_digest.to_string();
    let currentness_digest = runtime.scope_currentness_digest.to_string();
    let creation_request_digest = occurrence.creation_request_digest.to_string();
    for (field, expected) in [
        ("rotation_id", rotation_id.as_str()),
        ("scope_id", scope_id.as_str()),
        ("candidate_session_id", candidate_session.as_str()),
        (
            "candidate_thread_id",
            require_nonempty_string(binding, "thread_id", "continuation candidate binding")?,
        ),
        ("project_id", basis.project_id.as_str()),
        ("handoff_record_id", handoff_record_id.as_str()),
        ("handoff_digest", handoff_digest.as_str()),
        ("scope_currentness_digest", currentness_digest.as_str()),
        (
            "creation_operation_id",
            occurrence.creation_operation_id.as_str(),
        ),
        ("creation_request_digest", creation_request_digest.as_str()),
        (
            "continuation_packet_digest",
            require_nonempty_string(
                &occurrence.creation_request,
                "continuation_packet_digest",
                "continuation creation request",
            )?,
        ),
        (
            "initial_message_digest",
            require_nonempty_string(
                &occurrence.creation_request,
                "initial_message_digest",
                "continuation creation request",
            )?,
        ),
    ] {
        require_matching_string(acceptance, field, expected, label)?;
    }
    require_session_thread_id(
        connection,
        candidate_session_id,
        require_nonempty_string(binding, "thread_id", "continuation candidate binding")?,
        "candidate acceptance",
    )?;
    for field in ["handoff_consumed", "ready_to_continue"] {
        if !require_bool(acceptance, field, label)? {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "continuation candidate acceptance requires {field}=true"
            )));
        }
    }
    require_digest_string(acceptance, "next_action_digest", label)?;
    require_positive_integer(acceptance, "accepted_at_us", label)?;
    Ok(())
}

fn validate_abort_rationale(
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    rationale: &CanonicalValue,
) -> Result<()> {
    require_schema_version(rationale, "continuation abort rationale")?;
    require_matching_string(
        rationale,
        "creation_operation_id",
        &occurrence.creation_operation_id,
        "continuation abort rationale",
    )?;
    require_matching_string(
        rationale,
        "creation_request_digest",
        &occurrence.creation_request_digest.to_string(),
        "continuation abort rationale",
    )?;
    if runtime.abort_only {
        let recovery = runtime.recovery.as_ref().ok_or_else(|| {
            WorkVcsError::ContinuationInvalid(
                "abort-only continuation rotation is missing recovery provenance".to_owned(),
            )
        })?;
        require_matching_string(
            rationale,
            "recovery_mode",
            "abort_only",
            "continuation abort rationale",
        )?;
        require_matching_integer(
            rationale,
            "recovery_revision",
            runtime.recovery_revision,
            "continuation abort rationale",
        )?;
        require_matching_string(
            rationale,
            "recovery_receipt_digest",
            &document_digest(
                RECOVERY_RECEIPT_DIGEST_DOMAIN,
                "continuation abort-only recovery",
                recovery,
            )?
            .to_string(),
            "continuation abort rationale",
        )?;
    }
    match runtime.state {
        ContinuationRotationState::PendingCreation => require_matching_string(
            rationale,
            "resolution",
            "cancelled_before_dispatch",
            "continuation abort rationale",
        ),
        ContinuationRotationState::CreationDispatched
        | ContinuationRotationState::CreationUnknown => {
            require_matching_string(
                rationale,
                "resolution",
                "confirmed_not_created",
                "continuation abort rationale",
            )?;
            require_digest_string(rationale, "evidence_digest", "continuation abort rationale")?;
            Ok(())
        }
        ContinuationRotationState::CreationSucceeded
        | ContinuationRotationState::CandidateAttached
        | ContinuationRotationState::CandidateAccepted => {
            require_matching_string(
                rationale,
                "resolution",
                "candidate_disposition_confirmed",
                "continuation abort rationale",
            )?;
            let disposition = require_nonempty_string(
                rationale,
                "candidate_disposition",
                "continuation abort rationale",
            )?;
            if !matches!(disposition, "archived" | "retained_non_authoritative") {
                return Err(WorkVcsError::ContinuationInvalid(
                    "candidate-attached abort requires candidate_disposition=archived or retained_non_authoritative"
                        .to_owned(),
                ));
            }
            require_digest_string(rationale, "evidence_digest", "continuation abort rationale")?;
            Ok(())
        }
        _ => Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation rotation cannot be aborted from state {}",
            runtime.state.as_str()
        ))),
    }
}

fn parse_basis(value: &CanonicalValue) -> Result<ContinuationBasis> {
    require_schema_version(value, "continuation basis")?;
    let project_id = validate_embedded_identifier(
        "continuation project_id",
        require_nonempty_string(value, "project_id", "continuation basis")?,
    )?;
    let source_thread_id = validate_embedded_identifier(
        "continuation source_thread_id",
        require_nonempty_string(value, "source_thread_id", "continuation basis")?,
    )?;
    let handoff_record_id =
        parse_entity_id_field(value, "handoff_record_id", "continuation basis")?;
    let mut ids = Vec::new();
    if let Some(raw) = optional_object_field(value, "currentness_entity_ids") {
        let CanonicalValue::Array(values) = raw else {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation basis field \"currentness_entity_ids\" must be an array".to_owned(),
            ));
        };
        if values.len() > MAX_CURRENTNESS_ENTITY_IDS {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "continuation currentness entity count {} exceeds maximum {}",
                values.len(),
                MAX_CURRENTNESS_ENTITY_IDS
            )));
        }
        let mut seen = BTreeSet::new();
        for entry in values {
            let CanonicalValue::String(raw_id) = entry else {
                return Err(WorkVcsError::ContinuationInvalid(
                    "continuation currentness entity ids must be strings".to_owned(),
                ));
            };
            let entity_id = EntityId::parse_canonical(raw_id).map_err(|error| {
                WorkVcsError::ContinuationInvalid(format!(
                    "continuation currentness entity id is invalid: {error}"
                ))
            })?;
            if entity_id == handoff_record_id || !seen.insert(entity_id) {
                return Err(WorkVcsError::ContinuationInvalid(
                    "continuation currentness entity ids must be unique and must not repeat the Handoff Record"
                        .to_owned(),
                ));
            }
            ids.push(entity_id);
        }
        ids.sort();
    }
    Ok(ContinuationBasis {
        project_id,
        source_thread_id,
        handoff_record_id,
        currentness_entity_ids: ids,
    })
}

fn require_session_thread_id(
    transaction: &Connection,
    session_id: SessionId,
    expected_thread_id: &str,
    label: &str,
) -> Result<()> {
    let metadata = session::session_metadata_for_update(transaction, session_id)?;
    let actual = require_nonempty_string(&metadata, THREAD_ID_METADATA_KEY, "session metadata")?;
    if actual == expected_thread_id {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation {label} session thread identity {actual:?} does not match {expected_thread_id:?}"
        )))
    }
}

fn session_thread_id(connection: &Connection, session_id: SessionId) -> Result<String> {
    let metadata = session::session_metadata_for_update(connection, session_id)?;
    validate_embedded_identifier(
        "session thread_id",
        require_nonempty_string(&metadata, THREAD_ID_METADATA_KEY, "session metadata")?,
    )
}

fn require_unique_session_thread(
    connection: &Connection,
    session_id: SessionId,
    label: &str,
) -> Result<String> {
    let thread_id = session_thread_id(connection, session_id)?;
    let count = connection
        .query_row(
            "SELECT count(*) FROM session
             WHERE json_extract(metadata_json, '$.thread_id') = ?1",
            params![thread_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if count != 1 {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation {label} thread identity {thread_id:?} is shared by {count} Sessions"
        )));
    }
    Ok(thread_id)
}

fn require_rotation_participant_threads_unique(
    connection: &Connection,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    candidate_session_id: Option<SessionId>,
) -> Result<()> {
    let listener_thread_id =
        require_unique_session_thread(connection, runtime.listener_session_id, "listener")?;
    let source_thread_id =
        require_unique_session_thread(connection, occurrence.source_session_id, "source")?;
    if listener_thread_id == source_thread_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation listener and source must not alias the same product thread".to_owned(),
        ));
    }
    if let Some(candidate_session_id) = candidate_session_id {
        let candidate_thread_id =
            require_unique_session_thread(connection, candidate_session_id, "candidate")?;
        if candidate_thread_id == listener_thread_id || candidate_thread_id == source_thread_id {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation candidate must not alias the listener or source product thread"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn ensure_created_thread_identity_eligible(
    connection: &Connection,
    occurrence: &RotationOccurrence,
    runtime: &ActiveRotationProjection,
    target_thread_id: &str,
) -> Result<()> {
    let basis = parse_basis(&occurrence.basis)?;
    let listener_thread_id = session_thread_id(connection, runtime.listener_session_id)?;
    if target_thread_id == basis.source_thread_id || target_thread_id == listener_thread_id {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation target thread must differ from source and listener threads".to_owned(),
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT session_id, json_extract(metadata_json, '$.workvcs_session_role')
             FROM session WHERE json_extract(metadata_json, '$.thread_id') = ?1",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(params![target_thread_id], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(storage_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    if rows.len() > 1
        || rows
            .iter()
            .any(|(_, role)| role.as_deref() != Some(WORK_CANDIDATE_SESSION_ROLE))
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation target thread aliases another role or more than one Session".to_owned(),
        ));
    }
    ensure_candidate_thread_not_reused(connection, occurrence.rotation_id, target_thread_id)
}

fn historical_created_thread_count(connection: &Connection, thread_id: &str) -> Result<i64> {
    connection
        .query_row(
            "SELECT
                (SELECT count(*) FROM continuation_rotation_runtime AS runtime
                 WHERE json_extract(runtime.creation_receipt_json, '$.outcome') = 'created'
                   AND json_extract(runtime.creation_receipt_json, '$.thread_id') = ?1)
              + (SELECT count(*) FROM continuation_rotation_result AS result
                 WHERE json_extract(result.creation_receipt_json, '$.outcome') = 'created'
                   AND json_extract(result.creation_receipt_json, '$.thread_id') = ?1)",
            params![thread_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)
}

fn ensure_thread_not_historical_created_target(
    connection: &Connection,
    thread_id: &str,
    label: &str,
) -> Result<()> {
    let count = historical_created_thread_count(connection, thread_id)?;
    if count == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} thread identity {thread_id:?} is reserved by {count} historical created continuation target(s)"
        )))
    }
}

fn ensure_listener_thread_not_created_target(
    connection: &Connection,
    listener_thread_id: &str,
    listener_label: &str,
) -> Result<()> {
    ensure_thread_not_historical_created_target(connection, listener_thread_id, listener_label)
}

fn ensure_candidate_thread_not_reused(
    connection: &Connection,
    rotation_id: ContinuationRotationId,
    thread_id: &str,
) -> Result<()> {
    let count = connection
        .query_row(
            "SELECT
                (SELECT count(*) FROM continuation_rotation_runtime AS runtime
                 WHERE runtime.rotation_id <> ?1
                   AND json_extract(runtime.candidate_binding_json, '$.thread_id') = ?2)
              + (SELECT count(*) FROM continuation_rotation_result AS result
                 WHERE result.rotation_id <> ?1
                   AND json_extract(result.candidate_binding_json, '$.thread_id') = ?2)
              + (SELECT count(*) FROM continuation_rotation_runtime AS runtime
                 WHERE runtime.rotation_id <> ?1
                   AND json_extract(runtime.creation_receipt_json, '$.outcome') = 'created'
                   AND json_extract(runtime.creation_receipt_json, '$.thread_id') = ?2)
              + (SELECT count(*) FROM continuation_rotation_result AS result
                 WHERE result.rotation_id <> ?1
                   AND json_extract(result.creation_receipt_json, '$.outcome') = 'created'
                   AND json_extract(result.creation_receipt_json, '$.thread_id') = ?2)",
            params![&rotation_id.raw_bytes()[..], thread_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if count == 0 {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation target thread {thread_id:?} was already used by another rotation"
        )))
    }
}

fn load_active_rotation(
    connection: &Connection,
    rotation_id: ContinuationRotationId,
) -> Result<ActiveRotationProjection> {
    let row = connection
        .query_row(
            "SELECT status, listener_session_id, listener_revision,
                    scope_currentness_digest, abort_only, recovery_revision,
                    recovery_json, dispatch_json, creation_receipt_json,
                    initial_unknown_receipt_json, candidate_session_id,
                    candidate_binding_json, candidate_acceptance_json,
                    candidate_acceptance_history_json
             FROM continuation_rotation_runtime WHERE rotation_id = ?1",
            params![&rotation_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<Vec<u8>>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, String>(13)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        status,
        listener,
        revision,
        currentness,
        abort_only,
        recovery_revision,
        recovery,
        dispatch,
        receipt,
        initial_unknown_receipt,
        candidate,
        binding,
        acceptance,
        acceptance_history,
    )) = row
    else {
        return Err(WorkVcsError::ContinuationNotFound(format!(
            "active continuation rotation {rotation_id} does not exist"
        )));
    };
    Ok(ActiveRotationProjection {
        state: decode_rotation_state(&status)?,
        listener_session_id: decode_session_id("listener_session_id", listener)?,
        listener_revision: revision,
        scope_currentness_digest: decode_digest("scope_currentness_digest", currentness)?,
        abort_only: decode_sqlite_bool("abort_only", abort_only)?,
        recovery_revision,
        recovery: recovery
            .map(|json| parse_continuation_json("recovery_json", &json))
            .transpose()?,
        dispatch: dispatch
            .map(|json| parse_continuation_json("dispatch_json", &json))
            .transpose()?,
        creation_receipt: receipt
            .map(|json| parse_continuation_json("creation_receipt_json", &json))
            .transpose()?,
        initial_unknown_receipt: initial_unknown_receipt
            .map(|json| parse_continuation_json("initial_unknown_receipt_json", &json))
            .transpose()?,
        candidate_session_id: candidate
            .map(|bytes| decode_session_id("candidate_session_id", bytes))
            .transpose()?,
        candidate_binding: binding
            .map(|json| parse_continuation_json("candidate_binding_json", &json))
            .transpose()?,
        candidate_acceptance: acceptance
            .map(|json| parse_continuation_json("candidate_acceptance_json", &json))
            .transpose()?,
        candidate_acceptance_history: parse_acceptance_history(&acceptance_history)?,
    })
}

fn load_scope(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
) -> Result<ContinuationScopeSnapshot> {
    load_scope_optional(connection, workspace_id, branch_id, task_entity_id)?.ok_or_else(|| {
        WorkVcsError::ContinuationNotFound(format!(
            "continuation scope ({workspace_id}, {branch_id}, {task_entity_id}) does not exist"
        ))
    })
}

fn load_scope_optional(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
) -> Result<Option<ContinuationScopeSnapshot>> {
    let row = connection
        .query_row(
            "SELECT scope_id, listener_session_id, listener_revision, carrier_session_id,
                    carrier_claim_id, owner_epoch, updated_at_us
             FROM continuation_scope_runtime
             WHERE workspace_id = ?1 AND branch_id = ?2 AND task_entity_id = ?3",
            params![
                &workspace_id.raw_bytes()[..],
                &branch_id.raw_bytes()[..],
                &task_entity_id.raw_bytes()[..],
            ],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    row.map(
        |(scope_id, listener, revision, carrier, claim, epoch, updated)| {
            Ok(ContinuationScopeSnapshot {
                scope_id: decode_scope_id("scope_id", scope_id)?,
                workspace_id,
                branch_id,
                task_entity_id,
                listener_session_id: decode_session_id("listener_session_id", listener)?,
                listener_revision: revision,
                carrier_session_id: decode_session_id("carrier_session_id", carrier)?,
                carrier_claim_id: decode_claim_id("carrier_claim_id", claim)?,
                owner_epoch: epoch,
                updated_at_us: updated,
            })
        },
    )
    .transpose()
}

fn load_scope_by_carrier(
    connection: &Connection,
    source_session_id: SessionId,
    source_claim_id: ClaimId,
) -> Result<ContinuationScopeSnapshot> {
    let row = connection
        .query_row(
            "SELECT workspace_id, branch_id, task_entity_id
             FROM continuation_scope_runtime
             WHERE carrier_session_id = ?1 AND carrier_claim_id = ?2",
            params![
                &source_session_id.raw_bytes()[..],
                &source_claim_id.raw_bytes()[..]
            ],
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
    let Some((workspace, branch, task)) = row else {
        return Err(WorkVcsError::ContinuationNotFound(format!(
            "no continuation scope is carried by session {source_session_id} and claim {source_claim_id}"
        )));
    };
    load_scope(
        connection,
        decode_workspace_id("workspace_id", workspace)?,
        decode_branch_id("branch_id", branch)?,
        decode_entity_id("task_entity_id", task)?,
    )
}

fn load_scope_by_claim_optional(
    connection: &Connection,
    claim_id: ClaimId,
) -> Result<Option<ContinuationScopeSnapshot>> {
    let row = connection
        .query_row(
            "SELECT workspace_id, branch_id, task_entity_id
             FROM continuation_scope_runtime WHERE carrier_claim_id = ?1",
            params![&claim_id.raw_bytes()[..]],
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
    row.map(|(workspace, branch, task)| {
        load_scope(
            connection,
            decode_workspace_id("workspace_id", workspace)?,
            decode_branch_id("branch_id", branch)?,
            decode_entity_id("task_entity_id", task)?,
        )
    })
    .transpose()
}

fn active_rotation_for_scope(
    connection: &Connection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
) -> Result<Option<ContinuationRotationId>> {
    connection
        .query_row(
            "SELECT rotation_id FROM continuation_rotation_runtime
             WHERE workspace_id = ?1 AND branch_id = ?2 AND task_entity_id = ?3",
            params![
                &workspace_id.raw_bytes()[..],
                &branch_id.raw_bytes()[..],
                &task_entity_id.raw_bytes()[..]
            ],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?
        .map(|bytes| decode_rotation_id("rotation_id", bytes))
        .transpose()
}

fn load_rotation_by_idempotency(
    connection: &Connection,
    source_claim_id: ClaimId,
    idempotency_key: &str,
) -> Result<Option<RotationOccurrence>> {
    let rotation_id = connection
        .query_row(
            "SELECT rotation_id FROM continuation_rotation
             WHERE source_claim_id = ?1 AND idempotency_key = ?2",
            params![&source_claim_id.raw_bytes()[..], idempotency_key],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(storage_error)?;
    rotation_id
        .map(|bytes| {
            load_rotation_occurrence(connection, decode_rotation_id("rotation_id", bytes)?)
        })
        .transpose()
}

fn load_rotation_occurrence(
    connection: &Connection,
    rotation_id: ContinuationRotationId,
) -> Result<RotationOccurrence> {
    let row = connection
        .query_row(
            "SELECT scope_id, workspace_id, branch_id, task_entity_id,
                    initial_listener_session_id, initial_listener_revision,
                    source_session_id, source_claim_id, source_owner_epoch,
                    idempotency_key, creation_operation_id,
                    policy_decision_digest, policy_decision_json,
                    creation_request_digest, creation_request_json,
                    handoff_digest, basis_json, created_at_us
             FROM continuation_rotation WHERE rotation_id = ?1",
            params![&rotation_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Vec<u8>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, Vec<u8>>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, Vec<u8>>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, i64>(17)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((
        scope_id,
        workspace,
        branch,
        task,
        listener,
        listener_revision,
        source,
        claim,
        owner_epoch,
        key,
        creation_operation_id,
        policy_decision_digest,
        policy_decision_json,
        creation_request_digest,
        creation_request_json,
        handoff_digest,
        basis_json,
        created_at_us,
    )) = row
    else {
        return Err(WorkVcsError::ContinuationNotFound(format!(
            "continuation rotation {rotation_id} does not exist"
        )));
    };
    Ok(RotationOccurrence {
        rotation_id,
        scope_id: decode_scope_id("scope_id", scope_id)?,
        workspace_id: decode_workspace_id("workspace_id", workspace)?,
        branch_id: decode_branch_id("branch_id", branch)?,
        task_entity_id: decode_entity_id("task_entity_id", task)?,
        initial_listener_session_id: decode_session_id("initial_listener_session_id", listener)?,
        initial_listener_revision: listener_revision,
        source_session_id: decode_session_id("source_session_id", source)?,
        source_claim_id: decode_claim_id("source_claim_id", claim)?,
        source_owner_epoch: owner_epoch,
        idempotency_key: key,
        creation_operation_id,
        policy_decision_digest: decode_digest("policy_decision_digest", policy_decision_digest)?,
        policy_decision: parse_continuation_json("policy_decision_json", &policy_decision_json)?,
        policy_decision_json,
        creation_request_digest: decode_digest("creation_request_digest", creation_request_digest)?,
        creation_request: parse_continuation_json("creation_request_json", &creation_request_json)?,
        creation_request_json,
        handoff_digest: decode_digest("handoff_digest", handoff_digest)?,
        basis: parse_continuation_json("basis_json", &basis_json)?,
        basis_json,
        created_at_us,
    })
}

fn load_rotation_snapshot(
    connection: &Connection,
    rotation_id: ContinuationRotationId,
) -> Result<ContinuationRotationSnapshot> {
    let occurrence = load_rotation_occurrence(connection, rotation_id)?;
    let runtime = connection
        .query_row(
            "SELECT status, listener_session_id, listener_revision,
                    scope_currentness_digest, abort_only, recovery_revision,
                    recovery_json, dispatch_json, creation_receipt_json,
                    initial_unknown_receipt_json, candidate_session_id,
                    candidate_binding_json, candidate_acceptance_json,
                    candidate_acceptance_history_json, last_activity_at_us
             FROM continuation_rotation_runtime WHERE rotation_id = ?1",
            params![&rotation_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<Vec<u8>>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, i64>(14)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let terminal = connection
        .query_row(
            "SELECT status, listener_session_id, listener_revision,
                    scope_currentness_digest, abort_only, recovery_revision,
                    recovery_json, dispatch_json, creation_receipt_json,
                    initial_unknown_receipt_json, candidate_session_id,
                    resulting_claim_id, candidate_binding_json,
                    candidate_acceptance_json, candidate_acceptance_history_json,
                    ended_at_us, result_json
             FROM continuation_rotation_result WHERE rotation_id = ?1",
            params![&rotation_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<Vec<u8>>>(10)?,
                    row.get::<_, Option<Vec<u8>>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, String>(16)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let (
        state,
        listener,
        listener_revision,
        currentness,
        abort_only,
        recovery_revision,
        recovery,
        dispatch,
        receipt,
        initial_unknown_receipt,
        candidate,
        binding,
        acceptance,
        acceptance_history,
        resulting_claim,
        result,
        activity,
        ended_at,
    ) = match (runtime, terminal) {
        (
            Some((
                status,
                listener,
                revision,
                currentness,
                abort_only,
                recovery_revision,
                recovery,
                dispatch,
                receipt,
                initial_unknown_receipt,
                candidate,
                binding,
                acceptance,
                acceptance_history,
                activity,
            )),
            None,
        ) => (
            decode_rotation_state(&status)?,
            decode_session_id("listener_session_id", listener)?,
            revision,
            decode_digest("scope_currentness_digest", currentness)?,
            decode_sqlite_bool("abort_only", abort_only)?,
            recovery_revision,
            recovery
                .map(|json| parse_continuation_json("recovery_json", &json))
                .transpose()?,
            dispatch
                .map(|json| parse_continuation_json("dispatch_json", &json))
                .transpose()?,
            receipt
                .map(|json| parse_continuation_json("creation_receipt_json", &json))
                .transpose()?,
            initial_unknown_receipt
                .map(|json| parse_continuation_json("initial_unknown_receipt_json", &json))
                .transpose()?,
            candidate
                .map(|bytes| decode_session_id("candidate_session_id", bytes))
                .transpose()?,
            binding
                .map(|json| parse_continuation_json("candidate_binding_json", &json))
                .transpose()?,
            acceptance
                .map(|json| parse_continuation_json("candidate_acceptance_json", &json))
                .transpose()?,
            parse_acceptance_history(&acceptance_history)?,
            None,
            None,
            activity,
            None,
        ),
        (
            None,
            Some((
                status,
                listener,
                revision,
                currentness,
                abort_only,
                recovery_revision,
                recovery,
                dispatch,
                receipt,
                initial_unknown_receipt,
                candidate,
                claim,
                binding,
                acceptance,
                acceptance_history,
                ended,
                result_json,
            )),
        ) => {
            let result = parse_continuation_json("result_json", &result_json)?;
            (
                decode_rotation_state(&status)?,
                decode_session_id("listener_session_id", listener)?,
                revision,
                decode_digest("scope_currentness_digest", currentness)?,
                decode_sqlite_bool("abort_only", abort_only)?,
                recovery_revision,
                recovery
                    .map(|json| parse_continuation_json("recovery_json", &json))
                    .transpose()?,
                dispatch
                    .map(|json| parse_continuation_json("dispatch_json", &json))
                    .transpose()?,
                receipt
                    .map(|json| parse_continuation_json("creation_receipt_json", &json))
                    .transpose()?,
                initial_unknown_receipt
                    .map(|json| parse_continuation_json("initial_unknown_receipt_json", &json))
                    .transpose()?,
                candidate
                    .map(|bytes| decode_session_id("candidate_session_id", bytes))
                    .transpose()?,
                binding
                    .map(|json| parse_continuation_json("candidate_binding_json", &json))
                    .transpose()?,
                acceptance
                    .map(|json| parse_continuation_json("candidate_acceptance_json", &json))
                    .transpose()?,
                parse_acceptance_history(&acceptance_history)?,
                claim
                    .map(|bytes| decode_claim_id("resulting_claim_id", bytes))
                    .transpose()?,
                Some(result),
                ended,
                Some(ended),
            )
        }
        _ => {
            return Err(WorkVcsError::ContinuationInvalid(format!(
                "continuation rotation {rotation_id} must have exactly one runtime or terminal projection"
            )));
        }
    };
    let recovery_receipt_digest = recovery
        .as_ref()
        .map(|value| {
            document_digest(
                RECOVERY_RECEIPT_DIGEST_DOMAIN,
                "continuation abort-only recovery",
                value,
            )
        })
        .transpose()?;
    let recovery_history = load_abort_only_recovery_summary(
        connection,
        occurrence.workspace_id,
        rotation_id,
        recovery_revision,
        recovery.as_ref(),
        recovery_receipt_digest,
    )?;
    Ok(ContinuationRotationSnapshot {
        rotation_id,
        scope_id: occurrence.scope_id,
        workspace_id: occurrence.workspace_id,
        branch_id: occurrence.branch_id,
        task_entity_id: occurrence.task_entity_id,
        initiating_listener_session_id: occurrence.initial_listener_session_id,
        initial_listener_revision: occurrence.initial_listener_revision,
        listener_session_id: listener,
        listener_revision,
        source_session_id: occurrence.source_session_id,
        source_claim_id: occurrence.source_claim_id,
        source_owner_epoch: occurrence.source_owner_epoch,
        idempotency_key: occurrence.idempotency_key,
        creation_operation_id: occurrence.creation_operation_id,
        policy_decision_digest: occurrence.policy_decision_digest,
        policy_decision: occurrence.policy_decision,
        creation_request_digest: occurrence.creation_request_digest,
        creation_request: occurrence.creation_request,
        handoff_digest: occurrence.handoff_digest,
        scope_currentness_digest: currentness,
        basis: occurrence.basis,
        state,
        abort_only,
        recovery_revision,
        recovery,
        recovery_receipt_digest,
        recovery_history_count: recovery_history.count,
        recovery_history_chain_digest: recovery_history.chain_digest,
        dispatch,
        creation_receipt: receipt,
        initial_unknown_receipt,
        candidate_session_id: candidate,
        candidate_binding: binding,
        candidate_acceptance: acceptance,
        candidate_acceptance_history: acceptance_history,
        resulting_claim_id: resulting_claim,
        result,
        created_at_us: occurrence.created_at_us,
        last_activity_at_us: activity,
        ended_at_us: ended_at,
    })
}

fn ensure_idempotent_begin_matches(
    occurrence: &RotationOccurrence,
    options: &ContinuationRotationBeginOptions,
    basis_json: &str,
    policy_decision_json: &str,
    creation_request_json: &str,
) -> Result<()> {
    if occurrence.initial_listener_session_id == options.listener_session_id
        && occurrence.initial_listener_revision == options.expected_listener_revision
        && occurrence.source_session_id == options.source_session_id
        && occurrence.source_claim_id == options.source_claim_id
        && occurrence.scope_id == options.expected_scope_id
        && occurrence.source_owner_epoch == options.expected_owner_epoch
        && occurrence.creation_operation_id == options.creation_operation_id
        && occurrence.policy_decision_json == policy_decision_json
        && occurrence.creation_request_json == creation_request_json
        && occurrence.handoff_digest == options.handoff_digest
        && occurrence.basis_json == basis_json
    {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation idempotency key {:?} was reused with conflicting inputs",
            options.idempotency_key
        )))
    }
}

fn creation_operation_exists(connection: &Connection, creation_operation_id: &str) -> Result<bool> {
    let count = connection
        .query_row(
            "SELECT count(*) FROM continuation_rotation WHERE creation_operation_id = ?1",
            params![creation_operation_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    Ok(count != 0)
}

fn require_active_exclusive_claim(
    connection: &Connection,
    claim_id: ClaimId,
    session_id: SessionId,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    task_entity_id: EntityId,
) -> Result<()> {
    let row = connection
        .query_row(
            "SELECT claim.session_id, claim.workspace_id, claim.branch_id,
                    claim.task_entity_id, claim.mode
             FROM claim_runtime
             INNER JOIN claim ON claim.claim_id = claim_runtime.claim_id
             WHERE claim.claim_id = ?1",
            params![&claim_id.raw_bytes()[..]],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error)?;
    let Some((actual_session, actual_workspace, actual_branch, actual_task, mode)) = row else {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation claim {claim_id} is not active"
        )));
    };
    if decode_session_id("claim.session_id", actual_session)? != session_id
        || decode_workspace_id("claim.workspace_id", actual_workspace)? != workspace_id
        || decode_branch_id("claim.branch_id", actual_branch)? != branch_id
        || decode_entity_id("claim.task_entity_id", actual_task)? != task_entity_id
        || mode != "exclusive"
    {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "claim {claim_id} is not the exact active exclusive claim for the continuation scope"
        )));
    }
    Ok(())
}

fn require_active_session_target(
    label: &str,
    session: &session::ActiveSessionProjection,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
) -> Result<()> {
    if session.active_workspace_id != workspace_id || session.active_branch_id != branch_id {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation {label} session is not active on workspace {workspace_id} branch {branch_id}"
        )));
    }
    Ok(())
}

fn require_session_role(
    connection: &Connection,
    session_id: SessionId,
    allowed: &[ContinuationSessionRole],
) -> Result<ContinuationSessionRole> {
    let metadata = session::session_metadata_for_update(connection, session_id)?;
    let role = require_nonempty_string(&metadata, SESSION_ROLE_METADATA_KEY, "session metadata")?;
    let role = ContinuationSessionRole::from_str(role).ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(format!(
            "session {session_id} has unsupported continuation role {role:?}"
        ))
    })?;
    if !allowed.contains(&role) {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "session {session_id} continuation role {} is not allowed at this boundary",
            role.as_str()
        )));
    }
    Ok(role)
}

fn delete_scope(
    transaction: &Transaction<'_>,
    scope: &ContinuationScopeSnapshot,
    actor_session_id: SessionId,
    now_us: i64,
) -> Result<()> {
    let changed = transaction
        .execute(
            "DELETE FROM continuation_scope_runtime
             WHERE workspace_id = ?1 AND branch_id = ?2 AND task_entity_id = ?3
               AND scope_id = ?4 AND carrier_claim_id = ?5 AND owner_epoch = ?6",
            params![
                &scope.workspace_id.raw_bytes()[..],
                &scope.branch_id.raw_bytes()[..],
                &scope.task_entity_id.raw_bytes()[..],
                &scope.scope_id.raw_bytes()[..],
                &scope.carrier_claim_id.raw_bytes()[..],
                scope.owner_epoch,
            ],
        )
        .map_err(storage_error)?;
    if changed != 1 {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation scope retirement compare-and-swap failed".to_owned(),
        ));
    }
    insert_event(
        transaction,
        scope.workspace_id,
        actor_session_id,
        "continuation.scope_retired",
        now_us,
        CanonicalValue::object(vec![
            (
                "branch_id".to_owned(),
                CanonicalValue::String(scope.branch_id.to_string()),
            ),
            (
                "carrier_claim_id".to_owned(),
                CanonicalValue::String(scope.carrier_claim_id.to_string()),
            ),
            (
                "owner_epoch".to_owned(),
                CanonicalValue::safe_integer(scope.owner_epoch)?,
            ),
            (
                "scope_id".to_owned(),
                CanonicalValue::String(scope.scope_id.to_string()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(scope.task_entity_id.to_string()),
            ),
        ])?,
    )
}

fn parse_entity_id_field(value: &CanonicalValue, field: &str, label: &str) -> Result<EntityId> {
    EntityId::parse_canonical(require_nonempty_string(value, field, label)?).map_err(|error| {
        WorkVcsError::ContinuationInvalid(format!("{label} field {field:?} is invalid: {error}"))
    })
}

fn require_schema_version(value: &CanonicalValue, label: &str) -> Result<()> {
    if require_integer(value, "schema_version", label)? == 1 {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} schema_version must be 1"
        )))
    }
}

fn require_matching_string(
    value: &CanonicalValue,
    field: &str,
    expected: &str,
    label: &str,
) -> Result<()> {
    let actual = require_nonempty_string(value, field, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} is {actual:?}, expected {expected:?}"
        )))
    }
}

fn require_matching_integer(
    value: &CanonicalValue,
    field: &str,
    expected: i64,
    label: &str,
) -> Result<()> {
    let actual = require_integer(value, field, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} is {actual}, expected {expected}"
        )))
    }
}

fn require_positive_integer(value: &CanonicalValue, field: &str, label: &str) -> Result<i64> {
    let actual = require_integer(value, field, label)?;
    if actual > 0 {
        Ok(actual)
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} must be positive"
        )))
    }
}

fn require_bounded_positive_integer(
    value: &CanonicalValue,
    field: &str,
    maximum: i64,
    label: &str,
) -> Result<i64> {
    let actual = require_positive_integer(value, field, label)?;
    if actual <= maximum {
        Ok(actual)
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} exceeds maximum {maximum}"
        )))
    }
}

fn require_positive_revision_epoch(listener_revision: i64, owner_epoch: i64) -> Result<()> {
    if listener_revision < 1 || owner_epoch < 1 {
        Err(WorkVcsError::ContinuationInvalid(
            "continuation listener revision and owner epoch must be at least 1".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn validate_bounded_text(label: &str, value: String, max_len: usize) -> Result<String> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} must contain 1 to {max_len} bytes and no control characters"
        )))
    } else {
        Ok(value)
    }
}

fn validate_embedded_identifier(label: &str, value: &str) -> Result<String> {
    validate_bounded_text(label, value.to_owned(), 2_048)
}

fn validate_document(label: &str, value: &CanonicalValue) -> Result<()> {
    require_object(label, value)?;
    let bytes = canonical_bytes(value)?;
    if bytes.len() > MAX_CONTINUATION_DOCUMENT_BYTES {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} is {} bytes and exceeds the {}-byte limit",
            bytes.len(),
            MAX_CONTINUATION_DOCUMENT_BYTES
        )))
    } else {
        Ok(())
    }
}

fn document_digest(domain: &str, label: &str, value: &CanonicalValue) -> Result<Digest> {
    validate_document(label, value)?;
    Ok(Digest::domain_separated(domain, &canonical_bytes(value)?))
}

fn require_object<'a>(
    label: &str,
    value: &'a CanonicalValue,
) -> Result<&'a [(String, CanonicalValue)]> {
    match value {
        CanonicalValue::Object(entries) => Ok(entries),
        _ => Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} must be a JSON object"
        ))),
    }
}

fn require_exact_object_fields(
    value: &CanonicalValue,
    expected_fields: &[&str],
    label: &str,
) -> Result<()> {
    let actual_fields = require_object(label, value)?
        .iter()
        .map(|(field, _)| field.as_str())
        .collect::<BTreeSet<_>>();
    let expected_fields = expected_fields.iter().copied().collect::<BTreeSet<_>>();
    if actual_fields == expected_fields {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} fields do not match the supported schema"
        )))
    }
}

fn require_canonical_match(
    actual: &CanonicalValue,
    expected: &CanonicalValue,
    label: &str,
) -> Result<()> {
    if canonical_bytes(actual)? == canonical_bytes(expected)? {
        Ok(())
    } else {
        Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} does not match the expected canonical value"
        )))
    }
}

fn object_field<'a>(
    value: &'a CanonicalValue,
    field: &str,
    label: &str,
) -> Result<&'a CanonicalValue> {
    require_object(label, value)?
        .iter()
        .find_map(|(key, value)| (key == field).then_some(value))
        .ok_or_else(|| {
            WorkVcsError::ContinuationInvalid(format!("{label} requires field {field:?}"))
        })
}

fn optional_object_field<'a>(value: &'a CanonicalValue, field: &str) -> Option<&'a CanonicalValue> {
    match value {
        CanonicalValue::Object(entries) => entries
            .iter()
            .find_map(|(key, value)| (key == field).then_some(value)),
        _ => None,
    }
}

fn require_nonempty_string<'a>(
    value: &'a CanonicalValue,
    field: &str,
    label: &str,
) -> Result<&'a str> {
    match object_field(value, field, label)? {
        CanonicalValue::String(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} must be a non-empty string"
        ))),
    }
}

fn require_bool(value: &CanonicalValue, field: &str, label: &str) -> Result<bool> {
    match object_field(value, field, label)? {
        CanonicalValue::Bool(value) => Ok(*value),
        _ => Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} must be boolean"
        ))),
    }
}

fn require_digest_string(value: &CanonicalValue, field: &str, label: &str) -> Result<Digest> {
    let raw = require_nonempty_string(value, field, label)?;
    Digest::from_hex(raw).map_err(|error| {
        WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} must be a canonical digest: {error}"
        ))
    })
}

fn require_integer(value: &CanonicalValue, field: &str, label: &str) -> Result<i64> {
    match object_field(value, field, label)? {
        CanonicalValue::Integer(value) => Ok(value.get()),
        _ => Err(WorkVcsError::ContinuationInvalid(format!(
            "{label} field {field:?} must be an integer"
        ))),
    }
}

fn scope_event_payload(
    options: &ContinuationScopeBindOptions,
    scope_id: ContinuationScopeId,
    owner_epoch: i64,
    listener_revision: i64,
) -> Result<CanonicalValue> {
    CanonicalValue::object(vec![
        (
            "branch_id".to_owned(),
            CanonicalValue::String(options.branch_id.to_string()),
        ),
        (
            "carrier_claim_id".to_owned(),
            CanonicalValue::String(options.carrier_claim_id.to_string()),
        ),
        (
            "carrier_session_id".to_owned(),
            CanonicalValue::String(options.carrier_session_id.to_string()),
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(listener_revision)?,
        ),
        (
            "listener_session_id".to_owned(),
            CanonicalValue::String(options.listener_session_id.to_string()),
        ),
        (
            "owner_epoch".to_owned(),
            CanonicalValue::safe_integer(owner_epoch)?,
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(scope_id.to_string()),
        ),
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(options.task_entity_id.to_string()),
        ),
    ])
}

fn insert_rotation_event(
    transaction: &Transaction<'_>,
    scope: &ContinuationScopeSnapshot,
    rotation_id: ContinuationRotationId,
    session_id: SessionId,
    event_kind: &str,
    now_us: i64,
) -> Result<()> {
    insert_event(
        transaction,
        scope.workspace_id,
        session_id,
        event_kind,
        now_us,
        CanonicalValue::object(vec![
            (
                "branch_id".to_owned(),
                CanonicalValue::String(scope.branch_id.to_string()),
            ),
            (
                "listener_revision".to_owned(),
                CanonicalValue::safe_integer(scope.listener_revision)?,
            ),
            (
                "owner_epoch".to_owned(),
                CanonicalValue::safe_integer(scope.owner_epoch)?,
            ),
            (
                "rotation_id".to_owned(),
                CanonicalValue::String(rotation_id.to_string()),
            ),
            (
                "scope_id".to_owned(),
                CanonicalValue::String(scope.scope_id.to_string()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(scope.task_entity_id.to_string()),
            ),
        ])?,
    )
}

fn insert_abort_only_recovery_event(
    transaction: &Transaction<'_>,
    scope: &ContinuationScopeSnapshot,
    rotation_id: ContinuationRotationId,
    session_id: SessionId,
    now_us: i64,
    recovery_event: &AbortOnlyRecoveryEvent<'_>,
) -> Result<()> {
    insert_event(
        transaction,
        scope.workspace_id,
        session_id,
        "continuation.listener_recovered_abort_only",
        now_us,
        CanonicalValue::object(vec![
            (
                "branch_id".to_owned(),
                CanonicalValue::String(scope.branch_id.to_string()),
            ),
            (
                "listener_revision".to_owned(),
                CanonicalValue::safe_integer(scope.listener_revision)?,
            ),
            (
                "owner_epoch".to_owned(),
                CanonicalValue::safe_integer(scope.owner_epoch)?,
            ),
            ("recovery".to_owned(), recovery_event.recovery.clone()),
            (
                "recovery_receipt_digest".to_owned(),
                CanonicalValue::String(recovery_event.recovery_receipt_digest.to_string()),
            ),
            (
                "previous_recovery_history_chain_digest".to_owned(),
                CanonicalValue::String(recovery_event.previous_chain_digest.to_string()),
            ),
            (
                "recovery_history_chain_digest".to_owned(),
                CanonicalValue::String(recovery_event.history.chain_digest.to_string()),
            ),
            (
                "recovery_history_count".to_owned(),
                CanonicalValue::safe_integer(recovery_event.history.count)?,
            ),
            (
                "rotation_id".to_owned(),
                CanonicalValue::String(rotation_id.to_string()),
            ),
            (
                "scope_id".to_owned(),
                CanonicalValue::String(scope.scope_id.to_string()),
            ),
            (
                "task_entity_id".to_owned(),
                CanonicalValue::String(scope.task_entity_id.to_string()),
            ),
        ])?,
    )
}

fn insert_event(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    session_id: SessionId,
    event_kind: &str,
    occurred_at_us: i64,
    payload: CanonicalValue,
) -> Result<()> {
    let event_id = EventId::new_v7();
    transaction
        .execute(
            "INSERT INTO event(
                event_id, workspace_id, changeset_id, session_id, event_kind,
                occurred_at_us, payload_json
             ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![
                &event_id.raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                &session_id.raw_bytes()[..],
                event_kind,
                occurred_at_us,
                canonical_json("continuation event payload", &payload)?,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AbortOnlyRecoveryHistorySummary {
    count: i64,
    chain_digest: Digest,
}

struct AbortOnlyRecoveryEvent<'a> {
    recovery: &'a CanonicalValue,
    recovery_receipt_digest: Digest,
    previous_chain_digest: Digest,
    history: AbortOnlyRecoveryHistorySummary,
}

fn recovery_history_genesis_digest() -> Digest {
    Digest::domain_separated(RECOVERY_HISTORY_CHAIN_DIGEST_DOMAIN, b"genesis")
}

fn next_recovery_history_chain_digest(
    previous_chain_digest: Digest,
    recovery_revision: i64,
    recovery_receipt_digest: Digest,
) -> Result<Digest> {
    document_digest(
        RECOVERY_HISTORY_CHAIN_DIGEST_DOMAIN,
        "continuation abort-only recovery history link",
        &CanonicalValue::object(vec![
            (
                "previous_recovery_history_chain_digest".to_owned(),
                CanonicalValue::String(previous_chain_digest.to_string()),
            ),
            (
                "recovery_receipt_digest".to_owned(),
                CanonicalValue::String(recovery_receipt_digest.to_string()),
            ),
            (
                "recovery_revision".to_owned(),
                CanonicalValue::safe_integer(recovery_revision)?,
            ),
            (
                "schema_version".to_owned(),
                CanonicalValue::safe_integer(1)?,
            ),
        ])?,
    )
}

fn validate_abort_only_recovery_event_payload(
    payload: &CanonicalValue,
    rotation_id: ContinuationRotationId,
    expected_revision: i64,
    expected_recovery: Option<&CanonicalValue>,
    expected_recovery_receipt_digest: Option<Digest>,
) -> Result<(Digest, Digest)> {
    let label = "continuation abort-only recovery event payload";
    require_matching_string(payload, "rotation_id", &rotation_id.to_string(), label)?;
    require_matching_integer(payload, "recovery_history_count", expected_revision, label)?;
    let recovery = object_field(payload, "recovery", label)?;
    require_matching_integer(
        recovery,
        "recovery_revision",
        expected_revision,
        "continuation abort-only recovery history item",
    )?;
    let recovery_receipt_digest = document_digest(
        RECOVERY_RECEIPT_DIGEST_DOMAIN,
        "continuation abort-only recovery history item",
        recovery,
    )?;
    require_matching_string(
        payload,
        "recovery_receipt_digest",
        &recovery_receipt_digest.to_string(),
        label,
    )?;
    if let Some(expected_digest) = expected_recovery_receipt_digest
        && recovery_receipt_digest != expected_digest
    {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation latest recovery receipt digest does not match its append-only event"
                .to_owned(),
        ));
    }
    if let Some(expected) = expected_recovery {
        require_canonical_match(
            recovery,
            expected,
            "continuation latest recovery history item",
        )?;
    }
    let previous_chain_digest =
        require_digest_string(payload, "previous_recovery_history_chain_digest", label)?;
    let chain_digest = require_digest_string(payload, "recovery_history_chain_digest", label)?;
    let expected_chain_digest = next_recovery_history_chain_digest(
        previous_chain_digest,
        expected_revision,
        recovery_receipt_digest,
    )?;
    if chain_digest != expected_chain_digest {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation recovery history chain digest does not match its event".to_owned(),
        ));
    }
    Ok((previous_chain_digest, chain_digest))
}

fn load_abort_only_recovery_summary(
    connection: &Connection,
    workspace_id: WorkspaceId,
    rotation_id: ContinuationRotationId,
    expected_recovery_revision: i64,
    latest_recovery: Option<&CanonicalValue>,
    latest_recovery_receipt_digest: Option<Digest>,
) -> Result<AbortOnlyRecoveryHistorySummary> {
    let count = connection
        .query_row(
            "SELECT COUNT(*)
             FROM event
             WHERE workspace_id = ?1
               AND event_kind = 'continuation.listener_recovered_abort_only'
               AND json_extract(payload_json, '$.rotation_id') = ?2",
            params![&workspace_id.raw_bytes()[..], rotation_id.to_string()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage_error)?;
    if count != expected_recovery_revision {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation recovery history contains {count} entries, expected {expected_recovery_revision}"
        )));
    }
    if count == 0 {
        if latest_recovery.is_some() || latest_recovery_receipt_digest.is_some() {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation recovery projection exists without append-only history".to_owned(),
            ));
        }
        return Ok(AbortOnlyRecoveryHistorySummary {
            count,
            chain_digest: recovery_history_genesis_digest(),
        });
    }
    if latest_recovery.is_none() || latest_recovery_receipt_digest.is_none() {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation recovery history exists without its latest projection".to_owned(),
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT payload_json
             FROM event
             WHERE workspace_id = ?1
               AND event_kind = 'continuation.listener_recovered_abort_only'
               AND json_extract(payload_json, '$.rotation_id') = ?2
             ORDER BY CAST(json_extract(payload_json, '$.recovery.recovery_revision') AS INTEGER) DESC,
                      event_id DESC
             LIMIT 2",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![&workspace_id.raw_bytes()[..], rotation_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage_error)?;
    let mut payloads = Vec::new();
    for row in rows {
        payloads.push(parse_continuation_json(
            "continuation abort-only recovery event payload",
            &row.map_err(storage_error)?,
        )?);
    }
    let latest_payload = payloads.first().ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(
            "continuation recovery history count has no latest event".to_owned(),
        )
    })?;
    let (latest_previous_chain_digest, latest_chain_digest) =
        validate_abort_only_recovery_event_payload(
            latest_payload,
            rotation_id,
            expected_recovery_revision,
            latest_recovery,
            latest_recovery_receipt_digest,
        )?;
    if expected_recovery_revision == 1 {
        if latest_previous_chain_digest != recovery_history_genesis_digest() {
            return Err(WorkVcsError::ContinuationInvalid(
                "first continuation recovery event does not start at the history genesis digest"
                    .to_owned(),
            ));
        }
    } else {
        let previous_payload = payloads.get(1).ok_or_else(|| {
            WorkVcsError::ContinuationInvalid(
                "continuation recovery history is missing the preceding event".to_owned(),
            )
        })?;
        let (previous_previous_chain_digest, previous_chain_digest) =
            validate_abort_only_recovery_event_payload(
                previous_payload,
                rotation_id,
                expected_recovery_revision - 1,
                None,
                None,
            )?;
        if expected_recovery_revision == 2
            && previous_previous_chain_digest != recovery_history_genesis_digest()
        {
            return Err(WorkVcsError::ContinuationInvalid(
                "first continuation recovery event does not start at the history genesis digest"
                    .to_owned(),
            ));
        }
        if latest_previous_chain_digest != previous_chain_digest {
            return Err(WorkVcsError::ContinuationInvalid(
                "continuation recovery history latest link does not match its preceding event"
                    .to_owned(),
            ));
        }
    }
    if count < 0 {
        return Err(WorkVcsError::ContinuationInvalid(format!(
            "continuation recovery history contains invalid negative count {count}"
        )));
    }
    Ok(AbortOnlyRecoveryHistorySummary {
        count,
        chain_digest: latest_chain_digest,
    })
}

fn canonical_json(label: &str, value: &CanonicalValue) -> Result<String> {
    validate_document(label, value)?;
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::ContinuationInvalid(format!("{label} is not UTF-8: {error}"))
    })
}

fn parse_acceptance_history(json: &str) -> Result<CanonicalValue> {
    let history = parse_continuation_json("candidate_acceptance_history_json", json)?;
    acceptance_history_entries(&history)?;
    Ok(history)
}

fn append_acceptance_history(
    history: &CanonicalValue,
    acceptance: &CanonicalValue,
) -> Result<CanonicalValue> {
    let mut acceptances = acceptance_history_entries(history)?.clone();
    validate_document("prior continuation candidate acceptance", acceptance)?;
    acceptances.push(acceptance.clone());
    let updated = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1)?,
        ),
        ("acceptances".to_owned(), CanonicalValue::Array(acceptances)),
    ])?;
    validate_document("continuation candidate acceptance history", &updated)?;
    Ok(updated)
}

fn acceptance_history_entries(history: &CanonicalValue) -> Result<&Vec<CanonicalValue>> {
    require_schema_version(history, "continuation candidate acceptance history")?;
    let CanonicalValue::Object(entries) = history else {
        unreachable!("require_schema_version validates an object")
    };
    let value = entries
        .iter()
        .find(|(key, _)| key == "acceptances")
        .map(|(_, value)| value)
        .ok_or_else(|| {
            WorkVcsError::ContinuationInvalid(
                "continuation candidate acceptance history is missing acceptances".to_owned(),
            )
        })?;
    let CanonicalValue::Array(acceptances) = value else {
        return Err(WorkVcsError::ContinuationInvalid(
            "continuation candidate acceptance history acceptances must be an array".to_owned(),
        ));
    };
    for acceptance in acceptances {
        validate_document("prior continuation candidate acceptance", acceptance)?;
    }
    Ok(acceptances)
}

fn parse_continuation_json(label: &str, json: &str) -> Result<CanonicalValue> {
    parse_canonical_json(json.as_bytes()).map_err(|error| {
        WorkVcsError::ContinuationInvalid(format!("stored {label} is invalid: {error}"))
    })
}

fn decode_rotation_state(value: &str) -> Result<ContinuationRotationState> {
    ContinuationRotationState::from_str(value).ok_or_else(|| {
        WorkVcsError::ContinuationInvalid(format!(
            "stored continuation rotation state {value:?} is invalid"
        ))
    })
}

fn decode_sqlite_bool(column: &str, value: i64) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(WorkVcsError::ContinuationInvalid(format!(
            "stored {column} value {other} is not a boolean"
        ))),
    }
}

fn session_lifecycle_state_label(state: SessionLifecycleState) -> &'static str {
    match state {
        SessionLifecycleState::Active => "active",
        SessionLifecycleState::PotentiallyStale => "potentially_stale",
        SessionLifecycleState::Ended => "ended",
    }
}

fn decode_uuid_bytes(column: &str, bytes: Vec<u8>) -> Result<[u8; 16]> {
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ContinuationInvalid(format!(
            "stored {column} has {} bytes instead of 16",
            bytes.len()
        ))
    })
}

fn decode_digest(column: &str, bytes: Vec<u8>) -> Result<Digest> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        WorkVcsError::ContinuationInvalid(format!(
            "stored {column} has {} bytes instead of 32",
            bytes.len()
        ))
    })?;
    Ok(Digest::from_bytes(bytes))
}

macro_rules! decode_id {
    ($name:ident, $type:ty) => {
        fn $name(column: &str, bytes: Vec<u8>) -> Result<$type> {
            <$type>::from_bytes(decode_uuid_bytes(column, bytes)?).map_err(|error| {
                WorkVcsError::ContinuationInvalid(format!("stored {column} is invalid: {error}"))
            })
        }
    };
}

decode_id!(decode_workspace_id, WorkspaceId);
decode_id!(decode_branch_id, BranchId);
decode_id!(decode_entity_id, EntityId);
decode_id!(decode_entity_version_id, EntityVersionId);
decode_id!(decode_session_id, SessionId);
decode_id!(decode_claim_id, ClaimId);
decode_id!(decode_scope_id, ContinuationScopeId);
decode_id!(decode_rotation_id, ContinuationRotationId);
