use std::path::{Path, PathBuf};

use tempfile::TempDir;
use workvcs_core::{
    ChangeOperationSubject, CommitId, Engine, StoreInitOptions, TaskCreateOptions,
    TaskSchedulingRelationCreateCommit, TaskSchedulingRelationCreateOptions, TaskSnapshot,
    WhyDeferredRelationFamily, WhyEntityKind, WhyEvolutionSubjectDetail, WhyQueryOptions,
    WhyQueryTarget, WhyRelationDirection, WhyRelationEndpoint, WhyRelationKind, WorkspaceInfo,
    WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("phase4nq-why-task-scheduling-relation-store")
            .expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn create_task(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: CommitId,
    description: &str,
) -> TaskSnapshot {
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
            )
            .expect("task options"),
        )
        .expect("create task");
    engine
        .task_at(task.commit_id, task.task_entity_id)
        .expect("task snapshot")
}

fn task_endpoint(task: &TaskSnapshot) -> WhyRelationEndpoint {
    WhyRelationEndpoint::entity(task.task_entity_id, WhyEntityKind::Task)
}

fn assert_single_task_scheduling_edge(
    why: &workvcs_core::WhyQueryResult,
    relation: &TaskSchedulingRelationCreateCommit,
    relation_kind: WhyRelationKind,
    direction: WhyRelationDirection,
    source: &TaskSnapshot,
    target: &TaskSnapshot,
) {
    assert_eq!(why.relation_edges.len(), 1);
    let edge = &why.relation_edges[0];
    assert_eq!(edge.relation_kind, relation_kind);
    assert_eq!(edge.direction, direction);
    assert_eq!(edge.relation_id, relation.relation_id);
    assert_eq!(edge.relation_version_id, relation.relation_version_id);
    assert_eq!(edge.relation_label, None);
    assert_eq!(edge.source, task_endpoint(source));
    assert_eq!(edge.target, task_endpoint(target));
    assert_eq!(edge.state_digest, relation.relation_state_digest);
}

fn assert_single_task_scheduling_evolution(
    why: &workvcs_core::WhyQueryResult,
    relation: &TaskSchedulingRelationCreateCommit,
    relation_kind: WhyRelationKind,
    source: &TaskSnapshot,
    target: &TaskSnapshot,
) {
    assert_eq!(why.causal_anchor_changesets.len(), 0);
    assert_eq!(why.evolution_change_operations.len(), 1);
    assert_eq!(
        why.deferred_relation_families,
        vec![WhyDeferredRelationFamily::Evolution]
    );
    let operation = &why.evolution_change_operations[0];
    assert_eq!(operation.commit_id, relation.commit_id);
    assert_eq!(operation.changeset_id, relation.changeset_id);
    assert_eq!(
        operation.changeset_operation_type,
        "task.scheduling_relation.create"
    );
    assert_eq!(operation.changeset_operation_schema_version, 1);
    assert_eq!(operation.ordinal, 0);
    assert_eq!(
        operation.subject,
        ChangeOperationSubject::Relation(relation.relation_id)
    );
    assert_eq!(operation.operation_id, relation.operation_id);
    let Some(WhyEvolutionSubjectDetail::Relation(detail)) = &operation.subject_detail else {
        panic!("expected relation subject detail")
    };
    assert_eq!(detail.relation_kind, relation_kind);
    assert_eq!(detail.relation_version_id, relation.relation_version_id);
    assert_eq!(detail.relation_label, None);
    assert_eq!(detail.source, task_endpoint(source));
    assert_eq!(detail.target, task_endpoint(target));
    assert_eq!(detail.state_digest, relation.relation_state_digest);
}

#[test]
fn why_projects_depends_on_task_scheduling_relation_to_both_task_endpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prerequisite = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Build the prerequisite",
    );
    let dependent = create_task(
        &mut engine,
        &workspace,
        prerequisite.commit_id,
        "Run the dependent work",
    );
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::depends_on(
                workspace.initial_branch_id,
                dependent.commit_id,
                dependent.task_entity_id,
                prerequisite.task_entity_id,
            )
            .expect("depends_on options"),
        )
        .expect("create depends_on relation");

    let dependent_why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            dependent.task_entity_id,
        ))
        .expect("why dependent task");
    assert_eq!(
        dependent_why.subject.entity_kind(),
        Some(WhyEntityKind::Task)
    );
    assert_single_task_scheduling_edge(
        &dependent_why,
        &relation,
        WhyRelationKind::TaskDependsOn,
        WhyRelationDirection::Outgoing,
        &dependent,
        &prerequisite,
    );
    assert_single_task_scheduling_evolution(
        &dependent_why,
        &relation,
        WhyRelationKind::TaskDependsOn,
        &dependent,
        &prerequisite,
    );

    let prerequisite_why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            prerequisite.task_entity_id,
        ))
        .expect("why prerequisite task");
    assert_eq!(
        prerequisite_why.subject.entity_kind(),
        Some(WhyEntityKind::Task)
    );
    assert_single_task_scheduling_edge(
        &prerequisite_why,
        &relation,
        WhyRelationKind::TaskDependsOn,
        WhyRelationDirection::Incoming,
        &dependent,
        &prerequisite,
    );
    assert_single_task_scheduling_evolution(
        &prerequisite_why,
        &relation,
        WhyRelationKind::TaskDependsOn,
        &dependent,
        &prerequisite,
    );
}

#[test]
fn why_projects_ordered_before_task_scheduling_relation_to_both_task_endpoints() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let earlier = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Do the earlier work",
    );
    let later = create_task(
        &mut engine,
        &workspace,
        earlier.commit_id,
        "Do the later work",
    );
    let relation = engine
        .create_task_scheduling_relation(
            TaskSchedulingRelationCreateOptions::ordered_before(
                workspace.initial_branch_id,
                later.commit_id,
                earlier.task_entity_id,
                later.task_entity_id,
            )
            .expect("ordered_before options"),
        )
        .expect("create ordered_before relation");

    let earlier_why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            earlier.task_entity_id,
        ))
        .expect("why earlier task");
    assert_single_task_scheduling_edge(
        &earlier_why,
        &relation,
        WhyRelationKind::TaskOrderedBefore,
        WhyRelationDirection::Outgoing,
        &earlier,
        &later,
    );
    assert_single_task_scheduling_evolution(
        &earlier_why,
        &relation,
        WhyRelationKind::TaskOrderedBefore,
        &earlier,
        &later,
    );

    let later_why = engine
        .why(WhyQueryOptions::for_entity(
            WhyQueryTarget::commit(relation.commit_id),
            later.task_entity_id,
        ))
        .expect("why later task");
    assert_single_task_scheduling_edge(
        &later_why,
        &relation,
        WhyRelationKind::TaskOrderedBefore,
        WhyRelationDirection::Incoming,
        &earlier,
        &later,
    );
    assert_single_task_scheduling_evolution(
        &later_why,
        &relation,
        WhyRelationKind::TaskOrderedBefore,
        &earlier,
        &later,
    );
}
