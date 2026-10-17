use rusqlite::Connection;
use sha2::{Digest as ShaDigest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;
use workvcs_core::{
    CanonicalValue, ClaimForceTakeoverOptions, ClaimLifecycleState, ClaimListOptions,
    ClaimReleaseOptions, ClaimTaskOptions, ClaimTransferOptions,
    ContinuationAbortOnlyRecoveryOptions, ContinuationAbortOnlyRecoveryPreviewOptions,
    ContinuationCandidateAcceptOptions, ContinuationCandidateAttachOptions,
    ContinuationCreationDispatchOptions, ContinuationCreationResultOptions,
    ContinuationListenerReplaceOptions, ContinuationProjectPolicySetOptions,
    ContinuationRotationAbortOptions, ContinuationRotationBeginOptions,
    ContinuationRotationCommitOptions, ContinuationRotationState, ContinuationScopeBindOptions,
    ContinuationScopeCurrentnessOptions, ContinuationScopeId, Digest, Engine, ErrorCode,
    EventListOptions, RecordCreateOptions, SessionEndOptions, SessionId, SessionMarkStaleOptions,
    SessionStartOptions, SessionSwitchOptions, StoreInitOptions, TaskCreateOptions,
    WorkStateRestoreOptions, WorkspaceInfo, WorkspaceInitOptions, canonical_bytes,
};

const PROJECT_ID: &str = "g-p-continuation-test";
const SOURCE_THREAD: &str = "01a-source-thread";
const TARGET_THREAD: &str = "01a-target-thread";
const DIRECT_RECEIPT_DIGEST: &str =
    "1111111111111111111111111111111111111111111111111111111111111111";
const UNKNOWN_RECEIPT_DIGEST: &str =
    "2222222222222222222222222222222222222222222222222222222222222222";
const DISPOSITION_EVIDENCE_DIGEST: &str =
    "3333333333333333333333333333333333333333333333333333333333333333";
const CONTINUATION_PACKET_DIGEST: &str =
    "5555555555555555555555555555555555555555555555555555555555555555";
const INITIAL_MESSAGE_DIGEST: &str =
    "6666666666666666666666666666666666666666666666666666666666666666";

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("continuation-rotation-test").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn metadata(role: &str, thread_id: &str) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "thread_id".to_owned(),
            CanonicalValue::String(thread_id.to_owned()),
        ),
        (
            "workvcs_session_role".to_owned(),
            CanonicalValue::String(role.to_owned()),
        ),
    ])
    .expect("metadata")
}

fn start_session(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    role: &str,
    thread_id: &str,
) -> SessionId {
    engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options")
                .with_metadata(metadata(role, thread_id))
                .expect("session metadata"),
        )
        .expect("start session")
        .session_id
}

struct Fixture {
    engine: Engine,
    workspace: WorkspaceInfo,
    task_id: workvcs_core::EntityId,
    handoff_id: workvcs_core::EntityId,
    handoff_previous_head_commit_id: workvcs_core::CommitId,
    handoff_commit_id: workvcs_core::CommitId,
    handoff_digest: Digest,
    listener_id: SessionId,
    work_id: SessionId,
    candidate_id: SessionId,
    side_chat_id: SessionId,
    claim_id: workvcs_core::ClaimId,
    project_policy_revision: i64,
    project_policy_receipt_digest: Digest,
    project_policy_updated_at_us: i64,
}

fn project_policy_authority(
    actor_session_id: SessionId,
    expected_revision: i64,
    enabled: bool,
) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "action".to_owned(),
            CanonicalValue::String(if enabled { "enable" } else { "disable" }.to_owned()),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(actor_session_id.to_string()),
        ),
        (
            "expected_revision".to_owned(),
            CanonicalValue::safe_integer(expected_revision).unwrap(),
        ),
        (
            "authorization_envelope_kind".to_owned(),
            CanonicalValue::String("trusted_product_control_plane".to_owned()),
        ),
        (
            "product_actor_id".to_owned(),
            CanonicalValue::String("user:test-owner".to_owned()),
        ),
        (
            "user_authorization_receipt_digest".to_owned(),
            CanonicalValue::String(DIRECT_RECEIPT_DIGEST.to_owned()),
        ),
        (
            "project_membership_receipt_digest".to_owned(),
            CanonicalValue::String(DIRECT_RECEIPT_DIGEST.to_owned()),
        ),
        (
            "authorized_at_us".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
    ])
    .unwrap()
}

fn legacy_self_asserted_project_policy_authority(
    actor_session_id: SessionId,
    expected_revision: i64,
) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "action".to_owned(),
            CanonicalValue::String("disable".to_owned()),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(actor_session_id.to_string()),
        ),
        (
            "expected_revision".to_owned(),
            CanonicalValue::safe_integer(expected_revision).unwrap(),
        ),
        (
            "authorization_kind".to_owned(),
            CanonicalValue::String("user_explicit".to_owned()),
        ),
        (
            "authorization_receipt_digest".to_owned(),
            CanonicalValue::String(DIRECT_RECEIPT_DIGEST.to_owned()),
        ),
        (
            "authorized_at_us".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
    ])
    .unwrap()
}

fn fixture(path: &Path) -> Fixture {
    let (mut engine, workspace) = create_workspace(path);
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Continue this work",
            )
            .expect("task options"),
        )
        .expect("create task");
    let handoff = engine
        .create_record(
            RecordCreateOptions::handoff(
                workspace.initial_branch_id,
                task.commit_id,
                "Durable continuation handoff",
            )
            .expect("handoff options"),
        )
        .expect("create handoff");
    let listener_id = start_session(&mut engine, &workspace, "listener", "01a-listener-thread");
    let work_id = start_session(&mut engine, &workspace, "work", SOURCE_THREAD);
    let candidate_id = start_session(&mut engine, &workspace, "work_candidate", TARGET_THREAD);
    let side_chat_id = start_session(&mut engine, &workspace, "side_chat", "01a-side-chat-thread");
    let project_policy = engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                listener_id,
                PROJECT_ID,
                0,
                true,
                project_policy_authority(listener_id, 0, true),
            )
            .expect("project policy options"),
        )
        .expect("enable continuation project policy");
    let claim_id = engine
        .claim_task(ClaimTaskOptions::new(work_id, task.task_entity_id))
        .expect("claim task")
        .claim_id;
    Fixture {
        engine,
        workspace,
        task_id: task.task_entity_id,
        handoff_id: handoff.record_entity_id,
        handoff_previous_head_commit_id: handoff.previous_head_commit_id,
        handoff_commit_id: handoff.commit_id,
        handoff_digest: handoff.record_state_digest,
        listener_id,
        work_id,
        candidate_id,
        side_chat_id,
        claim_id,
        project_policy_revision: project_policy.revision,
        project_policy_receipt_digest: project_policy.receipt_digest,
        project_policy_updated_at_us: project_policy.updated_at_us,
    }
}

fn bind_scope(fixture: &mut Fixture) -> workvcs_core::ContinuationScopeSnapshot {
    let scope = fixture
        .engine
        .bind_continuation_scope(ContinuationScopeBindOptions::new(
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            fixture.task_id,
            fixture.listener_id,
            fixture.work_id,
            fixture.claim_id,
        ))
        .expect("bind continuation scope");
    assert_eq!(scope.owner_epoch, 1);
    assert_eq!(scope.listener_revision, 1);
    scope
}

fn basis(handoff_id: workvcs_core::EntityId, source_thread_id: &str) -> CanonicalValue {
    basis_with_entities(handoff_id, source_thread_id, Vec::new())
}

fn basis_with_entities(
    handoff_id: workvcs_core::EntityId,
    source_thread_id: &str,
    currentness_entity_ids: Vec<workvcs_core::EntityId>,
) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "source_thread_id".to_owned(),
            CanonicalValue::String(source_thread_id.to_owned()),
        ),
        (
            "handoff_record_id".to_owned(),
            CanonicalValue::String(handoff_id.to_string()),
        ),
        (
            "currentness_entity_ids".to_owned(),
            CanonicalValue::Array(
                currentness_entity_ids
                    .into_iter()
                    .map(|entity_id| CanonicalValue::String(entity_id.to_string()))
                    .collect(),
            ),
        ),
    ])
    .expect("basis")
}

fn policy_decision(
    fixture: &Fixture,
    scope_id: ContinuationScopeId,
    source_session_id: SessionId,
    source_claim_id: workvcs_core::ClaimId,
    listener_revision: i64,
    owner_epoch: i64,
    currentness: Digest,
) -> CanonicalValue {
    policy_decision_for(
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        fixture.task_id,
        scope_id,
        fixture.listener_id,
        source_session_id,
        source_claim_id,
        listener_revision,
        owner_epoch,
        fixture.handoff_digest,
        currentness,
        fixture.project_policy_revision,
        fixture.project_policy_receipt_digest,
        fixture.project_policy_updated_at_us,
    )
}

#[allow(clippy::too_many_arguments)]
fn policy_decision_for(
    workspace_id: workvcs_core::WorkspaceId,
    branch_id: workvcs_core::BranchId,
    task_entity_id: workvcs_core::EntityId,
    scope_id: ContinuationScopeId,
    listener_session_id: SessionId,
    source_session_id: SessionId,
    source_claim_id: workvcs_core::ClaimId,
    listener_revision: i64,
    owner_epoch: i64,
    handoff_digest: Digest,
    currentness: Digest,
    project_policy_revision: i64,
    project_policy_receipt_digest: Digest,
    project_policy_updated_at_us: i64,
) -> CanonicalValue {
    let decision_identity = CanonicalValue::object(vec![
        (
            "project_opt_in_receipt_kind".to_owned(),
            CanonicalValue::String("workvcs_project_policy".to_owned()),
        ),
        (
            "project_opt_in_receipt_digest".to_owned(),
            CanonicalValue::String(project_policy_receipt_digest.to_string()),
        ),
        (
            "project_opt_in_revision".to_owned(),
            CanonicalValue::safe_integer(project_policy_revision).unwrap(),
        ),
        (
            "project_opt_in_verified_at_us".to_owned(),
            CanonicalValue::safe_integer(project_policy_updated_at_us).unwrap(),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(scope_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
        (
            "branch_id".to_owned(),
            CanonicalValue::String(branch_id.to_string()),
        ),
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(task_entity_id.to_string()),
        ),
        (
            "listener_session_id".to_owned(),
            CanonicalValue::String(listener_session_id.to_string()),
        ),
        (
            "source_session_id".to_owned(),
            CanonicalValue::String(source_session_id.to_string()),
        ),
        (
            "source_claim_id".to_owned(),
            CanonicalValue::String(source_claim_id.to_string()),
        ),
        (
            "handoff_digest".to_owned(),
            CanonicalValue::String(handoff_digest.to_string()),
        ),
        (
            "scope_currentness_digest".to_owned(),
            CanonicalValue::String(currentness.to_string()),
        ),
        (
            "continuation_packet_digest".to_owned(),
            CanonicalValue::String(CONTINUATION_PACKET_DIGEST.to_owned()),
        ),
        (
            "continuation_packet_bytes".to_owned(),
            CanonicalValue::safe_integer(1_024).unwrap(),
        ),
        (
            "continuation_packet_items".to_owned(),
            CanonicalValue::safe_integer(8).unwrap(),
        ),
        (
            "continuation_packet_token_budget".to_owned(),
            CanonicalValue::safe_integer(2_048).unwrap(),
        ),
        (
            "initial_message_digest".to_owned(),
            CanonicalValue::String(INITIAL_MESSAGE_DIGEST.to_owned()),
        ),
        (
            "adapter_kind".to_owned(),
            CanonicalValue::String("codex-app".to_owned()),
        ),
        (
            "adapter_version".to_owned(),
            CanonicalValue::String("test-v1".to_owned()),
        ),
    ])
    .unwrap();
    let gate_summary = CanonicalValue::object(vec![
        (
            "continuation_enabled".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "durable_provider_available".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "actor_role".to_owned(),
            CanonicalValue::String("listener".to_owned()),
        ),
        (
            "source_role".to_owned(),
            CanonicalValue::String("work".to_owned()),
        ),
        (
            "target_role".to_owned(),
            CanonicalValue::String("work_candidate".to_owned()),
        ),
        ("source_committed".to_owned(), CanonicalValue::Bool(true)),
        (
            "source_is_current_carrier".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "source_has_exclusive_claim".to_owned(),
            CanonicalValue::Bool(true),
        ),
        ("listener_matches".to_owned(), CanonicalValue::Bool(true)),
        (
            "listener_revision_matches".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "scope_currentness_matches".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "project_binding_matches".to_owned(),
            CanonicalValue::Bool(true),
        ),
        ("active_rotation".to_owned(), CanonicalValue::Bool(false)),
        (
            "prior_rotation_terminal".to_owned(),
            CanonicalValue::Bool(true),
        ),
        ("cooldown_satisfied".to_owned(), CanonicalValue::Bool(true)),
        ("safe_semantic_point".to_owned(), CanonicalValue::Bool(true)),
        ("handoff_fresh".to_owned(), CanonicalValue::Bool(true)),
        (
            "progress_since_last_rotation".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "expected_benefit".to_owned(),
            CanonicalValue::String("high".to_owned()),
        ),
        (
            "continuation_risk".to_owned(),
            CanonicalValue::String("medium".to_owned()),
        ),
        (
            "signals".to_owned(),
            CanonicalValue::object(vec![
                ("context_pressure".to_owned(), CanonicalValue::Bool(true)),
                (
                    "repeated_compaction".to_owned(),
                    CanonicalValue::Bool(false),
                ),
                (
                    "unchanged_context_replay".to_owned(),
                    CanonicalValue::Bool(true),
                ),
                ("coordination_drag".to_owned(), CanonicalValue::Bool(false)),
            ])
            .unwrap(),
        ),
    ])
    .unwrap();
    let evidence_references = CanonicalValue::Array(vec![
        CanonicalValue::object(vec![
            (
                "kind".to_owned(),
                CanonicalValue::String("workvcs_record".to_owned()),
            ),
            (
                "reference".to_owned(),
                CanonicalValue::String("record:test-continuation-pressure".to_owned()),
            ),
            (
                "digest".to_owned(),
                CanonicalValue::String(UNKNOWN_RECEIPT_DIGEST.to_owned()),
            ),
        ])
        .unwrap(),
    ]);
    let evaluator_result = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "evaluator_version".to_owned(),
            CanonicalValue::String("wg-continuation-v1".to_owned()),
        ),
        (
            "action".to_owned(),
            CanonicalValue::String("prepare_rotation".to_owned()),
        ),
        (
            "reasons".to_owned(),
            CanonicalValue::Array(vec![CanonicalValue::String(
                "all_gates_satisfied".to_owned(),
            )]),
        ),
        (
            "pressure_signal_count".to_owned(),
            CanonicalValue::safe_integer(2).unwrap(),
        ),
        ("decision_identity".to_owned(), decision_identity),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(listener_revision).unwrap(),
        ),
        (
            "owner_epoch".to_owned(),
            CanonicalValue::safe_integer(owner_epoch).unwrap(),
        ),
        (
            "evaluated_at_us".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        ("gate_summary".to_owned(), gate_summary.clone()),
        (
            "evidence_references".to_owned(),
            evidence_references.clone(),
        ),
    ])
    .unwrap();
    let evaluator_result_digest = hex::encode(Sha256::digest(
        canonical_bytes(&evaluator_result).expect("evaluator result bytes"),
    ));
    let evaluator_receipt = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "evaluator_version".to_owned(),
            CanonicalValue::String("wg-continuation-v1".to_owned()),
        ),
        (
            "result_digest".to_owned(),
            CanonicalValue::String(evaluator_result_digest.clone()),
        ),
        ("result".to_owned(), evaluator_result),
        (
            "action".to_owned(),
            CanonicalValue::String("prepare_rotation".to_owned()),
        ),
        ("gate_summary".to_owned(), gate_summary),
        ("evidence_references".to_owned(), evidence_references),
    ])
    .unwrap();
    let evaluator_receipt_digest = hex::encode(Sha256::digest(
        canonical_bytes(&evaluator_receipt).expect("evaluator receipt bytes"),
    ));
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "action".to_owned(),
            CanonicalValue::String("prepare_rotation".to_owned()),
        ),
        (
            "continuation_enabled".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "durable_provider_available".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "project_opt_in_receipt_kind".to_owned(),
            CanonicalValue::String("workvcs_project_policy".to_owned()),
        ),
        (
            "project_opt_in_receipt_digest".to_owned(),
            CanonicalValue::String(project_policy_receipt_digest.to_string()),
        ),
        (
            "project_opt_in_revision".to_owned(),
            CanonicalValue::safe_integer(project_policy_revision).unwrap(),
        ),
        (
            "project_opt_in_verified_at_us".to_owned(),
            CanonicalValue::safe_integer(project_policy_updated_at_us).unwrap(),
        ),
        (
            "continuation_packet_digest".to_owned(),
            CanonicalValue::String(CONTINUATION_PACKET_DIGEST.to_owned()),
        ),
        (
            "continuation_packet_bytes".to_owned(),
            CanonicalValue::safe_integer(1_024).unwrap(),
        ),
        (
            "continuation_packet_items".to_owned(),
            CanonicalValue::safe_integer(8).unwrap(),
        ),
        (
            "continuation_packet_token_budget".to_owned(),
            CanonicalValue::safe_integer(2_048).unwrap(),
        ),
        (
            "initial_message_digest".to_owned(),
            CanonicalValue::String(INITIAL_MESSAGE_DIGEST.to_owned()),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(scope_id.to_string()),
        ),
        (
            "workspace_id".to_owned(),
            CanonicalValue::String(workspace_id.to_string()),
        ),
        (
            "branch_id".to_owned(),
            CanonicalValue::String(branch_id.to_string()),
        ),
        (
            "task_entity_id".to_owned(),
            CanonicalValue::String(task_entity_id.to_string()),
        ),
        (
            "listener_session_id".to_owned(),
            CanonicalValue::String(listener_session_id.to_string()),
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(listener_revision).unwrap(),
        ),
        (
            "source_session_id".to_owned(),
            CanonicalValue::String(source_session_id.to_string()),
        ),
        (
            "source_claim_id".to_owned(),
            CanonicalValue::String(source_claim_id.to_string()),
        ),
        (
            "owner_epoch".to_owned(),
            CanonicalValue::safe_integer(owner_epoch).unwrap(),
        ),
        (
            "handoff_digest".to_owned(),
            CanonicalValue::String(handoff_digest.to_string()),
        ),
        (
            "scope_currentness_digest".to_owned(),
            CanonicalValue::String(currentness.to_string()),
        ),
        (
            "adapter_kind".to_owned(),
            CanonicalValue::String("codex-app".to_owned()),
        ),
        (
            "adapter_version".to_owned(),
            CanonicalValue::String("test-v1".to_owned()),
        ),
        (
            "evaluated_at_us".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "evaluator_version".to_owned(),
            CanonicalValue::String("wg-continuation-v1".to_owned()),
        ),
        (
            "evaluator_result_digest".to_owned(),
            CanonicalValue::String(evaluator_result_digest),
        ),
        ("evaluator_receipt".to_owned(), evaluator_receipt),
        (
            "evaluator_receipt_digest".to_owned(),
            CanonicalValue::String(evaluator_receipt_digest),
        ),
    ])
    .unwrap()
}

