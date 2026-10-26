use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use workvcs_core::control_plane::ProjectRegistryV2;
use workvcs_core::{Engine, SearchSourceIdentity, SourceArchiveImportOptions};

struct ProjectHealthCliFixture {
    _temporary: TempDir,
    store_path: PathBuf,
    registry_path: PathBuf,
    project_path: PathBuf,
    routing_activation_path: PathBuf,
    journal_activation_path: PathBuf,
    identity: SearchSourceIdentity,
}

impl ProjectHealthCliFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let store_path = temporary.path().join("workvcs.sqlite");
        let registry_path = temporary.path().join("registry/project-bindings.json");
        let project_path = temporary.path().join("project");
        fs::create_dir_all(&project_path).expect("create project directory");

        run(&[
            "init",
            text(&store_path),
            "--display-name",
            "typed-project-health",
        ]);
        let workspace = run(&[
            "workspace",
            "create",
            text(&store_path),
            "--display-name",
            "typed-project-health",
        ]);
        run(&[
            "project",
            "bind",
            "--cwd",
            text(&project_path),
            "--registry",
            text(&registry_path),
            "--store",
            text(&store_path),
            "--workspace",
            field(&workspace, "workspace_id"),
            "--branch",
            field(&workspace, "branch_id"),
        ]);

        let legacy = run(&[
            "project",
            "health",
            "--cwd",
            text(&project_path),
            "--registry",
            text(&registry_path),
        ]);
        assert_eq!(field(&legacy, "registry_version"), "1");
        assert_eq!(field(&legacy, "health"), "degraded");
        assert!(legacy.starts_with("health=degraded\nread_only=true\n"));

        let migration = run(&[
            "project",
            "registry-migrate",
            "--preview",
            "--registry",
            text(&registry_path),
        ]);
        run(&[
            "project",
            "registry-migrate",
            "--apply",
            "--registry",
            text(&registry_path),
            "--expected-source-digest",
            field(&migration, "source_digest"),
            "--expected-preview-digest",
            field(&migration, "preview_digest"),
        ]);

        let routing = run(&[
            "project",
            "routing-activation",
            "--preview",
            "--registry",
            text(&registry_path),
        ]);
        let routing_activation_path = PathBuf::from(field(&routing, "activation_path"));
        run(&[
            "project",
            "routing-activation",
            "--apply",
            "--registry",
            text(&registry_path),
            "--expected-registry-digest",
            field(&routing, "registry_digest"),
            "--expected-candidate-digest",
            field(&routing, "candidate_digest"),
        ]);

        let journal = run(&[
            "project",
            "journal-admission-activation",
            "--preview",
            "--registry",
            text(&registry_path),
        ]);
        let journal_activation_path = PathBuf::from(field(&journal, "activation_path"));
        run(&[
            "project",
            "journal-admission-activation",
            "--apply",
            "--registry",
            text(&registry_path),
            "--expected-registry-digest",
            field(&journal, "registry_digest"),
            "--expected-candidate-digest",
            field(&journal, "candidate_digest"),
        ]);

        let registry = ProjectRegistryV2::from_json_bytes(
            &fs::read(&registry_path).expect("read migrated registry"),
        )
        .expect("registry v2");
        let binding = registry.bindings().first().expect("fixture binding");
        let identity = SearchSourceIdentity::new(
            binding.project_ref_id(),
            binding.store_id(),
            binding.workspace_id(),
            binding.branch_id(),
        );
        Self {
            _temporary: temporary,
            store_path,
            registry_path,
            project_path,
            routing_activation_path,
            journal_activation_path,
            identity,
        }
    }

    fn health_json(&self, selector: &[&str], optional: &[&str]) -> serde_json::Value {
        let mut arguments = vec![
            "project",
            "health",
            "--registry",
            text(&self.registry_path),
            "--format",
            "json",
        ];
        arguments.extend_from_slice(selector);
        arguments.extend_from_slice(optional);
        serde_json::from_str(&run(&arguments)).expect("typed project health JSON")
    }
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_workvcs")
}

