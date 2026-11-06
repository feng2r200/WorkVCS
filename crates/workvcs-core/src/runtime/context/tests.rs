use super::*;
use crate::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateOptions, Engine, StoreInitOptions,
    TaskCreateOptions, VerificationCreateOptions, VerificationRequirementCreateOptions,
    VerificationResult, WorkspaceInitOptions,
};

#[test]
fn acceptance_status_batch_reuses_one_replay_and_filters_before_status_resolution() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("context.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("context-request-cache").expect("options"),
    )
    .expect("init");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("options"))
        .expect("workspace");
    let branch = workspace.initial_branch_id;
    let mut head = workspace.genesis_commit_id;
    let mut tasks = Vec::new();
    let mut criteria = Vec::new();
    let mut requirements = Vec::new();
    for index in 0..3 {
        let task = engine
            .create_task(
                TaskCreateOptions::new(branch, head, format!("Task {index}")).expect("options"),
            )
            .expect("task");
        let criterion = engine
            .create_acceptance_criterion(
                AcceptanceCriterionCreateOptions::new(
                    branch,
                    task.commit_id,
                    task.task_entity_id,
                    task.task_entity_version_id,
                    "ac",
                    "Preserve status",
                    AcceptanceCriterionClassification::Required,
                )
                .expect("options"),
            )
            .expect("criterion");
        head = criterion.commit_id;
        if index < 2 {
            let requirement = engine
                .create_verification_requirement(
                    VerificationRequirementCreateOptions::new(
                        branch,
                        head,
                        criterion.acceptance_criterion_entity_id,
                        criterion.acceptance_criterion_entity_version_id,
                        "vr",
                        "Verify state",
                    )
                    .expect("options"),
                )
                .expect("requirement");
            head = requirement.commit_id;
            requirements.push(requirement);
        }
        tasks.push(task.task_entity_id);
        criteria.push(criterion.acceptance_criterion_entity_id);
    }
    let connection = StoreConnection::open_readonly(&path).expect("readonly");
    let selected = BTreeSet::from([tasks[0], tasks[2]]);
    // Positive control: deliberately separate caches must both be counted.
    history::ReplayCache::reset_cold_replay_count();
    history::state_at(&connection, head).expect("first standalone replay");
    history::state_at(&connection, head).expect("second standalone replay");
    assert_eq!(history::ReplayCache::cold_replay_count(), 2);

    history::ReplayCache::reset_cold_replay_count();
    let mut cache = history::ReplayCache::default();
    let batch = context_acceptance_criteria(
        &connection,
        head,
        workspace.workspace_id,
        branch,
        &selected,
        None,
        &mut cache,
    )
    .expect("batch");
    assert_eq!(batch.len(), 2);
    assert_eq!(
        history::ReplayCache::cold_replay_count(),
        1,
        "all nested status reads must share the request cache"
    );
    for item in &batch {
        assert!(selected.contains(&item.snapshot.task_entity_id));
        assert_eq!(
            item.effective_status,
            engine
                .acceptance_criterion_effective_status_for_branch(
                    branch,
                    item.snapshot.acceptance_criterion_entity_id,
                )
                .expect("standalone status"),
        );
        assert_eq!(
            item.effective_status,
            AcceptanceCriterionEffectiveStatus::Unverified
        );
    }

    // Focus can add a criterion outside the current-task set without a replay.
    history::ReplayCache::reset_cold_replay_count();
    let focused = context_acceptance_criteria(
        &connection,
        head,
        workspace.workspace_id,
        branch,
        &selected,
        Some(tasks[1]),
        &mut cache,
    )
    .expect("focused batch");
    assert_eq!(focused.len(), 3);
    assert_eq!(history::ReplayCache::cold_replay_count(), 0);

    let old_head = head;
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                branch,
                head,
                VerificationTarget::VerificationRequirement(
                    requirements[0].verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("options"),
        )
        .expect("verification");
    head = verification.commit_id;
    let stale_anchor = context_acceptance_criteria(
        &connection,
        old_head,
        workspace.workspace_id,
        branch,
        &selected,
        None,
        &mut cache,
    )
    .expect_err("a warmed cache must not conceal branch advancement");
    assert!(matches!(stale_anchor, WorkVcsError::BranchHeadConflict(_)));
    history::ReplayCache::reset_cold_replay_count();
    let mut next_request = history::ReplayCache::default();
    let updated = context_acceptance_criteria(
        &connection,
        head,
        workspace.workspace_id,
        branch,
        &selected,
        None,
        &mut next_request,
    )
    .expect("new request");
    let status = updated
        .iter()
        .find(|item| item.snapshot.acceptance_criterion_entity_id == criteria[0])
        .expect("updated criterion");
    assert_eq!(
        status.effective_status,
        AcceptanceCriterionEffectiveStatus::Verified
    );
    assert_eq!(history::ReplayCache::cold_replay_count(), 1);
    assert_eq!(
        engine
            .acceptance_criterion_effective_status(old_head, criteria[0])
            .expect("historical status"),
        AcceptanceCriterionEffectiveStatus::Unverified,
    );
}