fn creation_request(
    fixture: &Fixture,
    scope_id: ContinuationScopeId,
    source_thread_id: &str,
    listener_revision: i64,
    currentness: Digest,
    operation_id: &str,
) -> CanonicalValue {
    creation_request_for(
        scope_id,
        fixture.handoff_id,
        fixture.handoff_digest,
        currentness,
        source_thread_id,
        fixture.listener_id,
        "01a-listener-thread",
        listener_revision,
        operation_id,
        fixture.project_policy_revision,
        fixture.project_policy_receipt_digest,
    )
}

#[allow(clippy::too_many_arguments)]
fn creation_request_for(
    scope_id: ContinuationScopeId,
    handoff_id: workvcs_core::EntityId,
    handoff_digest: Digest,
    currentness: Digest,
    source_thread_id: &str,
    listener_session_id: SessionId,
    listener_thread_id: &str,
    listener_revision: i64,
    operation_id: &str,
    project_policy_revision: i64,
    project_policy_receipt_digest: Digest,
) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(operation_id.to_owned()),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "project_binding_kind".to_owned(),
            CanonicalValue::String("project_id".to_owned()),
        ),
        (
            "project_binding_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "project_opt_in_receipt_kind".to_owned(),
            CanonicalValue::String("workvcs_project_policy".to_owned()),
        ),
        (
            "project_opt_in_receipt_digest".to_owned(),
            CanonicalValue::String(project_policy_receipt_digest.to_string()),
        ),
        (
            "project_opt_in_revision".to_owned(),
            CanonicalValue::safe_integer(project_policy_revision).unwrap(),
        ),
        (
            "source_thread_id".to_owned(),
            CanonicalValue::String(source_thread_id.to_owned()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(scope_id.to_string()),
        ),
        (
            "handoff_record_id".to_owned(),
            CanonicalValue::String(handoff_id.to_string()),
        ),
        (
            "handoff_digest".to_owned(),
            CanonicalValue::String(handoff_digest.to_string()),
        ),
        (
            "scope_currentness_digest".to_owned(),
            CanonicalValue::String(currentness.to_string()),
        ),
        (
            "continuation_packet_kind".to_owned(),
            CanonicalValue::String("bounded_reference_packet".to_owned()),
        ),
        (
            "continuation_packet_digest".to_owned(),
            CanonicalValue::String(CONTINUATION_PACKET_DIGEST.to_owned()),
        ),
        (
            "continuation_packet_bytes".to_owned(),
            CanonicalValue::safe_integer(1_024).unwrap(),
        ),
        (
            "continuation_packet_items".to_owned(),
            CanonicalValue::safe_integer(8).unwrap(),
        ),
        (
            "continuation_packet_token_budget".to_owned(),
            CanonicalValue::safe_integer(2_048).unwrap(),
        ),
        (
            "initial_message_digest".to_owned(),
            CanonicalValue::String(INITIAL_MESSAGE_DIGEST.to_owned()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(listener_session_id.to_string()),
        ),
        (
            "actor_thread_id".to_owned(),
            CanonicalValue::String(listener_thread_id.to_owned()),
        ),
        (
            "actor_role".to_owned(),
            CanonicalValue::String("listener".to_owned()),
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(listener_revision).unwrap(),
        ),
        (
            "adapter_kind".to_owned(),
            CanonicalValue::String("codex-app".to_owned()),
        ),
        (
            "adapter_version".to_owned(),
            CanonicalValue::String("test-v1".to_owned()),
        ),
        (
            "creation_method".to_owned(),
            CanonicalValue::String("create_thread".to_owned()),
        ),
        (
            "target_conversation_kind".to_owned(),
            CanonicalValue::String("work".to_owned()),
        ),
        ("forked".to_owned(), CanonicalValue::Bool(false)),
        (
            "source_transcript_included".to_owned(),
            CanonicalValue::Bool(false),
        ),
        ("user_visible".to_owned(), CanonicalValue::Bool(true)),
        ("auditable".to_owned(), CanonicalValue::Bool(true)),
        ("interactive".to_owned(), CanonicalValue::Bool(true)),
        (
            "prepared_at_us".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
    ])
    .unwrap()
}

fn begin_for(
    fixture: &mut Fixture,
    source_session_id: SessionId,
    source_claim_id: workvcs_core::ClaimId,
    source_thread_id: &str,
    listener_revision: i64,
    owner_epoch: i64,
    key: &str,
) -> workvcs_core::ContinuationRotationSnapshot {
    begin_for_basis(
        fixture,
        source_session_id,
        source_claim_id,
        source_thread_id,
        listener_revision,
        owner_epoch,
        key,
        basis(fixture.handoff_id, source_thread_id),
    )
}

#[allow(clippy::too_many_arguments)]
fn begin_for_basis(
    fixture: &mut Fixture,
    source_session_id: SessionId,
    source_claim_id: workvcs_core::ClaimId,
    source_thread_id: &str,
    listener_revision: i64,
    owner_epoch: i64,
    key: &str,
    basis_value: CanonicalValue,
) -> workvcs_core::ContinuationRotationSnapshot {
    let scope = fixture
        .engine
        .continuation_scope(
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            fixture.task_id,
        )
        .expect("scope");
    let currentness = fixture
        .engine
        .preview_continuation_scope_currentness(
            ContinuationScopeCurrentnessOptions::new(
                source_session_id,
                source_claim_id,
                scope.scope_id,
                fixture.listener_id,
                listener_revision,
                owner_epoch,
                fixture.handoff_digest,
                basis_value.clone(),
            )
            .expect("currentness options"),
        )
        .expect("preview currentness")
        .scope_currentness_digest;
    let operation_id = format!("create:{key}");
    let decision = policy_decision(
        fixture,
        scope.scope_id,
        source_session_id,
        source_claim_id,
        listener_revision,
        owner_epoch,
        currentness,
    );
    let request = creation_request(
        fixture,
        scope.scope_id,
        source_thread_id,
        listener_revision,
        currentness,
        &operation_id,
    );
    fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                fixture.listener_id,
                source_session_id,
                source_claim_id,
                scope.scope_id,
                listener_revision,
                owner_epoch,
                key,
                operation_id.clone(),
                fixture.handoff_digest,
                basis_value,
                decision,
                request,
            )
            .expect("begin options"),
        )
        .expect("begin continuation rotation")
}

fn begin(fixture: &mut Fixture, key: &str) -> workvcs_core::ContinuationRotationSnapshot {
    begin_for(
        fixture,
        fixture.work_id,
        fixture.claim_id,
        SOURCE_THREAD,
        1,
        1,
        key,
    )
}

fn direct_binding(
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    source_thread_id: &str,
    target_thread_id: &str,
) -> CanonicalValue {
    binding(
        rotation,
        source_thread_id,
        target_thread_id,
        DIRECT_RECEIPT_DIGEST,
    )
}

fn binding(
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    source_thread_id: &str,
    target_thread_id: &str,
    product_receipt_digest: &str,
) -> CanonicalValue {
    let dispatch = rotation.dispatch.as_ref().expect("dispatch");
    let dispatch_digest = Digest::domain_separated(
        "workvcs.continuation.dispatch.v1",
        &workvcs_core::canonical_bytes(dispatch).expect("dispatch bytes"),
    );
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "conversation_kind".to_owned(),
            CanonicalValue::String("work".to_owned()),
        ),
        (
            "creation_method".to_owned(),
            CanonicalValue::String("create_thread".to_owned()),
        ),
        ("forked".to_owned(), CanonicalValue::Bool(false)),
        (
            "identity_kind".to_owned(),
            CanonicalValue::String("thread_id".to_owned()),
        ),
        (
            "resolution_status".to_owned(),
            CanonicalValue::String("resolved".to_owned()),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "dispatch_digest".to_owned(),
            CanonicalValue::String(dispatch_digest.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(rotation.scope_id.to_string()),
        ),
        (
            "source_thread_id".to_owned(),
            CanonicalValue::String(source_thread_id.to_owned()),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "project_binding_kind".to_owned(),
            CanonicalValue::String("project_id".to_owned()),
        ),
        (
            "project_binding_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "thread_id".to_owned(),
            CanonicalValue::String(target_thread_id.to_owned()),
        ),
        (
            "resolved_from_identity_kind".to_owned(),
            CanonicalValue::String("thread_id".to_owned()),
        ),
        (
            "resolved_from_identity".to_owned(),
            CanonicalValue::String(target_thread_id.to_owned()),
        ),
        (
            "creation_actor_session_id".to_owned(),
            CanonicalValue::String(rotation.listener_session_id.to_string()),
        ),
        (
            "creation_actor_thread_id".to_owned(),
            CanonicalValue::String("01a-listener-thread".to_owned()),
        ),
        (
            "creation_listener_revision".to_owned(),
            CanonicalValue::safe_integer(rotation.listener_revision).unwrap(),
        ),
        (
            "product_receipt_digest".to_owned(),
            CanonicalValue::String(product_receipt_digest.to_owned()),
        ),
        ("user_visible".to_owned(), CanonicalValue::Bool(true)),
        ("auditable".to_owned(), CanonicalValue::Bool(true)),
        ("interactive".to_owned(), CanonicalValue::Bool(true)),
    ])
    .expect("binding")
}

fn dispatch(rotation: &workvcs_core::ContinuationRotationSnapshot) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(rotation.scope_id.to_string()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(rotation.listener_session_id.to_string()),
        ),
        (
            "actor_thread_id".to_owned(),
            CanonicalValue::String("01a-listener-thread".to_owned()),
        ),
        (
            "actor_role".to_owned(),
            CanonicalValue::String("listener".to_owned()),
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(rotation.listener_revision).unwrap(),
        ),
        (
            "adapter_kind".to_owned(),
            CanonicalValue::String("codex-app".to_owned()),
        ),
        (
            "adapter_version".to_owned(),
            CanonicalValue::String("test-v1".to_owned()),
        ),
        (
            "continuation_packet_digest".to_owned(),
            CanonicalValue::String(CONTINUATION_PACKET_DIGEST.to_owned()),
        ),
        (
            "initial_message_digest".to_owned(),
            CanonicalValue::String(INITIAL_MESSAGE_DIGEST.to_owned()),
        ),
        (
            "source_transcript_included".to_owned(),
            CanonicalValue::Bool(false),
        ),
        (
            "request_will_be_sent".to_owned(),
            CanonicalValue::Bool(true),
        ),
        (
            "dispatched_at_us".to_owned(),
            CanonicalValue::safe_integer(2).unwrap(),
        ),
    ])
    .unwrap()
}

fn created_receipt(
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    target_thread_id: &str,
    product_receipt_digest: &str,
) -> CanonicalValue {
    let dispatch_value = rotation.dispatch.as_ref().expect("dispatch");
    let dispatch_digest = Digest::domain_separated(
        "workvcs.continuation.dispatch.v1",
        &workvcs_core::canonical_bytes(dispatch_value).expect("dispatch bytes"),
    );
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "dispatch_digest".to_owned(),
            CanonicalValue::String(dispatch_digest.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(rotation.scope_id.to_string()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(rotation.listener_session_id.to_string()),
        ),
        (
            "actor_thread_id".to_owned(),
            CanonicalValue::String("01a-listener-thread".to_owned()),
        ),
        (
            "actor_role".to_owned(),
            CanonicalValue::String("listener".to_owned()),
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(rotation.listener_revision).unwrap(),
        ),
        (
            "adapter_kind".to_owned(),
            CanonicalValue::String("codex-app".to_owned()),
        ),
        (
            "adapter_version".to_owned(),
            CanonicalValue::String("test-v1".to_owned()),
        ),
        (
            "continuation_packet_digest".to_owned(),
            CanonicalValue::String(CONTINUATION_PACKET_DIGEST.to_owned()),
        ),
        (
            "initial_message_digest".to_owned(),
            CanonicalValue::String(INITIAL_MESSAGE_DIGEST.to_owned()),
        ),
        (
            "source_transcript_included".to_owned(),
            CanonicalValue::Bool(false),
        ),
        (
            "creation_method".to_owned(),
            CanonicalValue::String("create_thread".to_owned()),
        ),
        ("forked".to_owned(), CanonicalValue::Bool(false)),
        ("request_sent".to_owned(), CanonicalValue::Bool(true)),
        (
            "outcome".to_owned(),
            CanonicalValue::String("created".to_owned()),
        ),
        (
            "observed_at_us".to_owned(),
            CanonicalValue::safe_integer(3).unwrap(),
        ),
        (
            "product_receipt_digest".to_owned(),
            CanonicalValue::String(product_receipt_digest.to_owned()),
        ),
        (
            "conversation_kind".to_owned(),
            CanonicalValue::String("work".to_owned()),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "project_binding_kind".to_owned(),
            CanonicalValue::String("project_id".to_owned()),
        ),
        (
            "project_binding_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "thread_id".to_owned(),
            CanonicalValue::String(target_thread_id.to_owned()),
        ),
        ("user_visible".to_owned(), CanonicalValue::Bool(true)),
        ("auditable".to_owned(), CanonicalValue::Bool(true)),
        ("interactive".to_owned(), CanonicalValue::Bool(true)),
    ])
    .unwrap()
}

fn reconciled_created_receipt(
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    target_thread_id: &str,
    product_receipt_digest: &str,
) -> CanonicalValue {
    reconciled_created_receipt_from_dispatch(
        rotation,
        rotation,
        target_thread_id,
        product_receipt_digest,
    )
}

fn reconciled_created_receipt_from_dispatch(
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    dispatch_rotation: &workvcs_core::ContinuationRotationSnapshot,
    target_thread_id: &str,
    product_receipt_digest: &str,
) -> CanonicalValue {
    let previous = rotation
        .initial_unknown_receipt
        .as_ref()
        .expect("initial unknown receipt");
    let previous_digest = Digest::domain_separated(
        "workvcs.continuation.creation-receipt.v1",
        &workvcs_core::canonical_bytes(previous).expect("previous receipt bytes"),
    );
    let CanonicalValue::Object(mut entries) =
        created_receipt(dispatch_rotation, target_thread_id, product_receipt_digest)
    else {
        unreachable!()
    };
    let CanonicalValue::Object(previous_entries) = previous else {
        unreachable!()
    };
    for (target, source) in [
        ("resolved_from_client_identity_kind", "client_identity_kind"),
        ("resolved_from_client_identity", "client_identity"),
    ] {
        let value = previous_entries
            .iter()
            .find(|(key, _)| key == source)
            .map(|(_, value)| value.clone())
            .expect("unknown identity field");
        entries.push((target.to_owned(), value));
    }
    entries.push((
        "previous_unknown_receipt_digest".to_owned(),
        CanonicalValue::String(previous_digest.to_string()),
    ));
    CanonicalValue::object(entries).expect("reconciled receipt")
}

fn unknown_receipt(rotation: &workvcs_core::ContinuationRotationSnapshot) -> CanonicalValue {
    let dispatch_value = rotation.dispatch.as_ref().expect("dispatch");
    let dispatch_digest = Digest::domain_separated(
        "workvcs.continuation.dispatch.v1",
        &workvcs_core::canonical_bytes(dispatch_value).expect("dispatch bytes"),
    );
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "dispatch_digest".to_owned(),
            CanonicalValue::String(dispatch_digest.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(rotation.scope_id.to_string()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(rotation.listener_session_id.to_string()),
        ),
        (
            "actor_thread_id".to_owned(),
            CanonicalValue::String("01a-listener-thread".to_owned()),
        ),
        (
            "actor_role".to_owned(),
            CanonicalValue::String("listener".to_owned()),
        ),
        (
            "listener_revision".to_owned(),
            CanonicalValue::safe_integer(rotation.listener_revision).unwrap(),
        ),
        (
            "adapter_kind".to_owned(),
            CanonicalValue::String("codex-app".to_owned()),
        ),
        (
            "adapter_version".to_owned(),
            CanonicalValue::String("test-v1".to_owned()),
        ),
        (
            "continuation_packet_digest".to_owned(),
            CanonicalValue::String(CONTINUATION_PACKET_DIGEST.to_owned()),
        ),
        (
            "initial_message_digest".to_owned(),
            CanonicalValue::String(INITIAL_MESSAGE_DIGEST.to_owned()),
        ),
        (
            "source_transcript_included".to_owned(),
            CanonicalValue::Bool(false),
        ),
        (
            "creation_method".to_owned(),
            CanonicalValue::String("create_thread".to_owned()),
        ),
        ("forked".to_owned(), CanonicalValue::Bool(false)),
        (
            "outcome".to_owned(),
            CanonicalValue::String("unknown".to_owned()),
        ),
        ("request_sent".to_owned(), CanonicalValue::Bool(true)),
        (
            "observed_at_us".to_owned(),
            CanonicalValue::safe_integer(3).unwrap(),
        ),
        (
            "client_identity_kind".to_owned(),
            CanonicalValue::String("client_thread_id".to_owned()),
        ),
        (
            "client_identity".to_owned(),
            CanonicalValue::String("client-target-1".to_owned()),
        ),
        (
            "product_receipt_digest".to_owned(),
            CanonicalValue::String(UNKNOWN_RECEIPT_DIGEST.to_owned()),
        ),
    ])
    .expect("unknown receipt")
}

fn replace_object_field(
    mut value: CanonicalValue,
    field: &str,
    replacement: CanonicalValue,
) -> CanonicalValue {
    let CanonicalValue::Object(entries) = &mut value else {
        panic!("test value must be an object");
    };
    let (_, field_value) = entries
        .iter_mut()
        .find(|(key, _)| key == field)
        .unwrap_or_else(|| panic!("test value must contain {field}"));
    *field_value = replacement;
    value
}