fn text(path: &Path) -> &str {
    path.to_str().expect("UTF-8 fixture path")
}

fn run(arguments: &[&str]) -> String {
    let output = Command::new(binary())
        .args(arguments)
        .output()
        .expect("run WorkVCS CLI");
    assert!(
        output.status.success(),
        "workvcs {:?} failed: stdout={} stderr={}",
        arguments,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 CLI output")
}

fn run_failure(arguments: &[&str]) -> String {
    let output = Command::new(binary())
        .args(arguments)
        .output()
        .expect("run failing WorkVCS CLI");
    assert!(
        !output.status.success(),
        "workvcs {:?} unexpectedly succeeded: stdout={} stderr={}",
        arguments,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stderr).expect("UTF-8 CLI error")
}

fn field<'a>(output: &'a str, key: &str) -> &'a str {
    output
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("missing {key} in {output}"))
}

fn snapshot(paths: impl IntoIterator<Item = PathBuf>) -> BTreeMap<PathBuf, Vec<u8>> {
    paths
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).expect("read fixture snapshot");
            (path, bytes)
        })
        .collect()
}

#[test]
fn registry_v1_json_marks_the_current_store_not_checked_and_rejects_identity_bound_optional_reads()
{
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store_path = temporary.path().join("workvcs.sqlite");
    let registry_path = temporary.path().join("registry/project-bindings.json");
    let project_path = temporary.path().join("project");
    fs::create_dir_all(&project_path).expect("create project directory");
    run(&[
        "init",
        text(&store_path),
        "--display-name",
        "legacy-typed-health",
    ]);
    let workspace = run(&[
        "workspace",
        "create",
        text(&store_path),
        "--display-name",
        "legacy-typed-health",
    ]);
    run(&[
        "project",
        "bind",
        "--cwd",
        text(&project_path),
        "--registry",
        text(&registry_path),
        "--store",
        text(&store_path),
        "--workspace",
        field(&workspace, "workspace_id"),
        "--branch",
        field(&workspace, "branch_id"),
    ]);

    let json: serde_json::Value = serde_json::from_str(&run(&[
        "project",
        "health",
        "--cwd",
        text(&project_path),
        "--registry",
        text(&registry_path),
        "--format",
        "json",
    ]))
    .expect("registry-v1 typed health JSON");
    assert_eq!(json["health"], "degraded");
    assert_eq!(json["normal_work_ready"], false);
    assert_eq!(json["control_plane"]["registry"]["version"], 1);
    assert_eq!(json["control_plane"]["health"], "degraded");
    assert_eq!(json["current_store"]["health"], "not_checked");
    assert_eq!(json["current_store"]["checked"], false);
    assert_eq!(json["historical_sources"]["health"], "not_checked");
    assert_eq!(json["retrieval"]["health"], "not_checked");

    let archive_path = temporary.path().join("archive");
    let error = run_failure(&[
        "project",
        "health",
        "--cwd",
        text(&project_path),
        "--registry",
        text(&registry_path),
        "--format",
        "json",
        "--source-archive",
        text(&archive_path),
    ]);
    assert!(error.contains("exact verified registry-v2 ProjectRef binding"));
}

