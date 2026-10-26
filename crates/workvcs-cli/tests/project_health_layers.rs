#[path = "../src/project_health_layers.rs"]
mod project_health_layers;

use project_health_layers::{
    ActivationHealthObservation, BindingHealthObservation, CapabilityHealthObservation,
    ControlPlaneHealthLayer, CurrentSearchRequest, CurrentStoreHealthLayer, HealthIssue,
    HistoricalArchiveRequest, LayerHealth, RegistryHealthObservation, ResolutionHealthObservation,
    inspect_layered_project_health, render_layered_project_health_json,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use workvcs_core::{
    Engine, ProjectRefId, RecordCreateOptions, SearchRefreshOptions, SearchSourceIdentity,
    SourceArchiveImportOptions, StoreInitOptions, WorkspaceInfo, WorkspaceInitOptions,
};

struct RealFixture {
    _temporary: TempDir,
    store_path: PathBuf,
    search_index_path: PathBuf,
    archive_root: PathBuf,
    source_path: PathBuf,
    project_ref_id: ProjectRefId,
    workspace: WorkspaceInfo,
    identity: SearchSourceIdentity,
}

impl RealFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let store_path = temporary.path().join("workvcs.sqlite");
        let search_index_path = temporary.path().join("search.sqlite");
        let archive_root = temporary.path().join("archives");
        let source_path = temporary.path().join("rollout.jsonl");
        let mut engine = Engine::init(
            &store_path,
            StoreInitOptions::new("layered-health-store").expect("Store options"),
        )
        .expect("initialize Store");
        let store_id = engine.store_info().expect("Store info").store_id;
        let workspace = engine
            .create_workspace(
                WorkspaceInitOptions::new("layered-health-workspace").expect("Workspace options"),
            )
            .expect("create Workspace");
        engine
            .create_record(
                RecordCreateOptions::finding(
                    workspace.initial_branch_id,
                    workspace.genesis_commit_id,
                    "layered project health canonical evidence",
                )
                .expect("Record options"),
            )
            .expect("create Record");
        let project_ref_id = ProjectRefId::new_v7();
        let identity = SearchSourceIdentity::new(
            project_ref_id,
            store_id,
            workspace.workspace_id,
            workspace.initial_branch_id,
        );
        drop(engine);
        Self {
            _temporary: temporary,
            store_path,
            search_index_path,
            archive_root,
            source_path,
            project_ref_id,
            workspace,
            identity,
        }
    }

    fn current_store_health(&self) -> CurrentStoreHealthLayer {
        let engine = Engine::open_readonly(&self.store_path).expect("open Store read-only");
        let store = engine.store_info().expect("Store info");
        let head = engine
            .branch_head(self.workspace.initial_branch_id)
            .expect("Branch head");
        CurrentStoreHealthLayer::verified(
            &self.store_path,
            self.project_ref_id,
            store.store_id,
            self.workspace.workspace_id,
            self.workspace.initial_branch_id,
            head.head_commit_id,
            head.state_digest,
            engine.validate_integrity().expect("Store integrity"),
            engine
                .validate_local_content_storage()
                .expect("local content storage"),
        )
    }

    fn prepare_search_index(&self) {
        Engine::open_readonly(&self.store_path)
            .expect("open Store read-only")
            .refresh_search_index(SearchRefreshOptions::new(
                self.identity,
                &self.search_index_path,
            ))
            .expect("prepare exact search index");
    }

    fn prepare_archive(&self) -> PathBuf {
        write_rollout(&self.source_path);
        let source = fs::canonicalize(&self.source_path).expect("canonical source path");
        Engine::import_source_archive(
            SourceArchiveImportOptions::new(self.identity, &self.archive_root, vec![source])
                .expect("archive import options"),
        )
        .expect("import source archive")
        .archive_path
    }
}

