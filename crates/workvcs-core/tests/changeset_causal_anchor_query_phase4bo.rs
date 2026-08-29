use rusqlite::{Connection, params};
use std::path::Path;
use tempfile::TempDir;
use workvcs_core::{
    Engine, EntityId, ErrorCategory, RelationId, StoreInitOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bo-changeset-anchor-query-store").expect("store options"),
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
fn changeset_causal_anchors_are_listed_in_ordinal_order() {
    let (tempdir, engine, workspace) = create_store();
    let entity_id = EntityId::new_v7();
    let relation_id = RelationId::new_v7();
    let connection = raw_connection(tempdir.path().join("workvcs.sqlite").as_path());
    connection
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES(?1, 'entity', 1)",
            params![&entity_id.raw_bytes()[..]],
        )
        .expect("insert entity object identity");
    connection
        .execute(
            "INSERT INTO object_identity(object_id, object_kind, created_at_us)
             VALUES(?1, 'relation', 2)",
            params![&relation_id.raw_bytes()[..]],
        )
        .expect("insert relation object identity");
    connection
        .execute(
            "INSERT INTO changeset_causal_anchor(changeset_id, ordinal, anchor_object_id)
             VALUES(?1, 1, ?2)",
            params![
                &workspace.genesis_changeset_id.raw_bytes()[..],
                &relation_id.raw_bytes()[..],
            ],
        )
        .expect("insert second anchor");
    connection
        .execute(
            "INSERT INTO changeset_causal_anchor(changeset_id, ordinal, anchor_object_id)
             VALUES(?1, 0, ?2)",
            params![
                &workspace.genesis_changeset_id.raw_bytes()[..],
                &entity_id.raw_bytes()[..],
            ],
        )
        .expect("insert first anchor");

    let summary = engine
        .changeset(workspace.genesis_changeset_id)
        .expect("changeset summary");
    assert_eq!(summary.causal_anchor_count, 2);

    let anchors = engine
        .changeset_causal_anchors(workspace.genesis_changeset_id)
        .expect("changeset anchors");
    assert_eq!(anchors.workspace_id, workspace.workspace_id);
    assert_eq!(anchors.changeset_id, workspace.genesis_changeset_id);
    assert_eq!(anchors.anchors.len(), 2);
    assert_eq!(anchors.anchors[0].ordinal, 0);
    assert_eq!(anchors.anchors[0].anchor_object_id, entity_id.to_string());
    assert_eq!(anchors.anchors[0].anchor_object_kind, "entity");
    assert_eq!(anchors.anchors[1].ordinal, 1);
    assert_eq!(anchors.anchors[1].anchor_object_id, relation_id.to_string());
    assert_eq!(anchors.anchors[1].anchor_object_kind, "relation");
}

#[test]
fn changeset_causal_anchor_query_rejects_non_uuidv7_anchor_id() {
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
        .changeset_causal_anchors(workspace.genesis_changeset_id)
        .expect_err("invalid anchor id should fail query");
    assert_eq!(error.category(), ErrorCategory::Query);
    assert!(
        error
            .to_string()
            .contains("changeset_causal_anchor.anchor_object_id is not a UUIDv7 value")
    );
}
