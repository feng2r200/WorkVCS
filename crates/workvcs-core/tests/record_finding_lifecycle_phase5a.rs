use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, EntityVersionId, FindingRecordCorrectionOptions, RecordCreateOptions, RecordKind,
    RecordListOptions, RecordRelationListOptions, RecordRelationType, RecordStatus,
    StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn create_workspace(path: &Path) -> (Engine, WorkspaceInfo) {
    let mut engine = Engine::init(
        path,
        StoreInitOptions::new("finding-lifecycle-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

#[test]
fn correcting_finding_supersedes_prior_atomically_and_preserves_history() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prior = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "The old behavior is still current",
            )
            .expect("prior finding options"),
        )
        .expect("create prior finding");
    let replacement = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                prior.commit_id,
                "The old behavior was removed by the validated correction",
            )
            .expect("replacement finding options"),
        )
        .expect("create replacement finding");

    let corrected = engine
        .correct_finding_record(
            FindingRecordCorrectionOptions::supersede(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "The installed behavior proves the later Finding replaces the earlier one",
            )
            .expect("correction options"),
        )
        .expect("supersede finding");

    assert_eq!(corrected.relation_type, RecordRelationType::Supersedes);
    assert_eq!(corrected.previous_target_state.status, RecordStatus::Active);
    assert_eq!(corrected.target_state.status, RecordStatus::Superseded);
    let current_prior = engine
        .record_at(corrected.commit_id, prior.record_entity_id)
        .expect("current prior finding");
    assert_eq!(current_prior.state.status, RecordStatus::Superseded);
    let historical_prior = engine
        .record_at(prior.commit_id, prior.record_entity_id)
        .expect("historical prior finding");
    assert_eq!(historical_prior.state.status, RecordStatus::Active);

    let active_findings = engine
        .records_at(
            RecordListOptions::new(corrected.commit_id)
                .with_kind(RecordKind::Finding)
                .with_status(RecordStatus::Active),
        )
        .expect("active findings");
    assert_eq!(active_findings.records.len(), 1);
    assert_eq!(
        active_findings.records[0].record_entity_id,
        replacement.record_entity_id
    );
    let relation = engine
        .record_relations_at(
            RecordRelationListOptions::new(corrected.commit_id)
                .with_relation_type(RecordRelationType::Supersedes),
        )
        .expect("supersedes relation");
    assert_eq!(relation.relations.len(), 1);
    assert_eq!(
        relation.relations[0].source_record_entity_id,
        replacement.record_entity_id
    );
    assert_eq!(
        relation.relations[0].target_record_entity_id,
        prior.record_entity_id
    );
}

#[test]
fn disproving_finding_invalidates_target_and_records_cause_atomically() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let target = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "The bundle already contains raw Evidence bodies",
            )
            .expect("target finding options"),
        )
        .expect("create target finding");
    let because = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                target.commit_id,
                "Bundle inspection found no raw Evidence object payloads",
            )
            .expect("because finding options"),
        )
        .expect("create because finding");

    let corrected = engine
        .correct_finding_record(
            FindingRecordCorrectionOptions::invalidate(
                workspace.initial_branch_id,
                because.commit_id,
                because.record_entity_id,
                target.record_entity_id,
                target.record_entity_version_id,
                "Direct bundle inspection disproves the earlier Finding",
            )
            .expect("invalidation options"),
        )
        .expect("invalidate finding");

    assert_eq!(corrected.relation_type, RecordRelationType::Invalidates);
    assert_eq!(corrected.target_state.status, RecordStatus::Invalidated);
    let relation = engine
        .record_relations_at(
            RecordRelationListOptions::new(corrected.commit_id)
                .with_relation_type(RecordRelationType::Invalidates),
        )
        .expect("invalidates relation");
    assert_eq!(relation.relations.len(), 1);
    assert_eq!(
        relation.relations[0].source_record_entity_id,
        because.record_entity_id
    );
    assert_eq!(
        relation.relations[0].target_record_entity_id,
        target.record_entity_id
    );
}

#[test]
fn finding_correction_rejects_stale_or_terminal_target_without_moving_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let prior = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Prior Finding",
            )
            .expect("prior finding options"),
        )
        .expect("create prior finding");
    let replacement = engine
        .create_record(
            RecordCreateOptions::finding(
                workspace.initial_branch_id,
                prior.commit_id,
                "Replacement Finding",
            )
            .expect("replacement finding options"),
        )
        .expect("create replacement finding");
    let before = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head before stale correction");

    let stale_error = engine
        .correct_finding_record(
            FindingRecordCorrectionOptions::supersede(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                EntityVersionId::new_v7(),
                "Stale caller",
            )
            .expect("stale correction options"),
        )
        .expect_err("stale target version must fail");
    assert!(stale_error.to_string().contains("expected version"));
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after stale correction"),
        before
    );

    let corrected = engine
        .correct_finding_record(
            FindingRecordCorrectionOptions::supersede(
                workspace.initial_branch_id,
                replacement.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                prior.record_entity_version_id,
                "Current correction",
            )
            .expect("current correction options"),
        )
        .expect("correct finding");
    let terminal_error = engine
        .correct_finding_record(
            FindingRecordCorrectionOptions::invalidate(
                workspace.initial_branch_id,
                corrected.commit_id,
                replacement.record_entity_id,
                prior.record_entity_id,
                corrected.target_record_entity_version_id,
                "Terminal Findings cannot be corrected again",
            )
            .expect("terminal correction options"),
        )
        .expect_err("terminal target must fail");
    assert!(terminal_error.to_string().contains("target must be active"));
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head after terminal correction")
            .head_commit_id,
        corrected.commit_id
    );
}