fn healthy_control_plane(project_ref_id: ProjectRefId) -> ControlPlaneHealthLayer {
    ControlPlaneHealthLayer {
        health: LayerHealth::Healthy,
        checked: true,
        required_for_normal_work: true,
        registry: RegistryHealthObservation {
            path: PathBuf::from("/verified/project-registry.json"),
            source: "explicit".to_owned(),
            version: 2,
            revision: Some(7),
            digest: "sha256:verified-registry".to_owned(),
            migration_required: false,
        },
        bindings: vec![BindingHealthObservation {
            index: 0,
            project_ref_id: Some(project_ref_id.to_string()),
            verified: true,
            local_content_objects_verified: Some(0),
            issue: None,
        }],
        routing: ActivationHealthObservation {
            state: "active".to_owned(),
            path: Some(PathBuf::from("/verified/routing.json")),
            issue: None,
            version: Some(1),
        },
        journal_admission: ActivationHealthObservation {
            state: "active".to_owned(),
            path: Some(PathBuf::from("/verified/journal.json")),
            issue: None,
            version: Some(1),
        },
        capabilities: ["cognition_capture", "plan_admit", "plan_evolve"]
            .into_iter()
            .map(|name| CapabilityHealthObservation {
                name: name.to_owned(),
                active: true,
            })
            .collect(),
        resolution: ResolutionHealthObservation {
            requested: true,
            status: "resolved".to_owned(),
            project_ref_id: Some(project_ref_id.to_string()),
            issue: None,
        },
        issues: Vec::new(),
    }
}

fn write_rollout(path: &Path) {
    let lines = [
        serde_json::json!({
            "type": "session_meta",
            "timestamp": "2026-10-26T00:00:00Z",
            "payload": {"id": "019a21d6-2b2d-7f43-a2fd-77f71ed66a01"}
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-10-26T00:00:01Z",
            "payload": {
                "type": "message",
                "role": "user",
                "id": "019a21d6-2b2d-7f43-a2fd-77f71ed66a02",
                "content": [{"type": "input_text", "text": "historical layered health evidence"}]
            }
        }),
    ];
    let bytes = lines
        .into_iter()
        .map(|line| serde_json::to_string(&line).expect("serialize JSON line"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{bytes}\n")).expect("write rollout");
}

fn byte_snapshot(paths: impl IntoIterator<Item = PathBuf>) -> BTreeMap<PathBuf, Vec<u8>> {
    paths
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).expect("read snapshot file");
            (path, bytes)
        })
        .collect()
}

#[test]
fn unselected_optional_layers_stay_not_checked_without_claiming_health() {
    let fixture = RealFixture::new();
    let report = inspect_layered_project_health(
        healthy_control_plane(fixture.project_ref_id),
        fixture.current_store_health(),
        Vec::new(),
        None,
    );

    assert_eq!(report.health, LayerHealth::Healthy);
    assert!(report.normal_work_ready);
    assert_eq!(report.historical_sources.health, LayerHealth::NotChecked);
    assert!(!report.historical_sources.checked);
    assert_eq!(report.retrieval.health, LayerHealth::NotChecked);
    assert_eq!(
        report.retrieval.current_lexical.health,
        LayerHealth::NotChecked
    );
    assert_eq!(
        report.retrieval.historical_lexical.health,
        LayerHealth::NotChecked
    );
    assert_eq!(report.retrieval.semantic.health, LayerHealth::NotChecked);
    assert!(report.issues.is_empty());
    assert!(!report.writes.store_written);
    assert!(!report.writes.registry_written);
    assert!(!report.writes.index_written);
    assert!(!report.writes.archive_written);

    let json = render_layered_project_health_json(&report).expect("render JSON");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
    assert_eq!(value["health"], "healthy");
    assert_eq!(value["historical_sources"]["health"], "not_checked");
    assert_eq!(value["retrieval"]["semantic"]["health"], "not_checked");
    assert_eq!(value["writes"]["store_written"], false);
}

#[test]
fn unchecked_required_store_cannot_be_reported_ready_or_healthy() {
    let fixture = RealFixture::new();
    let report = inspect_layered_project_health(
        healthy_control_plane(fixture.project_ref_id),
        CurrentStoreHealthLayer::not_checked("no resolved current Store was selected"),
        Vec::new(),
        None,
    );

    assert_eq!(report.health, LayerHealth::Degraded);
    assert!(!report.normal_work_ready);
    assert_eq!(report.current_store.health, LayerHealth::NotChecked);
    assert_eq!(report.issues.len(), 1);
    assert_eq!(report.issues[0].code, "store_not_selected");
    assert!(report.issues[0].affects_normal_work);
    assert!(
        report.issues[0]
            .recovery_action
            .as_ref()
            .is_some_and(|action| !action.automatic && !action.requires_authorization)
    );
}

