use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CommitId, Engine, EntityId, RecordCreateCommit, RecordCreateOptions,
    RecordRelationCreateCommit, RecordRelationCreateOptions, RecordTransitionCommit,
    RecordTransitionOptions, ResolvedWhyQuerySubject, StoreInitOptions, WhyDeferredRelationFamily,
    WhyEntityKind, WhyQueryOptions, WhyQueryResult, WhyQueryTarget, WhyRelationDirection,
    WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("phase3ax-why-record-relations-store").expect("store options"),
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

fn create_invalidated_assumption_with_finding(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> (
    RecordCreateCommit,
    RecordCreateCommit,
    RecordTransitionCommit,
    RecordRelationCreateCommit,
) {
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Serialized writes are sufficient",
            )
            .expect("assumption options"),
        )
        .expect("create assumption");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                assumption.commit_id,
                "Concurrent writer test failed",
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
                "Concurrent writer test failed",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate assumption");
    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::invalidates(
                workspace.initial_branch_id,
                invalidated.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "Finding invalidates the assumption",
            )
            .expect("relation options"),
        )
        .expect("create invalidates relation");
    (assumption, finding, invalidated, relation)
}

fn why_commit(engine: &Engine, commit_id: CommitId, subject_entity_id: EntityId) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::new(
            WhyQueryTarget::commit(commit_id),
            subject_entity_id,
        ))
        .expect("why commit")
}

fn edge_facts(
    why: &WhyQueryResult,
) -> BTreeSet<(
    WhyRelationKind,
    WhyRelationDirection,
    WhyRelationEndpoint,
    WhyRelationEndpoint,
)> {
    why.relation_edges
        .iter()
        .map(|edge| (edge.relation_kind, edge.direction, edge.source, edge.target))
        .collect()
}

fn record_endpoint(record_entity_id: EntityId) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(record_entity_id, WhyEntityKind::Record)
}

fn assert_deferred_families(why: &WhyQueryResult) {
    assert_eq!(
        why.deferred_relation_families,
        vec![
            WhyDeferredRelationFamily::Evolution,
            WhyDeferredRelationFamily::Epistemic,
        ]
    );
}

#[test]
fn why_reports_record_invalidates_edge_for_both_endpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (assumption, finding, invalidated, relation) =
        create_invalidated_assumption_with_finding(&mut engine, &workspace);

    let finding_why = why_commit(&engine, relation.commit_id, finding.record_entity_id);
    assert_eq!(
        finding_why.subject,
        ResolvedWhyQuerySubject::Entity {
            entity_id: finding.record_entity_id,
            entity_version_id: finding.record_entity_version_id,
        }
    );
    assert_deferred_families(&finding_why);
    assert_eq!(
        edge_facts(&finding_why),
        BTreeSet::from([(
            WhyRelationKind::RecordInvalidates,
            WhyRelationDirection::Outgoing,
            record_endpoint(finding.record_entity_id),
            record_endpoint(assumption.record_entity_id),
        )])
    );
    assert_eq!(
        finding_why.relation_edges[0].relation_id,
        relation.relation_id
    );

    let assumption_why = why_commit(&engine, relation.commit_id, assumption.record_entity_id);
    assert_eq!(
        assumption_why.subject,
        ResolvedWhyQuerySubject::Entity {
            entity_id: assumption.record_entity_id,
            entity_version_id: invalidated.record_entity_version_id,
        }
    );
    assert_deferred_families(&assumption_why);
    assert_eq!(
        edge_facts(&assumption_why),
        BTreeSet::from([(
            WhyRelationKind::RecordInvalidates,
            WhyRelationDirection::Incoming,
            record_endpoint(finding.record_entity_id),
            record_endpoint(assumption.record_entity_id),
        )])
    );
    assert_eq!(
        assumption_why.relation_edges[0].relation_id,
        relation.relation_id
    );

    let before_relation = why_commit(&engine, invalidated.commit_id, assumption.record_entity_id);
    assert!(before_relation.relation_edges.is_empty());
}
