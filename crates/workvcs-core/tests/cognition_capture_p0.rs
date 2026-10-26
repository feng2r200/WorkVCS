use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    BranchForkOptions, CognitionCaptureManifest, CognitionCaptureOptions, CognitionCaptureOutcome,
    Engine, GoalCreateOptions, KnowledgeListOptions, RecordListOptions, SessionFocusOptions,
    SessionStartOptions, SessionSwitchOptions, SessionWorkGuard, StoreInitOptions, WorkspaceInfo,
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
        StoreInitOptions::new("cognition-capture-store").expect("store options"),
    )
    .expect("init engine");
    let workspace = engine
        .create_workspace(WorkspaceInitOptions::new("workspace").expect("workspace options"))
        .expect("create workspace");
    (engine, workspace)
}

fn sqlite_file_bytes(path: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
    ]
    .into_iter()
    .map(|candidate| {
        let bytes = match fs::read(&candidate) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("read {}: {error}", candidate.display()),
        };
        (candidate, bytes)
    })
    .collect()
}

fn manifest(workspace: &WorkspaceInfo, idempotency_key: &str, relation_type: &str) -> String {
    manifest_at(
        idempotency_key,
        relation_type,
        workspace.genesis_commit_id,
        workspace.state_digest,
    )
}

fn manifest_at(
    idempotency_key: &str,
    relation_type: &str,
    head: workvcs_core::CommitId,
    state_digest: workvcs_core::Digest,
) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "idempotency_key": "{idempotency_key}",
  "expected_head_commit_id": "{head}",
  "expected_state_digest": "{state_digest}",
  "records": [
    {{
      "local_id": "finding",
      "kind": "finding",
      "statement": "The current task produced a reusable observation",
      "scope": {{"source": "focused-check"}}
    }},
    {{
      "local_id": "decision",
      "kind": "decision",
      "statement": "Keep cognition independent from Plan admission",
      "scope": {{"source": "architecture"}}
    }}
  ],
  "knowledge": [
    {{
      "local_id": "knowledge",
      "statement": "No-Plan work may still produce durable knowledge",
      "scope": {{"project": "workvcs"}},
      "provenance": {{"record": "finding"}}
    }}
  ],
  "evidence": [],
  "relations": [
    {{
      "local_id": "support",
      "type": "{relation_type}",
      "source_local_id": "finding",
      "target_local_id": "decision",
      "rationale": "The observation supports the selected boundary"
    }}
  ],
  "rationale": {{"mode": "standalone-cognition"}}
}}"#,
    )
}

#[test]
fn capture_is_atomic_idempotent_and_independent_from_plan() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let manifest = CognitionCaptureManifest::from_json_bytes(
        manifest(&workspace, "capture-independent", "supports").as_bytes(),
    )
    .expect("capture manifest");
    let options = CognitionCaptureOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        workspace.state_digest,
        manifest,
    );

    let created = engine
        .capture_cognition(options.clone())
        .expect("capture cognition");
    assert_eq!(created.outcome, CognitionCaptureOutcome::Created);
    assert_eq!(created.records.len(), 2);
    assert_eq!(created.knowledge.len(), 1);
    assert_eq!(created.relations.len(), 1);
    assert!(
        engine
            .goals_at(created.commit_id)
            .expect("goals")
            .is_empty()
    );
    assert!(
        engine
            .plans_at(created.commit_id)
            .expect("plans")
            .is_empty()
    );
    assert!(
        engine
            .tasks_at(created.commit_id)
            .expect("tasks")
            .is_empty()
    );
    assert_eq!(
        engine
            .records_at(RecordListOptions::new(created.commit_id))
            .expect("records")
            .records
            .len(),
        2
    );
    assert_eq!(
        engine
            .knowledges_at(KnowledgeListOptions::new(created.commit_id))
            .expect("knowledge")
            .knowledge
            .len(),
        1
    );

    let reused = engine
        .capture_cognition(options)
        .expect("reuse cognition capture");
    assert_eq!(reused.outcome, CognitionCaptureOutcome::Reused);
    assert_eq!(reused.commit_id, created.commit_id);
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("branch head")
            .head_commit_id,
        created.commit_id
    );
}

#[test]
fn readonly_capture_result_lookup_matches_content_rejects_conflict_and_writes_nothing() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let source = manifest(&workspace, "capture-readonly-result", "supports");
    let capture_manifest =
        CognitionCaptureManifest::from_json_bytes(source.as_bytes()).expect("capture manifest");
    let created = engine
        .capture_cognition(CognitionCaptureOptions::new(
            workspace.initial_branch_id,
            workspace.genesis_commit_id,
            workspace.state_digest,
            capture_manifest,
        ))
        .expect("capture cognition");
    let current = engine
        .branch_head(workspace.initial_branch_id)
        .expect("current branch head");
    assert_eq!(current.head_commit_id, created.commit_id);
    drop(engine);

    let before = sqlite_file_bytes(&path);
    let readonly = Engine::open_readonly(&path).expect("open readonly Store");
    let exact_manifest =
        CognitionCaptureManifest::from_json_bytes(source.as_bytes()).expect("exact manifest");
    let matched = readonly
        .find_cognition_capture_result(CognitionCaptureOptions::new(
            workspace.initial_branch_id,
            current.head_commit_id,
            current.state_digest,
            exact_manifest,
        ))
        .expect("find exact capture result")
        .expect("capture result exists");
    assert_eq!(matched.outcome, CognitionCaptureOutcome::Reused);
    assert_eq!(matched.commit_id, created.commit_id);
    assert_eq!(matched.changeset_id, created.changeset_id);
    assert_eq!(matched.payload_digest, created.payload_digest);

    let conflicting_source = source.replace(
        "The current task produced a reusable observation",
        "The current task produced a different observation",
    );
    let conflicting_manifest =
        CognitionCaptureManifest::from_json_bytes(conflicting_source.as_bytes())
            .expect("conflicting manifest");
    let conflict = readonly
        .find_cognition_capture_result(CognitionCaptureOptions::new(
            workspace.initial_branch_id,
            current.head_commit_id,
            current.state_digest,
            conflicting_manifest,
        ))
        .expect_err("same idempotency key with different content must fail");
    assert!(
        conflict
            .to_string()
            .contains("was already used with payload digest"),
        "{conflict}"
    );
    drop(readonly);
    assert_eq!(sqlite_file_bytes(&path), before);
}