#[test]
fn typed_json_uses_the_selected_v2_binding_proof_for_cwd_and_project_ref() {
    let fixture = ProjectHealthCliFixture::new();
    let before = snapshot([
        fixture.store_path.clone(),
        fixture.registry_path.clone(),
        fixture.routing_activation_path.clone(),
        fixture.journal_activation_path.clone(),
    ]);

    let cwd_health = fixture.health_json(&["--cwd", text(&fixture.project_path)], &[]);
    assert_eq!(cwd_health["schema_version"], 1);
    assert_eq!(cwd_health["health"], "healthy");
    assert_eq!(cwd_health["normal_work_ready"], true);
    assert_eq!(cwd_health["control_plane"]["health"], "healthy");
    assert_eq!(cwd_health["control_plane"]["registry"]["version"], 2);
    assert_eq!(cwd_health["control_plane"]["bindings"][0]["verified"], true);
    assert_eq!(cwd_health["current_store"]["health"], "healthy");
    assert_eq!(
        cwd_health["current_store"]["project_ref_id"],
        fixture.identity.project_ref_id.to_string()
    );
    assert_eq!(
        cwd_health["current_store"]["store_id"],
        fixture.identity.store_id.to_string()
    );
    assert!(cwd_health["current_store"]["integrity"]["checked_commits"].is_number());
    assert_eq!(cwd_health["historical_sources"]["health"], "not_checked");
    assert_eq!(cwd_health["retrieval"]["health"], "not_checked");
    assert_eq!(cwd_health["writes"]["store_written"], false);

    let project_ref = fixture.identity.project_ref_id.to_string();
    let explicit_health = fixture.health_json(&["--project-ref", &project_ref], &[]);
    assert_eq!(explicit_health["health"], "healthy");
    assert_eq!(explicit_health["normal_work_ready"], true);
    assert_eq!(
        explicit_health["current_store"]["head_commit_id"],
        cwd_health["current_store"]["head_commit_id"]
    );
    assert_eq!(
        explicit_health["current_store"]["head_state_digest"],
        cwd_health["current_store"]["head_state_digest"]
    );
    assert_eq!(snapshot(before.keys().cloned()), before);
}

#[test]
fn selected_optional_gap_degrades_json_without_blocking_or_writing_normal_work() {
    let fixture = ProjectHealthCliFixture::new();
    let source_path = fixture._temporary.path().join("rollout.jsonl");
    fs::write(
        &source_path,
        concat!(
            "{\"type\":\"session_meta\",\"timestamp\":\"2026-10-26T00:00:00Z\",",
            "\"payload\":{\"id\":\"019a21d6-2b2d-7f43-a2fd-77f71ed66a01\"}}\n",
            "{\"type\":\"response_item\",\"timestamp\":\"2026-10-26T00:00:01Z\",",
            "\"payload\":{\"type\":\"message\",\"role\":\"user\",",
            "\"id\":\"019a21d6-2b2d-7f43-a2fd-77f71ed66a02\",",
            "\"content\":[{\"type\":\"input_text\",\"text\":\"typed health evidence\"}]}}\n"
        ),
    )
    .expect("write source rollout");
    let archive = Engine::import_source_archive(
        SourceArchiveImportOptions::new(
            fixture.identity,
            fixture._temporary.path().join("archives"),
            vec![fs::canonicalize(&source_path).expect("canonical source")],
        )
        .expect("archive options"),
    )
    .expect("import source archive")
    .archive_path;
    let missing_search = fixture._temporary.path().join("missing-search.sqlite");
    let tracked = [
        fixture.store_path.clone(),
        fixture.registry_path.clone(),
        fixture.routing_activation_path.clone(),
        fixture.journal_activation_path.clone(),
        source_path,
        archive.join("manifest.json"),
        archive.join("documents.json"),
    ];
    let before = snapshot(tracked.clone());

    let health = fixture.health_json(
        &["--cwd", text(&fixture.project_path)],
        &[
            "--source-archive",
            text(&archive),
            "--search-index",
            text(&missing_search),
        ],
    );
    assert_eq!(health["health"], "degraded");
    assert_eq!(health["normal_work_ready"], true);
    assert_eq!(health["control_plane"]["health"], "healthy");
    assert_eq!(health["current_store"]["health"], "healthy");
    assert_eq!(health["historical_sources"]["health"], "healthy");
    assert_eq!(health["historical_sources"]["ready_archives"], 1);
    assert_eq!(health["retrieval"]["health"], "degraded");
    assert_eq!(
        health["retrieval"]["current_lexical"]["reason_code"],
        "missing_index"
    );
    assert_eq!(
        health["retrieval"]["historical_lexical"]["health"],
        "healthy"
    );
    assert_eq!(health["writes"]["index_written"], false);
    assert_eq!(health["writes"]["archive_written"], false);
    assert!(!missing_search.exists());
    assert_eq!(snapshot(tracked), before);
}
