use rusqlite::{Connection, params};
use std::path::Path;
use std::time::Instant;
use workvcs_core::{
    BranchId, CanonicalValue, ChangeSetId, CommitId, Engine, EntityId, EntityTransitionOptions,
    EntityVersionId, OperationId, RelationId, RelationVersionId, StoreInitOptions, WorkState,
    WorkspaceId, WorkspaceInitOptions, canonical_bytes, work_state_mapping_digest,
};

fn state(revision: usize) -> CanonicalValue {
    CanonicalValue::object(vec![
        (
            "revision".to_owned(),
            CanonicalValue::safe_integer(revision as i64).expect("revision"),
        ),
        (
            "title".to_owned(),
            CanonicalValue::String("integrity-replay-scale".to_owned()),
        ),
    ])
    .expect("state")
}

fn transition_payload(
    entity_id: EntityId,
    before_entity_version_id: EntityVersionId,
    after_entity_version_id: EntityVersionId,
) -> String {
    let value = CanonicalValue::object(vec![
        (
            "after_entity_version_id".to_owned(),
            CanonicalValue::String(after_entity_version_id.to_string()),
        ),
        (
            "before_entity_version_id".to_owned(),
            CanonicalValue::String(before_entity_version_id.to_string()),
        ),
        (
            "entity_id".to_owned(),
            CanonicalValue::String(entity_id.to_string()),
        ),
    ])
    .expect("transition payload");
    String::from_utf8(canonical_bytes(&value).expect("canonical payload")).expect("UTF-8 payload")
}

#[allow(clippy::too_many_arguments)]
fn append_transitions(
    path: &Path,
    workspace_id: WorkspaceId,
    branch_id: BranchId,
    entity_id: EntityId,
    version_a: EntityVersionId,
    version_b: EntityVersionId,
    mut parent_commit_id: CommitId,
    mut current_version_id: EntityVersionId,
    current_commit_count: usize,
    target_commit_count: usize,
) -> (CommitId, EntityVersionId) {
    let mut connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable foreign keys");
    let mut committed_at_us = connection
        .query_row(
            "SELECT max(committed_at_us) FROM workstate_commit",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("max commit time");
    let transaction = connection.transaction().expect("transaction");

    for _ in current_commit_count..target_commit_count {
        committed_at_us += 1;
        let after_version_id = if current_version_id == version_a {
            version_b
        } else {
            version_a
        };
        let changeset_id = ChangeSetId::new_v7();
        let commit_id = CommitId::new_v7();
        let operation_id = OperationId::new_v7();
        let payload = transition_payload(entity_id, current_version_id, after_version_id);
        let state_digest = work_state_mapping_digest(
            &WorkState::new(
                [(entity_id, after_version_id)],
                std::iter::empty::<(RelationId, RelationVersionId)>(),
            )
            .expect("WorkState"),
        );

        transaction
            .execute(
                "INSERT INTO changeset(
                    changeset_id, workspace_id, operation_type, operation_schema_version,
                    operation_payload_json, rationale_json, origin_session_id, created_at_us
                 ) VALUES (?1, ?2, 'entity.transition', 1, ?3, '{}', NULL, ?4)",
                params![
                    &changeset_id.raw_bytes()[..],
                    &workspace_id.raw_bytes()[..],
                    payload,
                    committed_at_us,
                ],
            )
            .expect("insert changeset");
        transaction
            .execute(
                "INSERT INTO change_operation(
                    operation_id, changeset_id, ordinal, subject_family,
                    subject_object_id, operation_payload_json
                 ) VALUES (?1, ?2, 0, 'entity', ?3, ?4)",
                params![
                    &operation_id.raw_bytes()[..],
                    &changeset_id.raw_bytes()[..],
                    &entity_id.raw_bytes()[..],
                    payload,
                ],
            )
            .expect("insert operation");
        transaction
            .execute(
                "INSERT INTO entity_membership_change(
                    operation_id, entity_id, before_entity_version_id,
                    after_entity_version_id, field_delta_json
                 ) VALUES (?1, ?2, ?3, ?4, '{}')",
                params![
                    &operation_id.raw_bytes()[..],
                    &entity_id.raw_bytes()[..],
                    &current_version_id.raw_bytes()[..],
                    &after_version_id.raw_bytes()[..],
                ],
            )
            .expect("insert membership change");
        transaction
            .execute(
                "INSERT INTO workstate_commit(
                    commit_id, workspace_id, changeset_id, commit_kind,
                    state_digest, committed_at_us
                 ) VALUES (?1, ?2, ?3, 'normal', ?4, ?5)",
                params![
                    &commit_id.raw_bytes()[..],
                    &workspace_id.raw_bytes()[..],
                    &changeset_id.raw_bytes()[..],
                    &state_digest.as_bytes()[..],
                    committed_at_us,
                ],
            )
            .expect("insert commit");
        transaction
            .execute(
                "INSERT INTO commit_parent(
                    commit_id, parent_ordinal, parent_role, parent_commit_id
                 ) VALUES (?1, 0, 'primary', ?2)",
                params![
                    &commit_id.raw_bytes()[..],
                    &parent_commit_id.raw_bytes()[..],
                ],
            )
            .expect("insert parent");

        parent_commit_id = commit_id;
        current_version_id = after_version_id;
    }
    transaction
        .execute(
            "UPDATE branch SET head_commit_id = ?1 WHERE branch_id = ?2",
            params![
                &parent_commit_id.raw_bytes()[..],
                &branch_id.raw_bytes()[..]
            ],
        )
        .expect("move branch");
    transaction.commit().expect("commit fixture");
    (parent_commit_id, current_version_id)
}

#[test]
#[ignore = "opt-in fixed-state history scale evidence"]
fn integrity_replay_scales_across_500_1000_and_5000_commits() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("integrity-replay-scale").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    let first = engine
        .commit_entity_transition(
            EntityTransitionOptions::create(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "generic_record",
                state(0),
            )
            .expect("first transition options"),
        )
        .expect("first transition");
    let second = engine
        .commit_entity_transition(
            EntityTransitionOptions::update(
                workspace.initial_branch_id,
                first.commit_id,
                first.entity_id,
                first.entity_version_id,
                state(1),
            )
            .expect("second transition options"),
        )
        .expect("second transition");
    drop(engine);

    let mut current_commit_count = 3;
    let mut head_commit_id = second.commit_id;
    let mut current_version_id = second.entity_version_id;
    for target_commit_count in [500, 1_000, 5_000] {
        (head_commit_id, current_version_id) = append_transitions(
            &path,
            workspace.workspace_id,
            workspace.initial_branch_id,
            first.entity_id,
            first.entity_version_id,
            second.entity_version_id,
            head_commit_id,
            current_version_id,
            current_commit_count,
            target_commit_count,
        );
        current_commit_count = target_commit_count;

        let engine = Engine::open_readonly(&path).expect("readonly engine");
        let started = Instant::now();
        let report = engine.validate_integrity().expect("integrity");
        let elapsed = started.elapsed();
        assert_eq!(report.checked_commits, target_commit_count);
        eprintln!(
            "integrity_replay_scale commits={target_commit_count} elapsed_ms={}",
            elapsed.as_millis()
        );
    }
}
