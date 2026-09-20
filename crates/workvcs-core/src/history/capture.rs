use super::admission::{
    PlanAdmissionEvidenceManifest, PlanAdmissionRecordManifest, PreparedEntity, PreparedEvidence,
    PreparedRelation, load_active_branch, move_branch_head, prepare_evidence, prepare_records,
    prepared_entity, write_entity, write_entity_change_operation, write_evidence, write_relation,
    write_relation_change_operation,
};
use super::knowledge::{KNOWLEDGE_ENTITY_KIND, KnowledgeState};
use super::record::RecordKind;
use super::state_at;
use crate::canonical::{
    CanonicalValue, WorkState, canonical_bytes, parse_canonical_json, relation_version_digest,
    work_state_mapping_digest,
};
use crate::error::{Result, WorkVcsError, storage_error};
use crate::identity::{
    BranchId, ChangeSetId, CommitId, Digest, EntityId, EntityVersionId, EventId, EvidenceId,
    RelationId, RelationVersionId, WorkspaceId,
};
use crate::store::{StoreConnection, current_epoch_micros};
use rusqlite::{Transaction, TransactionBehavior, params};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const CAPTURE_EVENT_KIND: &str = "cognition.captured";
pub(super) const CAPTURE_OPERATION_TYPE: &str = "cognition.capture";
pub(super) const CAPTURE_OPERATION_SCHEMA_VERSION: i64 = 1;
const NORMAL_COMMIT_KIND: &str = "normal";
const PAYLOAD_DIGEST_DOMAIN: &str = "workvcs.cognition-capture-manifest.v1";
const PRIMARY_PARENT_ROLE: &str = "primary";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CognitionCaptureOptions {
    branch_id: BranchId,
    current_head_commit_id: CommitId,
    current_state_digest: Digest,
    manifest: CognitionCaptureManifest,
}

