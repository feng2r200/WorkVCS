use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::{Connection, params};
use tempfile::TempDir;
use workvcs_core::{
    AcceptanceCriterionClassification, AcceptanceCriterionCreateCommit,
    AcceptanceCriterionCreateOptions, CloseoutInspectCategory, CloseoutInspectOptions,
    CloseoutInspectSourceKind, CloseoutInspectStoreFileKind, CloseoutInspectStoreFileMetadata,
    CloseoutInspectTargetDigestStatus, CloseoutInspectTargetResolution,
    CloseoutInspectVerificationTargetKind, Engine, EntityId, ErrorCode, EvidenceCreateOptions,
    EvidenceId, StoreInitOptions, TaskCreateCommit, TaskCreateOptions, VerificationCreateCommit,
    VerificationCreateOptions, VerificationRequirementCreateCommit,
    VerificationRequirementCreateOptions, VerificationResult, VerificationTarget, WorkspaceInfo,
    WorkspaceInitOptions,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSnapshot {
    exists: bool,
    len: Option<u64>,
    modified: Option<SystemTime>,
}

struct CloseoutFixture {
    workspace: WorkspaceInfo,
    task: TaskCreateCommit,
    criterion: AcceptanceCriterionCreateCommit,
    requirement: VerificationRequirementCreateCommit,
    evidence_id: EvidenceId,
    verification: VerificationCreateCommit,
}

fn store_path() -> (TempDir, PathBuf) {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("workvcs.sqlite");
    (tempdir, path)
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = OsString::from(path.as_os_str());
    raw.push(suffix);
    PathBuf::from(raw)
}

fn sqlite_file_paths(path: &Path) -> [PathBuf; 3] {
    [
        path.to_path_buf(),
        sqlite_sidecar_path(path, "-wal"),
        sqlite_sidecar_path(path, "-shm"),
    ]
}

fn file_snapshot(path: &Path) -> FileSnapshot {
    match std::fs::metadata(path) {
        Ok(metadata) => FileSnapshot {
            exists: true,
            len: Some(metadata.len()),
            modified: Some(metadata.modified().expect("modified time")),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FileSnapshot {
            exists: false,
            len: None,
            modified: None,
        },
        Err(error) => panic!("snapshot {path:?}: {error}"),
    }
}

fn sqlite_file_snapshots(path: &Path) -> Vec<(PathBuf, FileSnapshot)> {
    sqlite_file_paths(path)
        .into_iter()
        .map(|path| {
            let snapshot = file_snapshot(&path);
            (path, snapshot)
        })
        .collect()
}

fn assert_sqlite_files_unchanged(path: &Path, before: &[(PathBuf, FileSnapshot)]) {
    let after = sqlite_file_snapshots(path);
    assert_eq!(after, before, "SQLite main/WAL/SHM metadata changed");
}

fn system_time_epoch_nanos(time: SystemTime) -> String {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos().to_string(),
        Err(error) => format!("-{}", error.duration().as_nanos()),
    }
}

fn assert_store_file_metadata_matches(
    path: &Path,
    actual: &[CloseoutInspectStoreFileMetadata],
    expected: &[(PathBuf, FileSnapshot)],
) {
    assert_eq!(actual.len(), 3);
    let expected_kinds = [
        CloseoutInspectStoreFileKind::Main,
        CloseoutInspectStoreFileKind::Wal,
        CloseoutInspectStoreFileKind::Shm,
    ];
    for ((actual, (expected_path, expected_snapshot)), expected_kind) in actual
        .iter()
        .zip(expected.iter())
        .zip(expected_kinds.into_iter())
    {
        assert_eq!(actual.kind, expected_kind);
        assert_eq!(actual.path, expected_path.display().to_string());
        assert_eq!(actual.exists, expected_snapshot.exists);
        assert_eq!(actual.size_bytes, expected_snapshot.len);
        assert_eq!(
            actual.modified_unix_epoch_nanos,
            expected_snapshot.modified.map(system_time_epoch_nanos)
        );
    }
    assert_eq!(
        actual[0].path,
        path.display().to_string(),
        "main sqlite file metadata path should be the store path"
    );
}

fn raw_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("raw connection");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("enable foreign keys");
    connection
}

