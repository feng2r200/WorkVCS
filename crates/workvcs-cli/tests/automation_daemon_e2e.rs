#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const PROJECT_REF_ID: &str = "01900000-0000-7000-8000-000000000011";
const STORE_ID: &str = "01900000-0000-7000-8000-000000000012";
const WORKSPACE_ID: &str = "01900000-0000-7000-8000-000000000013";
const BRANCH_ID: &str = "01900000-0000-7000-8000-000000000014";

macro_rules! args {
    ($($value:expr),* $(,)?) => {
        [$($value.to_string()),*]
    };
}

struct Fixture {
    _temporary: TempDir,
    root: PathBuf,
    rollout_root: PathBuf,
    initial_source: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = TempDir::new().expect("temporary directory");
        let root = temporary.path().join("automation");
        let project_root = temporary.path().join("project");
        let rollout_root = temporary.path().join("rollouts");
        fs::create_dir(&root).expect("automation root");
        fs::create_dir(&project_root).expect("project root");
        fs::create_dir(&rollout_root).expect("rollout root");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
            .expect("protect automation root");
        let store_path = temporary.path().join("unavailable-store.sqlite");
        fs::write(&store_path, b"not a WorkVCS Store").expect("write unavailable Store");
        let initial_source = rollout_root.join("initial.jsonl");
        write_rollout(
            &initial_source,
            "initial watcher evidence",
            "initial assistant text",
        );
        let registry = json!({
            "format_version": 1,
            "revision": 1,
            "registrations": [{
                "registration_id": "daemon-e2e",
                "revision": 1,
                "project_ref_id": PROJECT_REF_ID,
                "store_path": store_path,
                "store_id": STORE_ID,
                "workspace_id": WORKSPACE_ID,
                "branch_id": BRANCH_ID,
                "project_root": project_root,
                "rollout_roots": [rollout_root],
                "role_policy": "user_only",
                "registry_revision": 1,
                "registry_digest": format!("blake3-256:{}", "1".repeat(64)),
                "enabled": true,
                "revoked": false,
                "created_at_us": 1,
                "updated_at_us": 1
            }]
        });
        let registry_path = root.join("registrations.json");
        fs::write(
            &registry_path,
            serde_json::to_vec(&registry).expect("serialize automation registry"),
        )
        .expect("write automation registry");
        fs::set_permissions(&registry_path, fs::Permissions::from_mode(0o600))
            .expect("protect automation registry");
        Self {
            _temporary: temporary,
            root,
            rollout_root,
            initial_source,
        }
    }
}

struct RegistryDerivedFixture {
    _temporary: TempDir,
    root: PathBuf,
    registry: PathBuf,
    project_a: PathBuf,
    project_b: PathBuf,
    rollout_b: PathBuf,
    store_b: PathBuf,
    branch_b: String,
    full_registry: Value,
}

impl RegistryDerivedFixture {
    fn new(binary: &Path) -> Self {
        let temporary = TempDir::new().expect("temporary directory");
        let project_a = temporary.path().join("project-a");
        let project_b = temporary.path().join("project-b");
        let stores = temporary.path().join("stores");
        let rollout_a = temporary.path().join("rollouts-a");
        let rollout_b = temporary.path().join("rollouts-b");
        let root = temporary.path().join("automation");
        let registry = temporary.path().join("project-bindings.json");
        for path in [&project_a, &project_b, &stores, &rollout_a, &rollout_b] {
            fs::create_dir(path).expect("fixture directory");
        }

        let ensured_a = run_ok(
            binary,
            &args![
                "project",
                "ensure",
                "--cwd",
                path_text(&project_a),
                "--registry",
                path_text(&registry),
                "--store-root",
                path_text(&stores),
            ],
        );
        let ensured_a = String::from_utf8(ensured_a.stdout).expect("project A ensure UTF-8");
        let ensured_b = run_ok(
            binary,
            &args![
                "project",
                "ensure",
                "--cwd",
                path_text(&project_b),
                "--registry",
                path_text(&registry),
                "--store-root",
                path_text(&stores),
            ],
        );
        let ensured_b = String::from_utf8(ensured_b.stdout).expect("project B ensure UTF-8");
        let store_b = PathBuf::from(key_value(&ensured_b, "store_path"));
        let branch_b = key_value(&ensured_b, "branch_id");

        let preview = run_ok(
            binary,
            &args![
                "project",
                "registry-migrate",
                "--preview",
                "--registry",
                path_text(&registry),
                "--format",
                "json".to_owned(),
            ],
        );
        let preview: Value =
            serde_json::from_slice(&preview.stdout).expect("registry migration preview JSON");
        run_ok(
            binary,
            &args![
                "project",
                "registry-migrate",
                "--apply",
                "--registry",
                path_text(&registry),
                "--expected-source-digest",
                string_field(&preview["preview"], "source_digest"),
                "--expected-preview-digest",
                string_field(&preview["preview"], "preview_digest"),
            ],
        );

        let mut full_registry: Value =
            serde_json::from_slice(&fs::read(&registry).expect("read full registry v2"))
                .expect("full registry v2 JSON");
        let canonical_a = fs::canonicalize(&project_a).expect("canonical project A");
        let project_a_ref = full_registry["locators"]
            .as_array()
            .expect("registry locators")
            .iter()
            .find(|locator| locator["normalized_value"] == path_text(&canonical_a))
            .and_then(|locator| locator["project_ref_id"].as_str())
            .expect("project A ProjectRef")
            .to_owned();
        let mut reduced_registry = full_registry.clone();
        reduced_registry["projects"]
            .as_array_mut()
            .expect("registry projects")
            .retain(|project| project["project_ref_id"] == project_a_ref);
        reduced_registry["bindings"]
            .as_array_mut()
            .expect("registry bindings")
            .retain(|binding| binding["project_ref_id"] == project_a_ref);
        reduced_registry["locators"]
            .as_array_mut()
            .expect("registry locators")
            .retain(|locator| locator["project_ref_id"] == project_a_ref);
        reduced_registry["migration"]["mappings"]
            .as_array_mut()
            .expect("migration mappings")
            .retain(|mapping| mapping["project_ref_id"] == project_a_ref);
        reduced_registry["revision"] = json!(1);
        full_registry["revision"] = json!(2);
        write_json_atomically(&registry, &reduced_registry);
        activate_routing(binary, &registry, None);

        write_project_rollout(
            &rollout_a.join("initial.jsonl"),
            &project_a,
            "global initial watcher evidence",
        );
        apply_source_policy(binary, &root, &registry, "source-a", &rollout_a);

        assert!(!ensured_a.is_empty());
        Self {
            _temporary: temporary,
            root,
            registry,
            project_a,
            project_b,
            rollout_b,
            store_b,
            branch_b,
            full_registry,
        }
    }
}

