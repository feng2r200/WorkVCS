use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, EntityId, KnowledgeCreateOptions, KnowledgeRelationCreateOptions,
    KnowledgeTransitionOptions, RecordCreateOptions, RecordKnowledgeRelationCreateOptions,
    RecordRelationCreateOptions, RecordTransitionOptions, StoreInitOptions, WhyEntityKind,
    WhyQueryOptions, WhyQueryTarget, WhyRelationDirection, WhyRelationEndpoint, WhyRelationKind,
    WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4nc-why-epistemic-explanation-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn record_endpoint(record_entity_id: EntityId) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(record_entity_id, WhyEntityKind::Record)
}

fn knowledge_endpoint(knowledge_entity_id: EntityId) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(knowledge_entity_id, WhyEntityKind::Knowledge)
}

#[test]
fn why_explains_record_supports_knowledge_with_statements() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge_statement = "The release matrix remains false until every blocking gate passes";
    let finding_statement = "The current matrix still marks the context and why row Partial";

    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                knowledge_statement,
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                finding_statement,
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
                "The finding supports the release matrix statement",
            )
            .expect("supports knowledge options"),
        )
        .expect("support knowledge");

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            knowledge.knowledge_entity_id,
        ))
        .expect("why knowledge");

    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(why.epistemic_explanations.len(), 1);
    assert!(why.deferred_relation_families.is_empty());
    let edge = &why.relation_edges[0];
    let explanation = &why.epistemic_explanations[0];
    assert_eq!(explanation.relation_kind, WhyRelationKind::RecordSupports);
    assert_eq!(explanation.direction, WhyRelationDirection::Incoming);
    assert_eq!(explanation.relation_id, edge.relation_id);
    assert_eq!(explanation.relation_version_id, edge.relation_version_id);
    assert_eq!(explanation.relation_id, relation.relation_id);
    assert_eq!(
        explanation.source,
        record_endpoint(finding.record_entity_id)
    );
    assert_eq!(
        explanation.target,
        knowledge_endpoint(knowledge.knowledge_entity_id)
    );
    assert_eq!(explanation.source_statement, finding_statement);
    assert_eq!(explanation.target_statement, knowledge_statement);
    assert_eq!(explanation.state_digest, relation.relation_state_digest);
}

#[test]
fn why_explains_record_validates_record_with_statements() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let assumption_statement = "Serialized writes are sufficient for V0.1";
    let finding_statement = "Concurrent writer validation passed under the serialized executor";

    let assumption = engine
        .create_record(
            RecordCreateOptions::assumption(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
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
    let validated = engine
        .transition_record(
            RecordTransitionOptions::validate_assumption(
                workspace.initial_branch_id,
                finding.commit_id,
                assumption.record_entity_id,
                assumption.record_entity_version_id,
                "Concurrent writer validation passed",
            )
            .expect("validate assumption options"),
        )
        .expect("validate assumption");
    let relation = engine
        .create_record_relation(
            RecordRelationCreateOptions::validates(
                workspace.initial_branch_id,
                validated.commit_id,
                finding.record_entity_id,
                assumption.record_entity_id,
                "The finding validates the assumption",
            )
            .expect("validates relation options"),
        )
        .expect("validate record");

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            assumption.record_entity_id,
        ))
        .expect("why assumption");

    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(why.epistemic_explanations.len(), 1);
    let explanation = &why.epistemic_explanations[0];
    assert_eq!(explanation.relation_kind, WhyRelationKind::RecordValidates);
    assert_eq!(explanation.direction, WhyRelationDirection::Incoming);
    assert_eq!(explanation.relation_id, relation.relation_id);
    assert_eq!(
        explanation.source,
        record_endpoint(finding.record_entity_id)
    );
    assert_eq!(
        explanation.target,
        record_endpoint(assumption.record_entity_id)
    );
    assert_eq!(explanation.source_statement, finding_statement);
    assert_eq!(explanation.target_statement, assumption_statement);
    assert_eq!(explanation.state_digest, relation.relation_state_digest);
}