fn init_engine(path: &Path) -> Engine {
    Engine::init(
        path,
        StoreInitOptions::new("closeout-inspect-p0-store").expect("store options"),
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

fn metadata(label: &str) -> workvcs_core::CanonicalValue {
    workvcs_core::CanonicalValue::object(vec![(
        "label".to_owned(),
        workvcs_core::CanonicalValue::String(label.to_owned()),
    )])
    .expect("metadata")
}

fn create_evidence(engine: &mut Engine, label: &str) -> EvidenceId {
    engine
        .create_evidence(
            EvidenceCreateOptions::new("command_output", metadata(label))
                .expect("evidence options"),
        )
        .expect("create evidence")
        .evidence_id
}

fn create_task(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    expected_head_commit_id: workvcs_core::CommitId,
    description: &str,
) -> TaskCreateCommit {
    engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                description,
            )
            .expect("task options"),
        )
        .expect("create task")
}

fn create_acceptance_criterion(
    engine: &mut Engine,
    workspace: &WorkspaceInfo,
    task: &TaskCreateCommit,
    expected_head_commit_id: workvcs_core::CommitId,
    expected_task_entity_version_id: workvcs_core::EntityVersionId,
    local_key: &str,
) -> AcceptanceCriterionCreateCommit {
    engine
        .create_acceptance_criterion(
            AcceptanceCriterionCreateOptions::new(
                workspace.initial_branch_id,
                expected_head_commit_id,
                task.task_entity_id,
                expected_task_entity_version_id,
                local_key,
                format!("{local_key} must be demonstrated."),
                AcceptanceCriterionClassification::Required,
            )
            .expect("acceptance criterion options"),
        )
        .expect("create acceptance criterion")
}

fn create_closeout_fixture(path: &Path) -> (Engine, CloseoutFixture) {
    let (mut engine, workspace) = create_workspace(path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Implement readonly closeout inspect",
    );
    let criterion = create_acceptance_criterion(
        &mut engine,
        &workspace,
        &task,
        task.commit_id,
        task.task_entity_version_id,
        "AC-1",
    );
    let requirement = engine
        .create_verification_requirement(
            VerificationRequirementCreateOptions::new(
                workspace.initial_branch_id,
                criterion.commit_id,
                criterion.acceptance_criterion_entity_id,
                criterion.acceptance_criterion_entity_version_id,
                "VR-1",
                "The closeout projection can be inspected read-only.",
            )
            .expect("verification requirement options"),
        )
        .expect("create verification requirement");
    let evidence_id = create_evidence(&mut engine, "cargo test closeout inspect");
    let verification = engine
        .create_verification(
            VerificationCreateOptions::new(
                workspace.initial_branch_id,
                requirement.commit_id,
                VerificationTarget::VerificationRequirement(
                    requirement.verification_requirement_entity_id,
                ),
                VerificationResult::Passed,
            )
            .expect("verification options")
            .with_evidence(vec![evidence_id])
            .expect("verification evidence"),
        )
        .expect("create verification");

    (
        engine,
        CloseoutFixture {
            workspace,
            task,
            criterion,
            requirement,
            evidence_id,
            verification,
        },
    )
}