struct RunningDaemon {
    child: Option<Child>,
    binary: PathBuf,
    root: PathBuf,
}

impl RunningDaemon {
    fn start(binary: &Path, root: &Path, periodic_seconds: u64) -> Self {
        Self::start_inner(binary, root, None, periodic_seconds)
    }

    fn start_with_registry(
        binary: &Path,
        root: &Path,
        registry: &Path,
        periodic_seconds: u64,
    ) -> Self {
        Self::start_inner(binary, root, Some(registry), periodic_seconds)
    }

    fn start_inner(
        binary: &Path,
        root: &Path,
        registry: Option<&Path>,
        periodic_seconds: u64,
    ) -> Self {
        let mut arguments = args![
            "automation",
            "daemon",
            "run",
            "--root",
            path_text(root),
            "--periodic-seconds",
            periodic_seconds,
        ]
        .to_vec();
        if let Some(registry) = registry {
            arguments.push("--registry".to_owned());
            arguments.push(path_text(registry));
        }
        let child = Command::new(binary)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start automation daemon");
        let mut daemon = Self {
            child: Some(child),
            binary: binary.to_path_buf(),
            root: root.to_path_buf(),
        };
        wait_for_status(&daemon.binary, &daemon.root, |_| true);
        assert!(daemon.child.as_mut().unwrap().try_wait().unwrap().is_none());
        daemon
    }

