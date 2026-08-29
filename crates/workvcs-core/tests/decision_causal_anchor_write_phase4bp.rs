use tempfile::TempDir;
use workvcs_core::{
    DecisionRecordSupersedeOptions, Engine, RecordCreateOptions, StoreInitOptions, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn create_store() -> (TempDir, Engine, WorkspaceInfo) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("phase4bp-decision-anchor-write-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (tempdir, engine, workspace)
}

#[test]
fn decision_supersede_because_record_writes_changeset_causal_anchor() {
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

    let superseded = engine
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

    let summary = engine
        .changeset(superseded.changeset_id)
        .expect("changeset summary");
    assert_eq!(summary.causal_anchor_count, 1);

    let anchors = engine
        .changeset_causal_anchors(superseded.changeset_id)
        .expect("changeset causal anchors");
    assert_eq!(anchors.anchors.len(), 1);
    assert_eq!(anchors.anchors[0].ordinal, 0);
    assert_eq!(
        anchors.anchors[0].anchor_object_id,
        finding.record_entity_id.to_string()
    );
    assert_eq!(anchors.anchors[0].anchor_object_kind, "entity");
}