#[test]
fn readonly_task_closeout_inspect_projects_direct_children_without_judgement() {
    let (_tempdir, path) = store_path();
    let (engine, fixture) = create_closeout_fixture(&path);
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            fixture.workspace.initial_branch_id,
            fixture.task.task_entity_id,
        ))
        .expect("inspect task");

    assert_eq!(projection.source.kind, CloseoutInspectSourceKind::Branch);
    assert_eq!(
        projection.source.branch_id,
        Some(fixture.workspace.initial_branch_id)
    );
    assert_eq!(projection.source.commit_id, fixture.verification.commit_id);
    assert_eq!(
        projection.target_resolution,
        CloseoutInspectTargetResolution::Found
    );
    assert!(!projection.truncated);
    assert!(projection.read_proof.stable);
    assert!(projection.read_proof.drift.is_empty());
    let task_snapshot = readonly
        .task_at(fixture.verification.commit_id, fixture.task.task_entity_id)
        .expect("task snapshot at projected head");
    assert_eq!(
        projection.read_proof.before.target_digest_status,
        CloseoutInspectTargetDigestStatus::Found
    );
    assert_eq!(
        projection.read_proof.before.target_state_digest,
        Some(task_snapshot.state_digest)
    );
    assert_eq!(
        projection.read_proof.before.branch_head,
        projection.read_proof.after.branch_head
    );
    assert_eq!(
        projection.read_proof.before.source_state_digest,
        projection.source.state_digest
    );
    assert_eq!(projection.task.as_ref().expect("task").status, "pending");
    assert_eq!(
        projection.task.as_ref().expect("task").state_digest,
        task_snapshot.state_digest
    );
    assert_eq!(
        projection
            .task
            .as_ref()
            .expect("task")
            .acceptance_criteria_count,
        1
    );

    assert_eq!(projection.counts.acceptance_criteria_total, 1);
    assert_eq!(projection.counts.verification_requirements_total, 1);
    assert_eq!(projection.counts.verifications_total, 1);
    assert_eq!(projection.counts.evidence_total, 1);
    assert_eq!(projection.counts.gaps_total, 0);
    assert!(projection.omitted.is_empty());
    assert!(projection.gaps.is_empty());

    let criterion = &projection.acceptance_criteria[0];
    let criterion_snapshot = readonly
        .acceptance_criterion_at(
            fixture.verification.commit_id,
            fixture.criterion.acceptance_criterion_entity_id,
        )
        .expect("criterion snapshot at projected head");
    assert_eq!(criterion.local_key, "AC-1");
    assert_eq!(
        criterion.acceptance_criterion_entity_id,
        fixture.criterion.acceptance_criterion_entity_id
    );
    assert_eq!(criterion.state_digest, criterion_snapshot.state_digest);
    assert_eq!(criterion.verification_requirements_count, 1);

    let requirement = &projection.verification_requirements[0];
    assert_eq!(requirement.local_key, "VR-1");
    assert_eq!(
        requirement.verification_requirement_entity_id,
        fixture.requirement.verification_requirement_entity_id
    );
    assert_eq!(
        requirement.state_digest,
        fixture.requirement.verification_requirement_state_digest
    );

    let verification = &projection.verifications[0];
    assert_eq!(
        verification.verification_entity_id,
        fixture.verification.verification_entity_id
    );
    assert_eq!(
        verification.target_kind,
        CloseoutInspectVerificationTargetKind::VerificationRequirement
    );
    assert_eq!(
        verification.target_entity_id,
        fixture.requirement.verification_requirement_entity_id
    );
    assert_eq!(verification.result, "passed");
    assert_eq!(verification.evidence_count, 1);

    let evidence = &projection.evidence[0];
    assert_eq!(evidence.evidence_id, fixture.evidence_id);
    assert_eq!(evidence.evidence_kind, "command_output");

    let serialized = serde_json::to_value(&projection).expect("serialize projection");
    assert_eq!(serialized["truncated"], serde_json::Value::Bool(false));
    assert!(serialized.get("read_proof").is_some());
    assert!(
        serialized.get("ready").is_none(),
        "mechanical projection must not judge readiness"
    );
    assert!(
        serialized.get("complete").is_none(),
        "mechanical projection must not judge completion"
    );
    assert!(
        serialized.get("authorized").is_none(),
        "mechanical projection must not judge authorization"
    );
}

#[test]
fn task_with_no_children_reports_mechanical_gap_only() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Implement later",
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            task.task_entity_id,
        ))
        .expect("inspect task with no children");

    assert_eq!(
        projection.target_resolution,
        CloseoutInspectTargetResolution::Found
    );
    assert!(projection.acceptance_criteria.is_empty());
    assert!(projection.verification_requirements.is_empty());
    assert!(projection.verifications.is_empty());
    assert!(projection.evidence.is_empty());
    assert_eq!(projection.counts.gaps_total, 1);
    assert_eq!(projection.gaps.len(), 1);
    assert_eq!(projection.gaps[0].code, "task_has_no_acceptance_criteria");
    assert_eq!(projection.gaps[0].subject_id, Some(task.task_entity_id));
}