fn clone_object_field(value: &CanonicalValue, field: &str) -> CanonicalValue {
    let CanonicalValue::Object(entries) = value else {
        panic!("test value must be an object");
    };
    entries
        .iter()
        .find(|(key, _)| key == field)
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| panic!("test value must contain {field}"))
}

fn create_target(
    fixture: &mut Fixture,
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    target_thread_id: &str,
    product_receipt_digest: &str,
) -> workvcs_core::ContinuationRotationSnapshot {
    let dispatched = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                rotation.rotation_id,
                rotation.listener_session_id,
                rotation.listener_revision,
                dispatch(rotation),
            )
            .expect("dispatch options"),
        )
        .expect("dispatch creation");
    fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                rotation.listener_session_id,
                rotation.listener_revision,
                created_receipt(&dispatched, target_thread_id, product_receipt_digest),
            )
            .expect("created options"),
        )
        .expect("record created result")
}

fn abort_created_before_attachment(
    fixture: &mut Fixture,
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    target_thread_id: &str,
) -> workvcs_core::ContinuationRotationSnapshot {
    let created = create_target(fixture, rotation, target_thread_id, DIRECT_RECEIPT_DIGEST);
    let rationale = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(created.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(created.creation_request_digest.to_string()),
        ),
        (
            "resolution".to_owned(),
            CanonicalValue::String("candidate_disposition_confirmed".to_owned()),
        ),
        (
            "candidate_disposition".to_owned(),
            CanonicalValue::String("retained_non_authoritative".to_owned()),
        ),
        (
            "evidence_digest".to_owned(),
            CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
        ),
    ])
    .unwrap();
    fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                created.rotation_id,
                created.listener_session_id,
                created.listener_revision,
                rationale,
            )
            .unwrap(),
        )
        .expect("abort created target before attachment")
}

fn listener_replacement_authority(
    scope_id: ContinuationScopeId,
    actor_session_id: SessionId,
    expected_listener_session_id: SessionId,
    replacement_listener_session_id: SessionId,
    expected_listener_revision: i64,
    expected_owner_epoch: i64,
    mode: &str,
) -> CanonicalValue {
    let mut entries = vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(scope_id.to_string()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(actor_session_id.to_string()),
        ),
        (
            "expected_listener_session_id".to_owned(),
            CanonicalValue::String(expected_listener_session_id.to_string()),
        ),
        (
            "replacement_listener_session_id".to_owned(),
            CanonicalValue::String(replacement_listener_session_id.to_string()),
        ),
        (
            "expected_listener_revision".to_owned(),
            CanonicalValue::safe_integer(expected_listener_revision).unwrap(),
        ),
        (
            "expected_owner_epoch".to_owned(),
            CanonicalValue::safe_integer(expected_owner_epoch).unwrap(),
        ),
        ("mode".to_owned(), CanonicalValue::String(mode.to_owned())),
    ];
    let (field, digest) = if mode == "current_listener_delegated" {
        ("delegation_digest", DIRECT_RECEIPT_DIGEST)
    } else {
        ("evidence_digest", DISPOSITION_EVIDENCE_DIGEST)
    };
    entries.push((field.to_owned(), CanonicalValue::String(digest.to_owned())));
    CanonicalValue::object(entries).expect("listener replacement authority")
}

#[allow(clippy::too_many_arguments)]
fn listener_replacement_options(
    fixture: &Fixture,
    scope_id: ContinuationScopeId,
    actor_session_id: SessionId,
    expected_listener_session_id: SessionId,
    replacement_listener_session_id: SessionId,
    expected_listener_revision: i64,
    expected_owner_epoch: i64,
    mode: &str,
) -> ContinuationListenerReplaceOptions {
    ContinuationListenerReplaceOptions::new(
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        fixture.task_id,
        scope_id,
        actor_session_id,
        expected_listener_session_id,
        replacement_listener_session_id,
        expected_listener_revision,
        expected_owner_epoch,
        listener_replacement_authority(
            scope_id,
            actor_session_id,
            expected_listener_session_id,
            replacement_listener_session_id,
            expected_listener_revision,
            expected_owner_epoch,
            mode,
        ),
    )
    .expect("listener replacement options")
}

fn preview_rotation_currentness(
    fixture: &mut Fixture,
    rotation: &workvcs_core::ContinuationRotationSnapshot,
) -> Digest {
    fixture
        .engine
        .preview_continuation_scope_currentness(
            ContinuationScopeCurrentnessOptions::new(
                rotation.source_session_id,
                rotation.source_claim_id,
                rotation.scope_id,
                rotation.listener_session_id,
                rotation.listener_revision,
                rotation.source_owner_epoch,
                rotation.handoff_digest,
                rotation.basis.clone(),
            )
            .expect("currentness options"),
        )
        .expect("preview currentness")
        .scope_currentness_digest
}

#[allow(clippy::too_many_arguments)]
fn abort_only_recovery_authority(
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    replacement_listener_session_id: SessionId,
    expected_listener_lifecycle_state: &str,
    expected_recovery_revision: i64,
    observed_currentness: Digest,
) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "mode".to_owned(),
            CanonicalValue::String("abort_only".to_owned()),
        ),
        (
            "rotation_id".to_owned(),
            CanonicalValue::String(rotation.rotation_id.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(rotation.scope_id.to_string()),
        ),
        (
            "actor_session_id".to_owned(),
            CanonicalValue::String(replacement_listener_session_id.to_string()),
        ),
        (
            "expected_listener_session_id".to_owned(),
            CanonicalValue::String(rotation.listener_session_id.to_string()),
        ),
        (
            "replacement_listener_session_id".to_owned(),
            CanonicalValue::String(replacement_listener_session_id.to_string()),
        ),
        (
            "expected_listener_lifecycle_state".to_owned(),
            CanonicalValue::String(expected_listener_lifecycle_state.to_owned()),
        ),
        (
            "expected_listener_revision".to_owned(),
            CanonicalValue::safe_integer(rotation.listener_revision).unwrap(),
        ),
        (
            "expected_recovery_revision".to_owned(),
            CanonicalValue::safe_integer(expected_recovery_revision).unwrap(),
        ),
        (
            "expected_owner_epoch".to_owned(),
            CanonicalValue::safe_integer(rotation.source_owner_epoch).unwrap(),
        ),
        (
            "original_scope_currentness_digest".to_owned(),
            CanonicalValue::String(rotation.scope_currentness_digest.to_string()),
        ),
        (
            "observed_scope_currentness_digest".to_owned(),
            CanonicalValue::String(observed_currentness.to_string()),
        ),
        (
            "currentness_drifted".to_owned(),
            CanonicalValue::Bool(observed_currentness != rotation.scope_currentness_digest),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "recovery_evidence_digest".to_owned(),
            CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
        ),
        (
            "authorization_receipt_digest".to_owned(),
            CanonicalValue::String(DIRECT_RECEIPT_DIGEST.to_owned()),
        ),
        (
            "authorized_at_us".to_owned(),
            CanonicalValue::safe_integer(4).unwrap(),
        ),
    ])
    .expect("abort-only recovery authority")
}

fn recover_abort_only(
    fixture: &mut Fixture,
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    replacement_thread_id: &str,
) -> workvcs_core::ContinuationRotationSnapshot {
    fixture
        .engine
        .clear_session_focus(rotation.source_session_id)
        .expect("touch source session before recovery");
    fixture
        .engine
        .end_session(
            SessionEndOptions::new(rotation.listener_session_id).expect("end listener options"),
        )
        .expect("end lost listener");
    let recovery_preview = fixture
        .engine
        .preview_abort_only_recovery(
            ContinuationAbortOnlyRecoveryPreviewOptions::new(
                rotation.rotation_id,
                rotation.scope_id,
                rotation.listener_session_id,
                rotation.listener_revision,
                rotation.recovery_revision,
                rotation.source_owner_epoch,
            )
            .expect("abort-only recovery preview options"),
        )
        .expect("abort-only recovery preview");
    let replacement = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        replacement_thread_id,
    );
    let recovered = fixture
        .engine
        .recover_continuation_listener_abort_only(
            ContinuationAbortOnlyRecoveryOptions::new(
                rotation.rotation_id,
                rotation.scope_id,
                replacement,
                rotation.listener_session_id,
                replacement,
                rotation.listener_revision,
                rotation.recovery_revision,
                rotation.source_owner_epoch,
                abort_only_recovery_authority(
                    rotation,
                    replacement,
                    "ended",
                    rotation.recovery_revision,
                    recovery_preview.observed_scope_currentness_digest,
                ),
            )
            .expect("abort-only recovery options"),
        )
        .expect("abort-only recovery");
    fixture.listener_id = replacement;
    recovered
}

fn abort_only_rationale(rotation: &workvcs_core::ContinuationRotationSnapshot) -> CanonicalValue {
    let recovery_digest = rotation
        .recovery_receipt_digest
        .expect("recovery receipt digest");
    let mut entries = vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "recovery_mode".to_owned(),
            CanonicalValue::String("abort_only".to_owned()),
        ),
        (
            "recovery_revision".to_owned(),
            CanonicalValue::safe_integer(rotation.recovery_revision).unwrap(),
        ),
        (
            "recovery_receipt_digest".to_owned(),
            CanonicalValue::String(recovery_digest.to_string()),
        ),
    ];
    match rotation.state {
        ContinuationRotationState::PendingCreation => entries.push((
            "resolution".to_owned(),
            CanonicalValue::String("cancelled_before_dispatch".to_owned()),
        )),
        ContinuationRotationState::CreationDispatched
        | ContinuationRotationState::CreationUnknown => {
            entries.push((
                "resolution".to_owned(),
                CanonicalValue::String("confirmed_not_created".to_owned()),
            ));
            entries.push((
                "evidence_digest".to_owned(),
                CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
            ));
        }
        ContinuationRotationState::CreationSucceeded
        | ContinuationRotationState::CandidateAttached
        | ContinuationRotationState::CandidateAccepted => {
            entries.push((
                "resolution".to_owned(),
                CanonicalValue::String("candidate_disposition_confirmed".to_owned()),
            ));
            entries.push((
                "candidate_disposition".to_owned(),
                CanonicalValue::String("retained_non_authoritative".to_owned()),
            ));
            entries.push((
                "evidence_digest".to_owned(),
                CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
            ));
        }
        ContinuationRotationState::Committed | ContinuationRotationState::Aborted => {
            panic!("terminal rotation cannot enter abort-only recovery")
        }
    }
    CanonicalValue::object(entries).expect("abort-only rationale")
}

fn attach_direct(
    fixture: &mut Fixture,
    rotation: &workvcs_core::ContinuationRotationSnapshot,
) -> workvcs_core::ContinuationRotationSnapshot {
    let created = create_target(fixture, rotation, TARGET_THREAD, DIRECT_RECEIPT_DIGEST);
    fixture
        .engine
        .attach_continuation_candidate(
            ContinuationCandidateAttachOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                rotation.listener_revision,
                fixture.candidate_id,
                direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
            )
            .expect("attach options"),
        )
        .expect("attach candidate")
}

fn acceptance(
    rotation: &workvcs_core::ContinuationRotationSnapshot,
    candidate_session_id: SessionId,
    candidate_thread_id: &str,
) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "rotation_id".to_owned(),
            CanonicalValue::String(rotation.rotation_id.to_string()),
        ),
        (
            "scope_id".to_owned(),
            CanonicalValue::String(rotation.scope_id.to_string()),
        ),
        (
            "candidate_session_id".to_owned(),
            CanonicalValue::String(candidate_session_id.to_string()),
        ),
        (
            "candidate_thread_id".to_owned(),
            CanonicalValue::String(candidate_thread_id.to_owned()),
        ),
        (
            "project_id".to_owned(),
            CanonicalValue::String(PROJECT_ID.to_owned()),
        ),
        (
            "handoff_record_id".to_owned(),
            CanonicalValue::String(match &rotation.basis {
                CanonicalValue::Object(entries) => entries
                    .iter()
                    .find(|(key, _)| key == "handoff_record_id")
                    .and_then(|(_, value)| match value {
                        CanonicalValue::String(value) => Some(value.clone()),
                        _ => None,
                    })
                    .unwrap(),
                _ => unreachable!(),
            }),
        ),
        (
            "handoff_digest".to_owned(),
            CanonicalValue::String(rotation.handoff_digest.to_string()),
        ),
        (
            "scope_currentness_digest".to_owned(),
            CanonicalValue::String(rotation.scope_currentness_digest.to_string()),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "continuation_packet_digest".to_owned(),
            CanonicalValue::String(CONTINUATION_PACKET_DIGEST.to_owned()),
        ),
        (
            "initial_message_digest".to_owned(),
            CanonicalValue::String(INITIAL_MESSAGE_DIGEST.to_owned()),
        ),
        ("handoff_consumed".to_owned(), CanonicalValue::Bool(true)),
        ("ready_to_continue".to_owned(), CanonicalValue::Bool(true)),
        (
            "next_action_digest".to_owned(),
            CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
        ),
        (
            "accepted_at_us".to_owned(),
            CanonicalValue::safe_integer(4).unwrap(),
        ),
    ])
    .unwrap()
}

fn commit(
    fixture: &mut Fixture,
    rotation: &workvcs_core::ContinuationRotationSnapshot,
) -> workvcs_core::ContinuationRotationCommitResult {
    let accepted = accept_candidate(fixture, rotation);
    fixture
        .engine
        .commit_continuation_rotation(ContinuationRotationCommitOptions::new(
            accepted.rotation_id,
            accepted.listener_session_id,
            accepted.listener_revision,
            accepted.source_owner_epoch,
            accepted.handoff_digest,
        ))
        .expect("commit continuation")
}

fn accept_candidate(
    fixture: &mut Fixture,
    rotation: &workvcs_core::ContinuationRotationSnapshot,
) -> workvcs_core::ContinuationRotationSnapshot {
    fixture
        .engine
        .accept_continuation_candidate(
            ContinuationCandidateAcceptOptions::new(
                rotation.rotation_id,
                rotation.candidate_session_id.expect("candidate"),
                acceptance(
                    rotation,
                    rotation.candidate_session_id.expect("candidate"),
                    rotation
                        .candidate_binding
                        .as_ref()
                        .and_then(|value| match value {
                            CanonicalValue::Object(entries) => entries
                                .iter()
                                .find(|(key, _)| key == "thread_id")
                                .and_then(|(_, value)| match value {
                                    CanonicalValue::String(value) => Some(value.as_str()),
                                    _ => None,
                                }),
                            _ => None,
                        })
                        .expect("candidate thread"),
                ),
            )
            .expect("accept options"),
        )
        .expect("candidate accepts handoff")
}

fn advance_to_nonterminal_state(
    fixture: &mut Fixture,
    began: &workvcs_core::ContinuationRotationSnapshot,
    target_state: ContinuationRotationState,
) -> workvcs_core::ContinuationRotationSnapshot {
    match target_state {
        ContinuationRotationState::PendingCreation => began.clone(),
        ContinuationRotationState::CreationDispatched => fixture
            .engine
            .dispatch_continuation_creation(
                ContinuationCreationDispatchOptions::new(
                    began.rotation_id,
                    began.listener_session_id,
                    began.listener_revision,
                    dispatch(began),
                )
                .unwrap(),
            )
            .expect("dispatch state"),
        ContinuationRotationState::CreationUnknown => {
            let dispatched = fixture
                .engine
                .dispatch_continuation_creation(
                    ContinuationCreationDispatchOptions::new(
                        began.rotation_id,
                        began.listener_session_id,
                        began.listener_revision,
                        dispatch(began),
                    )
                    .unwrap(),
                )
                .expect("dispatch unknown state");
            fixture
                .engine
                .record_continuation_creation_result(
                    ContinuationCreationResultOptions::new(
                        dispatched.rotation_id,
                        dispatched.listener_session_id,
                        dispatched.listener_revision,
                        unknown_receipt(&dispatched),
                    )
                    .unwrap(),
                )
                .expect("unknown state")
        }
        ContinuationRotationState::CreationSucceeded => {
            create_target(fixture, began, TARGET_THREAD, DIRECT_RECEIPT_DIGEST)
        }
        ContinuationRotationState::CandidateAttached => attach_direct(fixture, began),
        ContinuationRotationState::CandidateAccepted => {
            let attached = attach_direct(fixture, began);
            accept_candidate(fixture, &attached)
        }
        ContinuationRotationState::Committed | ContinuationRotationState::Aborted => {
            panic!("terminal state is not a recovery test input")
        }
    }
}

#[test]
fn visible_project_bound_candidate_commits_atomically_with_terminal_provenance() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "attempt-1");
    assert_eq!(rotation.state, ContinuationRotationState::PendingCreation);
    assert_ne!(rotation.scope_currentness_digest, Digest::raw(b""));

    let replay = begin(&mut fixture, "attempt-1");
    assert_eq!(replay.rotation_id, rotation.rotation_id);
    let attached = attach_direct(&mut fixture, &rotation);
    let committed = commit(&mut fixture, &attached);
    assert_eq!(
        committed.rotation.state,
        ContinuationRotationState::Committed
    );
    assert!(committed.rotation.result.is_some());
    assert!(committed.rotation.ended_at_us.is_some());
    assert_eq!(committed.scope.owner_epoch, 2);
    assert_eq!(committed.scope.carrier_session_id, fixture.candidate_id);
    assert_eq!(
        fixture
            .engine
            .claim_snapshot(fixture.claim_id)
            .expect("old claim")
            .lifecycle_state,
        ClaimLifecycleState::Released
    );
    assert_eq!(
        fixture
            .engine
            .active_claims_for_session(ClaimListOptions::for_session(fixture.candidate_id))
            .expect("candidate claims")
            .claims
            .len(),
        1
    );

    let terminal_begin_replay = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                rotation.listener_session_id,
                rotation.source_session_id,
                rotation.source_claim_id,
                rotation.scope_id,
                rotation.listener_revision,
                rotation.source_owner_epoch,
                rotation.idempotency_key.clone(),
                rotation.creation_operation_id.clone(),
                rotation.handoff_digest,
                rotation.basis.clone(),
                rotation.policy_decision.clone(),
                rotation.creation_request.clone(),
            )
            .expect("terminal replay options"),
        )
        .expect("terminal begin replay");
    assert_eq!(
        terminal_begin_replay.state,
        ContinuationRotationState::Committed
    );
    let terminal_commit_replay = commit(&mut fixture, &committed.rotation);
    assert_eq!(terminal_commit_replay, committed);
}

