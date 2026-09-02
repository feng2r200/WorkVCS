use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ChangeOperationSubject, CommitId, Engine, EntityId, RecordCreateCommit, RecordCreateOptions,
    RecordRelationCreateCommit, RecordRelationCreateOptions, RecordTransitionCommit,
    RecordTransitionOptions, ResolvedWhyQuerySubject, StoreInitOptions, WhyDeferredRelationFamily,
    WhyEntityKind, WhyEvolutionSubjectDetail, WhyQueryOptions, WhyQueryResult, WhyQueryTarget,
    WhyRelationDirection, WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo,
    WorkspaceInitOptions,
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

fn create_validated_then_invalidated_assumption(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> (
    RecordCreateCommit,
    RecordTransitionCommit,
    RecordTransitionCommit,
) {
    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "The cache is always fresh",
            )
            .expect("assumption options"),
        )
        .expect("create assumption");
    let validated = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                workspace.initial_branch_id,
                assumption.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "Freshness check passed",
            )
            .expect("validate options"),
        )
        .expect("validate assumption");
    let invalidated = engine
        .transition_record(
            RecordTransitionOptions::invalidate_assumption(
                workspace.initial_branch_id,
                validated.commit_id,
                assumption.record_entity_id,
                validated.record_entity_version_id,
                "Freshness check failed later",
            )
            .expect("invalidate options"),
        )
        .expect("invalidate assumption");
    (assumption, validated, invalidated)
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
    assert!(why.deferred_relation_families.is_empty());
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
            entity_kind: WhyEntityKind::Record,
        }
    );
    assert_deferred_families(&finding_why);
    assert!(finding_why.evolution_change_operations.is_empty());
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
            entity_kind: WhyEntityKind::Record,
        }
    );
    assert_eq!(
        assumption_why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(assumption_why.evolution_change_operations.len(), 1);
    let operation = &assumption_why.evolution_change_operations[0];
    assert_eq!(operation.commit_id, invalidated.commit_id);
    assert_eq!(operation.changeset_id, invalidated.changeset_id);
    assert_eq!(operation.changeset_operation_type, "entity.transition");
    assert_eq!(operation.operation_id, invalidated.operation_id);
    assert_eq!(operation.ordinal, 0);
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Entity(assumption.record_entity_id)
    );
    let Some(WhyEvolutionSubjectDetail::Entity(detail)) = &operation.subject_detail else {
        panic!("expected assumption entity subject detail")
    };
    assert_eq!(detail.entity_kind, WhyEntityKind::Record);
    assert_eq!(
        detail.entity_version_id,
        invalidated.record_entity_version_id
    );
    assert_eq!(
        detail.statement.as_deref(),
        Some("Serialized writes are sufficient")
    );
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
    assert_eq!(
        before_relation.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(before_relation.evolution_change_operations.len(), 1);
}

#[test]
fn why_reports_operation_local_entity_detail_for_multiple_direct_assumption_changes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (assumption, validated, invalidated) =
        create_validated_then_invalidated_assumption(&mut engine, &workspace);

    let why = why_commit(&engine, invalidated.commit_id, assumption.record_entity_id);
    assert_eq!(
        why.subject,
        ResolvedWhyQuerySubject::Entity {
            entity_id: assumption.record_entity_id,
            entity_version_id: invalidated.record_entity_version_id,
            entity_kind: WhyEntityKind::Record,
        }
    );
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 2);

    let newest = &why.evolution_change_operations[0];
    assert_eq!(newest.commit_id, invalidated.commit_id);
    assert_eq!(newest.changeset_id, invalidated.changeset_id);
    assert_eq!(newest.operation_id, invalidated.operation_id);
    assert_eq!(newest.changeset_operation_type, "entity.transition");
    assert_eq!(
        newest.subject,
        ChangeOperationSubject::Entity(assumption.record_entity_id)
    );
    let Some(WhyEvolutionSubjectDetail::Entity(detail)) = &newest.subject_detail else {
        panic!("expected newest operation entity detail")
    };
    assert_eq!(detail.entity_kind, WhyEntityKind::Record);
    assert_eq!(
        detail.entity_version_id,
        invalidated.record_entity_version_id
    );
    assert_eq!(
        detail.statement.as_deref(),
        Some("The cache is always fresh")
    );

    let older = &why.evolution_change_operations[1];
    assert_eq!(older.commit_id, validated.commit_id);
    assert_eq!(older.changeset_id, validated.changeset_id);
    assert_eq!(older.operation_id, validated.operation_id);
    assert_eq!(older.changeset_operation_type, "entity.transition");
    assert_eq!(
        older.subject,
        ChangeOperationSubject::Entity(assumption.record_entity_id)
    );
    let Some(WhyEvolutionSubjectDetail::Entity(detail)) = &older.subject_detail else {
        panic!("expected older operation entity detail")
    };
    assert_eq!(detail.entity_kind, WhyEntityKind::Record);
    assert_eq!(detail.entity_version_id, validated.record_entity_version_id);
    assert_eq!(
        detail.statement.as_deref(),
        Some("The cache is always fresh")
    );
}