#[test]
fn mismatched_acceptance_criterion_local_key_reports_corrupt_gap() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Detect mismatched criterion refs",
    );
    let criterion = create_acceptance_criterion(
        &mut engine,
        &workspace,
        &task,
        task.commit_id,
        task.task_entity_version_id,
        "AC-1",
    );
    drop(engine);

    let connection = raw_connection(&path);
    let criterion_id = criterion.acceptance_criterion_entity_id.raw_bytes();
    let updated = connection
        .execute(
            "UPDATE acceptance_criterion_identity
             SET local_key = 'AC-DRIFT'
             WHERE entity_id = ?1",
            params![&criterion_id[..]],
        )
        .expect("corrupt acceptance criterion local key");
    assert_eq!(updated, 1);
    drop(connection);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            task.task_entity_id,
        ))
        .expect("inspect task with mismatched AC local key");

    assert_eq!(
        projection.target_resolution,
        CloseoutInspectTargetResolution::Found
    );
    assert_eq!(
        projection.read_proof.before.target_digest_status,
        CloseoutInspectTargetDigestStatus::Found
    );
    assert!(projection.acceptance_criteria.is_empty());
    assert_eq!(projection.counts.gaps_total, 1);
    assert_eq!(projection.gaps.len(), 1);
    assert_eq!(
        projection.gaps[0].code,
        "acceptance_criterion_local_key_mismatch"
    );
    assert_eq!(
        projection.gaps[0].subject_id,
        Some(criterion.acceptance_criterion_entity_id)
    );
    assert_eq!(projection.gaps[0].related_id, Some(task.task_entity_id));
}

#[test]
fn stable_sort_and_budget_truncation_are_core_owned() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Budget inspect",
    );
    let first = create_acceptance_criterion(
        &mut engine,
        &workspace,
        &task,
        task.commit_id,
        task.task_entity_version_id,
        "AC-B",
    );
    let second = create_acceptance_criterion(
        &mut engine,
        &workspace,
        &task,
        first.commit_id,
        first.task_entity_version_id,
        "AC-A",
    );
    create_acceptance_criterion(
        &mut engine,
        &workspace,
        &task,
        second.commit_id,
        second.task_entity_version_id,
        "AC-C",
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let projection = readonly
        .closeout_inspect_task(
            CloseoutInspectOptions::for_task_on_branch(
                workspace.initial_branch_id,
                task.task_entity_id,
            )
            .with_budget(2)
            .expect("budget"),
        )
        .expect("inspect with budget");

    assert_eq!(projection.counts.acceptance_criteria_total, 3);
    assert!(projection.truncated);
    assert_eq!(
        projection
            .acceptance_criteria
            .iter()
            .map(|item| item.local_key.as_str())
            .collect::<Vec<_>>(),
        vec!["AC-A", "AC-B"]
    );
    assert_eq!(
        projection
            .omitted
            .iter()
            .map(|item| (item.category, item.count))
            .collect::<Vec<_>>(),
        vec![
            (CloseoutInspectCategory::AcceptanceCriteria, 1),
            (CloseoutInspectCategory::Gaps, 3),
        ]
    );
    let serialized = serde_json::to_value(&projection).expect("serialize truncated projection");
    assert_eq!(serialized["truncated"], serde_json::Value::Bool(true));

    let invalid_budget = CloseoutInspectOptions::for_task_on_branch(
        workspace.initial_branch_id,
        task.task_entity_id,
    )
    .with_budget(201)
    .expect_err("budget above hard limit is rejected");
    assert_eq!(invalid_budget.code(), ErrorCode::QueryInvalid);
}

#[test]
fn explicit_commit_snapshot_differs_from_branch_head_snapshot() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Historical inspect",
    );
    create_acceptance_criterion(
        &mut engine,
        &workspace,
        &task,
        task.commit_id,
        task.task_entity_version_id,
        "AC-1",
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let historical = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_at_commit(
            task.commit_id,
            task.task_entity_id,
        ))
        .expect("inspect historical commit");
    let current = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            task.task_entity_id,
        ))
        .expect("inspect branch head");

    assert_eq!(historical.source.kind, CloseoutInspectSourceKind::Commit);
    assert_eq!(historical.source.commit_id, task.commit_id);
    assert!(historical.read_proof.before.branch_head.is_none());
    assert!(historical.read_proof.after.branch_head.is_none());
    assert!(historical.acceptance_criteria.is_empty());
    assert_eq!(historical.counts.gaps_total, 1);
    assert_eq!(historical.gaps[0].code, "task_has_no_acceptance_criteria");

    assert_eq!(current.source.kind, CloseoutInspectSourceKind::Branch);
    assert_eq!(current.counts.acceptance_criteria_total, 1);
    assert_eq!(current.acceptance_criteria[0].local_key, "AC-1");
}

