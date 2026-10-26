use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::Connection;
use sha2::{Digest as ShaDigest, Sha256};
use tempfile::TempDir;
use workvcs_core::{
    BranchProjectionRefreshOptions, CanonicalValue, ClaimReleaseOptions, ClaimTaskOptions,
    ContinuationProjectPolicySetOptions, ContinuationScopeBindOptions, Engine, ErrorCode,
    SessionEndOptions, SessionMarkStaleOptions, SessionStartOptions, StoreInitOptions,
    TaskCreateOptions, WorkspaceInitOptions,
};

const PROJECT_ID: &str = "g-p-continuation-readiness-test";
const RECEIPT_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

struct Fixture {
    _tempdir: TempDir,
    path: PathBuf,
    engine: Engine,
    listener_id: workvcs_core::SessionId,
    carrier_id: workvcs_core::SessionId,
    claim_id: workvcs_core::ClaimId,
    workspace_id: workvcs_core::WorkspaceId,
    branch_id: workvcs_core::BranchId,
    task_entity_id: workvcs_core::EntityId,
    scope_id: workvcs_core::ContinuationScopeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSnapshot {
    is_file: bool,
    len: u64,
    modified: SystemTime,
    sha256: Option<[u8; 32]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ShmObservation {
    before: Option<FileSnapshot>,
    after: Option<FileSnapshot>,
}

fn object(entries: Vec<(&str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
    .unwrap()
}

fn metadata(role: &str, thread_id: &str) -> CanonicalValue {
    object(vec![
        ("thread_id", CanonicalValue::String(thread_id.to_owned())),
        (
            "workvcs_session_role",
            CanonicalValue::String(role.to_owned()),
        ),
    ])
}

fn policy_authority(
    actor_session_id: workvcs_core::SessionId,
    expected_revision: i64,
    enabled: bool,
) -> CanonicalValue {
    policy_authority_for(PROJECT_ID, actor_session_id, expected_revision, enabled)
}

fn policy_authority_for(
    project_id: &str,
    actor_session_id: workvcs_core::SessionId,
    expected_revision: i64,
    enabled: bool,
) -> CanonicalValue {
    object(vec![
        ("schema_version", CanonicalValue::safe_integer(1).unwrap()),
        (
            "action",
            CanonicalValue::String(if enabled { "enable" } else { "disable" }.to_owned()),
        ),
        ("project_id", CanonicalValue::String(project_id.to_owned())),
        (
            "actor_session_id",
            CanonicalValue::String(actor_session_id.to_string()),
        ),
        (
            "expected_revision",
            CanonicalValue::safe_integer(expected_revision).unwrap(),
        ),
        (
            "authorization_envelope_kind",
            CanonicalValue::String("trusted_product_control_plane".to_owned()),
        ),
        (
            "product_actor_id",
            CanonicalValue::String("user:readiness-test".to_owned()),
        ),
        (
            "user_authorization_receipt_digest",
            CanonicalValue::String(RECEIPT_DIGEST.to_owned()),
        ),
        (
            "project_membership_receipt_digest",
            CanonicalValue::String(RECEIPT_DIGEST.to_owned()),
        ),
        (
            "authorized_at_us",
            CanonicalValue::safe_integer(expected_revision + 1).unwrap(),
        ),
    ])
}

fn build_fixture() -> Fixture {
    build_fixture_with_project_binding(Some(PROJECT_ID))
}

fn build_fixture_with_project_binding(project_id: Option<&str>) -> Fixture {
    let tempdir = tempfile::tempdir().unwrap();
    let path = tempdir.path().join("workvcs.sqlite");
    let mut engine = Engine::init(
        &path,
        StoreInitOptions::new("continuation-readiness-test").unwrap(),
    )
    .unwrap();
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").unwrap())
        .unwrap();
    let task = engine
        .create_task(
            TaskCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Continue the active task",
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .refresh_branch_projection(BranchProjectionRefreshOptions::new(
            workspace.initial_branch_id,
        ))
        .expect("refresh task projection");
    let listener_id = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .unwrap()
                .with_metadata(metadata("listener", "01a-readiness-listener"))
                .unwrap(),
        )
        .unwrap()
        .session_id;
    let carrier_id = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .unwrap()
                .with_metadata(metadata("work", "01a-readiness-carrier"))
                .unwrap(),
        )
        .unwrap()
        .session_id;
    engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                listener_id,
                PROJECT_ID,
                0,
                true,
                policy_authority(listener_id, 0, true),
            )
            .unwrap(),
        )
        .unwrap();
    let claim_id = engine
        .claim_task(ClaimTaskOptions::new(carrier_id, task.task_entity_id))
        .unwrap()
        .claim_id;
    let mut bind_options = ContinuationScopeBindOptions::new(
        workspace.workspace_id,
        workspace.initial_branch_id,
        task.task_entity_id,
        listener_id,
        carrier_id,
        claim_id,
    );
    if let Some(project_id) = project_id {
        bind_options = bind_options.with_project_id(project_id).unwrap();
    }
    let scope_id = engine
        .bind_continuation_scope(bind_options)
        .unwrap()
        .scope_id;
    Fixture {
        _tempdir: tempdir,
        path,
        engine,
        listener_id,
        carrier_id,
        claim_id,
        workspace_id: workspace.workspace_id,
        branch_id: workspace.initial_branch_id,
        task_entity_id: task.task_entity_id,
        scope_id,
    }
}