    fn stop(&mut self) {
        if self.child.is_none() {
            return;
        }
        let output = daemon_command(&self.binary, &self.root, "stop");
        assert_success(&output, "stop automation daemon");
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self
                .child
                .as_mut()
                .expect("daemon child")
                .try_wait()
                .expect("inspect daemon child")
                .is_some()
            {
                self.child.take();
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("automation daemon did not stop within the bound");
    }

    fn signal_and_wait(&mut self, signal: libc::c_int) -> std::process::ExitStatus {
        let pid = self.child.as_ref().expect("running daemon child").id();
        assert_eq!(
            unsafe { libc::kill(pid as libc::pid_t, signal) },
            0,
            "signal automation daemon: {}",
            std::io::Error::last_os_error()
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Some(status) = self
                .child
                .as_mut()
                .expect("daemon child")
                .try_wait()
                .expect("inspect signalled daemon child")
            {
                self.child.take();
                return status;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("signalled automation daemon did not stop within the bound");
    }

    fn kill_hard(&mut self) {
        let mut child = self.child.take().expect("running daemon child");
        child.kill().expect("kill automation daemon");
        child.wait().expect("reap killed automation daemon");
    }
}

impl Drop for RunningDaemon {
    fn drop(&mut self) {
        if self.child.is_none() {
            return;
        }
        let _ = daemon_command(&self.binary, &self.root, "stop");
        if let Some(mut child) = self.child.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if child.try_wait().ok().flatten().is_some() {
                    return;
                }
                thread::sleep(Duration::from_millis(25));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn empty_registry_daemon_startup_projects_failed() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_workvcs"));
    let temporary = TempDir::new().expect("temporary directory");
    let root = temporary.path().join("empty-automation");
    fs::create_dir(&root).expect("empty automation root");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
        .expect("protect empty automation root");

    let mut daemon = RunningDaemon::start(&binary, &root, 60);
    wait_for_status(&binary, &root, |ledger| ledger["worker_active"] == false);
    let output = daemon_command(&binary, &root, "status");
    assert_success(&output, "read failed daemon status");
    let response: Value = serde_json::from_slice(&output.stdout).expect("daemon response JSON");
    assert_eq!(response["status"], "failed");
    let ledger: Value = serde_json::from_str(
        response["message"]
            .as_str()
            .expect("daemon ledger response message"),
    )
    .expect("daemon ledger JSON");
    assert_eq!(ledger["last_trigger"], "startup");
    assert_eq!(ledger["last_worker_status"], "failed");
    assert_eq!(ledger["last_catalog_complete"], false);
    assert_eq!(ledger["daemon_enabled_by_default"], false);
    assert_eq!(ledger["launch_agent_installed"], Value::Null);
    daemon.stop();
}

#[test]
fn daemon_reconciles_real_file_events_and_recovers_from_crash() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_workvcs"));
    let fixture = Fixture::new();
    let mut daemon = RunningDaemon::start(&binary, &fixture.root, 60);
    let startup = wait_for_status(&binary, &fixture.root, |ledger| {
        ledger["last_trigger"] == "startup" && ledger["worker_active"] == false
    });
    assert_eq!(startup["last_trigger"], "startup");
    assert_eq!(startup["daemon_enabled_by_default"], false);
    assert_eq!(startup["launch_agent_installed"], Value::Null);
    let mut updated_at = u64_field(&startup, "updated_at_us");

    let manual = daemon_command(&binary, &fixture.root, "reconcile");
    assert_success(&manual, "request manual daemon reconciliation");
    let manual_response: Value =
        serde_json::from_slice(&manual.stdout).expect("manual daemon response JSON");
    assert_eq!(manual_response["status"], "accepted");
    let manual_ledger = wait_for_status(&binary, &fixture.root, |ledger| {
        ledger["last_trigger"] == "manual"
            && ledger["worker_active"] == false
            && u64_field(ledger, "updated_at_us") > updated_at
    });
    updated_at = u64_field(&manual_ledger, "updated_at_us");

    let created = fixture.rollout_root.join("created.jsonl");
    write_rollout(
        &created,
        "created watcher evidence",
        "created assistant text",
    );
    let ledger = wait_for_reconciliation(&binary, &fixture.root, updated_at);
    updated_at = u64_field(&ledger, "updated_at_us");
    assert_eq!(ledger["last_trigger"], "watcher_dirty_or_overflow");
    let report = last_worker_report(&fixture.root);
    assert!(u64_field(&report, "rollout_files_new") >= 1);
    assert_eq!(u64_field(&report, "rollout_files_discovered"), 2);

    let appended = json!({"type":"response_item","timestamp":"2026-01-01T00:00:03Z","payload":{"type":"message","role":"user","id":"01900000-0000-7000-8000-000000000099","content":[{"type":"input_text","text":"appended watcher evidence"}]}});
    writeln!(
        OpenOptions::new()
            .append(true)
            .open(&created)
            .expect("open append source"),
        "{}",
        serde_json::to_string(&appended).expect("serialize append record")
    )
    .expect("append rollout record");
    let ledger = wait_for_reconciliation(&binary, &fixture.root, updated_at);
    updated_at = u64_field(&ledger, "updated_at_us");
    assert!(u64_field(&last_worker_report(&fixture.root), "rollout_files_appended") >= 1);

    let replacement = fixture.rollout_root.join("replacement.tmp");
    write_rollout(
        &replacement,
        "replacement watcher evidence with different bytes",
        "replacement assistant text",
    );
    fs::rename(&replacement, &created).expect("atomically replace rollout");
    let ledger = wait_for_reconciliation(&binary, &fixture.root, updated_at);
    updated_at = u64_field(&ledger, "updated_at_us");
    assert!(
        u64_field(
            &last_worker_report(&fixture.root),
            "rollout_files_rewritten"
        ) >= 1
    );

    let renamed = fixture.rollout_root.join("renamed.jsonl");
    fs::rename(&created, &renamed).expect("rename rollout");
    let ledger = wait_for_reconciliation(&binary, &fixture.root, updated_at);
    updated_at = u64_field(&ledger, "updated_at_us");
    let report = last_worker_report(&fixture.root);
    assert!(u64_field(&report, "rollout_files_new") >= 1);
    assert!(u64_field(&report, "rollout_files_deleted") >= 1);

    fs::remove_file(&renamed).expect("delete rollout");
    let ledger = wait_for_reconciliation(&binary, &fixture.root, updated_at);
    let report = last_worker_report(&fixture.root);
    assert!(u64_field(&report, "rollout_files_deleted") >= 1);
    assert!(u64_field(&ledger, "watcher_events") >= 5);
    assert!(fixture.initial_source.exists());

    let second = Command::new(&binary)
        .args([
            "automation",
            "daemon",
            "run",
            "--root",
            fixture.root.to_str().expect("UTF-8 automation root"),
            "--periodic-seconds",
            "60",
        ])
        .output()
        .expect("run second daemon");
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("already running"));

    daemon.stop();
    assert!(!fixture.root.join("runtime/v1.sock").exists());

    let mut periodic = RunningDaemon::start(&binary, &fixture.root, 1);
    let periodic_ledger = wait_for_status(&binary, &fixture.root, |ledger| {
        ledger["last_trigger"] == "periodic" && ledger["worker_active"] == false
    });
    assert_eq!(periodic_ledger["last_trigger"], "periodic");
    periodic.kill_hard();
    let stale_socket = fixture.root.join("runtime/v1.sock");
    assert!(
        fs::symlink_metadata(&stale_socket)
            .expect("stale socket metadata")
            .file_type()
            .is_socket()
    );

    let mut restarted = RunningDaemon::start(&binary, &fixture.root, 60);
    let restarted_ledger = wait_for_status(&binary, &fixture.root, |ledger| {
        ledger["last_trigger"] == "startup" && ledger["worker_active"] == false
    });
    assert_eq!(restarted_ledger["last_trigger"], "startup");
    assert_ne!(restarted_ledger["pid"], periodic_ledger["pid"]);
    restarted.stop();
    assert!(!stale_socket.exists());
}

#[test]
fn daemon_signals_cancel_and_reap_slow_worker_before_socket_and_lock_recovery() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_workvcs"));
    let fixture = Fixture::new();
    for index in 0..800 {
        write_rollout(
            &fixture.rollout_root.join(format!("slow-{index:04}.jsonl")),
            &format!("slow worker signal evidence {index}"),
            "assistant evidence",
        );
    }

    for (signal, expected_trigger) in [
        (libc::SIGTERM, "signal_sigterm"),
        (libc::SIGINT, "signal_sigint"),
    ] {
        let mut daemon = RunningDaemon::start(&binary, &fixture.root, 600);
        let active = wait_for_status(&binary, &fixture.root, |ledger| {
            ledger["worker_active"] == true && ledger["active_worker_pid"].is_number()
        });
        let worker_pid = u64_field(&active, "active_worker_pid") as libc::pid_t;
        let status = daemon.signal_and_wait(signal);
        assert!(status.success());
        assert!(!fixture.root.join("runtime/v1.sock").exists());

        let ledger: Value = serde_json::from_slice(
            &fs::read(fixture.root.join("runtime/status.json"))
                .expect("read graceful signal ledger"),
        )
        .expect("parse graceful signal ledger");
        assert_eq!(ledger["last_trigger"], expected_trigger);
        assert_eq!(ledger["worker_active"], false);
        assert_eq!(ledger["active_worker_pid"], Value::Null);
        assert_eq!(ledger["pending_reconcile"], false);
        assert_eq!(ledger["last_worker_cancelled"], true);
        assert!(ledger["last_worker_evidence_path"].is_string());

        assert_eq!(unsafe { libc::kill(worker_pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH),
            "reconciliation worker {worker_pid} remained after daemon signal"
        );
    }

    let mut recovered = RunningDaemon::start(&binary, &fixture.root, 600);
    recovered.stop();
    assert!(!fixture.root.join("runtime/v1.sock").exists());
}

#[test]
fn daemon_remains_responsive_and_runs_manual_request_queued_behind_slow_worker() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_workvcs"));
    let fixture = Fixture::new();
    for index in 0..400 {
        write_rollout(
            &fixture
                .rollout_root
                .join(format!("queued-slow-{index:04}.jsonl")),
            &format!("slow worker queued manual evidence {index}"),
            "assistant evidence",
        );
    }

    let mut daemon = RunningDaemon::start(&binary, &fixture.root, 600);
    let startup = wait_for_status(&binary, &fixture.root, |ledger| {
        ledger["worker_active"] == true && ledger["active_worker_trigger"] == "startup"
    });
    let startup_updated_at = u64_field(&startup, "updated_at_us");
    let request_started = Instant::now();
    let manual = daemon_command(&binary, &fixture.root, "reconcile");
    assert!(request_started.elapsed() < Duration::from_secs(2));
    assert_success(&manual, "queue manual reconciliation behind slow worker");
    let manual_response: Value =
        serde_json::from_slice(&manual.stdout).expect("manual daemon response JSON");
    assert_eq!(manual_response["status"], "accepted");
    assert!(
        manual_response["message"]
            .as_str()
            .expect("manual response message")
            .contains("coalesced behind the active worker")
    );

    let queued = daemon_status(&binary, &fixture.root);
    assert_eq!(queued["worker_active"], true);
    assert_eq!(queued["pending_reconcile"], true);
    assert_eq!(queued["active_worker_trigger"], "startup");
    let completed = wait_for_status(&binary, &fixture.root, |ledger| {
        ledger["last_trigger"] == "manual"
            && ledger["worker_active"] == false
            && u64_field(ledger, "updated_at_us") > startup_updated_at
    });
    assert_eq!(completed["pending_reconcile"], false);
    assert_eq!(completed["last_worker_cancelled"], false);
    daemon.stop();
}

#[test]
fn registry_derived_daemon_reloads_global_watch_targets_without_output_feedback() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_workvcs"));
    let fixture = RegistryDerivedFixture::new(&binary);
    let mut daemon =
        RunningDaemon::start_with_registry(&binary, &fixture.root, &fixture.registry, 600);
    let startup = wait_for_status(&binary, &fixture.root, |ledger| {
        ledger["last_trigger"] == "startup" && ledger["worker_active"] == false
    });
    assert_eq!(startup["worker_timeout_seconds"], Value::Null);
    assert_eq!(startup["launch_agent_installed"], Value::Null);
    let startup_report = last_worker_report(&fixture.root);
    assert_eq!(startup_report["membership_mode"], "registry_derived");
    assert_eq!(startup_report["registrations_considered"], 1);
    let startup_events = u64_field(&startup, "watcher_events");
    thread::sleep(Duration::from_millis(500));
    let after_readback = daemon_status(&binary, &fixture.root);
    assert_eq!(after_readback["updated_at_us"], startup["updated_at_us"]);
    assert_eq!(u64_field(&after_readback, "watcher_events"), startup_events);