#[test]
fn target_not_found_and_wrong_kind_are_structured_resolution_not_implicit_selection() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let task = create_task(
        &mut engine,
        &workspace,
        workspace.genesis_commit_id,
        "Resolution inspect",
    );
    let criterion = create_acceptance_criterion(
        &mut engine,
        &workspace,
        &task,
        task.commit_id,
        task.task_entity_version_id,
        "AC-1",
    );
    drop(engine);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let not_found = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            EntityId::new_v7(),
        ))
        .expect("missing target projects");
    assert_eq!(
        not_found.target_resolution,
        CloseoutInspectTargetResolution::TargetNotFound
    );
    assert!(not_found.task.is_none());
    assert_eq!(not_found.gaps[0].code, "target_not_found");
    assert_eq!(
        not_found.read_proof.before.target_digest_status,
        CloseoutInspectTargetDigestStatus::TargetNotFound
    );
    assert_eq!(not_found.read_proof.before.target_state_digest, None);

    let wrong_kind = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            workspace.initial_branch_id,
            criterion.acceptance_criterion_entity_id,
        ))
        .expect("wrong-kind target projects");
    assert_eq!(
        wrong_kind.target_resolution,
        CloseoutInspectTargetResolution::WrongKind
    );
    assert!(wrong_kind.task.is_none());
    assert_eq!(wrong_kind.gaps[0].code, "target_wrong_kind");
    assert_eq!(
        wrong_kind.read_proof.before.target_digest_status,
        CloseoutInspectTargetDigestStatus::WrongKind
    );
    assert_eq!(wrong_kind.read_proof.before.target_state_digest, None);
}

#[test]
fn readonly_closeout_inspect_preserves_head_state_digest_and_sqlite_metadata() {
    let (_tempdir, path) = store_path();
    let (engine, fixture) = create_closeout_fixture(&path);
    drop(engine);
    let before_files = sqlite_file_snapshots(&path);

    let readonly = Engine::open_readonly(&path).expect("open readonly");
    let before_head = readonly
        .branch_head(fixture.workspace.initial_branch_id)
        .expect("branch head before inspect");
    let before_state = readonly
        .state_at(before_head.head_commit_id)
        .expect("state before inspect");

    let projection = readonly
        .closeout_inspect_task(CloseoutInspectOptions::for_task_on_branch(
            fixture.workspace.initial_branch_id,
            fixture.task.task_entity_id,
        ))
        .expect("inspect task");
    assert_eq!(
        projection.source.state_digest, before_state.state_digest,
        "inspect source state digest must match replayed branch head"
    );
    assert!(projection.read_proof.stable);
    assert!(projection.read_proof.drift.is_empty());
    assert_eq!(
        projection
            .read_proof
            .before
            .branch_head
            .as_ref()
            .expect("branch proof before")
            .head_commit_id,
        before_head.head_commit_id
    );
    assert_eq!(
        projection.read_proof.before.source_state_digest,
        before_state.state_digest
    );
    assert_eq!(
        projection.read_proof.before.target_state_digest,
        projection.task.as_ref().map(|task| task.state_digest)
    );
    assert_store_file_metadata_matches(
        &path,
        &projection.read_proof.before.store_files,
        &before_files,
    );

    let after_head = readonly
        .branch_head(fixture.workspace.initial_branch_id)
        .expect("branch head after inspect");
    let after_state = readonly
        .state_at(after_head.head_commit_id)
        .expect("state after inspect");
    assert_eq!(after_head, before_head);
    assert_eq!(after_state.state_digest, before_state.state_digest);
    drop(readonly);

    let after_files = sqlite_file_snapshots(&path);
    assert_eq!(after_files, before_files);
    assert_store_file_metadata_matches(
        &path,
        &projection.read_proof.after.store_files,
        &after_files,
    );
    assert_sqlite_files_unchanged(&path, &before_files);
}