#[test]
fn concurrent_admission_creates_exactly_one_active_rotation() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let scope = bind_scope(&mut fixture);
    let basis_value = basis(fixture.handoff_id, SOURCE_THREAD);
    let currentness = fixture
        .engine
        .preview_continuation_scope_currentness(
            ContinuationScopeCurrentnessOptions::new(
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                fixture.listener_id,
                1,
                1,
                fixture.handoff_digest,
                basis_value.clone(),
            )
            .expect("currentness options"),
        )
        .expect("preview currentness")
        .scope_currentness_digest;
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for key in ["concurrent-a", "concurrent-b"] {
        let path = path.clone();
        let barrier = Arc::clone(&barrier);
        let listener_id = fixture.listener_id;
        let work_id = fixture.work_id;
        let claim_id = fixture.claim_id;
        let handoff_digest = fixture.handoff_digest;
        let scope_id = scope.scope_id;
        let decision = policy_decision(&fixture, scope_id, work_id, claim_id, 1, 1, currentness);
        let operation_id = format!("create:{key}");
        let request = creation_request(
            &fixture,
            scope_id,
            SOURCE_THREAD,
            1,
            currentness,
            &operation_id,
        );
        let attempt_basis = basis_value.clone();
        handles.push(thread::spawn(move || {
            let mut engine = Engine::open(&path).expect("open concurrent engine");
            barrier.wait();
            engine.begin_continuation_rotation(
                ContinuationRotationBeginOptions::new(
                    listener_id,
                    work_id,
                    claim_id,
                    scope_id,
                    1,
                    1,
                    key,
                    operation_id,
                    handoff_digest,
                    attempt_basis,
                    decision,
                    request,
                )
                .expect("begin options"),
            )
        }));
    }
    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("concurrent begin thread"))
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .filter(|error| error.code() == ErrorCode::ContinuationInvalid)
            .count(),
        1
    );
}

#[test]
fn independent_task_scopes_can_rotate_in_parallel_on_one_branch() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);

    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .expect("branch head")
        .head_commit_id;
    let second_task = fixture
        .engine
        .create_task(
            TaskCreateOptions::new(
                fixture.workspace.initial_branch_id,
                head,
                "Independent continuation lane",
            )
            .expect("second task options"),
        )
        .expect("second task");
    let second_handoff = fixture
        .engine
        .create_record(
            RecordCreateOptions::handoff(
                fixture.workspace.initial_branch_id,
                second_task.commit_id,
                "Independent lane handoff",
            )
            .expect("second handoff options"),
        )
        .expect("second handoff");
    let second_listener = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-second-listener",
    );
    let second_work = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "work",
        "01a-second-source",
    );
    let second_claim = fixture
        .engine
        .claim_task(ClaimTaskOptions::new(
            second_work,
            second_task.task_entity_id,
        ))
        .expect("second claim")
        .claim_id;
    let second_scope = fixture
        .engine
        .bind_continuation_scope(ContinuationScopeBindOptions::new(
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            second_task.task_entity_id,
            second_listener,
            second_work,
            second_claim,
        ))
        .expect("bind second scope");

    let first_rotation = begin(&mut fixture, "parallel-scope-one");
    let second_basis = basis(second_handoff.record_entity_id, "01a-second-source");
    let second_currentness = fixture
        .engine
        .preview_continuation_scope_currentness(
            ContinuationScopeCurrentnessOptions::new(
                second_work,
                second_claim,
                second_scope.scope_id,
                second_listener,
                1,
                1,
                second_handoff.record_state_digest,
                second_basis.clone(),
            )
            .expect("second currentness options"),
        )
        .expect("second currentness")
        .scope_currentness_digest;
    let second_decision = policy_decision_for(
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        second_task.task_entity_id,
        second_scope.scope_id,
        second_listener,
        second_work,
        second_claim,
        1,
        1,
        second_handoff.record_state_digest,
        second_currentness,
        fixture.project_policy_revision,
        fixture.project_policy_receipt_digest,
        fixture.project_policy_updated_at_us,
    );
    let duplicate_operation_error = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                second_listener,
                second_work,
                second_claim,
                second_scope.scope_id,
                1,
                1,
                "parallel-scope-two-conflict",
                first_rotation.creation_operation_id.clone(),
                second_handoff.record_state_digest,
                second_basis.clone(),
                second_decision.clone(),
                creation_request_for(
                    second_scope.scope_id,
                    second_handoff.record_entity_id,
                    second_handoff.record_state_digest,
                    second_currentness,
                    "01a-second-source",
                    second_listener,
                    "01a-second-listener",
                    1,
                    &first_rotation.creation_operation_id,
                    fixture.project_policy_revision,
                    fixture.project_policy_receipt_digest,
                ),
            )
            .expect("conflicting operation options"),
        )
        .expect_err("one creation operation cannot identify two rotations");
    assert_eq!(
        duplicate_operation_error.code(),
        ErrorCode::ContinuationInvalid
    );
    let second_rotation = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                second_listener,
                second_work,
                second_claim,
                second_scope.scope_id,
                1,
                1,
                "parallel-scope-two",
                "create:parallel-scope-two",
                second_handoff.record_state_digest,
                second_basis,
                second_decision,
                creation_request_for(
                    second_scope.scope_id,
                    second_handoff.record_entity_id,
                    second_handoff.record_state_digest,
                    second_currentness,
                    "01a-second-source",
                    second_listener,
                    "01a-second-listener",
                    1,
                    "create:parallel-scope-two",
                    fixture.project_policy_revision,
                    fixture.project_policy_receipt_digest,
                ),
            )
            .expect("second begin options"),
        )
        .expect("begin second scope rotation");

    assert_eq!(
        first_rotation.state,
        ContinuationRotationState::PendingCreation
    );
    assert_eq!(
        second_rotation.state,
        ContinuationRotationState::PendingCreation
    );
    assert_ne!(first_rotation.rotation_id, second_rotation.rotation_id);
    assert_ne!(
        first_rotation.task_entity_id,
        second_rotation.task_entity_id
    );

    let recovered_first = recover_abort_only(
        &mut fixture,
        &first_rotation,
        "01a-first-scope-recovery-listener",
    );
    assert!(recovered_first.abort_only);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(second_rotation.rotation_id)
            .expect("unrelated rotation during first-scope recovery"),
        second_rotation
    );
    assert_eq!(
        fixture
            .engine
            .continuation_scope(
                fixture.workspace.workspace_id,
                fixture.workspace.initial_branch_id,
                second_task.task_entity_id,
            )
            .expect("unrelated scope during first-scope recovery"),
        second_scope
    );
    assert_eq!(
        fixture
            .engine
            .claim_snapshot(second_claim)
            .expect("unrelated claim during first-scope recovery")
            .lifecycle_state,
        ClaimLifecycleState::Active
    );
}

#[test]
fn unknown_creation_is_correlated_and_reconciles_only_the_original_identity() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "attempt-unknown");
    let dispatched = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                rotation.listener_revision,
                dispatch(&rotation),
            )
            .expect("dispatch options"),
        )
        .expect("dispatch creation");
    let receipt = unknown_receipt(&dispatched);
    let malformed_receipt = replace_object_field(
        receipt.clone(),
        "product_receipt_digest",
        CanonicalValue::String("not-a-digest".to_owned()),
    );
    let malformed_error = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                malformed_receipt,
            )
            .expect("malformed unknown options"),
        )
        .expect_err("malformed product receipt digest must fail closed");
    assert_eq!(malformed_error.code(), ErrorCode::ContinuationInvalid);
    let unknown = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                receipt.clone(),
            )
            .expect("unknown options"),
        )
        .expect("mark unknown");
    assert_eq!(
        workvcs_core::canonical_bytes(
            unknown
                .creation_receipt
                .as_ref()
                .expect("stored unknown receipt")
        )
        .expect("stored receipt bytes"),
        workvcs_core::canonical_bytes(&receipt).expect("expected receipt bytes")
    );
    let replay = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                receipt.clone(),
            )
            .expect("unknown replay options"),
        )
        .expect("unknown replay");
    assert_eq!(replay, unknown);

    let scope = fixture
        .engine
        .continuation_scope(
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            fixture.task_id,
        )
        .expect("scope");
    let second_basis = basis(fixture.handoff_id, SOURCE_THREAD);
    let second_currentness = fixture
        .engine
        .preview_continuation_scope_currentness(
            ContinuationScopeCurrentnessOptions::new(
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                fixture.listener_id,
                1,
                1,
                fixture.handoff_digest,
                second_basis.clone(),
            )
            .expect("second currentness options"),
        )
        .expect("second currentness")
        .scope_currentness_digest;
    let second_error = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                fixture.listener_id,
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                1,
                1,
                "second-attempt",
                "create:second-attempt",
                fixture.handoff_digest,
                second_basis,
                policy_decision(
                    &fixture,
                    scope.scope_id,
                    fixture.work_id,
                    fixture.claim_id,
                    1,
                    1,
                    second_currentness,
                ),
                creation_request(
                    &fixture,
                    scope.scope_id,
                    SOURCE_THREAD,
                    1,
                    second_currentness,
                    "create:second-attempt",
                ),
            )
            .expect("second begin options"),
        )
        .expect_err("unknown result blocks a second create request");
    assert_eq!(second_error.code(), ErrorCode::ContinuationInvalid);

    let created = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                reconciled_created_receipt(&unknown, TARGET_THREAD, UNKNOWN_RECEIPT_DIGEST),
            )
            .expect("reconciliation receipt options"),
        )
        .expect("reconcile creation result");
    assert_eq!(created.initial_unknown_receipt, unknown.creation_receipt);
    let reconciled = fixture
        .engine
        .attach_continuation_candidate(
            ContinuationCandidateAttachOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                fixture.candidate_id,
                binding(
                    &created,
                    SOURCE_THREAD,
                    TARGET_THREAD,
                    UNKNOWN_RECEIPT_DIGEST,
                ),
            )
            .expect("reconcile options"),
        )
        .expect("reconcile candidate");
    assert_eq!(
        reconciled.state,
        ContinuationRotationState::CandidateAttached
    );
    let late_active_replay = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                receipt.clone(),
            )
            .expect("late active unknown replay options"),
        )
        .expect("late active unknown replay");
    assert_eq!(late_active_replay, reconciled);
    let aborted = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                CanonicalValue::object(vec![
                    (
                        "schema_version".to_owned(),
                        CanonicalValue::safe_integer(1).unwrap(),
                    ),
                    (
                        "creation_operation_id".to_owned(),
                        CanonicalValue::String(rotation.creation_operation_id.clone()),
                    ),
                    (
                        "creation_request_digest".to_owned(),
                        CanonicalValue::String(rotation.creation_request_digest.to_string()),
                    ),
                    (
                        "resolution".to_owned(),
                        CanonicalValue::String("candidate_disposition_confirmed".to_owned()),
                    ),
                    (
                        "candidate_disposition".to_owned(),
                        CanonicalValue::String("retained_non_authoritative".to_owned()),
                    ),
                    (
                        "evidence_digest".to_owned(),
                        CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
                    ),
                ])
                .unwrap(),
            )
            .expect("abort reconciled rotation options"),
        )
        .expect("abort reconciled rotation");
    let late_terminal_replay = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                receipt,
            )
            .expect("late terminal unknown replay options"),
        )
        .expect("late terminal unknown replay");
    assert_eq!(late_terminal_replay, aborted);
}

#[test]
fn abort_only_recovery_reconciles_only_the_original_creation_operation() {
    let (_dispatched_tempdir, dispatched_path) = store_path();
    let mut dispatched_fixture = fixture(&dispatched_path);
    bind_scope(&mut dispatched_fixture);
    let began = begin(
        &mut dispatched_fixture,
        "abort-only-dispatched-reconciliation",
    );
    let dispatched = dispatched_fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                began.rotation_id,
                began.listener_session_id,
                began.listener_revision,
                dispatch(&began),
            )
            .unwrap(),
        )
        .expect("dispatch original operation");
    let recovered = recover_abort_only(
        &mut dispatched_fixture,
        &dispatched,
        "01a-abort-only-dispatched-listener",
    );
    let exact_created_receipt = created_receipt(
        &dispatched,
        "01a-abort-only-dispatched-target",
        DIRECT_RECEIPT_DIGEST,
    );
    for conflicting_receipt in [
        replace_object_field(
            exact_created_receipt.clone(),
            "creation_operation_id",
            CanonicalValue::String("create:wrong-operation".to_owned()),
        ),
        replace_object_field(
            exact_created_receipt.clone(),
            "dispatch_digest",
            CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
        ),
    ] {
        let error = dispatched_fixture
            .engine
            .record_continuation_creation_result(
                ContinuationCreationResultOptions::new(
                    recovered.rotation_id,
                    recovered.listener_session_id,
                    recovered.listener_revision,
                    conflicting_receipt,
                )
                .unwrap(),
            )
            .expect_err("conflicting dispatched reconciliation must fail closed");
        assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
        assert_eq!(
            dispatched_fixture
                .engine
                .continuation_rotation(recovered.rotation_id)
                .unwrap(),
            recovered
        );
    }
    let created = dispatched_fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                recovered.rotation_id,
                recovered.listener_session_id,
                recovered.listener_revision,
                exact_created_receipt.clone(),
            )
            .unwrap(),
        )
        .expect("reconcile original dispatched operation");
    assert!(created.abort_only);
    assert_eq!(created.state, ContinuationRotationState::CreationSucceeded);
    let terminal = dispatched_fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                created.rotation_id,
                created.listener_session_id,
                created.listener_revision,
                abort_only_rationale(&created),
            )
            .unwrap(),
        )
        .expect("terminalize reconciled dispatched operation");
    let dispatched_rotation_id = terminal.rotation_id;
    drop(dispatched_fixture);
    let dispatched_engine = Engine::open(&dispatched_path).expect("reopen dispatched store");
    let dispatched_persisted = dispatched_engine
        .continuation_rotation(dispatched_rotation_id)
        .expect("read reconciled dispatched terminal result");
    assert_eq!(
        canonical_bytes(
            dispatched_persisted
                .creation_receipt
                .as_ref()
                .expect("persisted dispatched receipt"),
        )
        .unwrap(),
        canonical_bytes(&exact_created_receipt).unwrap()
    );

    let (_unknown_tempdir, unknown_path) = store_path();
    let mut unknown_fixture = fixture(&unknown_path);
    bind_scope(&mut unknown_fixture);
    let began = begin(&mut unknown_fixture, "abort-only-unknown-reconciliation");
    let dispatched = unknown_fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                began.rotation_id,
                began.listener_session_id,
                began.listener_revision,
                dispatch(&began),
            )
            .unwrap(),
        )
        .expect("dispatch unknown operation");
    let exact_unknown_receipt = unknown_receipt(&dispatched);
    let unknown = unknown_fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                dispatched.rotation_id,
                dispatched.listener_session_id,
                dispatched.listener_revision,
                exact_unknown_receipt.clone(),
            )
            .unwrap(),
        )
        .expect("record original unknown result");
    let recovered = recover_abort_only(
        &mut unknown_fixture,
        &unknown,
        "01a-abort-only-unknown-listener",
    );
    let replay = unknown_fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                recovered.rotation_id,
                recovered.listener_session_id,
                recovered.listener_revision,
                exact_unknown_receipt.clone(),
            )
            .unwrap(),
        )
        .expect("replay exact original unknown result");
    assert_eq!(replay, recovered);
    for conflicting_receipt in [
        replace_object_field(
            exact_unknown_receipt.clone(),
            "creation_operation_id",
            CanonicalValue::String("create:wrong-operation".to_owned()),
        ),
        replace_object_field(
            exact_unknown_receipt.clone(),
            "client_identity",
            CanonicalValue::String("wrong-client-target".to_owned()),
        ),
    ] {
        let error = unknown_fixture
            .engine
            .record_continuation_creation_result(
                ContinuationCreationResultOptions::new(
                    recovered.rotation_id,
                    recovered.listener_session_id,
                    recovered.listener_revision,
                    conflicting_receipt,
                )
                .unwrap(),
            )
            .expect_err("conflicting unknown reconciliation must fail closed");
        assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
        assert_eq!(
            unknown_fixture
                .engine
                .continuation_rotation(recovered.rotation_id)
                .unwrap(),
            recovered
        );
    }
    let reconciled_receipt = reconciled_created_receipt_from_dispatch(
        &recovered,
        &dispatched,
        "01a-abort-only-unknown-target",
        UNKNOWN_RECEIPT_DIGEST,
    );
    let created = unknown_fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                recovered.rotation_id,
                recovered.listener_session_id,
                recovered.listener_revision,
                reconciled_receipt.clone(),
            )
            .unwrap(),
        )
        .expect("reconcile original unknown operation");
    assert!(created.abort_only);
    assert_eq!(created.state, ContinuationRotationState::CreationSucceeded);
    assert_eq!(
        canonical_bytes(
            created
                .initial_unknown_receipt
                .as_ref()
                .expect("created initial unknown receipt"),
        )
        .unwrap(),
        canonical_bytes(&exact_unknown_receipt).unwrap()
    );
    let terminal = unknown_fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                created.rotation_id,
                created.listener_session_id,
                created.listener_revision,
                abort_only_rationale(&created),
            )
            .unwrap(),
        )
        .expect("terminalize reconciled unknown operation");
    let unknown_rotation_id = terminal.rotation_id;
    drop(unknown_fixture);
    let unknown_engine = Engine::open(&unknown_path).expect("reopen unknown store");
    let unknown_persisted = unknown_engine
        .continuation_rotation(unknown_rotation_id)
        .expect("read reconciled unknown terminal result");
    assert_eq!(
        canonical_bytes(
            unknown_persisted
                .initial_unknown_receipt
                .as_ref()
                .expect("persisted initial unknown receipt"),
        )
        .unwrap(),
        canonical_bytes(&exact_unknown_receipt).unwrap()
    );
    assert_eq!(
        canonical_bytes(
            unknown_persisted
                .creation_receipt
                .as_ref()
                .expect("persisted reconciled receipt"),
        )
        .unwrap(),
        canonical_bytes(&reconciled_receipt).unwrap()
    );
}

