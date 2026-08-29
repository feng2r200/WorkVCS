use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ErrorCode, KnowledgeCreateOptions, KnowledgeRelationCreateOptions, KnowledgeStatus,
    KnowledgeTransitionOptions, RecordRelationType, StoreInitOptions, WhyEntityKind,
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
        StoreInitOptions::new("phase3cf-knowledge-supersedes-relation-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn active_knowledge_can_supersede_prior_superseded_knowledge() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

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
    let superseded_prior = engine
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
                superseded_prior.commit_id,
                replacement.knowledge_entity_id,
                prior.knowledge_entity_id,
                "The new Knowledge replaces the prior statement",
            )
            .expect("knowledge relation options"),
        )
        .expect("create knowledge supersedes relation");

    assert_eq!(relation.workspace_id, workspace.workspace_id);
    assert_eq!(relation.branch_id, workspace.initial_branch_id);
    assert_eq!(relation.previous_head_commit_id, superseded_prior.commit_id);
    assert_eq!(relation.relation_type, RecordRelationType::Supersedes);
    assert_eq!(
        relation.replacement_knowledge_entity_id,
        replacement.knowledge_entity_id
    );
    assert_eq!(
        relation.prior_knowledge_entity_id,
        prior.knowledge_entity_id
    );

    let prior_snapshot = engine
        .knowledge_at(relation.commit_id, prior.knowledge_entity_id)
        .expect("prior after relation");
    assert_eq!(prior_snapshot.state.status, KnowledgeStatus::Superseded);

    let why_prior = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            prior.knowledge_entity_id,
        ))
        .expect("why prior knowledge");
    assert_eq!(why_prior.relation_edges.len(), 1);
    let prior_edge = &why_prior.relation_edges[0];
    assert_eq!(
        prior_edge.relation_kind,
        WhyRelationKind::KnowledgeSupersedes
    );
    assert_eq!(prior_edge.direction, WhyRelationDirection::Incoming);
    assert_eq!(
        prior_edge.source,
        WhyRelationEndpoint::entity(replacement.knowledge_entity_id, WhyEntityKind::Knowledge)
    );
    assert_eq!(
        prior_edge.target,
        WhyRelationEndpoint::entity(prior.knowledge_entity_id, WhyEntityKind::Knowledge)
    );

    let why_replacement = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            replacement.knowledge_entity_id,
        ))
        .expect("why replacement knowledge");
    assert_eq!(why_replacement.relation_edges.len(), 1);
    assert_eq!(
        why_replacement.relation_edges[0].direction,
        WhyRelationDirection::Outgoing
    );

    let connection = Connection::open(&path).expect("open sqlite");
    let operation_type: String = connection
        .query_row(
            "SELECT operation_type FROM changeset WHERE changeset_id = ?1",
            params![&relation.changeset_id.raw_bytes()[..]],
            |row| row.get(0),
        )
        .expect("knowledge relation changeset");
    assert_eq!(operation_type, "knowledge.relation.create");
}

#[test]
fn knowledge_supersedes_relation_requires_active_replacement_and_superseded_prior() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);

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

    let active_prior = engine
        .create_knowledge_relation(
            KnowledgeRelationCreateOptions::supersedes(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.knowledge_entity_id,
                prior.knowledge_entity_id,
                "The new Knowledge replaces the prior statement",
            )
            .expect("active prior relation options"),
        )
        .expect_err("active prior should be rejected");
    assert_eq!(active_prior.code(), ErrorCode::KnowledgeInvalid);

    let superseded_replacement = engine
        .transition_knowledge(
            KnowledgeTransitionOptions::supersede(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.knowledge_entity_id,
                replacement.knowledge_entity_version_id,
                "Replacement was itself obsolete",
            )
            .expect("supersede replacement options"),
        )
        .expect("supersede replacement");
    let inactive_replacement = engine
        .create_knowledge_relation(
            KnowledgeRelationCreateOptions::supersedes(
                workspace.initial_branch_id,
                superseded_replacement.commit_id,
                replacement.knowledge_entity_id,
                prior.knowledge_entity_id,
                "The superseded replacement should not replace prior Knowledge",
            )
            .expect("inactive replacement relation options"),
        )
        .expect_err("inactive replacement should be rejected");
    assert_eq!(inactive_replacement.code(), ErrorCode::KnowledgeInvalid);
}