#[test]
fn selected_store_verification_failure_keeps_typed_identity_and_blocks_work() {
    let fixture = RealFixture::new();
    let store_id = Engine::open_readonly(&fixture.store_path)
        .expect("open Store read-only")
        .store_info()
        .expect("Store info")
        .store_id;
    let failed = CurrentStoreHealthLayer::failed_selection(
        &fixture.store_path,
        fixture.project_ref_id,
        store_id,
        fixture.workspace.workspace_id,
        fixture.workspace.initial_branch_id,
        "digest_invalid",
        "selected Store failed exact digest verification",
    );
    let report = inspect_layered_project_health(
        healthy_control_plane(fixture.project_ref_id),
        failed,
        Vec::new(),
        None,
    );

    assert_eq!(report.health, LayerHealth::Blocked);
    assert!(!report.normal_work_ready);
    assert_eq!(report.current_store.health, LayerHealth::Blocked);
    assert_eq!(
        report.current_store.project_ref_id.as_deref(),
        Some(fixture.project_ref_id.to_string().as_str())
    );
    assert_eq!(report.current_store.issues[0].code, "digest_invalid");
    assert!(report.current_store.issues[0].affects_normal_work);
}

#[test]
fn real_archive_and_search_inspection_is_read_only_and_keeps_optional_gaps_non_blocking() {
    let fixture = RealFixture::new();
    fixture.prepare_search_index();
    let archive_path = fixture.prepare_archive();
    let moved_source = fixture.source_path.with_extension("jsonl.moved");
    fs::rename(&fixture.source_path, &moved_source).expect("move original source");

    let manifest_path = archive_path.join("manifest.json");
    let documents_path = archive_path.join("documents.json");
    let tracked = vec![
        fixture.store_path.clone(),
        fixture.search_index_path.clone(),
        manifest_path,
        documents_path,
        moved_source,
    ];
    let before = byte_snapshot(tracked.clone());
    let directory_before = fs::read_dir(fixture._temporary.path())
        .expect("list temporary root before")
        .map(|entry| entry.expect("directory entry").file_name())
        .collect::<Vec<_>>();

    let report = inspect_layered_project_health(
        healthy_control_plane(fixture.project_ref_id),
        fixture.current_store_health(),
        vec![HistoricalArchiveRequest {
            identity: fixture.identity,
            archive_path: archive_path.clone(),
        }],
        Some(CurrentSearchRequest {
            store_path: fixture.store_path.clone(),
            identity: fixture.identity,
            index_path: fixture.search_index_path.clone(),
        }),
    );

    assert_eq!(report.control_plane.health, LayerHealth::Healthy);
    assert_eq!(report.current_store.health, LayerHealth::Healthy);
    assert_eq!(report.historical_sources.health, LayerHealth::Degraded);
    assert_eq!(report.historical_sources.ready_archives, 1);
    assert_eq!(report.historical_sources.failed_archives, 0);
    assert_eq!(
        report.historical_sources.archives[0].source_files.missing,
        1
    );
    assert!(report.historical_sources.archives[0].archive_readback_verified);
    assert_eq!(
        report.retrieval.current_lexical.health,
        LayerHealth::Healthy
    );
    assert!(report.retrieval.current_lexical.readback_verified);
    assert_eq!(
        report.retrieval.historical_lexical.health,
        LayerHealth::Healthy
    );
    assert!(report.retrieval.historical_lexical.readback_verified);
    assert_eq!(report.retrieval.semantic.health, LayerHealth::NotChecked);
    assert_eq!(report.health, LayerHealth::Degraded);
    assert!(report.normal_work_ready);
    assert!(report.issues.iter().any(|issue| {
        issue.code == "historical_source_currentness_gap"
            && !issue.affects_normal_work
            && issue
                .recovery_action
                .as_ref()
                .is_some_and(|action| !action.automatic && action.requires_authorization)
    }));

    let after = byte_snapshot(tracked);
    assert_eq!(
        after, before,
        "health inspection must not change inspected bytes"
    );
    let directory_after = fs::read_dir(fixture._temporary.path())
        .expect("list temporary root after")
        .map(|entry| entry.expect("directory entry").file_name())
        .collect::<Vec<_>>();
    assert_eq!(directory_after, directory_before);
}