#[test]
fn candidate_binding_rejects_forked_unresolved_or_unproven_product_targets() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "binding-negative-controls");
    let created = create_target(
        &mut fixture,
        &rotation,
        TARGET_THREAD,
        DIRECT_RECEIPT_DIGEST,
    );

    let invalid_bindings = [
        replace_object_field(
            direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
            "forked",
            CanonicalValue::Bool(true),
        ),
        replace_object_field(
            direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
            "resolution_status",
            CanonicalValue::String("pending".to_owned()),
        ),
        replace_object_field(
            direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
            "project_binding_id",
            CanonicalValue::String("g-p-wrong-project".to_owned()),
        ),
        replace_object_field(
            direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
            "creation_operation_id",
            CanonicalValue::String("create:wrong-operation".to_owned()),
        ),
        replace_object_field(
            direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
            "user_visible",
            CanonicalValue::Bool(false),
        ),
        replace_object_field(
            direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
            "product_receipt_digest",
            CanonicalValue::String("not-a-digest".to_owned()),
        ),
    ];
    for binding in invalid_bindings {
        let error = fixture
            .engine
            .attach_continuation_candidate(
                ContinuationCandidateAttachOptions::new(
                    rotation.rotation_id,
                    fixture.listener_id,
                    rotation.listener_revision,
                    fixture.candidate_id,
                    binding,
                )
                .expect("negative attach options"),
            )
            .expect_err("invalid target evidence must fail closed");
        assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
        assert_eq!(
            fixture
                .engine
                .continuation_rotation(rotation.rotation_id)
                .expect("rotation readback")
                .state,
            ContinuationRotationState::CreationSucceeded
        );
    }

    let attached = attach_direct(&mut fixture, &rotation);
    assert_eq!(attached.state, ContinuationRotationState::CandidateAttached);
}

#[test]
fn listener_can_recover_active_rotation_and_revision_fences_aba() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let scope = bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "recover-active");
    let listener_b = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-listener-b",
    );
    let recovered = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            scope.scope_id,
            fixture.listener_id,
            fixture.listener_id,
            listener_b,
            1,
            1,
            "current_listener_delegated",
        ))
        .expect("replace active listener");
    assert_eq!(recovered.listener_revision, 2);
    let exact_replay = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            scope.scope_id,
            fixture.listener_id,
            fixture.listener_id,
            listener_b,
            1,
            1,
            "current_listener_delegated",
        ))
        .expect("exact listener replacement replay");
    assert_eq!(exact_replay, recovered);
    let unrelated_listener = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-unrelated-listener",
    );
    let forged_replay = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            scope.scope_id,
            unrelated_listener,
            fixture.listener_id,
            listener_b,
            1,
            1,
            "current_listener_delegated",
        ))
        .expect_err("replacement replay must match its recorded prior listener");
    assert_eq!(forged_replay.code(), ErrorCode::ContinuationInvalid);
    let old_actor_error = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                dispatch(&rotation),
            )
            .expect("old actor options"),
        )
        .expect_err("old listener is fenced");
    assert_eq!(old_actor_error.code(), ErrorCode::ContinuationInvalid);

    let back_to_a = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            scope.scope_id,
            listener_b,
            listener_b,
            fixture.listener_id,
            2,
            1,
            "current_listener_delegated",
        ))
        .expect("replace back to A");
    assert_eq!(back_to_a.listener_revision, 3);
    let stale_aba = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            scope.scope_id,
            fixture.listener_id,
            fixture.listener_id,
            listener_b,
            1,
            1,
            "current_listener_delegated",
        ))
        .expect_err("stale A-to-B replacement must not replay after ABA");
    assert_eq!(stale_aba.code(), ErrorCode::ContinuationInvalid);
}

#[test]
fn listener_recovery_cannot_rebase_other_currentness_drift() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let initial_scope = bind_scope(&mut fixture);
    begin(&mut fixture, "recover-with-drift");
    fixture
        .engine
        .clear_session_focus(fixture.work_id)
        .expect("touch source session");
    let listener_b = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-listener-b-drift",
    );

    let error = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            initial_scope.scope_id,
            fixture.listener_id,
            fixture.listener_id,
            listener_b,
            1,
            1,
            "current_listener_delegated",
        ))
        .expect_err("listener recovery must not rebase unrelated decision drift");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
    let scope = fixture
        .engine
        .continuation_scope(
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            fixture.task_id,
        )
        .expect("scope readback");
    assert_eq!(scope.listener_session_id, fixture.listener_id);
    assert_eq!(scope.listener_revision, 1);
}

#[test]
fn created_unattached_target_thread_cannot_become_any_replacement_listener() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let scope = bind_scope(&mut fixture);
    let began = begin(&mut fixture, "created-target-listener-alias");
    let created = create_target(
        &mut fixture,
        &began,
        "01a-unattached-created-target",
        DIRECT_RECEIPT_DIGEST,
    );
    let alias_listener = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-unattached-created-target",
    );
    let normal_error = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            scope.scope_id,
            fixture.listener_id,
            fixture.listener_id,
            alias_listener,
            created.listener_revision,
            created.source_owner_epoch,
            "current_listener_delegated",
        ))
        .expect_err("a created target cannot become the normal replacement listener");
    assert_eq!(normal_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(created.rotation_id)
            .expect("rotation after normal alias refusal"),
        created
    );

    fixture
        .engine
        .end_session(SessionEndOptions::new(created.listener_session_id).unwrap())
        .expect("end lost listener");
    let preview = fixture
        .engine
        .preview_abort_only_recovery(
            ContinuationAbortOnlyRecoveryPreviewOptions::new(
                created.rotation_id,
                created.scope_id,
                created.listener_session_id,
                created.listener_revision,
                created.recovery_revision,
                created.source_owner_epoch,
            )
            .unwrap(),
        )
        .expect("preview abort-only recovery");
    let abort_only_error = fixture
        .engine
        .recover_continuation_listener_abort_only(
            ContinuationAbortOnlyRecoveryOptions::new(
                created.rotation_id,
                created.scope_id,
                alias_listener,
                created.listener_session_id,
                alias_listener,
                created.listener_revision,
                created.recovery_revision,
                created.source_owner_epoch,
                abort_only_recovery_authority(
                    &created,
                    alias_listener,
                    "ended",
                    created.recovery_revision,
                    preview.observed_scope_currentness_digest,
                ),
            )
            .unwrap(),
        )
        .expect_err("a created target cannot become the abort-only recovery listener");
    assert_eq!(abort_only_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(created.rotation_id)
            .expect("rotation after abort-only alias refusal"),
        created
    );
}

#[test]
fn abort_only_recovery_observes_handoff_and_removed_dependency_drift_in_every_state() {
    for drift_kind in ["handoff_replaced", "dependency_removed"] {
        for target_state in [
            ContinuationRotationState::PendingCreation,
            ContinuationRotationState::CreationDispatched,
            ContinuationRotationState::CreationUnknown,
            ContinuationRotationState::CreationSucceeded,
            ContinuationRotationState::CandidateAttached,
            ContinuationRotationState::CandidateAccepted,
        ] {
            let (_tempdir, path) = store_path();
            let mut fixture = fixture(&path);
            let scope = bind_scope(&mut fixture);
            let (began, dependency_restore_target) = if drift_kind == "dependency_removed" {
                let restore_target = fixture
                    .engine
                    .branch_head(fixture.workspace.initial_branch_id)
                    .expect("branch head before dependency")
                    .head_commit_id;
                let dependency = fixture
                    .engine
                    .create_record(
                        RecordCreateOptions::finding(
                            fixture.workspace.initial_branch_id,
                            restore_target,
                            "Continuation dependency",
                        )
                        .unwrap(),
                    )
                    .expect("create currentness dependency");
                let basis_value = basis_with_entities(
                    fixture.handoff_id,
                    SOURCE_THREAD,
                    vec![dependency.record_entity_id],
                );
                let work_id = fixture.work_id;
                let claim_id = fixture.claim_id;
                (
                    begin_for_basis(
                        &mut fixture,
                        work_id,
                        claim_id,
                        SOURCE_THREAD,
                        1,
                        1,
                        &format!("{drift_kind}-{}", target_state.as_str()),
                        basis_value,
                    ),
                    Some(restore_target),
                )
            } else {
                (
                    begin(
                        &mut fixture,
                        &format!("{drift_kind}-{}", target_state.as_str()),
                    ),
                    None,
                )
            };
            let active = advance_to_nonterminal_state(&mut fixture, &began, target_state);
            if let Some(restore_target) = dependency_restore_target {
                let current_head = fixture
                    .engine
                    .branch_head(fixture.workspace.initial_branch_id)
                    .expect("branch head before dependency removal")
                    .head_commit_id;
                fixture
                    .engine
                    .restore_work_state(
                        WorkStateRestoreOptions::new(
                            fixture.workspace.initial_branch_id,
                            current_head,
                            restore_target,
                        )
                        .unwrap(),
                    )
                    .expect("remove dependency through a durable restore commit");
            } else {
                let current_head = fixture
                    .engine
                    .branch_head(fixture.workspace.initial_branch_id)
                    .expect("branch head before Handoff replacement")
                    .head_commit_id;
                let restored = fixture
                    .engine
                    .restore_work_state(
                        WorkStateRestoreOptions::new(
                            fixture.workspace.initial_branch_id,
                            current_head,
                            fixture.handoff_previous_head_commit_id,
                        )
                        .unwrap(),
                    )
                    .expect("remove original Handoff through a durable restore commit");
                fixture
                    .engine
                    .create_record(
                        RecordCreateOptions::handoff(
                            fixture.workspace.initial_branch_id,
                            restored.commit_id,
                            "Replacement continuation Handoff",
                        )
                        .unwrap(),
                    )
                    .expect("create replacement Handoff after rotation began");
            }

            let replacement = start_session(
                &mut fixture.engine,
                &fixture.workspace,
                "listener",
                &format!("01a-drift-recovery-{drift_kind}-{}", target_state.as_str()),
            );
            let normal_error = fixture
                .engine
                .replace_continuation_listener(listener_replacement_options(
                    &fixture,
                    scope.scope_id,
                    active.listener_session_id,
                    active.listener_session_id,
                    replacement,
                    active.listener_revision,
                    active.source_owner_epoch,
                    "current_listener_delegated",
                ))
                .expect_err("normal replacement must not rebase Handoff or dependency drift");
            assert_eq!(normal_error.code(), ErrorCode::ContinuationInvalid);
            assert_eq!(
                fixture
                    .engine
                    .continuation_rotation(active.rotation_id)
                    .expect("rotation after normal replacement refusal"),
                active
            );
            fixture
                .engine
                .end_session(SessionEndOptions::new(active.listener_session_id).unwrap())
                .expect("end lost listener");
            let preview = fixture
                .engine
                .preview_abort_only_recovery(
                    ContinuationAbortOnlyRecoveryPreviewOptions::new(
                        active.rotation_id,
                        active.scope_id,
                        active.listener_session_id,
                        active.listener_revision,
                        active.recovery_revision,
                        active.source_owner_epoch,
                    )
                    .unwrap(),
                )
                .expect("preview tolerant recovery currentness");
            assert!(preview.currentness_drifted);
            let recovered = fixture
                .engine
                .recover_continuation_listener_abort_only(
                    ContinuationAbortOnlyRecoveryOptions::new(
                        active.rotation_id,
                        active.scope_id,
                        replacement,
                        active.listener_session_id,
                        replacement,
                        active.listener_revision,
                        active.recovery_revision,
                        active.source_owner_epoch,
                        abort_only_recovery_authority(
                            &active,
                            replacement,
                            "ended",
                            active.recovery_revision,
                            preview.observed_scope_currentness_digest,
                        ),
                    )
                    .unwrap(),
                )
                .expect("recover drifted rotation in abort-only mode");
            assert!(recovered.abort_only);
            assert_eq!(
                recovered.scope_currentness_digest,
                active.scope_currentness_digest
            );
            let aborted = fixture
                .engine
                .abort_continuation_rotation(
                    ContinuationRotationAbortOptions::new(
                        recovered.rotation_id,
                        recovered.listener_session_id,
                        recovered.listener_revision,
                        abort_only_rationale(&recovered),
                    )
                    .unwrap(),
                )
                .expect("terminalize drifted recovery");
            assert_eq!(aborted.state, ContinuationRotationState::Aborted);
            fixture
                .engine
                .release_claim(ClaimReleaseOptions::new(
                    active.source_session_id,
                    active.source_claim_id,
                ))
                .expect("release source Claim after terminal abort");
        }
    }
}

#[test]
fn abort_only_recovery_preview_fences_post_preview_drift_without_mutation() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let active = begin(&mut fixture, "abort-only-preview-toctou");
    let first_head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .unwrap()
        .head_commit_id;
    let first_restore = fixture
        .engine
        .restore_work_state(
            WorkStateRestoreOptions::new(
                fixture.workspace.initial_branch_id,
                first_head,
                fixture.handoff_previous_head_commit_id,
            )
            .unwrap(),
        )
        .expect("remove original Handoff");
    let replacement_handoff = fixture
        .engine
        .create_record(
            RecordCreateOptions::handoff(
                fixture.workspace.initial_branch_id,
                first_restore.commit_id,
                "First replacement Handoff",
            )
            .unwrap(),
        )
        .expect("create first replacement Handoff");
    fixture
        .engine
        .end_session(SessionEndOptions::new(active.listener_session_id).unwrap())
        .expect("end lost listener");
    let stale_preview = fixture
        .engine
        .preview_abort_only_recovery(
            ContinuationAbortOnlyRecoveryPreviewOptions::new(
                active.rotation_id,
                active.scope_id,
                active.listener_session_id,
                active.listener_revision,
                active.recovery_revision,
                active.source_owner_epoch,
            )
            .unwrap(),
        )
        .expect("first recovery preview");
    fixture
        .engine
        .restore_work_state(
            WorkStateRestoreOptions::new(
                fixture.workspace.initial_branch_id,
                replacement_handoff.commit_id,
                fixture.handoff_commit_id,
            )
            .unwrap(),
        )
        .expect("restore original Handoff after preview");
    let replacement = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-abort-preview-toctou-replacement",
    );
    let stale_error = fixture
        .engine
        .recover_continuation_listener_abort_only(
            ContinuationAbortOnlyRecoveryOptions::new(
                active.rotation_id,
                active.scope_id,
                replacement,
                active.listener_session_id,
                replacement,
                active.listener_revision,
                active.recovery_revision,
                active.source_owner_epoch,
                abort_only_recovery_authority(
                    &active,
                    replacement,
                    "ended",
                    active.recovery_revision,
                    stale_preview.observed_scope_currentness_digest,
                ),
            )
            .unwrap(),
        )
        .expect_err("post-preview drift must invalidate the prior authority");
    assert_eq!(stale_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(active.rotation_id)
            .expect("zero-mutation rotation after stale preview"),
        active
    );
    let current_preview = fixture
        .engine
        .preview_abort_only_recovery(
            ContinuationAbortOnlyRecoveryPreviewOptions::new(
                active.rotation_id,
                active.scope_id,
                active.listener_session_id,
                active.listener_revision,
                active.recovery_revision,
                active.source_owner_epoch,
            )
            .unwrap(),
        )
        .expect("fresh recovery preview");
    assert_ne!(
        current_preview.observed_scope_currentness_digest,
        stale_preview.observed_scope_currentness_digest
    );
    let recovered = fixture
        .engine
        .recover_continuation_listener_abort_only(
            ContinuationAbortOnlyRecoveryOptions::new(
                active.rotation_id,
                active.scope_id,
                replacement,
                active.listener_session_id,
                replacement,
                active.listener_revision,
                active.recovery_revision,
                active.source_owner_epoch,
                abort_only_recovery_authority(
                    &active,
                    replacement,
                    "ended",
                    active.recovery_revision,
                    current_preview.observed_scope_currentness_digest,
                ),
            )
            .unwrap(),
        )
        .expect("fresh preview authorizes exact observed recovery state");
    assert!(recovered.abort_only);
}

