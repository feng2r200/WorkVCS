-- WorkVCS SQLite schema v0.1
-- Application ID: 0x57564353 ("WVCS")
-- Boundary: Store physical schema plus correctness indexes only.

PRAGMA application_id = 1465271123;

BEGIN;

CREATE TABLE store (
    store_id BLOB NOT NULL PRIMARY KEY CHECK(length(store_id) = 16),
    created_at_us INTEGER NOT NULL,
    display_name TEXT NOT NULL CHECK(length(display_name) > 0),
    metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json))
) STRICT;

CREATE TABLE store_manifest (
    store_id BLOB NOT NULL PRIMARY KEY CHECK(length(store_id) = 16)
        REFERENCES store(store_id) ON DELETE RESTRICT,
    store_format_version INTEGER NOT NULL CHECK(store_format_version > 0),
    schema_version INTEGER NOT NULL CHECK(schema_version = 1),
    object_store_format_version INTEGER NOT NULL CHECK(object_store_format_version > 0),
    id_scheme TEXT NOT NULL CHECK(id_scheme = 'uuidv7-blob16'),
    digest_algorithm TEXT NOT NULL CHECK(digest_algorithm = 'blake3-256'),
    canonical_json_profile TEXT NOT NULL CHECK(length(canonical_json_profile) > 0),
    created_at_us INTEGER NOT NULL,
    manifest_json TEXT NOT NULL CHECK(json_valid(manifest_json))
) STRICT;

