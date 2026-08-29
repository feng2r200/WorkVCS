use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, RecordCreateCommit, RecordCreateOptions, RecordRelationCreateCommit,
    RecordRelationCreateOptions, RecordRelationListOptions, RecordRelationType,
    RecordTransitionOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3aw-record-relation-list-store").expect("store options"),
    )
    .expect("init engine")
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = init_engine(path);
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn invalidated_assumption_with_finding(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: workvcs_core::CommitId,
    assumption_statement: &str,
    finding_statement: &str,
) -> (
    RecordCreateCommit,
    RecordCreateCommit,
    workvcs_core::CommitId,
) {
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                head,
                assumption_statement,
            )
            .expect("assumption options"),
        )
        .expect("create assumption");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                assumption.commit_id,
                finding_statement,
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let invalidated = engine
        .transition_record(
            RecordTransitionOptions::invalidate_assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                finding_statement,
            )
            .expect("invalidate options"),
        )
        .expect("invalidate assumption");
    (assumption, finding, invalidated.commit_id)
}

fn link_invalidates(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    head: workvcs_core::CommitId,
    finding: &RecordCreateCommit,
    assumption: &RecordCreateCommit,
) -> RecordRelationCreateCommit {
    engine
        .create_record_relation(
            RecordRelationCreateOptions::invalidates(
                workspace.initial_branch_id,
                head,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Finding invalidates the assumption",
            )
            .expect("relation options"),
        )
        .expect("create invalidates relation")
}

#[test]
fn record_relations_at_lists_current_invalidates_edges_with_filters() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

    let (assumption_a, finding_a, invalidated_a_head) = invalidated_assumption_with_finding(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Serialized writes are sufficient",
        "Concurrent writer test failed",
    );
    let relation_a = link_invalidates(
        &mut engine,
        &workspace,
        invalidated_a_head,
        &finding_a,
        &assumption_a,
    );

    let empty_before_relation = engine
        .record_relations_at(RecordRelationListOptions::new(invalidated_a_head))
        .expect("list before relation");
    assert!(empty_before_relation.relations.is_empty());

    let (assumption_b, finding_b, invalidated_b_head) = invalidated_assumption_with_finding(
        &mut engine,
        &workspace,
        relation_a.commit_id,
        "The remote service is healthy",
        "Health check returned 503",
    );
    let relation_b = link_invalidates(
        &mut engine,
        &workspace,
        invalidated_b_head,
        &finding_b,
        &assumption_b,
    );

    let all = engine
        .record_relations_at(RecordRelationListOptions::new(relation_b.commit_id))
        .expect("list relations");
    assert_eq!(all.workspace_id, workspace.workspace_id);
    assert_eq!(all.commit_id, relation_b.commit_id);
    assert_eq!(all.relations.len(), 2);
    assert!(all.relations.windows(2).all(|window| {
        (
            window[0].relation_type,
            window[0].source_record_entity_id,
            window[0].target_record_entity_id,
            window[0].relation_id,
        ) <= (
            window[1].relation_type,
            window[1].source_record_entity_id,
            window[1].target_record_entity_id,
            window[1].relation_id,
        )
    }));
    assert!(all.relations.iter().all(|relation| {
        relation.relation_type == RecordRelationType::Invalidates
            && relation.state_digest == relation_a.relation_state_digest
    }));

    let source_filtered = engine
        .record_relations_at(
            RecordRelationListOptions::new(relation_b.commit_id)
                .with_source_record(finding_a.record_entity_id),
        )
        .expect("list by source");
    assert_eq!(source_filtered.relations.len(), 1);
    assert_eq!(
        source_filtered.relations[0].relation_id,
        relation_a.relation_id
    );

    let target_filtered = engine
        .record_relations_at(
            RecordRelationListOptions::new(relation_b.commit_id)
                .with_target_record(assumption_b.record_entity_id),
        )
        .expect("list by target");
    assert_eq!(target_filtered.relations.len(), 1);
    assert_eq!(
        target_filtered.relations[0].relation_id,
        relation_b.relation_id
    );

    let type_filtered = engine
        .record_relations_at(
            RecordRelationListOptions::new(relation_b.commit_id)
                .with_relation_type(RecordRelationType::Invalidates),
        )
        .expect("list by type");
    assert_eq!(type_filtered.relations, all.relations);
}