    let before_policy = u64_field(&after_readback, "updated_at_us");
    apply_source_policy(
        &binary,
        &fixture.root,
        &fixture.registry,
        "source-b",
        &fixture.rollout_b,
    );
    let policy_reloaded = wait_for_reconciliation(&binary, &fixture.root, before_policy);
    assert_eq!(policy_reloaded["worker_active"], false);
    assert_eq!(
        last_worker_report(&fixture.root)["source_policy_revision"],
        2
    );

    let before_new_root = u64_field(&policy_reloaded, "updated_at_us");
    write_project_rollout(
        &fixture.rollout_b.join("policy-added.jsonl"),
        &fixture.project_a,
        "policy-added rollout watcher evidence",
    );
    let new_root = wait_for_reconciliation(&binary, &fixture.root, before_new_root);
    assert!(u64_field(&last_worker_report(&fixture.root), "rollout_files_new") >= 1);

    let prior_activation_digest = key_value(
        &routing_preview(&binary, &fixture.registry),
        "activation_digest",
    );
    let before_membership = u64_field(&new_root, "updated_at_us");
    write_json_atomically(&fixture.registry, &fixture.full_registry);
    activate_routing(&binary, &fixture.registry, Some(prior_activation_digest));
    let membership_reloaded =
        wait_for_report(&binary, &fixture.root, before_membership, |report| {
            report["registrations_considered"] == 2
        });
    let membership_events = u64_field(&membership_reloaded, "watcher_events");

