use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    ChangeOperationSubject, Engine, EntityId, KnowledgeCreateCommit, KnowledgeCreateOptions,
    KnowledgeRelationCreateCommit, KnowledgeRelationCreateOptions, KnowledgeRelationRemoveOptions,
    KnowledgeRelationRestoreOptions, KnowledgeTransitionOptions, RecordCreateCommit,
    RecordCreateOptions, RecordKnowledgeRelationCreateCommit, RecordKnowledgeRelationCreateOptions,
    RecordKnowledgeRelationRemoveOptions, RecordKnowledgeRelationRestoreOptions, StoreInitOptions,
    WhyDeferredRelationFamily, WhyEntityKind, WhyEvolutionChangeOperation,
    WhyEvolutionSubjectDetail, WhyQueryOptions, WhyQueryResult, WhyQueryTarget,
    WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4nm-why-knowledge-relation-endpoint-evolution-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_record_to_knowledge_support(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> (
    KnowledgeCreateCommit,
    RecordCreateCommit,
    RecordKnowledgeRelationCreateCommit,
) {
    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Knowledge endpoint should explain relation removal",
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                "Finding supports the Knowledge endpoint",
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let relation = engine
        .create_record_knowledge_relation(
            RecordKnowledgeRelationCreateOptions::supports(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                knowledge.knowledge_entity_id,
                "The Finding supports the Knowledge statement",
            )
            .expect("supports knowledge options"),
        )
        .expect("create record knowledge relation");
    (knowledge, finding, relation)
}

fn create_knowledge_supersedes_relation(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
) -> (
    KnowledgeCreateCommit,
    KnowledgeCreateCommit,
    KnowledgeRelationCreateCommit,
) {
    let prior = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Use the old context summary format",
            )
            .expect("prior knowledge options"),
        )
        .expect("create prior knowledge");
    let replacement = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                prior.commit_id,
                "Use the scoped context summary format",
            )
            .expect("replacement knowledge options"),
        )
        .expect("create replacement knowledge");
    let superseded = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::supersede(
                workspace.initial_branch_id,
                replacement.commit_id,
                prior.knowledge_entity_id,
                prior.knowledge_entity_version_id,
                "The scoped context summary format replaced it",
            )
            .expect("supersede prior options"),
        )
        .expect("supersede prior knowledge");
    let relation = engine
        .create_knowledge_relation(
            KnowledgeRelationCreateOptions::supersedes(
                workspace.initial_branch_id,
                superseded.commit_id,
                replacement.knowledge_entity_id,
                prior.knowledge_entity_id,
                "The replacement Knowledge supersedes the prior statement",
            )
            .expect("knowledge relation options"),
        )
        .expect("create knowledge relation");
    (prior, replacement, relation)
}

fn why_commit(
    engine: &Engine,
    commit_id: workvcs_core::CommitId,
    entity_id: EntityId,
) -> WhyQueryResult {
    engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(commit_id),
            entity_id,
        ))
        .expect("why commit")
}

fn record_endpoint(entity_id: EntityId) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(entity_id, WhyEntityKind::Record)
}

fn knowledge_endpoint(entity_id: EntityId) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(entity_id, WhyEntityKind::Knowledge)
}