fn directory_snapshot(path: &Path) -> BTreeMap<String, FileSnapshot> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let metadata = entry.metadata().unwrap();
            let sha256 = metadata.is_file().then(|| file_sha256(&entry.path()));
            (
                name,
                FileSnapshot {
                    is_file: metadata.is_file(),
                    len: metadata.len(),
                    modified: metadata.modified().unwrap(),
                    sha256,
                },
            )
        })
        .collect()
}

fn file_sha256(path: &Path) -> [u8; 32] {
    let mut file = File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8_192];
    loop {
        let read = file.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    digest.finalize().into()
}

fn assert_readonly_store_observation(
    before: &BTreeMap<String, FileSnapshot>,
    after: &BTreeMap<String, FileSnapshot>,
    database_name: &str,
) -> ShmObservation {
    let shm_name = format!("{database_name}-shm");
    let before_persistent = before
        .iter()
        .filter(|(name, _)| *name != &shm_name)
        .collect::<BTreeMap<_, _>>();
    let after_persistent = after
        .iter()
        .filter(|(name, _)| *name != &shm_name)
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        before_persistent.keys().collect::<Vec<_>>(),
        after_persistent.keys().collect::<Vec<_>>(),
        "readiness changed persistent Store directory membership"
    );
    for (name, before_snapshot) in before_persistent {
        let after_snapshot = after_persistent
            .get(name)
            .expect("persistent member set already compared");
        assert_eq!(
            *after_snapshot, before_snapshot,
            "readiness changed persistent Store member {name}"
        );
    }
    let before_shm = before.get(&shm_name).cloned();
    let after_shm = after.get(&shm_name).cloned();
    for (phase, snapshot) in [("before", &before_shm), ("after", &after_shm)] {
        if let Some(snapshot) = snapshot {
            assert!(snapshot.is_file, "WAL-index was not a file {phase}");
        }
    }
    ShmObservation {
        before: before_shm,
        after: after_shm,
    }
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut raw = OsString::from(path.as_os_str());
    raw.push(suffix);
    PathBuf::from(raw)
}

fn copy_wal_store_without_shm(source: &Path, target_directory: &Path) -> PathBuf {
    let target = target_directory.join(source.file_name().unwrap());
    let source_wal = sidecar_path(source, "-wal");
    let target_wal = sidecar_path(&target, "-wal");
    let target_shm = sidecar_path(&target, "-shm");
    assert!(source.is_file(), "source main database must exist");
    assert!(source_wal.is_file(), "source WAL must exist");
    fs::copy(source, &target).expect("copy main database");
    fs::copy(source_wal, target_wal).expect("copy WAL");
    assert!(
        !target_shm.exists(),
        "copied Store must start without WAL-index"
    );
    target
}

fn open_wal_connection(path: &Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    connection
        .pragma_update(None, "wal_autocheckpoint", 0_i64)
        .unwrap();
    connection
}

fn assert_wal_sidecars(path: &Path) {
    for suffix in ["-wal", "-shm"] {
        assert!(
            sidecar_path(path, suffix).is_file(),
            "expected {suffix} sidecar"
        );
    }
}

fn gap_codes(snapshot: &workvcs_core::ContinuationReadinessSnapshot) -> Vec<&str> {
    snapshot.gaps.iter().map(|gap| gap.code.as_str()).collect()
}