CREATE TABLE workspace (
    workspace_id BLOB NOT NULL PRIMARY KEY CHECK(length(workspace_id) = 16),
    store_id BLOB NOT NULL CHECK(length(store_id) = 16)
        REFERENCES store(store_id) ON DELETE RESTRICT,
    display_name TEXT NOT NULL CHECK(length(display_name) > 0),
    genesis_commit_id BLOB NOT NULL CHECK(length(genesis_commit_id) = 16)
        REFERENCES workstate_commit(commit_id)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE object_identity (
    object_id BLOB NOT NULL PRIMARY KEY CHECK(length(object_id) = 16),
    object_kind TEXT NOT NULL CHECK(object_kind IN (
        'entity',
        'relation',
        'session',
        'session_diff',
        'claim',
        'merge_attempt',
        'evidence',
        'resource',
        'resource_observation',
        'knowledge_space',
        'knowledge_exposure'
    )),
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE entity (
    object_id BLOB NOT NULL PRIMARY KEY CHECK(length(object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    entity_kind TEXT NOT NULL CHECK(length(entity_kind) > 0)
) STRICT;

CREATE TABLE entity_version (
    entity_version_id BLOB NOT NULL PRIMARY KEY CHECK(length(entity_version_id) = 16),
    entity_id BLOB NOT NULL CHECK(length(entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    state_schema_version INTEGER NOT NULL CHECK(state_schema_version > 0),
    state_json TEXT NOT NULL CHECK(json_valid(state_json)),
    state_digest BLOB NOT NULL CHECK(length(state_digest) = 32),
    UNIQUE(entity_id, entity_version_id)
) STRICT;

CREATE TABLE record_identity (
    entity_id BLOB NOT NULL PRIMARY KEY CHECK(length(entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    record_kind TEXT NOT NULL CHECK(length(record_kind) > 0)
) STRICT;

CREATE TABLE acceptance_criterion_identity (
    entity_id BLOB NOT NULL PRIMARY KEY CHECK(length(entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    owner_entity_id BLOB NOT NULL CHECK(length(owner_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    local_key TEXT NOT NULL CHECK(length(local_key) > 0),
    UNIQUE(owner_entity_id, local_key)
) STRICT;

CREATE TABLE verification_requirement_identity (
    entity_id BLOB NOT NULL PRIMARY KEY CHECK(length(entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    owner_entity_id BLOB NOT NULL CHECK(length(owner_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    local_key TEXT NOT NULL CHECK(length(local_key) > 0),
    UNIQUE(owner_entity_id, local_key)
) STRICT;

CREATE TABLE relation (
    object_id BLOB NOT NULL PRIMARY KEY CHECK(length(object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    relation_type TEXT NOT NULL CHECK(length(relation_type) > 0),
    source_object_id BLOB NOT NULL CHECK(length(source_object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    target_object_id BLOB NOT NULL CHECK(length(target_object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    relation_discriminator TEXT NOT NULL DEFAULT '',
    UNIQUE(workspace_id, relation_type, source_object_id, target_object_id, relation_discriminator)
) STRICT;

CREATE TABLE relation_version (
    relation_version_id BLOB NOT NULL PRIMARY KEY CHECK(length(relation_version_id) = 16),
    relation_id BLOB NOT NULL CHECK(length(relation_id) = 16)
        REFERENCES relation(object_id) ON DELETE RESTRICT,
    state_schema_version INTEGER NOT NULL CHECK(state_schema_version > 0),
    metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json)),
    state_digest BLOB NOT NULL CHECK(length(state_digest) = 32),
    UNIQUE(relation_id, relation_version_id)
) STRICT;

CREATE TABLE branch (
    branch_id BLOB NOT NULL PRIMARY KEY CHECK(length(branch_id) = 16),
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK(length(name) > 0),
    head_commit_id BLOB NOT NULL CHECK(length(head_commit_id) = 16),
    lifecycle_state TEXT NOT NULL CHECK(length(lifecycle_state) > 0),
    created_at_us INTEGER NOT NULL,
    UNIQUE(workspace_id, name),
    UNIQUE(workspace_id, branch_id),
    FOREIGN KEY(workspace_id, head_commit_id)
        REFERENCES workstate_commit(workspace_id, commit_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE changeset (
    changeset_id BLOB NOT NULL PRIMARY KEY CHECK(length(changeset_id) = 16),
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    operation_type TEXT NOT NULL CHECK(length(operation_type) > 0),
    operation_schema_version INTEGER NOT NULL CHECK(operation_schema_version > 0),
    operation_payload_json TEXT NOT NULL CHECK(json_valid(operation_payload_json)),
    rationale_json TEXT NOT NULL CHECK(json_valid(rationale_json)),
    origin_session_id BLOB CHECK(origin_session_id IS NULL OR length(origin_session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE changeset_causal_anchor (
    changeset_id BLOB NOT NULL CHECK(length(changeset_id) = 16)
        REFERENCES changeset(changeset_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    anchor_object_id BLOB NOT NULL CHECK(length(anchor_object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    PRIMARY KEY(changeset_id, ordinal),
    UNIQUE(changeset_id, anchor_object_id)
) STRICT;

CREATE TABLE change_operation (
    operation_id BLOB NOT NULL PRIMARY KEY CHECK(length(operation_id) = 16),
    changeset_id BLOB NOT NULL CHECK(length(changeset_id) = 16)
        REFERENCES changeset(changeset_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    subject_family TEXT NOT NULL CHECK(subject_family IN ('entity', 'relation')),
    subject_object_id BLOB NOT NULL CHECK(length(subject_object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    operation_payload_json TEXT NOT NULL CHECK(json_valid(operation_payload_json)),
    UNIQUE(changeset_id, ordinal),
    UNIQUE(changeset_id, subject_family, subject_object_id),
    UNIQUE(operation_id, subject_object_id)
) STRICT;

CREATE TABLE entity_membership_change (
    operation_id BLOB NOT NULL PRIMARY KEY CHECK(length(operation_id) = 16),
    entity_id BLOB NOT NULL CHECK(length(entity_id) = 16),
    before_entity_version_id BLOB CHECK(before_entity_version_id IS NULL OR length(before_entity_version_id) = 16),
    after_entity_version_id BLOB CHECK(after_entity_version_id IS NULL OR length(after_entity_version_id) = 16),
    field_delta_json TEXT NOT NULL CHECK(json_valid(field_delta_json)),
    CHECK(before_entity_version_id IS NOT after_entity_version_id),
    FOREIGN KEY(operation_id, entity_id)
        REFERENCES change_operation(operation_id, subject_object_id) ON DELETE RESTRICT,
    FOREIGN KEY(entity_id, before_entity_version_id)
        REFERENCES entity_version(entity_id, entity_version_id) ON DELETE RESTRICT,
    FOREIGN KEY(entity_id, after_entity_version_id)
        REFERENCES entity_version(entity_id, entity_version_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE relation_membership_change (
    operation_id BLOB NOT NULL PRIMARY KEY CHECK(length(operation_id) = 16),
    relation_id BLOB NOT NULL CHECK(length(relation_id) = 16),
    before_relation_version_id BLOB CHECK(before_relation_version_id IS NULL OR length(before_relation_version_id) = 16),
    after_relation_version_id BLOB CHECK(after_relation_version_id IS NULL OR length(after_relation_version_id) = 16),
    field_delta_json TEXT NOT NULL CHECK(json_valid(field_delta_json)),
    CHECK(before_relation_version_id IS NOT after_relation_version_id),
    FOREIGN KEY(operation_id, relation_id)
        REFERENCES change_operation(operation_id, subject_object_id) ON DELETE RESTRICT,
    FOREIGN KEY(relation_id, before_relation_version_id)
        REFERENCES relation_version(relation_id, relation_version_id) ON DELETE RESTRICT,
    FOREIGN KEY(relation_id, after_relation_version_id)
        REFERENCES relation_version(relation_id, relation_version_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE workstate_commit (
    commit_id BLOB NOT NULL PRIMARY KEY CHECK(length(commit_id) = 16),
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    changeset_id BLOB NOT NULL UNIQUE CHECK(length(changeset_id) = 16)
        REFERENCES changeset(changeset_id) ON DELETE RESTRICT,
    commit_kind TEXT NOT NULL CHECK(commit_kind IN ('genesis', 'normal', 'merge')),
    state_digest BLOB NOT NULL CHECK(length(state_digest) = 32),
    committed_at_us INTEGER NOT NULL,
    UNIQUE(workspace_id, commit_id)
) STRICT;

CREATE TABLE commit_parent (
    commit_id BLOB NOT NULL CHECK(length(commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    parent_ordinal INTEGER NOT NULL CHECK(parent_ordinal >= 0),
    parent_role TEXT NOT NULL CHECK(parent_role IN ('primary', 'secondary')),
    parent_commit_id BLOB NOT NULL CHECK(length(parent_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    PRIMARY KEY(commit_id, parent_ordinal),
    UNIQUE(commit_id, parent_role),
    CHECK(parent_commit_id <> commit_id)
) STRICT;

CREATE TABLE event (
    event_id BLOB NOT NULL PRIMARY KEY CHECK(length(event_id) = 16),
    workspace_id BLOB CHECK(workspace_id IS NULL OR length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    changeset_id BLOB CHECK(changeset_id IS NULL OR length(changeset_id) = 16)
        REFERENCES changeset(changeset_id) ON DELETE RESTRICT,
    session_id BLOB CHECK(session_id IS NULL OR length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    event_kind TEXT NOT NULL CHECK(length(event_kind) > 0),
    occurred_at_us INTEGER NOT NULL,
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json))
) STRICT;

CREATE TABLE session (
    session_id BLOB NOT NULL PRIMARY KEY CHECK(length(session_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    started_at_us INTEGER NOT NULL,
    metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json))
) STRICT;

CREATE TABLE session_runtime (
    session_id BLOB NOT NULL PRIMARY KEY CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    active_workspace_id BLOB CHECK(active_workspace_id IS NULL OR length(active_workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    active_branch_id BLOB CHECK(active_branch_id IS NULL OR length(active_branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE RESTRICT,
    last_activity_at_us INTEGER NOT NULL,
    runtime_json TEXT NOT NULL CHECK(json_valid(runtime_json))
) STRICT;

CREATE TABLE session_context_workspace (
    session_id BLOB NOT NULL CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    PRIMARY KEY(session_id, workspace_id)
) STRICT;

CREATE TABLE session_context_knowledge_space (
    session_id BLOB NOT NULL CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    knowledge_space_id BLOB NOT NULL CHECK(length(knowledge_space_id) = 16)
        REFERENCES knowledge_space(knowledge_space_id) ON DELETE RESTRICT,
    PRIMARY KEY(session_id, knowledge_space_id)
) STRICT;

CREATE TABLE session_focus (
    session_id BLOB NOT NULL PRIMARY KEY CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    focus_entity_id BLOB NOT NULL CHECK(length(focus_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE session_focus_path (
    session_id BLOB NOT NULL CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    path_entity_id BLOB NOT NULL CHECK(length(path_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    incoming_relation_id BLOB CHECK(incoming_relation_id IS NULL OR length(incoming_relation_id) = 16)
        REFERENCES relation(object_id) ON DELETE RESTRICT,
    PRIMARY KEY(session_id, ordinal)
) STRICT;

CREATE TABLE session_diff (
    session_diff_id BLOB NOT NULL PRIMARY KEY CHECK(length(session_diff_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    session_id BLOB NOT NULL UNIQUE CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    created_at_us INTEGER NOT NULL,
    summary_json TEXT NOT NULL CHECK(json_valid(summary_json)),
    detail_content_digest BLOB CHECK(detail_content_digest IS NULL OR length(detail_content_digest) = 32)
        REFERENCES content_object(content_digest) ON DELETE RESTRICT
) STRICT;

CREATE TABLE context_packet_snapshot (
    context_packet_id BLOB NOT NULL PRIMARY KEY CHECK(length(context_packet_id) = 16),
    session_id BLOB NOT NULL CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    branch_id BLOB NOT NULL CHECK(length(branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE RESTRICT,
    head_commit_id BLOB NOT NULL CHECK(length(head_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    state_digest BLOB NOT NULL CHECK(length(state_digest) = 32),
    profile TEXT NOT NULL CHECK(profile IN ('brief', 'normal', 'full')),
    budget_items INTEGER CHECK(budget_items IS NULL OR budget_items > 0),
    scope_json TEXT CHECK(scope_json IS NULL OR json_valid(scope_json)),
    available_items INTEGER NOT NULL CHECK(available_items >= 0),
    item_count INTEGER NOT NULL CHECK(item_count >= 0),
    omitted_items INTEGER NOT NULL CHECK(omitted_items >= 0),
    packet_digest BLOB NOT NULL CHECK(length(packet_digest) = 32),
    packet_json TEXT NOT NULL CHECK(json_valid(packet_json)),
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE claim (
    claim_id BLOB NOT NULL PRIMARY KEY CHECK(length(claim_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    session_id BLOB NOT NULL CHECK(length(session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    branch_id BLOB NOT NULL CHECK(length(branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE RESTRICT,
    task_entity_id BLOB NOT NULL CHECK(length(task_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    mode TEXT NOT NULL CHECK(mode IN ('exclusive', 'shared')),
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE claim_runtime (
    claim_id BLOB NOT NULL PRIMARY KEY CHECK(length(claim_id) = 16)
        REFERENCES claim(claim_id) ON DELETE RESTRICT,
    last_activity_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE merge_attempt (
    merge_id BLOB NOT NULL PRIMARY KEY CHECK(length(merge_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    target_branch_id BLOB NOT NULL CHECK(length(target_branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE RESTRICT,
    source_branch_id BLOB NOT NULL CHECK(length(source_branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE RESTRICT,
    merge_base_commit_id BLOB NOT NULL CHECK(length(merge_base_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    target_head_commit_id BLOB NOT NULL CHECK(length(target_head_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    source_head_commit_id BLOB NOT NULL CHECK(length(source_head_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    origin_session_id BLOB CHECK(origin_session_id IS NULL OR length(origin_session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    created_at_us INTEGER NOT NULL,
    CHECK(target_branch_id <> source_branch_id)
) STRICT;

CREATE TABLE merge_runtime (
    merge_id BLOB NOT NULL PRIMARY KEY CHECK(length(merge_id) = 16)
        REFERENCES merge_attempt(merge_id) ON DELETE RESTRICT,
    runtime_json TEXT NOT NULL CHECK(json_valid(runtime_json))
) STRICT;

CREATE TABLE merge_item (
    merge_item_id BLOB NOT NULL PRIMARY KEY CHECK(length(merge_item_id) = 16),
    merge_id BLOB NOT NULL CHECK(length(merge_id) = 16)
        REFERENCES merge_attempt(merge_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    classification TEXT NOT NULL CHECK(classification IN ('AUTO', 'CONFLICT', 'REVIEW')),
    subject_object_id BLOB CHECK(subject_object_id IS NULL OR length(subject_object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    item_payload_json TEXT NOT NULL CHECK(json_valid(item_payload_json)),
    UNIQUE(merge_id, ordinal)
) STRICT;

CREATE TABLE merge_resolution_runtime (
    merge_item_id BLOB NOT NULL PRIMARY KEY CHECK(length(merge_item_id) = 16)
        REFERENCES merge_item(merge_item_id) ON DELETE RESTRICT,
    resolution_kind TEXT NOT NULL CHECK(resolution_kind IN ('ours', 'theirs', 'custom')),
    custom_payload_json TEXT CHECK(custom_payload_json IS NULL OR json_valid(custom_payload_json)),
    rationale_json TEXT NOT NULL CHECK(json_valid(rationale_json)),
    resolved_by_session_id BLOB CHECK(resolved_by_session_id IS NULL OR length(resolved_by_session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    resolved_at_us INTEGER NOT NULL,
    CHECK(
        (resolution_kind = 'custom' AND custom_payload_json IS NOT NULL)
        OR (resolution_kind IN ('ours', 'theirs') AND custom_payload_json IS NULL)
    )
) STRICT;

CREATE TABLE merge_resolution (
    merge_item_id BLOB NOT NULL PRIMARY KEY CHECK(length(merge_item_id) = 16)
        REFERENCES merge_item(merge_item_id) ON DELETE RESTRICT,
    resolution_kind TEXT NOT NULL CHECK(resolution_kind IN ('ours', 'theirs', 'custom')),
    custom_payload_json TEXT CHECK(custom_payload_json IS NULL OR json_valid(custom_payload_json)),
    rationale_json TEXT NOT NULL CHECK(json_valid(rationale_json)),
    resolved_by_session_id BLOB CHECK(resolved_by_session_id IS NULL OR length(resolved_by_session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    resolved_at_us INTEGER NOT NULL,
    CHECK(
        (resolution_kind = 'custom' AND custom_payload_json IS NOT NULL)
        OR (resolution_kind IN ('ours', 'theirs') AND custom_payload_json IS NULL)
    )
) STRICT;

CREATE TABLE merge_attempt_outcome (
    merge_id BLOB NOT NULL PRIMARY KEY CHECK(length(merge_id) = 16)
        REFERENCES merge_attempt(merge_id) ON DELETE RESTRICT,
    outcome TEXT NOT NULL CHECK(outcome IN ('completed', 'aborted')),
    result_commit_id BLOB CHECK(result_commit_id IS NULL OR length(result_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    completed_at_us INTEGER NOT NULL,
    detail_json TEXT NOT NULL CHECK(json_valid(detail_json)),
    CHECK(
        (outcome = 'completed' AND result_commit_id IS NOT NULL)
        OR (outcome = 'aborted' AND result_commit_id IS NULL)
    )
) STRICT;

CREATE TABLE evidence (
    evidence_id BLOB NOT NULL PRIMARY KEY CHECK(length(evidence_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    evidence_kind TEXT NOT NULL CHECK(length(evidence_kind) > 0),
    captured_at_us INTEGER NOT NULL,
    source_session_id BLOB CHECK(source_session_id IS NULL OR length(source_session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json))
) STRICT;

CREATE TABLE content_object (
    content_digest BLOB NOT NULL PRIMARY KEY CHECK(length(content_digest) = 32),
    size_bytes INTEGER NOT NULL CHECK(size_bytes >= 0),
    media_type TEXT,
    format_metadata_json TEXT NOT NULL CHECK(json_valid(format_metadata_json))
) STRICT;

CREATE TABLE evidence_content (
    evidence_id BLOB NOT NULL CHECK(length(evidence_id) = 16)
        REFERENCES evidence(evidence_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    content_digest BLOB NOT NULL CHECK(length(content_digest) = 32)
        REFERENCES content_object(content_digest) ON DELETE RESTRICT,
    role TEXT NOT NULL CHECK(length(role) > 0),
    PRIMARY KEY(evidence_id, ordinal)
) STRICT;

CREATE TABLE content_storage_location (
    content_digest BLOB NOT NULL CHECK(length(content_digest) = 32)
        REFERENCES content_object(content_digest) ON DELETE RESTRICT,
    storage_backend TEXT NOT NULL CHECK(length(storage_backend) > 0),
    locator TEXT NOT NULL CHECK(length(locator) > 0),
    availability_state TEXT NOT NULL CHECK(length(availability_state) > 0),
    observed_at_us INTEGER NOT NULL,
    metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json)),
    PRIMARY KEY(content_digest, storage_backend, locator)
) STRICT;

CREATE TABLE resource (
    resource_id BLOB NOT NULL PRIMARY KEY CHECK(length(resource_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    resource_kind TEXT NOT NULL CHECK(length(resource_kind) > 0),
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE resource_binding (
    resource_id BLOB NOT NULL PRIMARY KEY CHECK(length(resource_id) = 16)
        REFERENCES resource(resource_id) ON DELETE RESTRICT,
    adapter_kind TEXT NOT NULL CHECK(length(adapter_kind) > 0),
    locator TEXT NOT NULL CHECK(length(locator) > 0),
    binding_config_json TEXT NOT NULL CHECK(json_valid(binding_config_json)),
    bound_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE workspace_resource (
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    resource_id BLOB NOT NULL CHECK(length(resource_id) = 16)
        REFERENCES resource(resource_id) ON DELETE RESTRICT,
    association_metadata_json TEXT NOT NULL CHECK(json_valid(association_metadata_json)),
    PRIMARY KEY(workspace_id, resource_id)
) STRICT;

CREATE TABLE resource_observation (
    observation_id BLOB NOT NULL PRIMARY KEY CHECK(length(observation_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    resource_id BLOB NOT NULL CHECK(length(resource_id) = 16)
        REFERENCES resource(resource_id) ON DELETE RESTRICT,
    adapter_kind TEXT NOT NULL CHECK(length(adapter_kind) > 0),
    adapter_schema_version INTEGER NOT NULL CHECK(adapter_schema_version > 0),
    captured_at_us INTEGER NOT NULL,
    fingerprint BLOB NOT NULL CHECK(length(fingerprint) = 32),
    summary_json TEXT NOT NULL CHECK(json_valid(summary_json)),
    detail_content_digest BLOB CHECK(detail_content_digest IS NULL OR length(detail_content_digest) = 32)
        REFERENCES content_object(content_digest) ON DELETE RESTRICT,
    source_session_id BLOB CHECK(source_session_id IS NULL OR length(source_session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE verification_basis (
    verification_entity_id BLOB NOT NULL PRIMARY KEY CHECK(length(verification_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    verified_at_commit_id BLOB NOT NULL CHECK(length(verified_at_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    basis_schema_version INTEGER NOT NULL CHECK(basis_schema_version > 0),
    basis_json TEXT NOT NULL CHECK(json_valid(basis_json))
) STRICT;

CREATE TABLE verification_resource_basis (
    verification_entity_id BLOB NOT NULL CHECK(length(verification_entity_id) = 16)
        REFERENCES verification_basis(verification_entity_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    resource_id BLOB NOT NULL CHECK(length(resource_id) = 16)
        REFERENCES resource(resource_id) ON DELETE RESTRICT,
    adapter_kind TEXT NOT NULL CHECK(length(adapter_kind) > 0),
    adapter_schema_version INTEGER NOT NULL CHECK(adapter_schema_version > 0),
    scope_kind TEXT NOT NULL CHECK(length(scope_kind) > 0),
    scope_schema_version INTEGER NOT NULL CHECK(scope_schema_version > 0),
    scope_payload_json TEXT NOT NULL CHECK(json_valid(scope_payload_json)),
    baseline_observation_id BLOB CHECK(baseline_observation_id IS NULL OR length(baseline_observation_id) = 16)
        REFERENCES resource_observation(observation_id) ON DELETE RESTRICT,
    baseline_fingerprint BLOB NOT NULL CHECK(length(baseline_fingerprint) = 32),
    PRIMARY KEY(verification_entity_id, ordinal)
) STRICT;

CREATE TABLE verification_artifact_basis (
    verification_entity_id BLOB NOT NULL CHECK(length(verification_entity_id) = 16)
        REFERENCES verification_basis(verification_entity_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    artifact_role TEXT NOT NULL CHECK(length(artifact_role) > 0),
    evidence_id BLOB CHECK(evidence_id IS NULL OR length(evidence_id) = 16)
        REFERENCES evidence(evidence_id) ON DELETE RESTRICT,
    content_digest BLOB CHECK(content_digest IS NULL OR length(content_digest) = 32)
        REFERENCES content_object(content_digest) ON DELETE RESTRICT,
    descriptor_json TEXT NOT NULL CHECK(json_valid(descriptor_json)),
    PRIMARY KEY(verification_entity_id, ordinal),
    CHECK(evidence_id IS NOT NULL OR content_digest IS NOT NULL)
) STRICT;

CREATE TABLE verification_semantic_dependency (
    verification_entity_id BLOB NOT NULL CHECK(length(verification_entity_id) = 16)
        REFERENCES verification_basis(verification_entity_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    dependency_entity_id BLOB NOT NULL CHECK(length(dependency_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    expected_entity_version_id BLOB NOT NULL CHECK(length(expected_entity_version_id) = 16),
    PRIMARY KEY(verification_entity_id, ordinal),
    FOREIGN KEY(dependency_entity_id, expected_entity_version_id)
        REFERENCES entity_version(entity_id, entity_version_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE verification_applicability_cache (
    branch_id BLOB NOT NULL CHECK(length(branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE CASCADE,
    verification_entity_id BLOB NOT NULL CHECK(length(verification_entity_id) = 16)
        REFERENCES verification_basis(verification_entity_id) ON DELETE CASCADE,
    evaluated_commit_id BLOB NOT NULL CHECK(length(evaluated_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    applicability TEXT NOT NULL CHECK(applicability IN ('applicable', 'stale', 'unknown')),
    reason_code TEXT NOT NULL CHECK(length(reason_code) > 0),
    detail_json TEXT NOT NULL CHECK(json_valid(detail_json)),
    evaluated_at_us INTEGER NOT NULL,
    PRIMARY KEY(branch_id, verification_entity_id)
) STRICT;

CREATE TABLE applicability_resource_stamp (
    branch_id BLOB NOT NULL CHECK(length(branch_id) = 16),
    verification_entity_id BLOB NOT NULL CHECK(length(verification_entity_id) = 16),
    resource_basis_ordinal INTEGER NOT NULL CHECK(resource_basis_ordinal >= 0),
    adapter_kind TEXT NOT NULL CHECK(length(adapter_kind) > 0),
    adapter_schema_version INTEGER NOT NULL CHECK(adapter_schema_version > 0),
    scope_schema_version INTEGER NOT NULL CHECK(scope_schema_version > 0),
    observation_status TEXT NOT NULL CHECK(observation_status IN ('observed', 'unavailable', 'error')),
    observed_fingerprint BLOB CHECK(observed_fingerprint IS NULL OR length(observed_fingerprint) = 32),
    observation_id BLOB CHECK(observation_id IS NULL OR length(observation_id) = 16)
        REFERENCES resource_observation(observation_id) ON DELETE RESTRICT,
    observed_at_us INTEGER NOT NULL,
    PRIMARY KEY(branch_id, verification_entity_id, resource_basis_ordinal),
    FOREIGN KEY(branch_id, verification_entity_id)
        REFERENCES verification_applicability_cache(branch_id, verification_entity_id) ON DELETE CASCADE,
    FOREIGN KEY(verification_entity_id, resource_basis_ordinal)
        REFERENCES verification_resource_basis(verification_entity_id, ordinal) ON DELETE CASCADE,
    CHECK(
        (observation_status = 'observed' AND observed_fingerprint IS NOT NULL)
        OR (observation_status IN ('unavailable', 'error')
            AND observed_fingerprint IS NULL
            AND observation_id IS NULL)
    )
) STRICT;

CREATE TABLE knowledge_space (
    knowledge_space_id BLOB NOT NULL PRIMARY KEY CHECK(length(knowledge_space_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    name TEXT NOT NULL CHECK(length(name) > 0),
    created_at_us INTEGER NOT NULL,
    UNIQUE(name)
) STRICT;

CREATE TABLE external_object_ref (
    external_ref_id BLOB NOT NULL PRIMARY KEY CHECK(length(external_ref_id) = 16),
    external_store_id BLOB NOT NULL CHECK(length(external_store_id) = 16),
    external_object_id BLOB NOT NULL CHECK(length(external_object_id) = 16),
    object_kind TEXT NOT NULL CHECK(length(object_kind) > 0),
    reference_scope TEXT NOT NULL CHECK(reference_scope IN ('object', 'version')),
    external_version_ref BLOB CHECK(external_version_ref IS NULL OR length(external_version_ref) = 16),
    descriptor_json TEXT NOT NULL CHECK(json_valid(descriptor_json)),
    created_at_us INTEGER NOT NULL,
    CHECK(
        (reference_scope = 'object' AND external_version_ref IS NULL)
        OR (reference_scope = 'version' AND external_version_ref IS NOT NULL)
    )
) STRICT;

CREATE TABLE knowledge_exposure (
    exposure_id BLOB NOT NULL PRIMARY KEY CHECK(length(exposure_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    knowledge_space_id BLOB NOT NULL CHECK(length(knowledge_space_id) = 16)
        REFERENCES knowledge_space(knowledge_space_id) ON DELETE RESTRICT,
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE knowledge_exposure_local_source (
    exposure_id BLOB NOT NULL PRIMARY KEY CHECK(length(exposure_id) = 16)
        REFERENCES knowledge_exposure(exposure_id) ON DELETE RESTRICT,
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    knowledge_entity_id BLOB NOT NULL CHECK(length(knowledge_entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    knowledge_entity_version_id BLOB NOT NULL CHECK(length(knowledge_entity_version_id) = 16),
    FOREIGN KEY(knowledge_entity_id, knowledge_entity_version_id)
        REFERENCES entity_version(entity_id, entity_version_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE knowledge_exposure_external_source (
    exposure_id BLOB NOT NULL PRIMARY KEY CHECK(length(exposure_id) = 16)
        REFERENCES knowledge_exposure(exposure_id) ON DELETE RESTRICT,
    external_ref_id BLOB NOT NULL CHECK(length(external_ref_id) = 16)
        REFERENCES external_object_ref(external_ref_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE knowledge_exposure_transition (
    transition_id BLOB NOT NULL PRIMARY KEY CHECK(length(transition_id) = 16),
    exposure_id BLOB NOT NULL CHECK(length(exposure_id) = 16)
        REFERENCES knowledge_exposure(exposure_id) ON DELETE RESTRICT,
    previous_transition_id BLOB UNIQUE CHECK(previous_transition_id IS NULL OR length(previous_transition_id) = 16)
        REFERENCES knowledge_exposure_transition(transition_id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK(lifecycle_status IN ('active', 'withdrawn')),
    changed_at_us INTEGER NOT NULL,
    event_id BLOB CHECK(event_id IS NULL OR length(event_id) = 16)
        REFERENCES event(event_id) ON DELETE RESTRICT,
    detail_json TEXT NOT NULL CHECK(json_valid(detail_json)),
    CHECK(previous_transition_id IS NULL OR previous_transition_id <> transition_id)
) STRICT;

CREATE TABLE knowledge_exposure_current (
    exposure_id BLOB NOT NULL PRIMARY KEY CHECK(length(exposure_id) = 16)
        REFERENCES knowledge_exposure(exposure_id) ON DELETE CASCADE,
    transition_id BLOB NOT NULL CHECK(length(transition_id) = 16)
        REFERENCES knowledge_exposure_transition(transition_id) ON DELETE RESTRICT,
    lifecycle_status TEXT NOT NULL CHECK(lifecycle_status IN ('active', 'withdrawn')),
    updated_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE knowledge_exposure_source_status (
    exposure_id BLOB NOT NULL PRIMARY KEY CHECK(length(exposure_id) = 16)
        REFERENCES knowledge_exposure(exposure_id) ON DELETE CASCADE,
    source_status TEXT NOT NULL CHECK(source_status IN ('current', 'stale', 'unknown', 'unresolved')),
    checked_at_us INTEGER NOT NULL,
    detail_json TEXT NOT NULL CHECK(json_valid(detail_json))
) STRICT;

CREATE TABLE branch_projection_state (
    branch_id BLOB NOT NULL PRIMARY KEY CHECK(length(branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE CASCADE,
    projection_status TEXT NOT NULL CHECK(projection_status IN ('complete', 'not_materialized', 'invalid')),
    projected_commit_id BLOB CHECK(projected_commit_id IS NULL OR length(projected_commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    projection_state_digest BLOB CHECK(projection_state_digest IS NULL OR length(projection_state_digest) = 32),
    updated_at_us INTEGER NOT NULL,
    CHECK(
        (projected_commit_id IS NULL AND projection_state_digest IS NULL)
        OR (projected_commit_id IS NOT NULL AND projection_state_digest IS NOT NULL)
    ),
    CHECK(
        (projection_status = 'complete'
            AND projected_commit_id IS NOT NULL
            AND projection_state_digest IS NOT NULL)
        OR (projection_status = 'not_materialized'
            AND projected_commit_id IS NULL
            AND projection_state_digest IS NULL)
        OR projection_status = 'invalid'
    )
) STRICT;

CREATE TABLE branch_entity_current (
    branch_id BLOB NOT NULL CHECK(length(branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE CASCADE,
    entity_id BLOB NOT NULL CHECK(length(entity_id) = 16)
        REFERENCES entity(object_id) ON DELETE RESTRICT,
    entity_version_id BLOB NOT NULL CHECK(length(entity_version_id) = 16),
    PRIMARY KEY(branch_id, entity_id),
    FOREIGN KEY(entity_id, entity_version_id)
        REFERENCES entity_version(entity_id, entity_version_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE branch_relation_current (
    branch_id BLOB NOT NULL CHECK(length(branch_id) = 16)
        REFERENCES branch(branch_id) ON DELETE CASCADE,
    relation_id BLOB NOT NULL CHECK(length(relation_id) = 16)
        REFERENCES relation(object_id) ON DELETE RESTRICT,
    relation_version_id BLOB NOT NULL CHECK(length(relation_version_id) = 16),
    relation_type TEXT NOT NULL CHECK(length(relation_type) > 0),
    source_object_id BLOB NOT NULL CHECK(length(source_object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    target_object_id BLOB NOT NULL CHECK(length(target_object_id) = 16)
        REFERENCES object_identity(object_id) ON DELETE RESTRICT,
    relation_discriminator TEXT NOT NULL,
    PRIMARY KEY(branch_id, relation_id),
    FOREIGN KEY(relation_id, relation_version_id)
        REFERENCES relation_version(relation_id, relation_version_id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE checkpoint (
    checkpoint_id BLOB NOT NULL PRIMARY KEY CHECK(length(checkpoint_id) = 16),
    workspace_id BLOB NOT NULL CHECK(length(workspace_id) = 16)
        REFERENCES workspace(workspace_id) ON DELETE RESTRICT,
    commit_id BLOB NOT NULL CHECK(length(commit_id) = 16)
        REFERENCES workstate_commit(commit_id) ON DELETE RESTRICT,
    state_digest BLOB NOT NULL CHECK(length(state_digest) = 32),
    checkpoint_format_version INTEGER NOT NULL CHECK(checkpoint_format_version > 0),
    content_digest BLOB NOT NULL CHECK(length(content_digest) = 32)
        REFERENCES content_object(content_digest) ON DELETE RESTRICT,
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE checkpoint_status (
    checkpoint_id BLOB NOT NULL PRIMARY KEY CHECK(length(checkpoint_id) = 16)
        REFERENCES checkpoint(checkpoint_id) ON DELETE CASCADE,
    usability_state TEXT NOT NULL CHECK(length(usability_state) > 0),
    last_validated_at_us INTEGER NOT NULL,
    detail_json TEXT NOT NULL CHECK(json_valid(detail_json))
) STRICT;

CREATE TABLE import_attempt (
    import_id BLOB NOT NULL PRIMARY KEY CHECK(length(import_id) = 16),
    source_store_id BLOB NOT NULL CHECK(length(source_store_id) = 16),
    bundle_digest BLOB NOT NULL CHECK(length(bundle_digest) = 32),
    import_profile TEXT NOT NULL CHECK(length(import_profile) > 0),
    origin_session_id BLOB CHECK(origin_session_id IS NULL OR length(origin_session_id) = 16)
        REFERENCES session(session_id) ON DELETE RESTRICT,
    started_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE import_attempt_outcome (
    import_id BLOB NOT NULL PRIMARY KEY CHECK(length(import_id) = 16)
        REFERENCES import_attempt(import_id) ON DELETE RESTRICT,
    outcome TEXT NOT NULL CHECK(length(outcome) > 0),
    completed_at_us INTEGER NOT NULL,
    detail_json TEXT NOT NULL CHECK(json_valid(detail_json))
) STRICT;

CREATE TABLE store_migration_attempt (
    migration_id BLOB NOT NULL PRIMARY KEY CHECK(length(migration_id) = 16),
    from_store_format_version INTEGER NOT NULL CHECK(from_store_format_version > 0),
    to_store_format_version INTEGER NOT NULL CHECK(to_store_format_version > 0),
    from_schema_version INTEGER NOT NULL CHECK(from_schema_version > 0),
    to_schema_version INTEGER NOT NULL CHECK(to_schema_version > 0),
    tool_version TEXT NOT NULL CHECK(length(tool_version) > 0),
    started_at_us INTEGER NOT NULL
) STRICT;

CREATE TABLE store_migration_outcome (
    migration_id BLOB NOT NULL PRIMARY KEY CHECK(length(migration_id) = 16)
        REFERENCES store_migration_attempt(migration_id) ON DELETE RESTRICT,
    outcome TEXT NOT NULL CHECK(length(outcome) > 0),
    completed_at_us INTEGER NOT NULL,
    detail_json TEXT NOT NULL CHECK(json_valid(detail_json))
) STRICT;

CREATE TABLE store_lineage (
    lineage_id BLOB NOT NULL PRIMARY KEY CHECK(length(lineage_id) = 16),
    source_store_id BLOB NOT NULL CHECK(length(source_store_id) = 16),
    derivation_kind TEXT NOT NULL CHECK(length(derivation_kind) > 0),
    source_root_descriptor_json TEXT NOT NULL CHECK(json_valid(source_root_descriptor_json)),
    source_bundle_digest BLOB CHECK(source_bundle_digest IS NULL OR length(source_bundle_digest) = 32),
    created_at_us INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX uq_external_object_ref_object
ON external_object_ref(external_store_id, external_object_id)
WHERE reference_scope = 'object';

CREATE UNIQUE INDEX uq_external_object_ref_version
ON external_object_ref(external_store_id, external_object_id, external_version_ref)
WHERE reference_scope = 'version';

CREATE UNIQUE INDEX uq_exposure_initial_transition
ON knowledge_exposure_transition(exposure_id)
WHERE previous_transition_id IS NULL;

CREATE INDEX idx_context_packet_snapshot_session_created
ON context_packet_snapshot(session_id, created_at_us, context_packet_id);

CREATE INDEX idx_context_packet_snapshot_branch_head
ON context_packet_snapshot(branch_id, head_commit_id);

COMMIT;