#[test]
fn capture_session_guard_is_atomic_and_idempotent_reuse_survives_focus_change() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let goal = engine
        .create_goal(
            GoalCreateOptions::new(
                workspace.initial_branch_id,
                workspace.genesis_commit_id,
                "Session focus target",
            )
            .expect("goal options"),
        )
        .expect("create focus target");
    let started = engine
        .start_session(
            SessionStartOptions::new(workspace.workspace_id, workspace.initial_branch_id)
                .expect("session options"),
        )
        .expect("start session");
    let guard = SessionWorkGuard::new(
        started.session_id,
        workspace.workspace_id,
        workspace.initial_branch_id,
        None,
    );
    let source = manifest_at(
        "capture-session-guard",
        "supports",
        goal.commit_id,
        goal.work_state_digest,
    );
    let created = engine
        .capture_cognition(
            CognitionCaptureOptions::new(
                workspace.initial_branch_id,
                goal.commit_id,
                goal.work_state_digest,
                CognitionCaptureManifest::from_json_bytes(source.as_bytes())
                    .expect("capture manifest"),
            )
            .with_session_work_guard(guard),
        )
        .expect("guarded capture");

    engine
        .set_session_focus(SessionFocusOptions::new(
            started.session_id,
            goal.goal_entity_id,
        ))
        .expect("change session focus after capture");
    let current = engine
        .branch_head(workspace.initial_branch_id)
        .expect("current branch head");
    let reused = engine
        .capture_cognition(
            CognitionCaptureOptions::new(
                workspace.initial_branch_id,
                current.head_commit_id,
                current.state_digest,
                CognitionCaptureManifest::from_json_bytes(source.as_bytes())
                    .expect("same capture manifest"),
            )
            .with_session_work_guard(guard),
        )
        .expect("existing idempotent result must remain readable after focus change");
    assert_eq!(reused.outcome, CognitionCaptureOutcome::Reused);
    assert_eq!(reused.commit_id, created.commit_id);

    let new_source = manifest_at(
        "capture-session-guard-new",
        "supports",
        goal.commit_id,
        goal.work_state_digest,
    );
    let error = engine
        .capture_cognition(
            CognitionCaptureOptions::new(
                workspace.initial_branch_id,
                current.head_commit_id,
                current.state_digest,
                CognitionCaptureManifest::from_json_bytes(new_source.as_bytes())
                    .expect("new capture manifest"),
            )
            .with_session_work_guard(guard),
        )
        .expect_err("changed focus must reject a new guarded capture");
    assert!(
        error.to_string().contains("work guard requires focus none"),
        "{error}"
    );
    assert_eq!(
        engine
            .branch_head(workspace.initial_branch_id)
            .expect("head after rejected capture")
            .head_commit_id,
        created.commit_id
    );

    let fork = engine
        .fork_branch(
            BranchForkOptions::from_branch(workspace.initial_branch_id, "guard-switched")
                .expect("fork options"),
        )
        .expect("fork current branch");
    engine
        .switch_session(SessionSwitchOptions::new(
            started.session_id,
            workspace.workspace_id,
            fork.branch_id,
        ))
        .expect("switch guarded session branch");
    let switched_source = manifest_at(
        "capture-session-guard-switched",
        "supports",
        goal.commit_id,
        goal.work_state_digest,
    );
    let error = engine
        .capture_cognition(
            CognitionCaptureOptions::new(
                workspace.initial_branch_id,
                current.head_commit_id,
                current.state_digest,
                CognitionCaptureManifest::from_json_bytes(switched_source.as_bytes())
                    .expect("switched capture manifest"),
            )
            .with_session_work_guard(guard),
        )
        .expect_err("changed session branch must reject a new guarded capture");
    assert!(
        error.to_string().contains("work guard requires branch"),
        "{error}"
    );
    assert!(
        error.to_string().contains(&fork.branch_id.to_string()),
        "{error}"
    );
}

#[test]
fn invalid_capture_does_not_move_the_branch_head() {
    let (_tempdir, path) = store_path();
    let (mut engine, workspace) = create_workspace(&path);
    let manifest = CognitionCaptureManifest::from_json_bytes(
        manifest(&workspace, "capture-invalid", "supersedes").as_bytes(),
    )
    .expect("capture manifest shape");
    let result = engine.capture_cognition(CognitionCaptureOptions::new(
        workspace.initial_branch_id,
        workspace.genesis_commit_id,
        workspace.state_digest,
        manifest,
    ));

    assert!(result.is_err());
    let head = engine
        .branch_head(workspace.initial_branch_id)
        .expect("branch head");
    assert_eq!(head.head_commit_id, workspace.genesis_commit_id);
    assert_eq!(head.state_digest, workspace.state_digest);
}