impl CognitionCaptureOptions {
    pub fn new(
        branch_id: BranchId,
        current_head_commit_id: CommitId,
        current_state_digest: Digest,
        manifest: CognitionCaptureManifest,
    ) -> Self {
        Self {
            branch_id,
            current_head_commit_id,
            current_state_digest,
            manifest,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CognitionCaptureManifest {
    pub schema_version: i64,
    pub idempotency_key: String,
    pub expected_head_commit_id: Option<String>,
    pub expected_state_digest: Option<String>,
    pub records: Vec<PlanAdmissionRecordManifest>,
    pub knowledge: Vec<CognitionCaptureKnowledgeManifest>,
    pub evidence: Vec<PlanAdmissionEvidenceManifest>,
    pub relations: Vec<CognitionCaptureRelationManifest>,
    pub rationale: CanonicalValue,
    payload_digest: Digest,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CognitionCaptureManifestShape {
    schema_version: i64,
    idempotency_key: String,
    #[serde(default)]
    expected_head_commit_id: Option<String>,
    #[serde(default)]
    expected_state_digest: Option<String>,
    #[serde(default)]
    records: Vec<PlanAdmissionRecordManifest>,
    #[serde(default)]
    knowledge: Vec<CognitionCaptureKnowledgeManifest>,
    #[serde(default)]
    evidence: Vec<PlanAdmissionEvidenceManifest>,
    #[serde(default)]
    relations: Vec<CognitionCaptureRelationManifest>,
    #[serde(default = "empty_object")]
    rationale: CanonicalValue,
}

impl CognitionCaptureManifest {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let value = parse_canonical_json(bytes).map_err(|error| {
            WorkVcsError::RecordInvalid(format!(
                "cognition capture manifest is not valid JSON: {error}"
            ))
        })?;
        let canonical = canonical_bytes(&value).map_err(capture_invalid_from)?;
        let payload_digest = Digest::domain_separated(PAYLOAD_DIGEST_DOMAIN, &canonical);
        let shape: CognitionCaptureManifestShape =
            serde_json::from_slice(&canonical).map_err(|error| {
                WorkVcsError::RecordInvalid(format!(
                    "cognition capture manifest has invalid shape: {error}"
                ))
            })?;
        Ok(Self {
            schema_version: shape.schema_version,
            idempotency_key: shape.idempotency_key,
            expected_head_commit_id: shape.expected_head_commit_id,
            expected_state_digest: shape.expected_state_digest,
            records: shape.records,
            knowledge: shape.knowledge,
            evidence: shape.evidence,
            relations: shape.relations,
            rationale: shape.rationale,
            payload_digest,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CognitionCaptureKnowledgeManifest {
    pub local_id: String,
    pub statement: String,
    #[serde(default = "empty_object")]
    pub scope: CanonicalValue,
    #[serde(default = "empty_object")]
    pub provenance: CanonicalValue,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CognitionCaptureRelationManifest {
    pub local_id: String,
    #[serde(rename = "type")]
    pub relation_type: String,
    pub source_local_id: String,
    pub target_local_id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub rationale: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CognitionCaptureOutcome {
    Created,
    Reused,
}

impl CognitionCaptureOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Reused => "reused",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CognitionCaptureResult {
    pub outcome: CognitionCaptureOutcome,
    pub workspace_id: WorkspaceId,
    pub branch_id: BranchId,
    pub previous_head_commit_id: CommitId,
    pub commit_id: CommitId,
    pub changeset_id: ChangeSetId,
    pub idempotency_key: String,
    pub payload_digest: Digest,
    pub work_state_digest: Digest,
    pub records: Vec<CognitionCaptureEntityResult>,
    pub knowledge: Vec<CognitionCaptureEntityResult>,
    pub evidence: Vec<CognitionCaptureEvidenceResult>,
    pub relations: Vec<CognitionCaptureRelationResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CognitionCaptureEntityResult {
    pub local_id: String,
    pub kind: String,
    pub entity_id: EntityId,
    pub entity_version_id: EntityVersionId,
    pub state_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CognitionCaptureEvidenceResult {
    pub local_id: String,
    pub evidence_id: EvidenceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CognitionCaptureRelationResult {
    pub local_id: String,
    pub relation_type: String,
    pub relation_id: RelationId,
    pub relation_version_id: RelationVersionId,
    pub source_entity_id: EntityId,
    pub target_entity_id: EntityId,
}

#[derive(Clone, Debug)]
struct PreparedKnowledge {
    local_id: String,
    entity: PreparedEntity,
}

#[derive(Clone, Debug)]
struct PreparedCaptureRelation {
    local_id: String,
    relation: PreparedRelation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalEntityKind {
    Record(RecordKind),
    Knowledge,
}

#[derive(Clone, Copy, Debug)]
struct LocalEntity {
    entity_id: EntityId,
    kind: LocalEntityKind,
}

#[derive(Clone, Debug)]
struct PreparedCapture {
    branch_id: BranchId,
    previous_head_commit_id: CommitId,
    expected_state_digest: Digest,
    idempotency_key: String,
    payload_digest: Digest,
    rationale: CanonicalValue,
    records: Vec<super::admission::PreparedRecord>,
    knowledge: Vec<PreparedKnowledge>,
    evidence: Vec<PreparedEvidence>,
    relations: Vec<PreparedCaptureRelation>,
}

pub(crate) fn capture_cognition(
    connection: &mut StoreConnection,
    options: &CognitionCaptureOptions,
) -> Result<CognitionCaptureResult> {
    connection.verify_foreign_keys()?;
    let prepared = prepare_capture(options)?;
    let parent = state_at(connection, prepared.previous_head_commit_id)?;
    let now_us = current_epoch_micros()?;
    let changeset_id = ChangeSetId::new_v7();
    let commit_id = CommitId::new_v7();

    let transaction = connection
        .inner_mut()
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let branch = load_active_branch(&transaction, prepared.branch_id)?;
    if let Some(reused) = find_idempotent_capture(
        &transaction,
        branch.workspace_id,
        prepared.branch_id,
        &prepared.idempotency_key,
        prepared.payload_digest,
    )? {
        return Ok(reused);
    }
    if branch.head_commit_id != prepared.previous_head_commit_id {
        return Err(WorkVcsError::BranchHeadConflict(format!(
            "branch {} expected head {}, found {}",
            prepared.branch_id, prepared.previous_head_commit_id, branch.head_commit_id
        )));
    }
    if parent.workspace_id != branch.workspace_id {
        return Err(WorkVcsError::RecordInvalid(format!(
            "branch {} belongs to workspace {}, but expected head {} belongs to workspace {}",
            prepared.branch_id,
            branch.workspace_id,
            prepared.previous_head_commit_id,
            parent.workspace_id
        )));
    }
    if parent.state_digest != prepared.expected_state_digest {
        return Err(WorkVcsError::DigestInvalid(format!(
            "expected head {} state digest {} does not match expected {}",
            prepared.previous_head_commit_id, parent.state_digest, prepared.expected_state_digest
        )));
    }

    let next_state = capture_work_state(&parent.state, &prepared)?;
    let work_state_digest = work_state_mapping_digest(&next_state);
    let payload = capture_payload_value(
        branch.workspace_id,
        commit_id,
        changeset_id,
        work_state_digest,
        &prepared,
    )?;
    let payload_json = canonical_json_string(&payload)?;
    let rationale_json = canonical_json_string(&prepared.rationale)?;
    write_capture(
        &transaction,
        branch.workspace_id,
        commit_id,
        changeset_id,
        now_us,
        work_state_digest,
        &payload_json,
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

    result_from_payload(&payload, CognitionCaptureOutcome::Created)
}

fn prepare_capture(options: &CognitionCaptureOptions) -> Result<PreparedCapture> {
    let manifest = &options.manifest;
    if manifest.schema_version != 1 {
        return Err(WorkVcsError::RecordInvalid(format!(
            "cognition capture manifest schema_version {} is not supported",
            manifest.schema_version
        )));
    }
    validate_text("idempotency key", &manifest.idempotency_key)?;
    if manifest.idempotency_key.len() > 256 {
        return Err(WorkVcsError::RecordInvalid(
            "idempotency key must be at most 256 bytes".to_owned(),
        ));
    }
    require_object("capture rationale", &manifest.rationale)?;
    if manifest.records.is_empty() && manifest.knowledge.is_empty() {
        return Err(WorkVcsError::RecordInvalid(
            "cognition capture requires at least one Record or Knowledge item; standalone Evidence may use evidence create"
                .to_owned(),
        ));
    }
    validate_unique_local_ids(
        manifest
            .records
            .iter()
            .map(|item| ("record", &item.local_id))
            .chain(
                manifest
                    .knowledge
                    .iter()
                    .map(|item| ("knowledge", &item.local_id)),
            )
            .chain(
                manifest
                    .evidence
                    .iter()
                    .map(|item| ("evidence", &item.local_id)),
            ),
    )?;
    validate_unique_local_ids(
        manifest
            .relations
            .iter()
            .map(|item| ("relation", &item.local_id)),
    )?;

    let records = prepare_records(&manifest.records)?;
    let knowledge = manifest
        .knowledge
        .iter()
        .map(|item| {
            validate_text("knowledge local_id", &item.local_id)?;
            require_object("knowledge scope", &item.scope)?;
            require_object("knowledge provenance", &item.provenance)?;
            let state = KnowledgeState::active(item.statement.clone())?
                .with_scope(item.scope.clone())?
                .with_provenance(item.provenance.clone())?;
            Ok(PreparedKnowledge {
                local_id: item.local_id.clone(),
                entity: prepared_entity(KNOWLEDGE_ENTITY_KIND, state.to_canonical_value()?)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let evidence = prepare_evidence(&manifest.evidence)?;

    let mut local_entities = BTreeMap::new();
    for record in &records {
        local_entities.insert(
            record.local_id.clone(),
            LocalEntity {
                entity_id: record.entity.entity_id,
                kind: LocalEntityKind::Record(record.kind),
            },
        );
    }
    for item in &knowledge {
        local_entities.insert(
            item.local_id.clone(),
            LocalEntity {
                entity_id: item.entity.entity_id,
                kind: LocalEntityKind::Knowledge,
            },
        );
    }
    let relations = prepare_relations(&manifest.relations, &local_entities)?;
    let previous_head_commit_id = manifest
        .expected_head_commit_id
        .as_deref()
        .map(CommitId::parse_canonical)
        .transpose()?
        .unwrap_or(options.current_head_commit_id);
    let expected_state_digest = manifest
        .expected_state_digest
        .as_deref()
        .map(Digest::from_hex)
        .transpose()?
        .unwrap_or(options.current_state_digest);

    Ok(PreparedCapture {
        branch_id: options.branch_id,
        previous_head_commit_id,
        expected_state_digest,
        idempotency_key: manifest.idempotency_key.clone(),
        payload_digest: manifest.payload_digest,
        rationale: manifest.rationale.clone(),
        records,
        knowledge,
        evidence,
        relations,
    })
}

fn prepare_relations(
    manifests: &[CognitionCaptureRelationManifest],
    local_entities: &BTreeMap<String, LocalEntity>,
) -> Result<Vec<PreparedCaptureRelation>> {
    let empty_state = CanonicalValue::object(Vec::new())?;
    let state_json = canonical_json_string(&empty_state)?;
    let state_digest = relation_version_digest(&empty_state)?;
    let mut logical_keys = BTreeSet::new();
    manifests
        .iter()
        .map(|manifest| {
            validate_text("relation local_id", &manifest.local_id)?;
            validate_text("relation rationale", &manifest.rationale)?;
            let source = local_entities.get(&manifest.source_local_id).ok_or_else(|| {
                WorkVcsError::RecordInvalid(format!(
                    "relation {} source local_id {:?} does not name a captured Record or Knowledge item",
                    manifest.local_id, manifest.source_local_id
                ))
            })?;
            let target = local_entities.get(&manifest.target_local_id).ok_or_else(|| {
                WorkVcsError::RecordInvalid(format!(
                    "relation {} target local_id {:?} does not name a captured Record or Knowledge item",
                    manifest.local_id, manifest.target_local_id
                ))
            })?;
            if source.entity_id == target.entity_id {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "relation {} cannot use the same item as source and target",
                    manifest.local_id
                )));
            }
            let relation_type = capture_relation_type(&manifest.relation_type)?;
            validate_capture_relation_endpoints(relation_type, *source, *target)?;
            let discriminator = match relation_type {
                "related_to" => {
                    let label = manifest.label.as_deref().ok_or_else(|| {
                        WorkVcsError::RecordInvalid(format!(
                            "related_to relation {} requires label",
                            manifest.local_id
                        ))
                    })?;
                    validate_text("related_to label", label)?;
                    label
                }
                _ => {
                    if manifest.label.is_some() {
                        return Err(WorkVcsError::RecordInvalid(format!(
                            "relation {} type {} does not accept label",
                            manifest.local_id, relation_type
                        )));
                    }
                    ""
                }
            };
            let logical_key = (
                relation_type.to_owned(),
                source.entity_id,
                target.entity_id,
                discriminator.to_owned(),
            );
            if !logical_keys.insert(logical_key) {
                return Err(WorkVcsError::RecordInvalid(format!(
                    "relation {} duplicates another relation in the capture manifest",
                    manifest.local_id
                )));
            }
            Ok(PreparedCaptureRelation {
                local_id: manifest.local_id.clone(),
                relation: PreparedRelation {
                    relation_id: RelationId::new_v7(),
                    relation_version_id: RelationVersionId::new_v7(),
                    relation_type: relation_type.to_owned(),
                    relation_discriminator: discriminator.to_owned(),
                    source_entity_id: source.entity_id,
                    target_entity_id: target.entity_id,
                    state_json: state_json.clone(),
                    state_digest,
                },
            })
        })
        .collect()
}

fn capture_relation_type(value: &str) -> Result<&'static str> {
    match value {
        "contradicts" => Ok("contradicts"),
        "derived_from" => Ok("derived_from"),
        "related_to" => Ok("related_to"),
        "supports" => Ok("supports"),
        "validates" => Ok("validates"),
        "invalidates" | "supersedes" => Err(WorkVcsError::RecordInvalid(format!(
            "capture relation type {value:?} requires an existing target lifecycle transition; use the dedicated semantic command"
        ))),
        other => Err(WorkVcsError::RecordInvalid(format!(
            "capture relation type {other:?} is not supported"
        ))),
    }
}

fn validate_capture_relation_endpoints(
    relation_type: &str,
    source: LocalEntity,
    target: LocalEntity,
) -> Result<()> {
    match relation_type {
        "derived_from" | "related_to" => match (source.kind, target.kind) {
            (LocalEntityKind::Record(_), LocalEntityKind::Record(_)) => Ok(()),
            _ => Err(WorkVcsError::RecordInvalid(format!(
                "{relation_type} capture relation requires Record source and target"
            ))),
        },
        "supports" | "contradicts" => match (source.kind, target.kind) {
            (
                LocalEntityKind::Record(RecordKind::Finding),
                LocalEntityKind::Record(RecordKind::Decision),
            )
            | (LocalEntityKind::Record(RecordKind::Finding), LocalEntityKind::Knowledge) => Ok(()),
            _ => Err(WorkVcsError::RecordInvalid(format!(
                "{relation_type} capture relation requires a Finding source and Decision or Knowledge target"
            ))),
        },
        "validates" => match (source.kind, target.kind) {
            (LocalEntityKind::Record(RecordKind::Finding), LocalEntityKind::Knowledge) => Ok(()),
            _ => Err(WorkVcsError::RecordInvalid(
                "validates capture relation requires a Finding source and Knowledge target"
                    .to_owned(),
            )),
        },
        _ => unreachable!("capture relation type already validated"),
    }
}

fn capture_work_state(parent: &WorkState, prepared: &PreparedCapture) -> Result<WorkState> {
    let mut entities = parent
        .entities()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    for record in &prepared.records {
        insert_entity_mapping(&mut entities, &record.entity)?;
    }
    for item in &prepared.knowledge {
        insert_entity_mapping(&mut entities, &item.entity)?;
    }
    let mut relations = parent
        .relations()
        .iter()
        .copied()
        .collect::<BTreeMap<_, _>>();
    for item in &prepared.relations {
        if relations
            .insert(item.relation.relation_id, item.relation.relation_version_id)
            .is_some()
        {
            return Err(WorkVcsError::RelationInvalid(format!(
                "capture generated duplicate relation id {}",
                item.relation.relation_id
            )));
        }
    }
    WorkState::new(entities, relations)
}

fn insert_entity_mapping(
    entities: &mut BTreeMap<EntityId, EntityVersionId>,
    entity: &PreparedEntity,
) -> Result<()> {
    if entities
        .insert(entity.entity_id, entity.entity_version_id)
        .is_some()
    {
        return Err(WorkVcsError::EntityTransitionInvalid(format!(
            "capture generated duplicate entity id {}",
            entity.entity_id
        )));
    }
    Ok(())
}

fn capture_payload_value(
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
    work_state_digest: Digest,
    prepared: &PreparedCapture,
) -> Result<CanonicalValue> {
    let records = prepared
        .records
        .iter()
        .map(|record| {
            object(vec![
                ("local_id", string(&record.local_id)),
                ("kind", string(record.kind.as_str())),
                ("entity_id", string(&record.entity.entity_id.to_string())),
                (
                    "entity_version_id",
                    string(&record.entity.entity_version_id.to_string()),
                ),
                ("state_digest", string(&record.entity.state_digest.to_hex())),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let knowledge = prepared
        .knowledge
        .iter()
        .map(|item| {
            object(vec![
                ("local_id", string(&item.local_id)),
                ("kind", string("knowledge")),
                ("entity_id", string(&item.entity.entity_id.to_string())),
                (
                    "entity_version_id",
                    string(&item.entity.entity_version_id.to_string()),
                ),
                ("state_digest", string(&item.entity.state_digest.to_hex())),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let evidence = prepared
        .evidence
        .iter()
        .map(|item| {
            object(vec![
                ("local_id", string(&item.local_id)),
                ("evidence_id", string(&item.evidence_id.to_string())),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let relations = prepared
        .relations
        .iter()
        .map(|item| {
            object(vec![
                ("local_id", string(&item.local_id)),
                ("type", string(&item.relation.relation_type)),
                (
                    "relation_id",
                    string(&item.relation.relation_id.to_string()),
                ),
                (
                    "relation_version_id",
                    string(&item.relation.relation_version_id.to_string()),
                ),
                (
                    "source_entity_id",
                    string(&item.relation.source_entity_id.to_string()),
                ),
                (
                    "target_entity_id",
                    string(&item.relation.target_entity_id.to_string()),
                ),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    object(vec![
        ("schema_version", CanonicalValue::safe_integer(1)?),
        ("operation", string(CAPTURE_OPERATION_TYPE)),
        ("idempotency_key", string(&prepared.idempotency_key)),
        ("payload_digest", string(&prepared.payload_digest.to_hex())),
        ("workspace_id", string(&workspace_id.to_string())),
        ("branch_id", string(&prepared.branch_id.to_string())),
        (
            "previous_head_commit_id",
            string(&prepared.previous_head_commit_id.to_string()),
        ),
        ("commit_id", string(&commit_id.to_string())),
        ("changeset_id", string(&changeset_id.to_string())),
        ("work_state_digest", string(&work_state_digest.to_hex())),
        ("records", CanonicalValue::Array(records)),
        ("knowledge", CanonicalValue::Array(knowledge)),
        ("evidence", CanonicalValue::Array(evidence)),
        ("relations", CanonicalValue::Array(relations)),
    ])
}

// The arguments mirror the immutable changeset envelope at this transaction boundary.
#[allow(clippy::too_many_arguments)]
fn write_capture(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    commit_id: CommitId,
    changeset_id: ChangeSetId,
    now_us: i64,
    work_state_digest: Digest,
    payload_json: &str,
    rationale_json: &str,
    prepared: &PreparedCapture,
) -> Result<()> {
    for evidence in &prepared.evidence {
        write_evidence(transaction, evidence, now_us)?;
    }
    for record in &prepared.records {
        write_entity(transaction, workspace_id, &record.entity, now_us)?;
    }
    for item in &prepared.knowledge {
        write_entity(transaction, workspace_id, &item.entity, now_us)?;
    }
    for item in &prepared.relations {
        write_relation(transaction, workspace_id, &item.relation, now_us)?;
    }

    transaction
        .execute(
            "INSERT INTO changeset(
                changeset_id, workspace_id, operation_type, operation_schema_version,
                operation_payload_json, rationale_json, origin_session_id, created_at_us
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            params![
                &changeset_id.raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                CAPTURE_OPERATION_TYPE,
                CAPTURE_OPERATION_SCHEMA_VERSION,
                payload_json,
                rationale_json,
                now_us
            ],
        )
        .map_err(storage_error)?;
    let mut ordinal = 0_i64;
    for record in &prepared.records {
        write_entity_change_operation(transaction, changeset_id, ordinal, &record.entity)?;
        ordinal += 1;
    }
    for item in &prepared.knowledge {
        write_entity_change_operation(transaction, changeset_id, ordinal, &item.entity)?;
        ordinal += 1;
    }
    for item in &prepared.relations {
        write_relation_change_operation(transaction, changeset_id, ordinal, &item.relation)?;
        ordinal += 1;
    }
    debug_assert!(ordinal > 0);

    transaction
        .execute(
            "INSERT INTO workstate_commit(
                commit_id, workspace_id, changeset_id, commit_kind, state_digest, committed_at_us
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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
            "INSERT INTO commit_parent(commit_id, parent_ordinal, parent_role, parent_commit_id)
             VALUES (?1, 0, ?2, ?3)",
            params![
                &commit_id.raw_bytes()[..],
                PRIMARY_PARENT_ROLE,
                &prepared.previous_head_commit_id.raw_bytes()[..]
            ],
        )
        .map_err(storage_error)?;
    transaction
        .execute(
            "INSERT INTO event(
                event_id, workspace_id, changeset_id, session_id, event_kind, occurred_at_us,
                payload_json
             ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6)",
            params![
                &EventId::new_v7().raw_bytes()[..],
                &workspace_id.raw_bytes()[..],
                &changeset_id.raw_bytes()[..],
                CAPTURE_EVENT_KIND,
                now_us,
                payload_json
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn find_idempotent_capture(
    transaction: &Transaction<'_>,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    idempotency_key: &str,
    payload_digest: Digest,
) -> Result<Option<CognitionCaptureResult>> {
    let mut statement = transaction
        .prepare(
            "SELECT changeset.operation_payload_json
             FROM changeset
             JOIN workstate_commit ON workstate_commit.changeset_id = changeset.changeset_id
             WHERE changeset.workspace_id = ?1 AND changeset.operation_type = ?2
             ORDER BY changeset.created_at_us, changeset.changeset_id",
        )
        .map_err(storage_error)?;
    let rows = statement
        .query_map(
            params![&workspace_id.raw_bytes()[..], CAPTURE_OPERATION_TYPE],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage_error)?;
    let mut matched = None;
    for row in rows {
        let payload_json = row.map_err(storage_error)?;
        let value = parse_canonical_json(payload_json.as_bytes()).map_err(capture_invalid_from)?;
        let json: serde_json::Value = serde_json::from_str(&payload_json).map_err(|error| {
            WorkVcsError::RecordInvalid(format!("stored capture payload is invalid: {error}"))
        })?;
        if json
            .get("idempotency_key")
            .and_then(serde_json::Value::as_str)
            != Some(idempotency_key)
        {
            continue;
        }
        let stored_branch = required_json_str(&json, "branch_id")?;
        if stored_branch != branch_id.to_string() {
            return Err(WorkVcsError::RecordInvalid(format!(
                "idempotency key {idempotency_key:?} already belongs to branch {stored_branch}, not {branch_id}"
            )));
        }
        let stored_digest = Digest::from_hex(required_json_str(&json, "payload_digest")?)?;
        if stored_digest != payload_digest {
            return Err(WorkVcsError::RecordInvalid(format!(
                "idempotency key {idempotency_key:?} was already used with payload digest {stored_digest}, not {payload_digest}"
            )));
        }
        let result = result_from_payload(&value, CognitionCaptureOutcome::Reused)?;
        if matched.replace(result).is_some() {
            return Err(WorkVcsError::RecordInvalid(format!(
                "idempotency key {idempotency_key:?} has multiple matching capture payloads"
            )));
        }
    }
    Ok(matched)
}

fn result_from_payload(
    payload: &CanonicalValue,
    outcome: CognitionCaptureOutcome,
) -> Result<CognitionCaptureResult> {
    let bytes = canonical_bytes(payload).map_err(capture_invalid_from)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        WorkVcsError::RecordInvalid(format!("capture payload decode failed: {error}"))
    })?;
    Ok(CognitionCaptureResult {
        outcome,
        workspace_id: WorkspaceId::parse_canonical(required_json_str(&value, "workspace_id")?)?,
        branch_id: BranchId::parse_canonical(required_json_str(&value, "branch_id")?)?,
        previous_head_commit_id: CommitId::parse_canonical(required_json_str(
            &value,
            "previous_head_commit_id",
        )?)?,
        commit_id: CommitId::parse_canonical(required_json_str(&value, "commit_id")?)?,
        changeset_id: ChangeSetId::parse_canonical(required_json_str(&value, "changeset_id")?)?,
        idempotency_key: required_json_str(&value, "idempotency_key")?.to_owned(),
        payload_digest: Digest::from_hex(required_json_str(&value, "payload_digest")?)?,
        work_state_digest: Digest::from_hex(required_json_str(&value, "work_state_digest")?)?,
        records: parse_entity_results(&value, "records")?,
        knowledge: parse_entity_results(&value, "knowledge")?,
        evidence: parse_evidence_results(&value)?,
        relations: parse_relation_results(&value)?,
    })
}

fn parse_entity_results(
    value: &serde_json::Value,
    field: &str,
) -> Result<Vec<CognitionCaptureEntityResult>> {
    required_json_array(value, field)?
        .iter()
        .map(|item| {
            Ok(CognitionCaptureEntityResult {
                local_id: required_json_str(item, "local_id")?.to_owned(),
                kind: required_json_str(item, "kind")?.to_owned(),
                entity_id: EntityId::parse_canonical(required_json_str(item, "entity_id")?)?,
                entity_version_id: EntityVersionId::parse_canonical(required_json_str(
                    item,
                    "entity_version_id",
                )?)?,
                state_digest: Digest::from_hex(required_json_str(item, "state_digest")?)?,
            })
        })
        .collect()
}

fn parse_evidence_results(
    value: &serde_json::Value,
) -> Result<Vec<CognitionCaptureEvidenceResult>> {
    required_json_array(value, "evidence")?
        .iter()
        .map(|item| {
            Ok(CognitionCaptureEvidenceResult {
                local_id: required_json_str(item, "local_id")?.to_owned(),
                evidence_id: EvidenceId::parse_canonical(required_json_str(item, "evidence_id")?)?,
            })
        })
        .collect()
}

fn parse_relation_results(
    value: &serde_json::Value,
) -> Result<Vec<CognitionCaptureRelationResult>> {
    required_json_array(value, "relations")?
        .iter()
        .map(|item| {
            Ok(CognitionCaptureRelationResult {
                local_id: required_json_str(item, "local_id")?.to_owned(),
                relation_type: required_json_str(item, "type")?.to_owned(),
                relation_id: RelationId::parse_canonical(required_json_str(item, "relation_id")?)?,
                relation_version_id: RelationVersionId::parse_canonical(required_json_str(
                    item,
                    "relation_version_id",
                )?)?,
                source_entity_id: EntityId::parse_canonical(required_json_str(
                    item,
                    "source_entity_id",
                )?)?,
                target_entity_id: EntityId::parse_canonical(required_json_str(
                    item,
                    "target_entity_id",
                )?)?,
            })
        })
        .collect()
}

fn required_json_str<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            WorkVcsError::RecordInvalid(format!("capture payload missing string {field}"))
        })
}

fn required_json_array<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a [serde_json::Value]> {
    value
        .get(field)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| {
            WorkVcsError::RecordInvalid(format!("capture payload missing array {field}"))
        })
}

fn validate_unique_local_ids<'a>(
    values: impl IntoIterator<Item = (&'a str, &'a String)>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for (kind, value) in values {
        validate_text(&format!("{kind} local_id"), value)?;
        if !seen.insert(value.clone()) {
            return Err(WorkVcsError::RecordInvalid(format!(
                "duplicate local_id {value:?} in cognition capture manifest"
            )));
        }
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(WorkVcsError::RecordInvalid(format!(
            "{label} must be non-empty text without surrounding whitespace or control characters"
        )));
    }
    Ok(())
}

fn require_object(label: &str, value: &CanonicalValue) -> Result<()> {
    if matches!(value, CanonicalValue::Object(_)) {
        Ok(())
    } else {
        Err(WorkVcsError::RecordInvalid(format!(
            "{label} must be an object"
        )))
    }
}

fn object(values: Vec<(&str, CanonicalValue)>) -> Result<CanonicalValue> {
    CanonicalValue::object(
        values
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
    .map_err(capture_invalid_from)
}

fn string(value: &str) -> CanonicalValue {
    CanonicalValue::String(value.to_owned())
}

fn canonical_json_string(value: &CanonicalValue) -> Result<String> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| {
        WorkVcsError::CanonicalEncodingInvalid(format!("canonical JSON is not UTF-8: {error}"))
    })
}

fn empty_object() -> CanonicalValue {
    CanonicalValue::object(Vec::new()).expect("empty canonical object")
}

fn capture_invalid_from(error: impl std::fmt::Display) -> WorkVcsError {
    WorkVcsError::RecordInvalid(error.to_string())
}