    record_finding(
        &binary,
        &fixture.store_b,
        &fixture.branch_b,
        "new registry member Store watcher evidence",
    );
    let member_store = wait_for_reconciliation(
        &binary,
        &fixture.root,
        u64_field(&membership_reloaded, "updated_at_us"),
    );
    assert!(u64_field(&member_store, "watcher_events") > membership_events);
    assert_eq!(
        last_worker_report(&fixture.root)["registrations_considered"],
        2
    );

    write_project_rollout(
        &fixture.rollout_b.join("member-added.jsonl"),
        &fixture.project_b,
        "new ProjectRef rollout watcher evidence",
    );
    let member_rollout = wait_for_reconciliation(
        &binary,
        &fixture.root,
        u64_field(&member_store, "updated_at_us"),
    );
    assert!(u64_field(&member_rollout, "watcher_events") > membership_events);
    assert_eq!(
        last_worker_report(&fixture.root)["rollout_archives_failed"],
        0
    );

    daemon.stop();
}

#[test]
fn live_project_ref_binding_drift_quarantines_canonical_but_preserves_history() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_workvcs"));
    let temporary = TempDir::new().expect("temporary directory");
    let project = temporary.path().join("project");
    let peer_project = temporary.path().join("peer-project");
    let stores = temporary.path().join("stores");
    let rollout_root = temporary.path().join("rollouts");
    let peer_rollout_root = temporary.path().join("peer-rollouts");
    let automation_root = temporary.path().join("automation");
    let project_registry = temporary.path().join("project-bindings.json");
    fs::create_dir(&project).expect("project root");
    fs::create_dir(&peer_project).expect("peer project root");
    fs::create_dir(&stores).expect("Store root");
    fs::create_dir(&rollout_root).expect("rollout root");
    fs::create_dir(&peer_rollout_root).expect("peer rollout root");
    let source = rollout_root.join("binding-drift.jsonl");
    write_rollout(
        &source,
        "binding drift historical evidence",
        "assistant-only drift text",
    );
    write_rollout(
        &peer_rollout_root.join("peer.jsonl"),
        "peer historical evidence",
        "peer assistant-only text",
    );

    let ensured = run_ok(
        &binary,
        &args![
            "project",
            "ensure",
            "--cwd",
            path_text(&project),
            "--registry",
            path_text(&project_registry),
            "--store-root",
            path_text(&stores),
        ],
    );
    let ensured_text = String::from_utf8(ensured.stdout).expect("project ensure UTF-8");
    let store_path = PathBuf::from(key_value(&ensured_text, "store_path"));
    let initial_branch_id = key_value(&ensured_text, "branch_id");

    let peer_ensured = run_ok(
        &binary,
        &args![
            "project",
            "ensure",
            "--cwd",
            path_text(&peer_project),
            "--registry",
            path_text(&project_registry),
            "--store-root",
            path_text(&stores),
        ],
    );
    let peer_ensured_text =
        String::from_utf8(peer_ensured.stdout).expect("peer project ensure UTF-8");
    let peer_store_path = PathBuf::from(key_value(&peer_ensured_text, "store_path"));
    let peer_branch_id = key_value(&peer_ensured_text, "branch_id");

    let equivalence_query = "canonical federation equivalence evidence";
    record_finding(&binary, &store_path, &initial_branch_id, equivalence_query);
    record_finding(
        &binary,
        &peer_store_path,
        &peer_branch_id,
        equivalence_query,
    );

    let preview = run_ok(
        &binary,
        &args![
            "project",
            "registry-migrate",
            "--preview",
            "--registry",
            path_text(&project_registry),
            "--format",
            "json".to_owned(),
        ],
    );
    let preview: Value = serde_json::from_slice(&preview.stdout).expect("migration preview JSON");
    let source_digest = string_field(&preview["preview"], "source_digest");
    let preview_digest = string_field(&preview["preview"], "preview_digest");
    run_ok(
        &binary,
        &args![
            "project",
            "registry-migrate",
            "--apply",
            "--registry",
            path_text(&project_registry),
            "--expected-source-digest",
            source_digest,
            "--expected-preview-digest",
            preview_digest,
        ],
    );
    activate_routing(&binary, &project_registry, None);

    let project_ref_id = register_automation_project(
        &binary,
        &project,
        &project_registry,
        &automation_root,
        "binding-drift",
        &rollout_root,
    );
    let peer_project_ref_id = register_automation_project(
        &binary,
        &peer_project,
        &project_registry,
        &automation_root,
        "federation-peer",
        &peer_rollout_root,
    );

    let initial = reconcile(&binary, &automation_root, &project_registry);
    assert_eq!(initial["status"], "completed");
    assert_eq!(initial["complete"], true);
    let initial_catalog = read_catalog(&initial);
    let initial_history = ready_history_source_for(&initial_catalog, &project_ref_id);

    let catalog_query = run_ok(
        &binary,
        &args![
            "automation",
            "catalog",
            "query",
            "--root",
            path_text(&automation_root),
            "--catalog",
            string_field(&initial, "catalog_path"),
            "--query",
            equivalence_query.to_owned(),
        ],
    );
    let catalog_query: Value =
        serde_json::from_slice(&catalog_query.stdout).expect("catalog query JSON");
    assert_eq!(catalog_query["status"], "completed");
    assert_eq!(catalog_query["complete"], true);

    let explicit_federation_root = fs::canonicalize(temporary.path())
        .expect("canonical temporary root")
        .join("explicit-federation");
    let explicit_federation = run_ok(
        &binary,
        &args![
            "search",
            "federated-query",
            "--cwd",
            path_text(&project),
            "--registry",
            path_text(&project_registry),
            "--source-project-ref",
            project_ref_id.clone(),
            "--source-project-ref",
            peer_project_ref_id,
            "--derived-dir",
            path_text(&explicit_federation_root),
            "--query",
            equivalence_query.to_owned(),
            "--format",
            "json".to_owned(),
        ],
    );
    let explicit_federation: Value = serde_json::from_slice(&explicit_federation.stdout)
        .expect("explicit federation query JSON");
    assert_eq!(
        explicit_federation["status"], "ready",
        "{explicit_federation:#}"
    );
    assert_eq!(explicit_federation["complete"], true);
    assert_eq!(explicit_federation["sources_requested"], 2);
    assert_eq!(explicit_federation["sources_succeeded"], 2);
    let catalog_canonical_items = catalog_query["items"]
        .as_array()
        .expect("catalog query items")
        .iter()
        .filter(|item| item["lane"] == "canonical_current")
        .collect::<Vec<_>>();
    let explicit_items = explicit_federation["items"]
        .as_array()
        .expect("explicit federation items");
    assert_eq!(catalog_canonical_items.len(), explicit_items.len());
    assert_eq!(catalog_canonical_items.len(), 2);
    for (catalog_item, explicit_item) in catalog_canonical_items.iter().zip(explicit_items) {
        assert_eq!(
            catalog_item["project_ref_id"],
            explicit_item["source_identity"]["project_ref_id"]
        );
        assert_eq!(
            catalog_item["source_local_rank"],
            explicit_item["local_rank"]
        );
        assert_eq!(catalog_item["source_score"], explicit_item["local_score"]);
        assert_eq!(
            catalog_item["federated_score"],
            explicit_item["federated_score"]
        );
        assert_eq!(catalog_item["payload"], explicit_item["payload"]["item"]);
    }

    let forked = run_ok(
        &binary,
        &args![
            "branch",
            "fork",
            path_text(&store_path),
            "--from-branch",
            initial_branch_id,
            "--name",
            "automation-drift".to_owned(),
        ],
    );
    let forked_text = String::from_utf8(forked.stdout).expect("branch fork UTF-8");
    let drifted_branch_id = key_value(&forked_text, "branch_id");
    let store_sha_before = sha256_file(&store_path);
    let source_sha_before = sha256_file(&source);

    let activation_before = routing_preview(&binary, &project_registry);
    let prior_activation_digest = key_value(&activation_before, "activation_digest");
    let mut registry: Value =
        serde_json::from_slice(&fs::read(&project_registry).expect("read v2 registry"))
            .expect("v2 registry JSON");
    registry["revision"] = json!(2);
    let binding = registry["bindings"]
        .as_array_mut()
        .expect("registry bindings")
        .iter_mut()
        .find(|binding| binding["project_ref_id"] == project_ref_id)
        .expect("primary project binding");
    binding["branch_id"] = json!(drifted_branch_id);
    let mut registry_bytes = serde_json::to_vec(&registry).expect("serialize drifted registry");
    registry_bytes.push(b'\n');
    fs::write(&project_registry, registry_bytes).expect("publish drifted registry fixture");
    activate_routing(&binary, &project_registry, Some(prior_activation_digest));

    let drifted = reconcile(&binary, &automation_root, &project_registry);
    assert_eq!(drifted["status"], "partial_failure");
    assert_eq!(drifted["complete"], false);
    assert_eq!(drifted["registrations_quarantined"], 2);
    assert_eq!(drifted["canonical_generations_ready"], 0);
    assert_eq!(drifted["canonical_generations_failed"], 2);
    assert_eq!(drifted["rollout_archives_ready"], 2);
    let drifted_catalog = read_catalog(&drifted);
    assert!(
        drifted_catalog["sources"]
            .as_array()
            .expect("catalog sources")
            .iter()
            .any(|source| source["lane"] == "canonical_current"
                && source["status"] == "failed"
                && source["error_code"] == "binding_drift")
    );
    let drifted_history = ready_history_source_for(&drifted_catalog, &project_ref_id);
    assert_eq!(
        initial_history["artifact_path"],
        drifted_history["artifact_path"]
    );
    assert_eq!(
        initial_history["artifact_manifest_sha256"],
        drifted_history["artifact_manifest_sha256"]
    );

    let query = run_ok(
        &binary,
        &args![
            "automation",
            "catalog",
            "query",
            "--root",
            path_text(&automation_root),
            "--catalog",
            string_field(&drifted, "catalog_path"),
            "--query",
            "binding drift historical evidence".to_owned(),
        ],
    );
    let query: Value = serde_json::from_slice(&query.stdout).expect("catalog query JSON");
    assert_eq!(query["status"], "partial_failure");
    assert_eq!(query["sources_succeeded"], 2);
    assert!(u64_field(&query, "matched_items") >= 1);
    assert!(
        query["items"]
            .as_array()
            .expect("query items")
            .iter()
            .all(|item| item["lane"] == "historical_reference")
    );
    assert_eq!(sha256_file(&store_path), store_sha_before);
    assert_eq!(sha256_file(&source), source_sha_before);
}