#[test]
fn readonly_readiness_is_non_mutating_and_rejects_writes() {
    let mut fixture = build_fixture();
    let wal_connection = open_wal_connection(&fixture.path);
    fixture
        .engine
        .create_workspace(WorkspaceInitOptions::new("WAL evidence workspace").unwrap())
        .expect("append a public Engine write to the WAL");
    assert_wal_sidecars(&fixture.path);
    let before = directory_snapshot(fixture.path.parent().unwrap());
    assert!(before.contains_key("workvcs.sqlite"));
    assert!(before.contains_key("workvcs.sqlite-wal"));
    assert!(before.contains_key("workvcs.sqlite-shm"));
    let mut readonly = Engine::open_readonly(&fixture.path).unwrap();
    let first = readonly
        .continuation_readiness(fixture.scope_id)
        .expect("read readiness");
    assert!(first.read_only);
    assert!(first.store_opened_readonly);
    assert!(first.provider_protocol_ready, "{:?}", first.gaps);
    assert!(!first.product_membership_verified);
    assert!(!first.product_dispatch_authorized);
    assert!(first.gaps.is_empty());
    assert_eq!(first.scope.project_id.as_deref(), Some(PROJECT_ID));
    assert_eq!(first.scope.project_policy_revision, Some(1));
    assert_eq!(
        first.scope.project_binding_source.as_deref(),
        Some("continuation.scope_bound")
    );
    assert_eq!(first.policy.resolution, "scope_binding");
    assert_eq!(first.policy.project_id.as_deref(), Some(PROJECT_ID));
    assert_eq!(
        first.scope.project_policy_receipt_digest,
        first.policy.receipt_digest
    );
    assert_eq!(first.listener.role.as_deref(), Some("listener"));
    assert_eq!(first.carrier.session.role.as_deref(), Some("work"));
    assert_eq!(first.carrier.provenance, "work");
    assert!(first.claim.active);
    assert!(first.branch.projection_current);
    assert!(first.task.current);
    assert!(!first.rotation.active);
    let json = serde_json::to_value(&first).expect("typed readiness JSON");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["provider_protocol_ready"], true);
    assert_eq!(json["product_membership_verified"], false);
    assert_eq!(json["product_dispatch_authorized"], false);

    let error = readonly
        .mark_session_potentially_stale(
            SessionMarkStaleOptions::new(fixture.listener_id, "readonly write probe").unwrap(),
        )
        .expect_err("read-only Engine must reject writes");
    assert_eq!(error.code(), ErrorCode::StorageFailure);
    assert_eq!(
        readonly
            .continuation_readiness(fixture.scope_id)
            .expect("readiness remains readable"),
        first
    );
    drop(readonly);
    let after = directory_snapshot(fixture.path.parent().unwrap());
    let shm = assert_readonly_store_observation(&before, &after, "workvcs.sqlite");
    eprintln!("readiness WAL-index observation: {shm:?}");
    drop(wal_connection);

    fixture
        .engine
        .mark_session_potentially_stale(
            SessionMarkStaleOptions::new(fixture.listener_id, "lost listener heartbeat").unwrap(),
        )
        .unwrap();
    let stale = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(!stale.provider_protocol_ready);
    assert!(gap_codes(&stale).contains(&"listener_not_active"));
    assert_ne!(stale.currentness_digest, first.currentness_digest);
}

#[test]
fn readonly_wal_copy_without_shm_creates_wal_index_and_preserves_snapshot() {
    let mut fixture = build_fixture();
    let wal_connection = open_wal_connection(&fixture.path);
    fixture
        .engine
        .create_workspace(WorkspaceInitOptions::new("copied WAL evidence workspace").unwrap())
        .expect("append a public Engine write to the copied WAL source");
    assert_wal_sidecars(&fixture.path);
    let source_snapshot = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .expect("read source readiness");
    let copied_directory = tempfile::tempdir().unwrap();
    let copied_path = copy_wal_store_without_shm(&fixture.path, copied_directory.path());
    let before = directory_snapshot(copied_directory.path());
    assert!(before.contains_key("workvcs.sqlite"));
    assert!(before.contains_key("workvcs.sqlite-wal"));
    assert!(!before.contains_key("workvcs.sqlite-shm"));

    let readonly = Engine::open_readonly(&copied_path).expect("open copied WAL Store read-only");
    let copied_snapshot = readonly
        .continuation_readiness(fixture.scope_id)
        .expect("read copied readiness");
    assert_eq!(copied_snapshot, source_snapshot);
    let after = directory_snapshot(copied_directory.path());
    let shm = assert_readonly_store_observation(&before, &after, "workvcs.sqlite");
    assert_eq!(shm.before, None);
    assert!(
        shm.after.is_some(),
        "read-only WAL open must create WAL-index"
    );
    eprintln!("copied WAL-index observation: {shm:?}");
    drop(readonly);
    drop(wal_connection);
}

