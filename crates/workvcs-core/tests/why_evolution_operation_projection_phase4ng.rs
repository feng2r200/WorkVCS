use std::path::{Path, PathBuf};

use tempfile::TempDir;
use workvcs_core::{
    ChangeOperationSubject, DecisionRecordSupersedeCommit, DecisionRecordSupersedeOptions, Engine,
    RecordCreateCommit, RecordCreateOptions, StoreInitOptions, WhyDeferredRelationFamily,
    WhyEntityKind, WhyEvolutionSubjectDetail, WhyQueryOptions, WhyQueryTarget, WhyRelationEndpoint,
    WhyRelationKind, WorkspaceInfo, WorkspaceInitOptions,
};

struct SupersedeFixture {
    prior: RecordCreateCommit,
    finding: RecordCreateCommit,
    superseded: DecisionRecordSupersedeCommit,
}

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4ng-why-evolution-operation-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_supersede_fixture(engine: &mut Engine, workspace: &WorkspaceInfo) -> SupersedeFixture {
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

    SupersedeFixture {
        prior,
        finding,
        superseded,
    }
}

#[test]
fn why_causal_anchor_projects_direct_evolution_operations() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_supersede_fixture(&mut engine, &workspace);

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(fixture.superseded.commit_id),
            fixture.finding.record_entity_id,
        ))
        .expect("why causal anchor finding");

    assert_eq!(why.causal_anchor_changesets.len(), 1);
    assert_eq!(why.evolution_change_operations.len(), 3);
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );

    let operation = &why.evolution_change_operations[0];
    assert_eq!(operation.commit_id, fixture.superseded.commit_id);
    assert_eq!(operation.changeset_id, fixture.superseded.changeset_id);
    assert_eq!(
        operation.changeset_operation_type,
        "record.decision.supersede"
    );
    assert_eq!(operation.changeset_operation_schema_version, 1);
    assert_eq!(operation.ordinal, 0);
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Entity(fixture.prior.record_entity_id)
    );
    let Some(WhyEvolutionSubjectDetail::Entity(detail)) = &operation.subject_detail else {
        panic!("expected entity subject detail")
    };
    assert_eq!(detail.entity_kind, WhyEntityKind::Record);
    assert_eq!(
        detail.entity_version_id,
        fixture.superseded.prior_record_entity_version_id
    );
    assert_eq!(detail.statement.as_deref(), Some("Use optimistic writes"));

    let supersedes_operation = why
        .evolution_change_operations
        .iter()
        .find(|operation| operation.ordinal == 1)
        .expect("supersedes relation operation");
    assert_eq!(
        supersedes_operation.subject,
        ChangeOperationSubject::Relation(fixture.superseded.relation_id)
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &supersedes_operation.subject_detail
    else {
        panic!("expected supersedes relation subject detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::RecordSupersedes);
    assert_eq!(
        detail.relation_version_id,
        fixture.superseded.relation_version_id
    );
    assert_eq!(
        detail.source,
        WhyRelationEndpoint::entity(
            fixture.superseded.replacement_record_entity_id,
            WhyEntityKind::Record
        )
    );
    assert_eq!(
        detail.target,
        WhyRelationEndpoint::entity(fixture.prior.record_entity_id, WhyEntityKind::Record)
    );
    assert_eq!(
        detail.state_digest,
        fixture.superseded.relation_state_digest
    );

    let causal_operation = why
        .evolution_change_operations
        .iter()
        .find(|operation| operation.ordinal == 2)
        .expect("causal relation operation");
    assert_eq!(
        causal_operation.subject,
        ChangeOperationSubject::Relation(
            fixture
                .superseded
                .causal_relation_id
                .expect("causal relation id")
        )
    );
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &causal_operation.subject_detail else {
        panic!("expected causal relation subject detail")
    };
    assert_eq!(detail.relation_kind, WhyRelationKind::RecordDerivedFrom);
    assert_eq!(
        detail.relation_version_id,
        fixture
            .superseded
            .causal_relation_version_id
            .expect("causal relation version id")
    );
    assert_eq!(
        detail.source,
        WhyRelationEndpoint::entity(
            fixture.superseded.replacement_record_entity_id,
            WhyEntityKind::Record
        )
    );
    assert_eq!(
        detail.target,
        WhyRelationEndpoint::entity(fixture.finding.record_entity_id, WhyEntityKind::Record)
    );
    assert_eq!(
        Some(detail.state_digest),
        fixture.superseded.causal_relation_state_digest
    );
    assert!(why.evolution_change_operations.iter().all(|operation| {
        operation.operation_payload_size_bytes > 0
            && !operation.operation_payload_digest.to_string().is_empty()
    }));
}

#[test]
fn why_changed_entity_projects_own_direct_evolution_operation() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let fixture = create_supersede_fixture(&mut engine, &workspace);

    let why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(fixture.superseded.commit_id),
            fixture.prior.record_entity_id,
        ))
        .expect("why prior decision");

    assert!(why.causal_anchor_changesets.is_empty());
    assert_eq!(why.evolution_change_operations.len(), 1);
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );

    let operation = &why.evolution_change_operations[0];
    assert_eq!(operation.commit_id, fixture.superseded.commit_id);
    assert_eq!(operation.changeset_id, fixture.superseded.changeset_id);
    assert_eq!(
        operation.changeset_operation_type,
        "record.decision.supersede"
    );
    assert_eq!(operation.changeset_operation_schema_version, 1);
    assert_eq!(operation.ordinal, 0);
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Entity(fixture.prior.record_entity_id)
    );
    let Some(WhyEvolutionSubjectDetail::Entity(detail)) = &operation.subject_detail else {
        panic!("expected changed entity subject detail")
    };
    assert_eq!(detail.entity_kind, WhyEntityKind::Record);
    assert_eq!(
        detail.entity_version_id,
        fixture.superseded.prior_record_entity_version_id
    );
    assert_eq!(detail.statement.as_deref(), Some("Use optimistic writes"));
}