fn assert_record_knowledge_relation_create_evolution(
    operation: &WhyEvolutionChangeOperation,
    relation: &RecordKnowledgeRelationCreateCommit,
    source_record_entity_id: EntityId,
    target_knowledge_entity_id: EntityId,
) {
    assert_eq!(operation.commit_id, relation.commit_id);
    assert_eq!(operation.changeset_id, relation.changeset_id);
    assert_eq!(operation.operation_id, relation.operation_id);
    assert_eq!(operation.changeset_operation_type, "record.relation.create");
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Relation(relation.relation_id)
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &operation.subject_detail else {
        panic!("expected record knowledge relation create detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::RecordSupports);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(detail.relation_label, None);
    assert_eq!(detail.source, record_endpoint(source_record_entity_id));
    assert_eq!(
        detail.target,
        knowledge_endpoint(target_knowledge_entity_id)
    );
    assert_eq!(detail.state_digest, relation.relation_state_digest);
}

fn assert_knowledge_relation_create_evolution(
    operation: &WhyEvolutionChangeOperation,
    relation: &KnowledgeRelationCreateCommit,
    replacement_knowledge_entity_id: EntityId,
    prior_knowledge_entity_id: EntityId,
) {
    assert_eq!(operation.commit_id, relation.commit_id);
    assert_eq!(operation.changeset_id, relation.changeset_id);
    assert_eq!(operation.operation_id, relation.operation_id);
    assert_eq!(
        operation.changeset_operation_type,
        "knowledge.relation.create"
    );
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Relation(relation.relation_id)
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &operation.subject_detail else {
        panic!("expected knowledge relation create detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::KnowledgeSupersedes);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(detail.relation_label, None);
    assert_eq!(
        detail.source,
        knowledge_endpoint(replacement_knowledge_entity_id)
    );
    assert_eq!(detail.target, knowledge_endpoint(prior_knowledge_entity_id));
    assert_eq!(detail.state_digest, relation.relation_state_digest);
}

#[test]
fn why_projects_created_record_knowledge_relation_as_knowledge_endpoint_evolution() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (knowledge, finding, relation) =
        create_record_to_knowledge_support(&mut engine, &workspace);

    let why = why_commit(&engine, relation.commit_id, knowledge.knowledge_entity_id);
    assert_eq!(why.relation_edges.len(), 1);
    assert!(why.causal_anchor_changesets.is_empty());
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 1);
    assert_record_knowledge_relation_create_evolution(
        &why.evolution_change_operations[0],
        &relation,
        finding.record_entity_id,
        knowledge.knowledge_entity_id,
    );
}

#[test]
fn why_projects_removed_record_knowledge_relation_as_knowledge_endpoint_evolution() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (knowledge, finding, relation) =
        create_record_to_knowledge_support(&mut engine, &workspace);

    let removed = engine
        .remove_record_knowledge_relation(
            RecordKnowledgeRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Remove the Record-to-Knowledge support edge",
            )
            .expect("remove options"),
        )
        .expect("remove record knowledge relation");

    let why = why_commit(&engine, removed.commit_id, knowledge.knowledge_entity_id);
    assert!(why.relation_edges.is_empty());
    assert!(why.causal_anchor_changesets.is_empty());
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 2);

    let operation = &why.evolution_change_operations[0];
    assert_eq!(operation.commit_id, removed.commit_id);
    assert_eq!(operation.changeset_id, removed.changeset_id);
    assert_eq!(operation.operation_id, removed.operation_id);
    assert_eq!(operation.changeset_operation_type, "record.relation.remove");
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Relation(relation.relation_id)
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &operation.subject_detail else {
        panic!("expected operation-local relation detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::RecordSupports);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(detail.relation_label, None);
    assert_eq!(detail.source, record_endpoint(finding.record_entity_id));
    assert_eq!(
        detail.target,
        knowledge_endpoint(knowledge.knowledge_entity_id)
    );
    assert_eq!(detail.state_digest, relation.relation_state_digest);

    assert_record_knowledge_relation_create_evolution(
        &why.evolution_change_operations[1],
        &relation,
        finding.record_entity_id,
        knowledge.knowledge_entity_id,
    );
}

#[test]
fn why_projects_restored_record_knowledge_relation_as_knowledge_endpoint_evolution() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (knowledge, finding, relation) =
        create_record_to_knowledge_support(&mut engine, &workspace);
    let removed = engine
        .remove_record_knowledge_relation(
            RecordKnowledgeRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Remove the Record-to-Knowledge support edge",
            )
            .expect("remove options"),
        )
        .expect("remove record knowledge relation");
    let restored = engine
        .restore_record_knowledge_relation(
            RecordKnowledgeRelationRestoreOptions::new(
                workspace.initial_branch_id,
                removed.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Restore the Record-to-Knowledge support edge",
            )
            .expect("restore options"),
        )
        .expect("restore record knowledge relation");

    let why = why_commit(&engine, restored.commit_id, knowledge.knowledge_entity_id);
    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 3);

    let newest = &why.evolution_change_operations[0];
    assert_eq!(newest.commit_id, restored.commit_id);
    assert_eq!(newest.operation_id, restored.operation_id);
    assert_eq!(newest.changeset_operation_type, "record.relation.restore");
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &newest.subject_detail else {
        panic!("expected restored relation detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::RecordSupports);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(detail.source, record_endpoint(finding.record_entity_id));
    assert_eq!(
        detail.target,
        knowledge_endpoint(knowledge.knowledge_entity_id)
    );

    let older = &why.evolution_change_operations[1];
    assert_eq!(older.commit_id, removed.commit_id);
    assert_eq!(older.operation_id, removed.operation_id);
    assert_eq!(older.changeset_operation_type, "record.relation.remove");

    assert_record_knowledge_relation_create_evolution(
        &why.evolution_change_operations[2],
        &relation,
        finding.record_entity_id,
        knowledge.knowledge_entity_id,
    );
}

#[test]
fn why_projects_created_knowledge_relation_as_replacement_endpoint_evolution() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (prior, replacement, relation) =
        create_knowledge_supersedes_relation(&mut engine, &workspace);

    let why = why_commit(&engine, relation.commit_id, replacement.knowledge_entity_id);
    assert_eq!(why.relation_edges.len(), 1);
    assert!(why.causal_anchor_changesets.is_empty());
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 1);
    assert_knowledge_relation_create_evolution(
        &why.evolution_change_operations[0],
        &relation,
        replacement.knowledge_entity_id,
        prior.knowledge_entity_id,
    );
}