#[test]
fn abort_only_recovery_terminates_every_nonterminal_state_without_rebasing() {
    for target_state in [
        ContinuationRotationState::PendingCreation,
        ContinuationRotationState::CreationDispatched,
        ContinuationRotationState::CreationUnknown,
        ContinuationRotationState::CreationSucceeded,
        ContinuationRotationState::CandidateAttached,
        ContinuationRotationState::CandidateAccepted,
    ] {
        let (_tempdir, path) = store_path();
        let mut fixture = fixture(&path);
        bind_scope(&mut fixture);
        let began = begin(
            &mut fixture,
            &format!("abort-only-{}", target_state.as_str()),
        );
        let active = match target_state {
            ContinuationRotationState::PendingCreation => began,
            ContinuationRotationState::CreationDispatched => fixture
                .engine
                .dispatch_continuation_creation(
                    ContinuationCreationDispatchOptions::new(
                        began.rotation_id,
                        began.listener_session_id,
                        began.listener_revision,
                        dispatch(&began),
                    )
                    .unwrap(),
                )
                .expect("dispatch state"),
            ContinuationRotationState::CreationUnknown => {
                let dispatched = fixture
                    .engine
                    .dispatch_continuation_creation(
                        ContinuationCreationDispatchOptions::new(
                            began.rotation_id,
                            began.listener_session_id,
                            began.listener_revision,
                            dispatch(&began),
                        )
                        .unwrap(),
                    )
                    .expect("dispatch unknown state");
                fixture
                    .engine
                    .record_continuation_creation_result(
                        ContinuationCreationResultOptions::new(
                            dispatched.rotation_id,
                            dispatched.listener_session_id,
                            dispatched.listener_revision,
                            unknown_receipt(&dispatched),
                        )
                        .unwrap(),
                    )
                    .expect("unknown state")
            }
            ContinuationRotationState::CreationSucceeded => {
                create_target(&mut fixture, &began, TARGET_THREAD, DIRECT_RECEIPT_DIGEST)
            }
            ContinuationRotationState::CandidateAttached => attach_direct(&mut fixture, &began),
            ContinuationRotationState::CandidateAccepted => {
                let attached = attach_direct(&mut fixture, &began);
                accept_candidate(&mut fixture, &attached)
            }
            ContinuationRotationState::Committed | ContinuationRotationState::Aborted => {
                unreachable!()
            }
        };
        assert_eq!(active.state, target_state);
        let original_currentness = active.scope_currentness_digest;
        let original_acceptance = active.candidate_acceptance.clone();
        let recovered = recover_abort_only(
            &mut fixture,
            &active,
            &format!("01a-recovery-{}", target_state.as_str()),
        );
        assert_eq!(recovered.state, target_state);
        assert!(recovered.abort_only);
        assert_eq!(recovered.recovery_revision, 1);
        assert!(recovered.recovery_receipt_digest.is_some());
        assert_eq!(recovered.scope_currentness_digest, original_currentness);
        assert_eq!(recovered.candidate_acceptance, original_acceptance);
        let CanonicalValue::Object(recovery_entries) =
            recovered.recovery.as_ref().expect("recovery provenance")
        else {
            panic!("recovery provenance must be an object")
        };
        assert_eq!(
            recovery_entries
                .iter()
                .find(|(key, _)| key == "currentness_drifted")
                .map(|(_, value)| value),
            Some(&CanonicalValue::Bool(true))
        );

        let dispatch_error = fixture
            .engine
            .dispatch_continuation_creation(
                ContinuationCreationDispatchOptions::new(
                    recovered.rotation_id,
                    recovered.listener_session_id,
                    recovered.listener_revision,
                    dispatch(&recovered),
                )
                .unwrap(),
            )
            .expect_err("abort-only recovery forbids dispatch");
        assert_eq!(dispatch_error.code(), ErrorCode::ContinuationInvalid);
        let attach_error = fixture
            .engine
            .attach_continuation_candidate(
                ContinuationCandidateAttachOptions::new(
                    recovered.rotation_id,
                    recovered.listener_session_id,
                    recovered.listener_revision,
                    fixture.candidate_id,
                    CanonicalValue::object(Vec::new()).unwrap(),
                )
                .unwrap(),
            )
            .expect_err("abort-only recovery forbids attachment");
        assert_eq!(attach_error.code(), ErrorCode::ContinuationInvalid);
        let accept_error = fixture
            .engine
            .accept_continuation_candidate(
                ContinuationCandidateAcceptOptions::new(
                    recovered.rotation_id,
                    fixture.candidate_id,
                    CanonicalValue::object(Vec::new()).unwrap(),
                )
                .unwrap(),
            )
            .expect_err("abort-only recovery forbids acceptance");
        assert_eq!(accept_error.code(), ErrorCode::ContinuationInvalid);
        let commit_error = fixture
            .engine
            .commit_continuation_rotation(ContinuationRotationCommitOptions::new(
                recovered.rotation_id,
                recovered.listener_session_id,
                recovered.listener_revision,
                recovered.source_owner_epoch,
                recovered.handoff_digest,
            ))
            .expect_err("abort-only recovery forbids commit");
        assert_eq!(commit_error.code(), ErrorCode::ContinuationInvalid);
        assert_eq!(
            fixture
                .engine
                .continuation_rotation(recovered.rotation_id)
                .expect("zero-mutation readback"),
            recovered
        );

        let aborted = fixture
            .engine
            .abort_continuation_rotation(
                ContinuationRotationAbortOptions::new(
                    recovered.rotation_id,
                    recovered.listener_session_id,
                    recovered.listener_revision,
                    abort_only_rationale(&recovered),
                )
                .unwrap(),
            )
            .expect("abort-only terminalization");
        assert_eq!(aborted.state, ContinuationRotationState::Aborted);
        assert!(aborted.abort_only);
        assert_eq!(aborted.recovery_revision, 1);
        assert_eq!(aborted.recovery, recovered.recovery);
        assert_eq!(
            fixture
                .engine
                .claim_snapshot(fixture.claim_id)
                .expect("source claim after abort")
                .lifecycle_state,
            ClaimLifecycleState::Active
        );
        if target_state == ContinuationRotationState::PendingCreation {
            fixture
                .engine
                .release_claim(ClaimReleaseOptions::new(fixture.work_id, fixture.claim_id))
                .expect("release source claim after abort-only terminalization");
            fixture
                .engine
                .end_session(SessionEndOptions::new(fixture.work_id).expect("end source options"))
                .expect("end source after abort-only terminalization");
        }
    }
}

#[test]
fn abort_only_recovery_requires_inactive_listener_and_monotonic_cas() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "abort-only-cas");
    fixture
        .engine
        .clear_session_focus(rotation.source_session_id)
        .expect("touch source");
    let observed = preview_rotation_currentness(&mut fixture, &rotation);
    let listener_b = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-abort-only-listener-b",
    );
    let first_options = ContinuationAbortOnlyRecoveryOptions::new(
        rotation.rotation_id,
        rotation.scope_id,
        listener_b,
        rotation.listener_session_id,
        listener_b,
        rotation.listener_revision,
        0,
        rotation.source_owner_epoch,
        abort_only_recovery_authority(&rotation, listener_b, "ended", 0, observed),
    )
    .unwrap();
    let active_error = fixture
        .engine
        .recover_continuation_listener_abort_only(first_options.clone())
        .expect_err("active listener cannot be recovered away");
    assert_eq!(active_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(rotation.rotation_id)
            .expect("unchanged rotation after active-listener refusal"),
        rotation
    );
    fixture
        .engine
        .end_session(SessionEndOptions::new(rotation.listener_session_id).unwrap())
        .expect("end listener A");
    let recovered_b = fixture
        .engine
        .recover_continuation_listener_abort_only(first_options.clone())
        .expect("recover listener B");
    let replay_b = fixture
        .engine
        .recover_continuation_listener_abort_only(first_options.clone())
        .expect("exact recovery replay");
    assert_eq!(replay_b, recovered_b);

    fixture
        .engine
        .clear_session_focus(rotation.source_session_id)
        .expect("touch source again");
    let observed_again = preview_rotation_currentness(&mut fixture, &recovered_b);
    fixture
        .engine
        .end_session(SessionEndOptions::new(listener_b).unwrap())
        .expect("end recovery listener B");
    let listener_c = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-abort-only-listener-c",
    );
    let recovered_c = fixture
        .engine
        .recover_continuation_listener_abort_only(
            ContinuationAbortOnlyRecoveryOptions::new(
                recovered_b.rotation_id,
                recovered_b.scope_id,
                listener_c,
                recovered_b.listener_session_id,
                listener_c,
                recovered_b.listener_revision,
                recovered_b.recovery_revision,
                recovered_b.source_owner_epoch,
                abort_only_recovery_authority(
                    &recovered_b,
                    listener_c,
                    "ended",
                    recovered_b.recovery_revision,
                    observed_again,
                ),
            )
            .unwrap(),
        )
        .expect("replace lost recovery listener");
    assert_eq!(recovered_c.listener_revision, 3);
    assert_eq!(recovered_c.recovery_revision, 2);
    assert_eq!(recovered_c.recovery_history_count, 2);
    assert_eq!(
        recovered_c.scope_currentness_digest,
        rotation.scope_currentness_digest
    );
    let stale_replay = fixture
        .engine
        .recover_continuation_listener_abort_only(first_options)
        .expect_err("stale recovery input cannot replay after a second CAS");
    assert_eq!(stale_replay.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(rotation.rotation_id)
            .expect("rotation after stale replay"),
        recovered_c
    );

    let recovery_history_chain_digest = recovered_c.recovery_history_chain_digest;
    let rotation_id = recovered_c.rotation_id;
    let workspace_id = recovered_c.workspace_id;
    let aborted = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                recovered_c.rotation_id,
                recovered_c.listener_session_id,
                recovered_c.listener_revision,
                abort_only_rationale(&recovered_c),
            )
            .unwrap(),
        )
        .expect("terminalize twice-recovered rotation");
    assert_eq!(aborted.state, ContinuationRotationState::Aborted);
    assert_eq!(aborted.recovery_history_count, 2);
    assert_eq!(
        aborted.recovery_history_chain_digest,
        recovery_history_chain_digest
    );
    drop(fixture);

    let engine = Engine::open(&path).expect("reopen terminal recovered rotation");
    let persisted = engine
        .continuation_rotation(rotation_id)
        .expect("read terminal recovered rotation after reopen");
    assert_eq!(persisted.recovery_history_count, 2);
    assert_eq!(
        persisted.recovery_history_chain_digest,
        recovery_history_chain_digest
    );
    let recovery_events = engine
        .events(
            EventListOptions::for_workspace(workspace_id)
                .with_limit(256)
                .unwrap(),
        )
        .expect("bounded recovery event readback")
        .events
        .into_iter()
        .filter(|event| event.event_kind == "continuation.listener_recovered_abort_only")
        .collect::<Vec<_>>();
    assert_eq!(recovery_events.len(), 2);
    let mut recovery_revisions = Vec::new();
    for event in recovery_events {
        let payload: serde_json::Value =
            serde_json::from_str(&event.payload_json).expect("decode recovery event payload");
        let recovery = payload
            .get("recovery")
            .and_then(serde_json::Value::as_object)
            .expect("full recovery evidence");
        recovery_revisions.push(
            recovery
                .get("recovery_revision")
                .and_then(serde_json::Value::as_i64)
                .expect("recovery revision"),
        );
        let authority = recovery
            .get("authority")
            .and_then(serde_json::Value::as_object)
            .expect("full recovery authority");
        assert_eq!(
            authority
                .get("recovery_evidence_digest")
                .and_then(serde_json::Value::as_str),
            Some(DISPOSITION_EVIDENCE_DIGEST)
        );
        assert!(
            payload
                .get("previous_recovery_history_chain_digest")
                .is_some()
        );
        assert!(payload.get("recovery_history_chain_digest").is_some());
    }
    assert_eq!(recovery_revisions, vec![1, 2]);
}

#[test]
fn concurrent_abort_only_recovery_allows_exactly_one_listener_cas() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "abort-only-concurrent-cas");
    fixture
        .engine
        .clear_session_focus(rotation.source_session_id)
        .expect("touch source");
    let observed = preview_rotation_currentness(&mut fixture, &rotation);
    fixture
        .engine
        .end_session(SessionEndOptions::new(rotation.listener_session_id).unwrap())
        .expect("end listener");
    let listener_b = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-concurrent-recovery-b",
    );
    let listener_c = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-concurrent-recovery-c",
    );
    let options_b = ContinuationAbortOnlyRecoveryOptions::new(
        rotation.rotation_id,
        rotation.scope_id,
        listener_b,
        rotation.listener_session_id,
        listener_b,
        rotation.listener_revision,
        0,
        rotation.source_owner_epoch,
        abort_only_recovery_authority(&rotation, listener_b, "ended", 0, observed),
    )
    .unwrap();
    let options_c = ContinuationAbortOnlyRecoveryOptions::new(
        rotation.rotation_id,
        rotation.scope_id,
        listener_c,
        rotation.listener_session_id,
        listener_c,
        rotation.listener_revision,
        0,
        rotation.source_owner_epoch,
        abort_only_recovery_authority(&rotation, listener_c, "ended", 0, observed),
    )
    .unwrap();
    drop(fixture.engine);
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for options in [options_b, options_c] {
        let path = path.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let mut engine = Engine::open(path).expect("open concurrent engine");
            barrier.wait();
            engine.recover_continuation_listener_abort_only(options)
        }));
    }
    barrier.wait();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("join recovery thread"))
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|result| result.is_err()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .find_map(|result| result.as_ref().err())
            .expect("losing recovery error")
            .code(),
        ErrorCode::ContinuationInvalid
    );
    let engine = Engine::open(path).expect("reopen after concurrent recovery");
    let recovered = engine
        .continuation_rotation(rotation.rotation_id)
        .expect("recovered rotation");
    assert!(recovered.abort_only);
    assert_eq!(recovered.listener_revision, 2);
    assert_eq!(recovered.recovery_revision, 1);
    assert!(matches!(
        recovered.listener_session_id,
        id if id == listener_b || id == listener_c
    ));
}

#[test]
fn ended_listener_can_be_replaced_without_splitting_the_monitored_work() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let initial_scope = bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "ended-listener-recovery");
    fixture
        .engine
        .end_session(SessionEndOptions::new(fixture.listener_id).expect("end options"))
        .expect("listener may end");
    let replacement = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-replacement-listener",
    );
    fixture.listener_id = replacement;
    let scope = fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            initial_scope.scope_id,
            replacement,
            rotation.listener_session_id,
            replacement,
            1,
            1,
            "unavailable_listener_recovery",
        ))
        .expect("recover ended listener");
    assert_eq!(scope.listener_revision, 2);
    let refreshed = fixture
        .engine
        .continuation_rotation(rotation.rotation_id)
        .expect("rotation readback");
    let replacement_thread = "01a-replacement-listener";
    let dispatch_document = replace_object_field(
        dispatch(&refreshed),
        "actor_thread_id",
        CanonicalValue::String(replacement_thread.to_owned()),
    );
    let dispatched = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                refreshed.rotation_id,
                replacement,
                refreshed.listener_revision,
                dispatch_document,
            )
            .expect("replacement dispatch options"),
        )
        .expect("replacement dispatch");
    let receipt = replace_object_field(
        created_receipt(&dispatched, TARGET_THREAD, DIRECT_RECEIPT_DIGEST),
        "actor_thread_id",
        CanonicalValue::String(replacement_thread.to_owned()),
    );
    let created = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                refreshed.rotation_id,
                replacement,
                refreshed.listener_revision,
                receipt,
            )
            .expect("replacement receipt options"),
        )
        .expect("replacement creation result");
    let candidate_binding = replace_object_field(
        direct_binding(&created, SOURCE_THREAD, TARGET_THREAD),
        "creation_actor_thread_id",
        CanonicalValue::String(replacement_thread.to_owned()),
    );
    let attached = fixture
        .engine
        .attach_continuation_candidate(
            ContinuationCandidateAttachOptions::new(
                created.rotation_id,
                replacement,
                created.listener_revision,
                fixture.candidate_id,
                candidate_binding,
            )
            .expect("replacement attach options"),
        )
        .expect("replacement attach");
    commit(&mut fixture, &attached);
}

#[test]
fn accepted_candidate_can_reaccept_after_listener_recovery_without_losing_history() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let initial_scope = bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "accepted-listener-recovery");
    let attached = attach_direct(&mut fixture, &rotation);
    let accepted = accept_candidate(&mut fixture, &attached);
    let prior_acceptance = accepted
        .candidate_acceptance
        .clone()
        .expect("prior acceptance");
    fixture
        .engine
        .end_session(SessionEndOptions::new(fixture.listener_id).expect("end options"))
        .expect("listener may end after target acceptance");
    let replacement = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "listener",
        "01a-accepted-replacement-listener",
    );
    let original_listener = fixture.listener_id;
    fixture.listener_id = replacement;
    fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &fixture,
            initial_scope.scope_id,
            replacement,
            original_listener,
            replacement,
            1,
            1,
            "unavailable_listener_recovery",
        ))
        .expect("recover accepted rotation listener");
    let refreshed = fixture
        .engine
        .continuation_rotation(rotation.rotation_id)
        .expect("refreshed accepted rotation");
    assert_eq!(
        refreshed.state,
        ContinuationRotationState::CandidateAttached
    );
    let CanonicalValue::Object(history) = &refreshed.candidate_acceptance_history else {
        panic!("acceptance history must be an object")
    };
    let Some((_, CanonicalValue::Array(prior))) =
        history.iter().find(|(key, _)| key == "acceptances")
    else {
        panic!("acceptance history must contain an array")
    };
    assert_eq!(prior, std::slice::from_ref(&prior_acceptance));

    let stale_error = fixture
        .engine
        .accept_continuation_candidate(
            ContinuationCandidateAcceptOptions::new(
                refreshed.rotation_id,
                fixture.candidate_id,
                prior_acceptance,
            )
            .unwrap(),
        )
        .expect_err("old acceptance is fenced by listener revision and currentness");
    assert_eq!(stale_error.code(), ErrorCode::ContinuationInvalid);

    let reaccepted = accept_candidate(&mut fixture, &refreshed);
    assert_eq!(reaccepted.listener_revision, 2);
    assert_eq!(
        reaccepted.state,
        ContinuationRotationState::CandidateAccepted
    );
    let committed = fixture
        .engine
        .commit_continuation_rotation(ContinuationRotationCommitOptions::new(
            reaccepted.rotation_id,
            replacement,
            2,
            1,
            fixture.handoff_digest,
        ))
        .expect("commit after target reacceptance");
    assert_eq!(
        committed.rotation.candidate_acceptance_history,
        refreshed.candidate_acceptance_history
    );
}

#[test]
fn currentness_recomputation_blocks_relevant_drift_but_ignores_unrelated_work() {
    let (_tempdir, path) = store_path();
    let mut drift = fixture(&path);
    bind_scope(&mut drift);
    let rotation = begin(&mut drift, "currentness-drift");
    let attached = attach_direct(&mut drift, &rotation);
    let accepted = accept_candidate(&mut drift, &attached);
    drift
        .engine
        .clear_session_focus(drift.work_id)
        .expect("touch source session");
    let error = drift
        .engine
        .commit_continuation_rotation(ContinuationRotationCommitOptions::new(
            accepted.rotation_id,
            drift.listener_id,
            1,
            1,
            drift.handoff_digest,
        ))
        .expect_err("source activity drift must fence commit");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);

    let (_tempdir2, path2) = store_path();
    let mut clean = fixture(&path2);
    bind_scope(&mut clean);
    let rotation = begin(&mut clean, "unrelated-work");
    let attached = attach_direct(&mut clean, &rotation);
    let head = clean
        .engine
        .branch_head(clean.workspace.initial_branch_id)
        .expect("head")
        .head_commit_id;
    clean
        .engine
        .create_task(
            TaskCreateOptions::new(
                clean.workspace.initial_branch_id,
                head,
                "Unrelated parallel task",
            )
            .expect("other task options"),
        )
        .expect("create unrelated task");
    commit(&mut clean, &attached);
}