#[test]
fn unbound_scope_never_guesses_single_or_multiple_project_policies() {
    const UNRELATED_PROJECT_ID: &str = "g-p-unrelated-readiness-test";
    let mut fixture = build_fixture_with_project_binding(None);
    let single_policy = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(!single_policy.provider_protocol_ready);
    assert!(gap_codes(&single_policy).contains(&"project_binding_missing"));
    assert_eq!(single_policy.scope.project_id, None);
    assert_eq!(single_policy.policy.project_id, None);
    assert_eq!(single_policy.policy.resolution, "project_binding_missing");
    assert_eq!(single_policy.policy.candidate_count, 0);

    fixture
        .engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                fixture.listener_id,
                UNRELATED_PROJECT_ID,
                0,
                true,
                policy_authority_for(UNRELATED_PROJECT_ID, fixture.listener_id, 0, true),
            )
            .unwrap(),
        )
        .unwrap();
    let multiple_policies = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert_eq!(multiple_policies, single_policy);
}

#[test]
fn scope_bind_replay_cannot_claim_or_change_project_binding() {
    const OTHER_PROJECT_ID: &str = "g-p-other-readiness-test";
    let mut fixture = build_fixture();
    fixture
        .engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                fixture.listener_id,
                OTHER_PROJECT_ID,
                0,
                true,
                policy_authority_for(OTHER_PROJECT_ID, fixture.listener_id, 0, true),
            )
            .unwrap(),
        )
        .unwrap();
    fixture
        .engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                fixture.listener_id,
                PROJECT_ID,
                1,
                false,
                policy_authority(fixture.listener_id, 1, false),
            )
            .unwrap(),
        )
        .unwrap();
    let replayed = fixture
        .engine
        .bind_continuation_scope(
            ContinuationScopeBindOptions::new(
                fixture.workspace_id,
                fixture.branch_id,
                fixture.task_entity_id,
                fixture.listener_id,
                fixture.carrier_id,
                fixture.claim_id,
            )
            .with_project_id(PROJECT_ID)
            .unwrap(),
        )
        .expect("same Project bind replay preserves the original association");
    assert_eq!(replayed.scope_id, fixture.scope_id);
    assert_eq!(replayed.project_policy_revision, Some(1));
    let conflicting = fixture
        .engine
        .bind_continuation_scope(
            ContinuationScopeBindOptions::new(
                fixture.workspace_id,
                fixture.branch_id,
                fixture.task_entity_id,
                fixture.listener_id,
                fixture.carrier_id,
                fixture.claim_id,
            )
            .with_project_id(OTHER_PROJECT_ID)
            .unwrap(),
        )
        .expect_err("existing scope Project binding must be immutable");
    assert_eq!(conflicting.code(), ErrorCode::ContinuationInvalid);
    let retained = fixture
        .engine
        .continuation_scope(
            fixture.workspace_id,
            fixture.branch_id,
            fixture.task_entity_id,
        )
        .unwrap();
    assert_eq!(retained.project_id.as_deref(), Some(PROJECT_ID));

    let mut unbound = build_fixture_with_project_binding(None);
    let claim_attempt = unbound
        .engine
        .bind_continuation_scope(
            ContinuationScopeBindOptions::new(
                unbound.workspace_id,
                unbound.branch_id,
                unbound.task_entity_id,
                unbound.listener_id,
                unbound.carrier_id,
                unbound.claim_id,
            )
            .with_project_id(PROJECT_ID)
            .unwrap(),
        )
        .expect_err("legacy unbound scope must not be claimed by bind replay");
    assert_eq!(claim_attempt.code(), ErrorCode::ContinuationInvalid);
    assert_eq!(
        unbound
            .engine
            .continuation_scope(
                unbound.workspace_id,
                unbound.branch_id,
                unbound.task_entity_id,
            )
            .unwrap()
            .project_id,
        None
    );
}