#[test]
fn why_explains_record_contradicts_knowledge_with_statements() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge_statement = "Validation output is deterministic across machines";
    let finding_statement = "The observed output includes machine-local temporary paths";

    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                knowledge_statement,
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                finding_statement,
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let relation = engine
        .create_record_knowledge_relation(
            RecordKnowledgeRelationCreateOptions::contradicts(
                workspace.initial_branch_id,
                finding.commit_id,
                finding.record_entity_id,
                knowledge.knowledge_entity_id,
                "The finding contradicts the knowledge statement",
            )
            .expect("contradicts knowledge options"),
        )
        .expect("contradict knowledge");

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            knowledge.knowledge_entity_id,
        ))
        .expect("why knowledge");

    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(why.epistemic_explanations.len(), 1);
    let explanation = &why.epistemic_explanations[0];
    assert_eq!(
        explanation.relation_kind,
        WhyRelationKind::RecordContradicts
    );
    assert_eq!(explanation.direction, WhyRelationDirection::Incoming);
    assert_eq!(explanation.relation_id, relation.relation_id);
    assert_eq!(
        explanation.source,
        record_endpoint(finding.record_entity_id)
    );
    assert_eq!(
        explanation.target,
        knowledge_endpoint(knowledge.knowledge_entity_id)
    );
    assert_eq!(explanation.source_statement, finding_statement);
    assert_eq!(explanation.target_statement, knowledge_statement);
    assert_eq!(explanation.state_digest, relation.relation_state_digest);
}

#[test]
fn why_explains_record_invalidates_knowledge_with_statements() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let knowledge_statement = "Context packets should always include stale Knowledge";
    let finding_statement = "Current context output excludes invalidated Knowledge";

    let knowledge = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                knowledge_statement,
            )
            .expect("knowledge options"),
        )
        .expect("create knowledge");
    let finding = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                knowledge.commit_id,
                finding_statement,
            )
            .expect("finding options"),
        )
        .expect("create finding");
    let invalidated = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::invalidate(
                workspace.initial_branch_id,
                finding.commit_id,
                knowledge.knowledge_entity_id,
                knowledge.knowledge_entity_version_id,
                "Current context output excludes invalidated Knowledge",
            )
            .expect("invalidate knowledge options"),
        )
        .expect("invalidate knowledge");
    let relation = engine
        .create_record_knowledge_relation(
            RecordKnowledgeRelationCreateOptions::invalidates(
                workspace.initial_branch_id,
                invalidated.commit_id,
                finding.record_entity_id,
                knowledge.knowledge_entity_id,
                "The finding invalidates the knowledge statement",
            )
            .expect("invalidates knowledge options"),
        )
        .expect("invalidate knowledge");

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            knowledge.knowledge_entity_id,
        ))
        .expect("why knowledge");

    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(why.epistemic_explanations.len(), 1);
    let explanation = &why.epistemic_explanations[0];
    assert_eq!(
        explanation.relation_kind,
        WhyRelationKind::RecordInvalidates
    );
    assert_eq!(explanation.direction, WhyRelationDirection::Incoming);
    assert_eq!(explanation.relation_id, relation.relation_id);
    assert_eq!(
        explanation.source,
        record_endpoint(finding.record_entity_id)
    );
    assert_eq!(
        explanation.target,
        knowledge_endpoint(knowledge.knowledge_entity_id)
    );
    assert_eq!(explanation.source_statement, finding_statement);
    assert_eq!(explanation.target_statement, knowledge_statement);
    assert_eq!(explanation.state_digest, relation.relation_state_digest);
}

#[test]
fn why_does_not_explain_non_epistemic_knowledge_supersedes() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prior = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Context packets should be transient only",
            )
            .expect("prior knowledge options"),
        )
        .expect("create prior knowledge");
    let replacement = engine
        .create_knowledge(
            KnowledgeCreateOptions::new(
                workspace.initial_branch_id,
                prior.commit_id,
                "Context packets can be saved as immutable snapshots",
            )
            .expect("replacement knowledge options"),
        )
        .expect("create replacement knowledge");
    let superseded_prior = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::supersede(
                workspace.initial_branch_id,
                replacement.commit_id,
                prior.knowledge_entity_id,
                prior.knowledge_entity_version_id,
                "Snapshots preserve exact resolver output",
            )
            .expect("supersede options"),
        )
        .expect("supersede prior knowledge");
    let relation = engine
        .create_knowledge_relation(
            KnowledgeRelationCreateOptions::supersedes(
                workspace.initial_branch_id,
                superseded_prior.commit_id,
                replacement.knowledge_entity_id,
                prior.knowledge_entity_id,
                "The new Knowledge replaces the prior statement",
            )
            .expect("knowledge relation options"),
        )
        .expect("create knowledge supersedes relation");

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            prior.knowledge_entity_id,
        ))
        .expect("why prior knowledge");

    assert_eq!(why.relation_edges.len(), 1);
    assert_eq!(
        why.relation_edges[0].relation_kind,
        WhyRelationKind::KnowledgeSupersedes
    );
    assert!(why.epistemic_explanations.is_empty());
}
