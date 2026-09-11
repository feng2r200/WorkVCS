use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    CognitionCaptureManifest, CognitionCaptureOptions, CognitionCaptureOutcome, Engine,
    KnowledgeListOptions, RecordListOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
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

fn manifest(workspace: &WorkspaceInfo, idempotency_key: &str, relation_type: &str) -> String {
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
        head = workspace.genesis_commit_id,
        state_digest = workspace.state_digest,
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