#[test]
fn readiness_accepts_closed_branch_as_non_ready_and_rejects_unknown_lifecycle() {
    let mut fixture = build_fixture();
    let connection = open_wal_connection(&fixture.path);
    fixture
        .engine
        .create_workspace(
            WorkspaceInitOptions::new("closed Branch WAL evidence workspace").unwrap(),
        )
        .expect("append a public Engine write to the WAL");
    assert_wal_sidecars(&fixture.path);
    connection
        .execute(
            "UPDATE branch SET lifecycle_state = 'closed' WHERE branch_id = ?1",
            [&fixture.branch_id.raw_bytes()[..]],
        )
        .unwrap();
    let before = directory_snapshot(fixture.path.parent().unwrap());
    let closed = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(!closed.provider_protocol_ready);
    assert!(!closed.product_membership_verified);
    assert!(!closed.product_dispatch_authorized);
    assert!(gap_codes(&closed).contains(&"branch_not_active"));
    let after = directory_snapshot(fixture.path.parent().unwrap());
    let shm = assert_readonly_store_observation(&before, &after, "workvcs.sqlite");
    eprintln!("closed-Branch WAL-index observation: {shm:?}");

    connection
        .execute(
            "UPDATE branch SET lifecycle_state = 'bogus_lifecycle' WHERE branch_id = ?1",
            [&fixture.branch_id.raw_bytes()[..]],
        )
        .unwrap();
    let error = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .expect_err("unknown Branch lifecycle must fail before a readiness snapshot");
    assert_eq!(error.code(), ErrorCode::ContinuationInvalid);
    assert!(
        error
            .to_string()
            .contains("stored branch lifecycle state \"bogus_lifecycle\" is invalid")
    );
}

#[test]
fn revocation_ended_listener_and_released_claim_never_report_ready() {
    let mut fixture = build_fixture();
    let baseline = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(baseline.provider_protocol_ready, "{:?}", baseline.gaps);

    fixture
        .engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                fixture.listener_id,
                PROJECT_ID,
                1,
                false,
                policy_authority(fixture.listener_id, 1, false),
            )
            .unwrap(),
        )
        .unwrap();
    let revoked = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(!revoked.provider_protocol_ready);
    assert!(gap_codes(&revoked).contains(&"project_policy_disabled"));
    assert_ne!(revoked.currentness_digest, baseline.currentness_digest);

    fixture
        .engine
        .set_continuation_project_policy(
            ContinuationProjectPolicySetOptions::new(
                fixture.listener_id,
                PROJECT_ID,
                2,
                true,
                policy_authority(fixture.listener_id, 2, true),
            )
            .unwrap(),
        )
        .unwrap();
    fixture
        .engine
        .end_session(SessionEndOptions::new(fixture.listener_id).unwrap())
        .unwrap();
    let ended = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(!ended.provider_protocol_ready);
    assert_eq!(ended.listener.lifecycle_state, "ended");
    assert!(gap_codes(&ended).contains(&"listener_not_active"));
    assert_ne!(ended.currentness_digest, revoked.currentness_digest);

    let mut claim_fixture = build_fixture();
    claim_fixture
        .engine
        .release_claim(ClaimReleaseOptions::new(
            claim_fixture.carrier_id,
            claim_fixture.claim_id,
        ))
        .unwrap();
    let error = Engine::open_readonly(&claim_fixture.path)
        .unwrap()
        .continuation_readiness(claim_fixture.scope_id)
        .expect_err("released idle Claim atomically retires its scope");
    assert_eq!(error.code(), ErrorCode::ContinuationNotFound);
}

#[test]
fn branch_head_drift_changes_digest_and_is_not_silently_refreshed() {
    let mut fixture = build_fixture();
    let baseline = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(baseline.provider_protocol_ready, "{:?}", baseline.gaps);
    let prior_head = fixture.engine.branch_head(fixture.branch_id).unwrap();
    fixture
        .engine
        .create_task(
            TaskCreateOptions::new(
                fixture.branch_id,
                prior_head.head_commit_id,
                "Advance the Branch without refreshing its projection",
            )
            .unwrap(),
        )
        .unwrap();

    let drifted = Engine::open_readonly(&fixture.path)
        .unwrap()
        .continuation_readiness(fixture.scope_id)
        .unwrap();
    assert!(!drifted.provider_protocol_ready);
    assert!(gap_codes(&drifted).contains(&"branch_projection_not_current"));
    assert_ne!(
        drifted.branch.head_commit_id,
        baseline.branch.head_commit_id
    );
    assert_ne!(drifted.currentness_digest, baseline.currentness_digest);
}