#[test]
fn creation_dispatch_rechecks_currentness_before_external_effect() {
    for drift_kind in ["source_activity", "handoff_replaced"] {
        let (_tempdir, path) = store_path();
        let mut fixture = fixture(&path);
        bind_scope(&mut fixture);
        let rotation = begin(&mut fixture, &format!("dispatch-{drift_kind}"));
        if drift_kind == "source_activity" {
            fixture
                .engine
                .clear_session_focus(fixture.work_id)
                .expect("change source activity before dispatch");
        } else {
            let current_head = fixture
                .engine
                .branch_head(fixture.workspace.initial_branch_id)
                .expect("head before Handoff replacement")
                .head_commit_id;
            let restored = fixture
                .engine
                .restore_work_state(
                    WorkStateRestoreOptions::new(
                        fixture.workspace.initial_branch_id,
                        current_head,
                        fixture.handoff_previous_head_commit_id,
                    )
                    .unwrap(),
                )
                .expect("remove admitted Handoff before dispatch");
            fixture
                .engine
                .create_record(
                    RecordCreateOptions::handoff(
                        fixture.workspace.initial_branch_id,
                        restored.commit_id,
                        "Replacement Handoff before product effect",
                    )
                    .unwrap(),
                )
                .expect("replace admitted Handoff before dispatch");
        }
        let error = fixture
            .engine
            .dispatch_continuation_creation(
                ContinuationCreationDispatchOptions::new(
                    rotation.rotation_id,
                    rotation.listener_session_id,
                    rotation.listener_revision,
                    dispatch(&rotation),
                )
                .unwrap(),
            )
            .expect_err("relevant drift must fail before the product-effect boundary");
        assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
        assert_eq!(
            fixture
                .engine
                .continuation_rotation(rotation.rotation_id)
                .expect("pending rotation after dispatch refusal"),
            rotation
        );
    }

    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "dispatch-unrelated-work");
    let head = fixture
        .engine
        .branch_head(fixture.workspace.initial_branch_id)
        .expect("head before unrelated work")
        .head_commit_id;
    fixture
        .engine
        .create_task(
            TaskCreateOptions::new(
                fixture.workspace.initial_branch_id,
                head,
                "Unrelated task before continuation dispatch",
            )
            .unwrap(),
        )
        .expect("create unrelated task before dispatch");
    let dispatched = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                rotation.rotation_id,
                rotation.listener_session_id,
                rotation.listener_revision,
                dispatch(&rotation),
            )
            .unwrap(),
        )
        .expect("unrelated work must not fence dispatch");
    assert_eq!(
        dispatched.state,
        ContinuationRotationState::CreationDispatched
    );
}

#[test]
fn claim_and_session_bypasses_are_blocked_during_active_rotation() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "guard-bypasses");
    let release_error = fixture
        .engine
        .release_claim(ClaimReleaseOptions::new(fixture.work_id, fixture.claim_id))
        .expect_err("release must be blocked");
    assert_eq!(release_error.code(), ErrorCode::ContinuationInvalid);
    let transfer_target = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "work",
        "01a-transfer-target",
    );
    let transfer_error = fixture
        .engine
        .transfer_claim(ClaimTransferOptions::new(
            fixture.work_id,
            transfer_target,
            fixture.claim_id,
        ))
        .expect_err("ordinary transfer must be blocked");
    assert_eq!(transfer_error.code(), ErrorCode::ContinuationInvalid);
    let end_source_error = fixture
        .engine
        .end_session(SessionEndOptions::new(fixture.work_id).expect("end source options"))
        .expect_err("source end must be blocked");
    assert_eq!(end_source_error.code(), ErrorCode::ContinuationInvalid);

    let other_workspace = fixture
        .engine
        .create_workspace(WorkspaceInitOptions::new("other workspace").expect("workspace options"))
        .expect("other workspace");
    let switch_error = fixture
        .engine
        .switch_session(SessionSwitchOptions::new(
            fixture.work_id,
            other_workspace.workspace_id,
            other_workspace.initial_branch_id,
        ))
        .expect_err("source branch switch must be blocked");
    assert_eq!(switch_error.code(), ErrorCode::ContinuationInvalid);

    attach_direct(&mut fixture, &rotation);
    let end_candidate_error = fixture
        .engine
        .end_session(SessionEndOptions::new(fixture.candidate_id).expect("end candidate options"))
        .expect_err("attached candidate end must be blocked");
    assert_eq!(end_candidate_error.code(), ErrorCode::ContinuationInvalid);

    fixture
        .engine
        .mark_session_potentially_stale(
            SessionMarkStaleOptions::new(fixture.work_id, "simulate lost source")
                .expect("mark-stale options"),
        )
        .expect("mark source potentially stale");
    let takeover_error = fixture
        .engine
        .force_takeover_claim(
            ClaimForceTakeoverOptions::new(
                transfer_target,
                fixture.claim_id,
                "attempt to bypass continuation",
            )
            .expect("takeover options"),
        )
        .expect_err("force takeover must be blocked");
    assert_eq!(takeover_error.code(), ErrorCode::ContinuationInvalid);
}

#[test]
fn ordinary_claim_release_retires_an_idle_scope_atomically() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    fixture
        .engine
        .release_claim(ClaimReleaseOptions::new(fixture.work_id, fixture.claim_id))
        .expect("release idle scope claim");
    let error = fixture
        .engine
        .continuation_scope(
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            fixture.task_id,
        )
        .expect_err("scope must retire with the claim");
    assert_eq!(error.code(), ErrorCode::ContinuationNotFound);
}

#[test]
fn committed_work_candidate_can_rotate_again_but_uncommitted_roles_cannot_target() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let first = begin(&mut fixture, "first");
    let first_created = create_target(&mut fixture, &first, TARGET_THREAD, DIRECT_RECEIPT_DIGEST);
    let side_chat_error = fixture
        .engine
        .attach_continuation_candidate(
            ContinuationCandidateAttachOptions::new(
                first.rotation_id,
                fixture.listener_id,
                first.listener_revision,
                fixture.side_chat_id,
                direct_binding(&first_created, SOURCE_THREAD, TARGET_THREAD),
            )
            .expect("side-chat attach options"),
        )
        .expect_err("side chat cannot become the continuation target");
    assert_eq!(side_chat_error.code(), ErrorCode::ContinuationInvalid);
    let attached = fixture
        .engine
        .attach_continuation_candidate(
            ContinuationCandidateAttachOptions::new(
                first.rotation_id,
                fixture.listener_id,
                first.listener_revision,
                fixture.candidate_id,
                direct_binding(&first_created, SOURCE_THREAD, TARGET_THREAD),
            )
            .expect("first attach options"),
        )
        .expect("first attach");
    let first_commit = commit(&mut fixture, &attached);
    let second_candidate = start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "work_candidate",
        "01a-second-target",
    );
    let committed_candidate_id = fixture.candidate_id;
    let second = begin_for(
        &mut fixture,
        committed_candidate_id,
        first_commit.resulting_claim_id,
        TARGET_THREAD,
        1,
        2,
        "second",
    );
    let second_created = create_target(
        &mut fixture,
        &second,
        "01a-second-target",
        DIRECT_RECEIPT_DIGEST,
    );
    let attached_second = fixture
        .engine
        .attach_continuation_candidate(
            ContinuationCandidateAttachOptions::new(
                second.rotation_id,
                fixture.listener_id,
                1,
                second_candidate,
                direct_binding(&second_created, TARGET_THREAD, "01a-second-target"),
            )
            .expect("second attach options"),
        )
        .expect("second attach");
    commit(&mut fixture, &attached_second);
}

#[test]
fn abort_is_state_specific_and_conflicting_replay_is_rejected() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "abort-before-request");
    let rationale = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "resolution".to_owned(),
            CanonicalValue::String("cancelled_before_dispatch".to_owned()),
        ),
    ])
    .expect("abort rationale");
    let aborted = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                rationale.clone(),
            )
            .expect("abort options"),
        )
        .expect("abort");
    assert_eq!(aborted.state, ContinuationRotationState::Aborted);
    fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                rationale,
            )
            .expect("abort replay options"),
        )
        .expect("exact abort replay");
    let conflicting = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "resolution".to_owned(),
            CanonicalValue::String("confirmed_not_created".to_owned()),
        ),
        (
            "evidence_digest".to_owned(),
            CanonicalValue::String(
                "4444444444444444444444444444444444444444444444444444444444444444".to_owned(),
            ),
        ),
    ])
    .expect("conflicting rationale");
    let error = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                conflicting,
            )
            .expect("conflicting options"),
        )
        .expect_err("conflicting abort replay");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
}

#[test]
fn attached_candidate_abort_requires_confirmed_external_disposition() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "abort-attached");
    let attached = attach_direct(&mut fixture, &rotation);
    let insufficient = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "resolution".to_owned(),
            CanonicalValue::String("candidate_disposition_confirmed".to_owned()),
        ),
    ])
    .expect("insufficient rationale");
    let error = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                insufficient,
            )
            .expect("insufficient abort options"),
        )
        .expect_err("attached target cannot be abandoned without disposition evidence");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);

    let rationale = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).expect("safe integer"),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "resolution".to_owned(),
            CanonicalValue::String("candidate_disposition_confirmed".to_owned()),
        ),
        (
            "candidate_disposition".to_owned(),
            CanonicalValue::String("retained_non_authoritative".to_owned()),
        ),
        (
            "evidence_digest".to_owned(),
            CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
        ),
    ])
    .expect("abort rationale");
    let malformed_evidence = replace_object_field(
        rationale.clone(),
        "evidence_digest",
        CanonicalValue::String("not-a-digest".to_owned()),
    );
    let malformed_error = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                malformed_evidence,
            )
            .expect("malformed evidence options"),
        )
        .expect_err("malformed disposition evidence digest must fail closed");
    assert_eq!(malformed_error.code(), ErrorCode::ContinuationInvalid);
    let aborted = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                rationale,
            )
            .expect("abort options"),
        )
        .expect("abort attached candidate");
    assert_eq!(aborted.state, ContinuationRotationState::Aborted);
    assert_eq!(aborted.candidate_session_id, Some(fixture.candidate_id));
    assert!(aborted.candidate_binding.is_some());
    assert_eq!(
        fixture
            .engine
            .claim_snapshot(fixture.claim_id)
            .expect("source claim")
            .lifecycle_state,
        ClaimLifecycleState::Active
    );
    assert_eq!(
        fixture
            .engine
            .continuation_scope(
                fixture.workspace.workspace_id,
                fixture.workspace.initial_branch_id,
                fixture.task_id,
            )
            .expect("scope")
            .carrier_session_id,
        fixture.work_id
    );
    assert_eq!(attached.candidate_session_id, aborted.candidate_session_id);
}

#[test]
fn commit_requires_target_authored_acceptance_before_claim_transfer() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "acceptance-gate");
    let attached = attach_direct(&mut fixture, &rotation);

    let error = fixture
        .engine
        .commit_continuation_rotation(ContinuationRotationCommitOptions::new(
            attached.rotation_id,
            attached.listener_session_id,
            attached.listener_revision,
            attached.source_owner_epoch,
            attached.handoff_digest,
        ))
        .expect_err("claim transfer requires target-authored acceptance");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .claim_snapshot(fixture.claim_id)
            .expect("source claim")
            .lifecycle_state,
        ClaimLifecycleState::Active
    );

    let accepted = accept_candidate(&mut fixture, &attached);
    assert_eq!(accepted.state, ContinuationRotationState::CandidateAccepted);
    commit(&mut fixture, &accepted);
}

#[test]
fn durable_dispatch_boundary_controls_abort_and_result_provenance() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "dispatch-boundary");
    let dispatched = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                dispatch(&rotation),
            )
            .expect("dispatch options"),
        )
        .expect("durable dispatch");
    assert_eq!(
        dispatched.state,
        ContinuationRotationState::CreationDispatched
    );

    let pre_dispatch_rationale = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(rotation.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(rotation.creation_request_digest.to_string()),
        ),
        (
            "resolution".to_owned(),
            CanonicalValue::String("cancelled_before_dispatch".to_owned()),
        ),
    ])
    .unwrap();
    let abort_error = fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                pre_dispatch_rationale,
            )
            .unwrap(),
        )
        .expect_err("post-dispatch abort requires external disposition evidence");
    assert_eq!(abort_error.code(), ErrorCode::ContinuationInvalid);

    let bad_receipt = replace_object_field(
        created_receipt(&dispatched, TARGET_THREAD, DIRECT_RECEIPT_DIGEST),
        "actor_session_id",
        CanonicalValue::String(fixture.work_id.to_string()),
    );
    let receipt_error = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                bad_receipt,
            )
            .unwrap(),
        )
        .expect_err("product receipt must bind the actual dispatch actor");
    assert_eq!(receipt_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(rotation.rotation_id)
            .unwrap()
            .state,
        ContinuationRotationState::CreationDispatched
    );
}

#[test]
fn project_policy_rejects_caller_self_asserted_authority_without_mutation() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let before = fixture
        .engine
        .continuation_project_policy(PROJECT_ID)
        .expect("current project policy");

    let error = fixture
        .engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                fixture.listener_id,
                PROJECT_ID,
                fixture.project_policy_revision,
                false,
                legacy_self_asserted_project_policy_authority(
                    fixture.listener_id,
                    fixture.project_policy_revision,
                ),
            )
            .expect("legacy project policy options"),
        )
        .expect_err("caller-authored user_explicit fields cannot self-prove authority");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_project_policy(PROJECT_ID)
            .expect("project policy after rejected update"),
        before
    );
}

#[test]
fn durable_project_policy_revocation_fences_product_dispatch() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "project-policy-revoked");
    let disabled = fixture
        .engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                fixture.listener_id,
                PROJECT_ID,
                fixture.project_policy_revision,
                false,
                project_policy_authority(
                    fixture.listener_id,
                    fixture.project_policy_revision,
                    false,
                ),
            )
            .unwrap(),
        )
        .expect("revoke project continuation policy");
    assert!(!disabled.enabled);
    assert_eq!(disabled.revision, fixture.project_policy_revision + 1);
    assert_eq!(
        fixture
            .engine
            .continuation_project_policy(PROJECT_ID)
            .unwrap(),
        disabled
    );

    let dispatch_error = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                rotation.listener_revision,
                dispatch(&rotation),
            )
            .unwrap(),
        )
        .expect_err("revoked Project opt-in must fence the product-effect boundary");
    assert_eq!(dispatch_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(rotation.rotation_id)
            .unwrap()
            .state,
        ContinuationRotationState::PendingCreation
    );
}

#[test]
fn target_identity_aliases_and_non_visible_results_fail_before_attachment() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let rotation = begin(&mut fixture, "target-proof");
    let dispatched = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                dispatch(&rotation),
            )
            .unwrap(),
        )
        .unwrap();

    let hidden_receipt = replace_object_field(
        created_receipt(&dispatched, TARGET_THREAD, DIRECT_RECEIPT_DIGEST),
        "user_visible",
        CanonicalValue::Bool(false),
    );
    let hidden_error = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                hidden_receipt,
            )
            .unwrap(),
        )
        .expect_err("hidden product result is not a valid Work target");
    assert_eq!(hidden_error.code(), ErrorCode::ContinuationInvalid);

    start_session(
        &mut fixture.engine,
        &fixture.workspace,
        "work_candidate",
        TARGET_THREAD,
    );
    let alias_error = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                rotation.rotation_id,
                fixture.listener_id,
                1,
                created_receipt(&dispatched, TARGET_THREAD, DIRECT_RECEIPT_DIGEST),
            )
            .unwrap(),
        )
        .expect_err("one product thread must not alias multiple Sessions");
    assert_eq!(alias_error.code(), ErrorCode::ContinuationInvalid);
}

#[test]
fn participant_thread_aliases_are_rechecked_at_each_authoritative_phase() {
    let (_tempdir, path) = store_path();
    {
        let mut fixture = fixture(&path);
        bind_scope(&mut fixture);
        let rotation = begin(&mut fixture, "participant-alias-dispatch");
        start_session(
            &mut fixture.engine,
            &fixture.workspace,
            "listener",
            "01a-listener-thread",
        );
        let dispatch_error = fixture
            .engine
            .dispatch_continuation_creation(
                ContinuationCreationDispatchOptions::new(
                    rotation.rotation_id,
                    fixture.listener_id,
                    1,
                    dispatch(&rotation),
                )
                .unwrap(),
            )
            .expect_err("listener thread aliases introduced after admission must fail closed");
        assert_eq!(dispatch_error.code(), ErrorCode::ContinuationInvalid);
    }

    let (_tempdir2, path2) = store_path();
    let mut replacement_fixture = fixture(&path2);
    let replacement_scope = bind_scope(&mut replacement_fixture);
    let replacement = start_session(
        &mut replacement_fixture.engine,
        &replacement_fixture.workspace,
        "listener",
        "01a-replacement-alias",
    );
    start_session(
        &mut replacement_fixture.engine,
        &replacement_fixture.workspace,
        "side_chat",
        "01a-replacement-alias",
    );
    let replacement_error = replacement_fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &replacement_fixture,
            replacement_scope.scope_id,
            replacement_fixture.listener_id,
            replacement_fixture.listener_id,
            replacement,
            1,
            1,
            "current_listener_delegated",
        ))
        .expect_err("a replacement listener must identify one product thread uniquely");
    assert_eq!(replacement_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        replacement_fixture
            .engine
            .continuation_scope(
                replacement_fixture.workspace.workspace_id,
                replacement_fixture.workspace.initial_branch_id,
                replacement_fixture.task_id,
            )
            .unwrap()
            .listener_session_id,
        replacement_fixture.listener_id
    );

    let (_tempdir3, path3) = store_path();
    let mut acceptance_fixture = fixture(&path3);
    bind_scope(&mut acceptance_fixture);
    let acceptance_rotation = begin(&mut acceptance_fixture, "participant-alias-accept");
    let attached = attach_direct(&mut acceptance_fixture, &acceptance_rotation);
    start_session(
        &mut acceptance_fixture.engine,
        &acceptance_fixture.workspace,
        "work_candidate",
        TARGET_THREAD,
    );
    let acceptance_error = acceptance_fixture
        .engine
        .accept_continuation_candidate(
            ContinuationCandidateAcceptOptions::new(
                attached.rotation_id,
                acceptance_fixture.candidate_id,
                acceptance(&attached, acceptance_fixture.candidate_id, TARGET_THREAD),
            )
            .unwrap(),
        )
        .expect_err("candidate aliases introduced after attachment must block acceptance");
    assert_eq!(acceptance_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        acceptance_fixture
            .engine
            .continuation_rotation(attached.rotation_id)
            .unwrap()
            .state,
        ContinuationRotationState::CandidateAttached
    );

    let (_tempdir4, path4) = store_path();
    let mut commit_fixture = fixture(&path4);
    bind_scope(&mut commit_fixture);
    let commit_rotation = begin(&mut commit_fixture, "participant-alias-commit");
    let attached = attach_direct(&mut commit_fixture, &commit_rotation);
    let accepted = accept_candidate(&mut commit_fixture, &attached);
    start_session(
        &mut commit_fixture.engine,
        &commit_fixture.workspace,
        "work_candidate",
        TARGET_THREAD,
    );
    let commit_error = commit_fixture
        .engine
        .commit_continuation_rotation(ContinuationRotationCommitOptions::new(
            accepted.rotation_id,
            accepted.listener_session_id,
            accepted.listener_revision,
            accepted.source_owner_epoch,
            accepted.handoff_digest,
        ))
        .expect_err("candidate aliases introduced after acceptance must block commit");
    assert_eq!(commit_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        commit_fixture
            .engine
            .claim_snapshot(commit_fixture.claim_id)
            .unwrap()
            .lifecycle_state,
        ClaimLifecycleState::Active
    );
}