#[test]
fn why_projects_removed_knowledge_relation_as_replacement_endpoint_evolution() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (prior, replacement, relation) =
        create_knowledge_supersedes_relation(&mut engine, &workspace);

    let removed = engine
        .remove_knowledge_relation(
            KnowledgeRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Remove the Knowledge supersedes edge",
            )
            .expect("remove options"),
        )
        .expect("remove knowledge relation");

    let why = why_commit(&engine, removed.commit_id, replacement.knowledge_entity_id);
    assert!(why.relation_edges.is_empty());
    assert!(why.causal_anchor_changesets.is_empty());
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 2);

    let operation = &why.evolution_change_operations[0];
    assert_eq!(operation.commit_id, removed.commit_id);
    assert_eq!(operation.changeset_id, removed.changeset_id);
    assert_eq!(operation.operation_id, removed.operation_id);
    assert_eq!(
        operation.changeset_operation_type,
        "knowledge.relation.remove"
    );
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Relation(relation.relation_id)
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &operation.subject_detail else {
        panic!("expected operation-local relation detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::KnowledgeSupersedes);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(detail.relation_label, None);
    assert_eq!(
        detail.source,
        knowledge_endpoint(replacement.knowledge_entity_id)
    );
    assert_eq!(detail.target, knowledge_endpoint(prior.knowledge_entity_id));
    assert_eq!(detail.state_digest, relation.relation_state_digest);

    assert_knowledge_relation_create_evolution(
        &why.evolution_change_operations[1],
        &relation,
        replacement.knowledge_entity_id,
        prior.knowledge_entity_id,
    );
}

#[test]
fn why_projects_removed_knowledge_relation_as_prior_endpoint_evolution() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (prior, replacement, relation) =
        create_knowledge_supersedes_relation(&mut engine, &workspace);

    let removed = engine
        .remove_knowledge_relation(
            KnowledgeRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Remove the Knowledge supersedes edge",
            )
            .expect("remove options"),
        )
        .expect("remove knowledge relation");

    let why = why_commit(&engine, removed.commit_id, prior.knowledge_entity_id);
    assert!(why.relation_edges.is_empty());
    assert!(why.causal_anchor_changesets.is_empty());
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 3);

    let operation = why
        .evolution_change_operations
        .iter()
        .find(|operation| operation.operation_id == removed.operation_id)
        .expect("removed relation operation");
    assert_eq!(operation.commit_id, removed.commit_id);
    assert_eq!(operation.changeset_id, removed.changeset_id);
    assert_eq!(
        operation.changeset_operation_type,
        "knowledge.relation.remove"
    );
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Relation(relation.relation_id)
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &operation.subject_detail else {
        panic!("expected operation-local relation detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::KnowledgeSupersedes);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(detail.relation_label, None);
    assert_eq!(
        detail.source,
        knowledge_endpoint(replacement.knowledge_entity_id)
    );
    assert_eq!(detail.target, knowledge_endpoint(prior.knowledge_entity_id));
    assert_eq!(detail.state_digest, relation.relation_state_digest);

    let create_operation = why
        .evolution_change_operations
        .iter()
        .find(|operation| operation.operation_id == relation.operation_id)
        .expect("created relation operation");
    assert_knowledge_relation_create_evolution(
        create_operation,
        &relation,
        replacement.knowledge_entity_id,
        prior.knowledge_entity_id,
    );
}

#[test]
fn why_projects_restored_knowledge_relation_as_replacement_endpoint_evolution() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let (prior, replacement, relation) =
        create_knowledge_supersedes_relation(&mut engine, &workspace);
    let removed = engine
        .remove_knowledge_relation(
            KnowledgeRelationRemoveOptions::new(
                workspace.initial_branch_id,
                relation.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Remove the Knowledge supersedes edge",
            )
            .expect("remove options"),
        )
        .expect("remove knowledge relation");
    let restored = engine
        .restore_knowledge_relation(
            KnowledgeRelationRestoreOptions::new(
                workspace.initial_branch_id,
                removed.commit_id,
                relation.relation_id,
                relation.relation_version_id,
                "Restore the Knowledge supersedes edge",
            )
            .expect("restore options"),
        )
        .expect("restore knowledge relation");

    let why = why_commit(&engine, restored.commit_id, replacement.knowledge_entity_id);
    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    assert_eq!(why.evolution_change_operations.len(), 3);

    let newest = &why.evolution_change_operations[0];
    assert_eq!(newest.commit_id, restored.commit_id);
    assert_eq!(newest.operation_id, restored.operation_id);
    assert_eq!(
        newest.changeset_operation_type,
        "knowledge.relation.restore"
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &newest.subject_detail else {
        panic!("expected restored relation detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::KnowledgeSupersedes);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(
        detail.source,
        knowledge_endpoint(replacement.knowledge_entity_id)
    );
    assert_eq!(detail.target, knowledge_endpoint(prior.knowledge_entity_id));

    let older = &why.evolution_change_operations[1];
    assert_eq!(older.commit_id, removed.commit_id);
    assert_eq!(older.operation_id, removed.operation_id);
    assert_eq!(older.changeset_operation_type, "knowledge.relation.remove");

    assert_knowledge_relation_create_evolution(
        &why.evolution_change_operations[2],
        &relation,
        replacement.knowledge_entity_id,
        prior.knowledge_entity_id,
    );
}