#[test]
fn missing_selected_search_index_is_a_structured_optional_gap_without_writing_it() {
    let fixture = RealFixture::new();
    let missing_index = fixture._temporary.path().join("missing-search.sqlite");
    assert!(!missing_index.exists());

    let report = inspect_layered_project_health(
        healthy_control_plane(fixture.project_ref_id),
        fixture.current_store_health(),
        Vec::new(),
        Some(CurrentSearchRequest {
            store_path: fixture.store_path.clone(),
            identity: fixture.identity,
            index_path: missing_index.clone(),
        }),
    );

    assert_eq!(report.health, LayerHealth::Degraded);
    assert!(report.normal_work_ready);
    assert_eq!(report.retrieval.health, LayerHealth::Degraded);
    assert_eq!(
        report.retrieval.current_lexical.health,
        LayerHealth::Degraded
    );
    assert_eq!(
        report.retrieval.current_lexical.reason_code.as_deref(),
        Some("missing_index")
    );
    assert!(!report.retrieval.current_lexical.available);
    assert!(!missing_index.exists());
    let issue = report
        .issues
        .iter()
        .find(|issue| issue.code == "missing_index")
        .expect("structured missing index issue");
    assert!(!issue.affects_normal_work);
    assert!(
        issue
            .recovery_action
            .as_ref()
            .is_some_and(|action| { !action.automatic && action.requires_authorization })
    );
}

#[test]
fn mixed_real_archives_preserve_success_and_exact_failure_without_writes() {
    let fixture = RealFixture::new();
    let ready_archive = fixture.prepare_archive();
    let damaged_root = fixture._temporary.path().join("damaged-archives");
    let source = fs::canonicalize(&fixture.source_path).expect("canonical source path");
    let damaged_archive = Engine::import_source_archive(
        SourceArchiveImportOptions::new(fixture.identity, &damaged_root, vec![source])
            .expect("damaged archive import options"),
    )
    .expect("prepare archive to damage")
    .archive_path;
    let damaged_documents = damaged_archive.join("documents.json");
    fs::write(&damaged_documents, b"{").expect("damage archive documents fixture");

    let tracked = vec![
        fixture.store_path.clone(),
        fixture.source_path.clone(),
        ready_archive.join("manifest.json"),
        ready_archive.join("documents.json"),
        damaged_archive.join("manifest.json"),
        damaged_documents,
    ];
    let before = byte_snapshot(tracked.clone());
    let report = inspect_layered_project_health(
        healthy_control_plane(fixture.project_ref_id),
        fixture.current_store_health(),
        vec![
            HistoricalArchiveRequest {
                identity: fixture.identity,
                archive_path: ready_archive.clone(),
            },
            HistoricalArchiveRequest {
                identity: fixture.identity,
                archive_path: damaged_archive.clone(),
            },
        ],
        None,
    );

    assert_eq!(report.historical_sources.health, LayerHealth::Degraded);
    assert_eq!(report.historical_sources.ready_archives, 1);
    assert_eq!(report.historical_sources.failed_archives, 1);
    assert_eq!(
        report.historical_sources.archives[0].health,
        LayerHealth::Healthy
    );
    assert!(report.historical_sources.archives[0].archive_readback_verified);
    assert_eq!(
        report.historical_sources.archives[1].health,
        LayerHealth::Blocked
    );
    assert!(!report.historical_sources.archives[1].archive_readback_verified);
    let failure = report.historical_sources.archives[1]
        .issue
        .as_ref()
        .expect("damaged archive issue");
    assert!(!failure.code.is_empty());
    assert!(
        failure
            .message
            .contains(&damaged_archive.display().to_string())
    );
    assert_eq!(
        report.retrieval.historical_lexical.health,
        LayerHealth::Degraded
    );
    assert!(report.retrieval.historical_lexical.available);
    assert_eq!(report.health, LayerHealth::Degraded);
    assert!(report.normal_work_ready);
    assert_eq!(byte_snapshot(tracked), before);
}

#[test]
fn blocked_required_control_plane_blocks_normal_work_even_when_store_is_verified() {
    let fixture = RealFixture::new();
    let mut control = healthy_control_plane(fixture.project_ref_id);
    control.health = LayerHealth::Blocked;
    control.issues.push(HealthIssue::new(
        "control_plane",
        "control_plane_invalid",
        "routing activation is stale",
        true,
        None,
    ));
    let report =
        inspect_layered_project_health(control, fixture.current_store_health(), Vec::new(), None);

    assert_eq!(report.health, LayerHealth::Blocked);
    assert!(!report.normal_work_ready);
    assert_eq!(report.issues[0].code, "control_plane_invalid");
}
