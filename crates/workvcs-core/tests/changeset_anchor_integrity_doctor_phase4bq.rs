use rusqlite::{Connection, params};
use std::path::Path;
use tempfile::TempDir;
use workvcs_core::{
    DecisionRecordSupersedeOptions, Engine, ErrorCategory, RecordCreateOptions, StoreInitOptions,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bq-changeset-anchor-doctor-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (tempdir, engine, workspace)
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable raw foreign keys");
    connection
}

#[test]
fn doctor_counts_decision_supersede_causal_anchor() {
    let (_tempdir, mut engine, workspace) = create_store();
    let prior = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use optimistic writes",
            )
            .expect("prior decision options"),
        )
        .expect("create prior decision");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                prior.commit_id,
                "Concurrent write tests fail without serialization",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let replacement = engine
        .create_record(
            RecordCreateOptions::decision(
                workspace.initial_branch_id,
                finding.commit_id,
                "Use serialized writes",
            )
            .expect("replacement decision options"),
        )
        .expect("create replacement decision");

    engine
        .supersede_decision_record(
            DecisionRecordSupersedeOptions::new(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "Finding caused the replacement decision",
            )
            .expect("supersede options")
            .with_causal_record(finding.record_entity_id),
        )
        .expect("supersede decision");

    let report = engine.validate_integrity().expect("integrity");
    assert_eq!(report.checked_changeset_causal_anchors, 1);
}

#[test]
fn doctor_rejects_invalid_changeset_causal_anchor_id() {
    let (tempdir, engine, workspace) = create_store();
    let invalid_object_id = [0u8; 16];
    let connection = raw_connection(tempdir.path().join("workvcs.sqlite").as_path());
    connection
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES(?1, 'entity', 1)",
            params![&invalid_object_id[..]],
        )
        .expect("insert invalid object identity");
    connection
        .execute(
            "INSERT INTO changeset_causal_anchor(changeset_id, ordinal, anchor_object_id)
             VALUES(?1, 0, ?2)",
            params![
                &workspace.genesis_changeset_id.raw_bytes()[..],
                &invalid_object_id[..],
            ],
        )
        .expect("insert invalid anchor");

    let error = engine
        .validate_integrity()
        .expect_err("invalid anchor id should fail integrity");
    assert_eq!(error.category(), ErrorCategory::Integrity);
    assert!(
        error
            .to_string()
            .contains("changeset_causal_anchor.anchor_object_id is not a UUIDv7 value")
    );
}