#[test]
fn created_target_identity_is_reserved_even_when_aborted_before_attachment() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    bind_scope(&mut fixture);
    let first = begin(&mut fixture, "reserve-created-target");
    let created = create_target(&mut fixture, &first, TARGET_THREAD, DIRECT_RECEIPT_DIGEST);
    let rationale = CanonicalValue::object(vec![
        (
            "schema_version".to_owned(),
            CanonicalValue::safe_integer(1).unwrap(),
        ),
        (
            "creation_operation_id".to_owned(),
            CanonicalValue::String(created.creation_operation_id.clone()),
        ),
        (
            "creation_request_digest".to_owned(),
            CanonicalValue::String(created.creation_request_digest.to_string()),
        ),
        (
            "resolution".to_owned(),
            CanonicalValue::String("candidate_disposition_confirmed".to_owned()),
        ),
        (
            "candidate_disposition".to_owned(),
            CanonicalValue::String("retained_non_authoritative".to_owned()),
        ),
        (
            "evidence_digest".to_owned(),
            CanonicalValue::String(DISPOSITION_EVIDENCE_DIGEST.to_owned()),
        ),
    ])
    .unwrap();
    fixture
        .engine
        .abort_continuation_rotation(
            ContinuationRotationAbortOptions::new(
                created.rotation_id,
                fixture.listener_id,
                created.listener_revision,
                rationale,
            )
            .unwrap(),
        )
        .expect("abort after product creation and before attachment");

    let second = begin(&mut fixture, "do-not-reuse-created-target");
    let dispatched = fixture
        .engine
        .dispatch_continuation_creation(
            ContinuationCreationDispatchOptions::new(
                second.rotation_id,
                fixture.listener_id,
                second.listener_revision,
                dispatch(&second),
            )
            .unwrap(),
        )
        .unwrap();
    let reuse_error = fixture
        .engine
        .record_continuation_creation_result(
            ContinuationCreationResultOptions::new(
                second.rotation_id,
                fixture.listener_id,
                second.listener_revision,
                created_receipt(&dispatched, TARGET_THREAD, DIRECT_RECEIPT_DIGEST),
            )
            .unwrap(),
        )
        .expect_err("a product target is single-use from the first created receipt");
    assert_eq!(reuse_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        fixture
            .engine
            .continuation_rotation(second.rotation_id)
            .unwrap()
            .state,
        ContinuationRotationState::CreationDispatched
    );
}

#[test]
fn retained_created_target_cannot_reenter_as_work_carrier_or_listener() {
    let (_work_tempdir, work_path) = store_path();
    let mut work_fixture = fixture(&work_path);
    bind_scope(&mut work_fixture);
    let work_rotation = begin(&mut work_fixture, "retained-target-work-role");
    let retained_work_thread = "01a-retained-target-work-role";
    abort_created_before_attachment(&mut work_fixture, &work_rotation, retained_work_thread);
    work_fixture
        .engine
        .release_claim(ClaimReleaseOptions::new(
            work_fixture.work_id,
            work_fixture.claim_id,
        ))
        .expect("retire original scope after terminal abort");
    let reserved_work = start_session(
        &mut work_fixture.engine,
        &work_fixture.workspace,
        "work",
        retained_work_thread,
    );
    let reserved_claim = work_fixture
        .engine
        .claim_task(ClaimTaskOptions::new(reserved_work, work_fixture.task_id))
        .expect("claim before continuation-role admission")
        .claim_id;
    let carrier_error = work_fixture
        .engine
        .bind_continuation_scope(ContinuationScopeBindOptions::new(
            work_fixture.workspace.workspace_id,
            work_fixture.workspace.initial_branch_id,
            work_fixture.task_id,
            work_fixture.listener_id,
            reserved_work,
            reserved_claim,
        ))
        .expect_err("retained target must not reenter as a work carrier");
    assert_eq!(carrier_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        work_fixture
            .engine
            .claim_snapshot(reserved_claim)
            .expect("reserved work claim after refusal")
            .lifecycle_state,
        ClaimLifecycleState::Active
    );
    assert_eq!(
        work_fixture
            .engine
            .continuation_scope(
                work_fixture.workspace.workspace_id,
                work_fixture.workspace.initial_branch_id,
                work_fixture.task_id,
            )
            .expect_err("refused carrier must not recreate a scope")
            .code(),
        ErrorCode::ContinuationNotFound
    );

    let (_listener_tempdir, listener_path) = store_path();
    let mut listener_fixture = fixture(&listener_path);
    let scope = bind_scope(&mut listener_fixture);
    let listener_rotation = begin(&mut listener_fixture, "retained-target-listener-role");
    let retained_listener_thread = "01a-retained-target-listener-role";
    abort_created_before_attachment(
        &mut listener_fixture,
        &listener_rotation,
        retained_listener_thread,
    );
    let reserved_listener = start_session(
        &mut listener_fixture.engine,
        &listener_fixture.workspace,
        "listener",
        retained_listener_thread,
    );
    let replace_error = listener_fixture
        .engine
        .replace_continuation_listener(listener_replacement_options(
            &listener_fixture,
            scope.scope_id,
            listener_fixture.listener_id,
            listener_fixture.listener_id,
            reserved_listener,
            scope.listener_revision,
            scope.owner_epoch,
            "current_listener_delegated",
        ))
        .expect_err("retained target must not reenter as an idle replacement listener");
    assert_eq!(replace_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        listener_fixture
            .engine
            .continuation_scope(
                listener_fixture.workspace.workspace_id,
                listener_fixture.workspace.initial_branch_id,
                listener_fixture.task_id,
            )
            .expect("scope after historical listener refusal"),
        scope
    );

    let next_rotation = begin(&mut listener_fixture, "historical-listener-recovery");
    listener_fixture
        .engine
        .end_session(SessionEndOptions::new(next_rotation.listener_session_id).unwrap())
        .expect("end current listener before recovery attempt");
    let preview = listener_fixture
        .engine
        .preview_abort_only_recovery(
            ContinuationAbortOnlyRecoveryPreviewOptions::new(
                next_rotation.rotation_id,
                next_rotation.scope_id,
                next_rotation.listener_session_id,
                next_rotation.listener_revision,
                next_rotation.recovery_revision,
                next_rotation.source_owner_epoch,
            )
            .unwrap(),
        )
        .expect("preview recovery before historical listener refusal");
    let recovery_error = listener_fixture
        .engine
        .recover_continuation_listener_abort_only(
            ContinuationAbortOnlyRecoveryOptions::new(
                next_rotation.rotation_id,
                next_rotation.scope_id,
                reserved_listener,
                next_rotation.listener_session_id,
                reserved_listener,
                next_rotation.listener_revision,
                next_rotation.recovery_revision,
                next_rotation.source_owner_epoch,
                abort_only_recovery_authority(
                    &next_rotation,
                    reserved_listener,
                    "ended",
                    next_rotation.recovery_revision,
                    preview.observed_scope_currentness_digest,
                ),
            )
            .unwrap(),
        )
        .expect_err("retained target must not reenter as an abort-only listener");
    assert_eq!(recovery_error.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        listener_fixture
            .engine
            .continuation_rotation(next_rotation.rotation_id)
            .expect("rotation after historical listener recovery refusal"),
        next_rotation
    );
}

#[test]
fn scope_identity_fences_delete_and_rebind_aba() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let original_scope = bind_scope(&mut fixture);
    fixture
        .engine
        .release_claim(ClaimReleaseOptions::new(fixture.work_id, fixture.claim_id))
        .expect("release original claim and retire scope");
    let replacement_claim = fixture
        .engine
        .claim_task(ClaimTaskOptions::new(fixture.work_id, fixture.task_id))
        .expect("replacement claim")
        .claim_id;
    let replacement_scope = fixture
        .engine
        .bind_continuation_scope(ContinuationScopeBindOptions::new(
            fixture.workspace.workspace_id,
            fixture.workspace.initial_branch_id,
            fixture.task_id,
            fixture.listener_id,
            fixture.work_id,
            replacement_claim,
        ))
        .expect("replacement scope");
    assert_ne!(replacement_scope.scope_id, original_scope.scope_id);

    let error = fixture
        .engine
        .preview_continuation_scope_currentness(
            ContinuationScopeCurrentnessOptions::new(
                fixture.work_id,
                replacement_claim,
                original_scope.scope_id,
                fixture.listener_id,
                1,
                1,
                fixture.handoff_digest,
                basis(fixture.handoff_id, SOURCE_THREAD),
            )
            .unwrap(),
        )
        .expect_err("stale scope identity must not survive delete and rebind");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
}

#[test]
fn continuation_inputs_reject_control_characters_and_unbounded_documents() {
    let (_tempdir, path) = store_path();
    let mut fixture = fixture(&path);
    let scope = bind_scope(&mut fixture);
    let basis_value = basis(fixture.handoff_id, SOURCE_THREAD);
    let currentness = fixture
        .engine
        .preview_continuation_scope_currentness(
            ContinuationScopeCurrentnessOptions::new(
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                fixture.listener_id,
                1,
                1,
                fixture.handoff_digest,
                basis_value.clone(),
            )
            .unwrap(),
        )
        .unwrap()
        .scope_currentness_digest;
    let decision = policy_decision(
        &fixture,
        scope.scope_id,
        fixture.work_id,
        fixture.claim_id,
        1,
        1,
        currentness,
    );
    let request = creation_request(
        &fixture,
        scope.scope_id,
        SOURCE_THREAD,
        1,
        currentness,
        "create:bounded",
    );

    let control_error = ContinuationRotationBeginOptions::new(
        fixture.listener_id,
        fixture.work_id,
        fixture.claim_id,
        scope.scope_id,
        1,
        1,
        "line-one\nline-two",
        "create:bounded",
        fixture.handoff_digest,
        basis_value.clone(),
        decision.clone(),
        request,
    )
    .expect_err("control characters would corrupt key-value output");
    assert_eq!(control_error.code(), ErrorCode::ContinuationInvalid);

    let tampered_evaluator_decision = replace_object_field(
        decision.clone(),
        "evaluator_receipt_digest",
        CanonicalValue::String(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned(),
        ),
    );
    let tampered_evaluator_error = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                fixture.listener_id,
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                1,
                1,
                "tampered-evaluator-receipt",
                "create:tampered-evaluator-receipt",
                fixture.handoff_digest,
                basis_value.clone(),
                tampered_evaluator_decision,
                creation_request(
                    &fixture,
                    scope.scope_id,
                    SOURCE_THREAD,
                    1,
                    currentness,
                    "create:tampered-evaluator-receipt",
                ),
            )
            .unwrap(),
        )
        .expect_err("the durable evaluator receipt digest must match its canonical content");
    assert_eq!(
        tampered_evaluator_error.code(),
        ErrorCode::ContinuationInvalid
    );

    let evaluator_receipt = clone_object_field(&decision, "evaluator_receipt");
    let evaluator_result = clone_object_field(&evaluator_receipt, "result");
    let gate_summary = clone_object_field(&evaluator_result, "gate_summary");
    let invalid_gate_summary = replace_object_field(
        gate_summary,
        "safe_semantic_point",
        CanonicalValue::Bool(false),
    );
    let invalid_result = replace_object_field(
        evaluator_result,
        "gate_summary",
        invalid_gate_summary.clone(),
    );
    let invalid_result_digest = hex::encode(Sha256::digest(
        canonical_bytes(&invalid_result).expect("invalid evaluator result bytes"),
    ));
    let invalid_receipt = replace_object_field(
        replace_object_field(
            replace_object_field(evaluator_receipt, "result", invalid_result),
            "result_digest",
            CanonicalValue::String(invalid_result_digest.clone()),
        ),
        "gate_summary",
        invalid_gate_summary,
    );
    let invalid_receipt_digest = hex::encode(Sha256::digest(
        canonical_bytes(&invalid_receipt).expect("invalid evaluator receipt bytes"),
    ));
    let self_consistent_invalid_decision = replace_object_field(
        replace_object_field(
            replace_object_field(
                decision.clone(),
                "evaluator_result_digest",
                CanonicalValue::String(invalid_result_digest),
            ),
            "evaluator_receipt",
            invalid_receipt,
        ),
        "evaluator_receipt_digest",
        CanonicalValue::String(invalid_receipt_digest),
    );
    let invalid_gate_error = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                fixture.listener_id,
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                1,
                1,
                "self-consistent-invalid-evaluator-gate",
                "create:self-consistent-invalid-evaluator-gate",
                fixture.handoff_digest,
                basis_value.clone(),
                self_consistent_invalid_decision,
                creation_request(
                    &fixture,
                    scope.scope_id,
                    SOURCE_THREAD,
                    1,
                    currentness,
                    "create:self-consistent-invalid-evaluator-gate",
                ),
            )
            .unwrap(),
        )
        .expect_err("a self-consistent receipt cannot authorize an evaluator gate failure");
    assert_eq!(invalid_gate_error.code(), ErrorCode::ContinuationInvalid);

    let oversized_packet_decision = replace_object_field(
        decision.clone(),
        "continuation_packet_bytes",
        CanonicalValue::safe_integer(8_193).unwrap(),
    );
    let oversized_packet_request = creation_request(
        &fixture,
        scope.scope_id,
        SOURCE_THREAD,
        1,
        currentness,
        "create:oversized-packet",
    );
    let packet_error = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                fixture.listener_id,
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                1,
                1,
                "oversized-packet",
                "create:oversized-packet",
                fixture.handoff_digest,
                basis_value.clone(),
                oversized_packet_decision,
                oversized_packet_request,
            )
            .unwrap(),
        )
        .expect_err("continuation packet metadata must enforce the protocol byte bound");
    assert_eq!(packet_error.code(), ErrorCode::ContinuationInvalid);

    let transcript_request = replace_object_field(
        creation_request(
            &fixture,
            scope.scope_id,
            SOURCE_THREAD,
            1,
            currentness,
            "create:transcript-injection",
        ),
        "source_transcript_included",
        CanonicalValue::Bool(true),
    );
    let transcript_error = fixture
        .engine
        .begin_continuation_rotation(
            ContinuationRotationBeginOptions::new(
                fixture.listener_id,
                fixture.work_id,
                fixture.claim_id,
                scope.scope_id,
                1,
                1,
                "transcript-injection",
                "create:transcript-injection",
                fixture.handoff_digest,
                basis_value.clone(),
                decision.clone(),
                transcript_request,
            )
            .unwrap(),
        )
        .expect_err("clean continuation may not inject the source transcript");
    assert_eq!(transcript_error.code(), ErrorCode::ContinuationInvalid);

    let oversized = CanonicalValue::object(vec![(
        "payload".to_owned(),
        CanonicalValue::String("x".repeat(65_537)),
    )])
    .unwrap();
    let size_error = ContinuationRotationBeginOptions::new(
        fixture.listener_id,
        fixture.work_id,
        fixture.claim_id,
        scope.scope_id,
        1,
        1,
        "bounded",
        "create:bounded",
        fixture.handoff_digest,
        basis_value,
        oversized,
        decision,
    )
    .expect_err("continuation documents must have a hard size bound");
    assert_eq!(size_error.code(), ErrorCode::ContinuationInvalid);
}

#[test]
fn explicit_schema_migration_records_the_recognized_addition() {
    let (_tempdir, path) = store_path();
    let (_engine, _workspace) = create_workspace(&path);
    let connection = Connection::open(&path).expect("raw connection");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             DROP TABLE continuation_rotation_result;
             DROP TABLE continuation_rotation_runtime;
             DROP TABLE continuation_rotation;
             DROP TABLE continuation_scope_runtime;
             DROP TABLE continuation_project_policy;",
        )
        .expect("drop continuation tables");
    drop(connection);

    let migrated = Engine::migrate_continuation_rotation_schema(&path).expect("migrate schema");
    assert!(migrated.migrated);
    assert_eq!(migrated.added_schema_objects.len(), 5);
    let migration_id = migrated
        .migration
        .as_ref()
        .expect("migration receipt")
        .migration_id;
    let replay = Engine::migrate_continuation_rotation_schema(&path).expect("idempotent migration");
    assert!(!replay.migrated);
    assert!(replay.added_schema_objects.is_empty());
    assert_eq!(
        replay
            .migration
            .as_ref()
            .expect("replayed migration receipt")
            .migration_id,
        migration_id
    );
    Engine::open(&path).expect("open migrated store");
}

#[test]
fn partial_continuation_schema_fails_closed_without_mutation() {
    let (_tempdir, path) = store_path();
    let (_engine, _workspace) = create_workspace(&path);
    let connection = Connection::open(&path).expect("raw connection");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;
             DROP TABLE continuation_rotation_result;
             DROP TABLE continuation_rotation_runtime;
             DROP TABLE continuation_rotation;",
        )
        .expect("create partial continuation schema");
    drop(connection);

    let error = Engine::migrate_continuation_rotation_schema(&path)
        .expect_err("partial continuation schema must fail closed");
    assert_eq!(error.code(), ErrorCode::StoreBootstrapInvalid);

    let connection = Connection::open(&path).expect("reopen raw connection");
    let continuation_tables = connection
        .query_row(
            "SELECT count(*) FROM sqlite_schema
             WHERE type = 'table' AND name IN (
                'continuation_project_policy',
                'continuation_scope_runtime',
                'continuation_rotation',
                'continuation_rotation_runtime',
                'continuation_rotation_result'
             )",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("continuation table count");
    assert_eq!(continuation_tables, 2);
}