fn write_rollout(path: &Path, user_text: &str, assistant_text: &str) {
    let records = [
        json!({"type":"session_meta","timestamp":"2026-01-01T00:00:00Z","payload":{"id":"01900000-0000-7000-8000-000000000001"}}),
        json!({"type":"response_item","timestamp":"2026-01-01T00:00:01Z","payload":{"type":"message","role":"user","id":"01900000-0000-7000-8000-000000000002","content":[{"type":"input_text","text":user_text}]}}),
        json!({"type":"response_item","timestamp":"2026-01-01T00:00:02Z","payload":{"type":"message","role":"assistant","id":"01900000-0000-7000-8000-000000000003","content":[{"type":"output_text","text":assistant_text}]}}),
    ];
    let bytes = records
        .iter()
        .map(|record| serde_json::to_string(record).expect("serialize rollout record"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{bytes}\n")).expect("write rollout source");
}

fn write_project_rollout(path: &Path, cwd: &Path, user_text: &str) {
    let records = [
        json!({"type":"session_meta","timestamp":"2026-01-01T00:00:00Z","payload":{"id":"01900000-0000-7000-8000-000000000021","cwd":cwd}}),
        json!({"type":"response_item","timestamp":"2026-01-01T00:00:01Z","payload":{"type":"message","role":"user","id":"01900000-0000-7000-8000-000000000022","content":[{"type":"input_text","text":user_text}]}}),
        json!({"type":"response_item","timestamp":"2026-01-01T00:00:02Z","payload":{"type":"message","role":"assistant","id":"01900000-0000-7000-8000-000000000023","content":[{"type":"output_text","text":"assistant text must remain excluded"}]}}),
    ];
    let bytes = records
        .iter()
        .map(|record| serde_json::to_string(record).expect("serialize project rollout record"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{bytes}\n")).expect("write project rollout source");
}

fn write_json_atomically(path: &Path, value: &Value) {
    let temporary = path.with_extension("watcher-test.tmp");
    let mut bytes = serde_json::to_vec(value).expect("serialize JSON fixture");
    bytes.push(b'\n');
    fs::write(&temporary, bytes).expect("write JSON fixture candidate");
    fs::rename(&temporary, path).expect("publish JSON fixture atomically");
}

fn apply_source_policy(
    binary: &Path,
    root: &Path,
    registry: &Path,
    source_id: &str,
    rollout_root: &Path,
) {
    let preview = run_ok(
        binary,
        &args![
            "automation",
            "source-policy",
            "preview",
            "--registry",
            path_text(registry),
            "--root",
            path_text(root),
            "--source-id",
            source_id,
            "--rollout-root",
            path_text(rollout_root),
            "--role-policy",
            "user-only",
            "--format",
            "json",
        ],
    );
    let preview: Value =
        serde_json::from_slice(&preview.stdout).expect("source-policy preview JSON");
    run_ok(
        binary,
        &args![
            "automation",
            "source-policy",
            "apply",
            "--registry",
            path_text(registry),
            "--root",
            path_text(root),
            "--source-id",
            source_id,
            "--rollout-root",
            path_text(rollout_root),
            "--role-policy",
            "user-only",
            "--expected-revision",
            preview["policy_revision"],
            "--expected-digest",
            string_field(&preview, "policy_digest_sha256"),
            "--format",
            "json",
        ],
    );
}

fn daemon_command(binary: &Path, root: &Path, action: &str) -> Output {
    Command::new(binary)
        .args([
            "automation",
            "daemon",
            action,
            "--root",
            root.to_str().expect("UTF-8 automation root"),
        ])
        .output()
        .expect("run daemon command")
}

fn daemon_status(binary: &Path, root: &Path) -> Value {
    let output = daemon_command(binary, root, "status");
    assert_success(&output, "read daemon status");
    let response: Value = serde_json::from_slice(&output.stdout).expect("daemon response JSON");
    let message = response["message"].as_str().expect("daemon ledger message");
    serde_json::from_str(message).expect("daemon ledger JSON")
}

fn wait_for_reconciliation(binary: &Path, root: &Path, after_us: u64) -> Value {
    wait_for_status(binary, root, |ledger| {
        ledger["last_trigger"] == "watcher_dirty_or_overflow"
            && ledger["worker_active"] == false
            && u64_field(ledger, "updated_at_us") > after_us
    })
}

fn wait_for_report(
    binary: &Path,
    root: &Path,
    after_us: u64,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = None;
    while Instant::now() < deadline {
        let ledger = daemon_status(binary, root);
        if ledger["worker_active"] == false && u64_field(&ledger, "updated_at_us") > after_us {
            let report = last_worker_report(root);
            if predicate(&report) {
                return ledger;
            }
        }
        last = Some(ledger);
        thread::sleep(Duration::from_millis(100));
    }
    panic!("daemon report predicate timed out; last ledger: {last:?}");
}

fn wait_for_status(binary: &Path, root: &Path, predicate: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut last = None;
    while Instant::now() < deadline {
        let output = daemon_command(binary, root, "status");
        if output.status.success()
            && let Ok(response) = serde_json::from_slice::<Value>(&output.stdout)
            && let Some(message) = response["message"].as_str()
            && let Ok(ledger) = serde_json::from_str::<Value>(message)
        {
            if predicate(&ledger) {
                return ledger;
            }
            last = Some(ledger);
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("daemon status predicate timed out; last ledger: {last:?}");
}

fn last_worker_report(root: &Path) -> Value {
    serde_json::from_slice(
        &fs::read(root.join("runtime/worker-evidence/last-worker.stdout.log"))
            .expect("read worker stdout evidence"),
    )
    .expect("worker report JSON")
}

fn u64_field(value: &Value, field: &str) -> u64 {
    value[field]
        .as_u64()
        .unwrap_or_else(|| panic!("{field} must be a u64 in {value}"))
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_ok(binary: &Path, args: &[String]) -> Output {
    let output = Command::new(binary)
        .args(args)
        .output()
        .expect("run WorkVCS command");
    assert_success(&output, &args.join(" "));
    output
}

fn path_text(path: &Path) -> String {
    path.to_str().expect("UTF-8 test path").to_owned()
}

fn key_value(output: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    output
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("missing {key} in {output}"))
        .to_owned()
}

fn string_field(value: &Value, field: &str) -> String {
    value[field]
        .as_str()
        .unwrap_or_else(|| panic!("{field} must be a string in {value}"))
        .to_owned()
}

fn routing_preview(binary: &Path, registry: &Path) -> String {
    let output = run_ok(
        binary,
        &args![
            "project",
            "routing-activation",
            "--preview",
            "--registry",
            path_text(registry),
        ],
    );
    String::from_utf8(output.stdout).expect("routing preview UTF-8")
}

fn activate_routing(binary: &Path, registry: &Path, prior_digest: Option<String>) {
    let preview = routing_preview(binary, registry);
    let mut args = vec![
        "project".to_owned(),
        "routing-activation".to_owned(),
        "--apply".to_owned(),
        "--registry".to_owned(),
        path_text(registry),
        "--expected-registry-digest".to_owned(),
        key_value(&preview, "registry_digest"),
        "--expected-candidate-digest".to_owned(),
        key_value(&preview, "candidate_digest"),
    ];
    if let Some(prior_digest) = prior_digest {
        args.push("--expected-activation-digest".to_owned());
        args.push(prior_digest);
    }
    run_ok(binary, &args);
}

fn record_finding(binary: &Path, store: &Path, branch: &str, statement: &str) {
    let head = run_ok(
        binary,
        &args![
            "branch",
            "head",
            path_text(store),
            "--branch",
            branch.to_owned(),
        ],
    );
    let head = String::from_utf8(head.stdout).expect("branch head UTF-8");
    run_ok(
        binary,
        &args![
            "record",
            "finding",
            path_text(store),
            "--branch",
            branch.to_owned(),
            "--head",
            key_value(&head, "head_commit_id"),
            "--statement",
            statement.to_owned(),
        ],
    );
}

fn register_automation_project(
    binary: &Path,
    project: &Path,
    project_registry: &Path,
    automation_root: &Path,
    registration_id: &str,
    rollout_root: &Path,
) -> String {
    let preview = run_ok(
        binary,
        &args![
            "automation",
            "register-preview",
            "--cwd",
            path_text(project),
            "--registry",
            path_text(project_registry),
            "--root",
            path_text(automation_root),
            "--registration-id",
            registration_id.to_owned(),
            "--rollout-root",
            path_text(rollout_root),
            "--role-policy",
            "user-only".to_owned(),
            "--format",
            "json".to_owned(),
        ],
    );
    let preview: Value =
        serde_json::from_slice(&preview.stdout).expect("registration preview JSON");
    let project_ref_id = string_field(&preview["registration"], "project_ref_id");
    run_ok(
        binary,
        &args![
            "automation",
            "register-apply",
            "--cwd",
            path_text(project),
            "--registry",
            path_text(project_registry),
            "--root",
            path_text(automation_root),
            "--registration-id",
            registration_id.to_owned(),
            "--rollout-root",
            path_text(rollout_root),
            "--role-policy",
            "user-only".to_owned(),
            "--expected-revision",
            preview["registry_revision"].to_string(),
            "--expected-digest",
            string_field(&preview, "registry_digest_sha256"),
            "--format",
            "json".to_owned(),
        ],
    );
    project_ref_id
}

fn reconcile(binary: &Path, root: &Path, registry: &Path) -> Value {
    let output = run_ok(
        binary,
        &args![
            "automation",
            "reconcile",
            "--once",
            "--root",
            path_text(root),
            "--registry",
            path_text(registry),
            "--format",
            "json".to_owned(),
        ],
    );
    serde_json::from_slice(&output.stdout).expect("reconcile report JSON")
}

fn read_catalog(report: &Value) -> Value {
    let path = PathBuf::from(string_field(report, "catalog_path")).join("manifest.json");
    serde_json::from_slice(&fs::read(path).expect("read catalog manifest"))
        .expect("catalog manifest JSON")
}

fn ready_history_source_for<'a>(catalog: &'a Value, project_ref_id: &str) -> &'a Value {
    catalog["sources"]
        .as_array()
        .expect("catalog sources")
        .iter()
        .find(|source| {
            source["project_ref_id"] == project_ref_id
                && source["lane"] == "historical_reference"
                && source["artifact_kind"] == "source_archive"
                && source["status"] == "ready"
        })
        .expect("ready historical source")
}

fn sha256_file(path: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(path).expect("read digest source")))
}
