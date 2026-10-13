#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn script() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/workvcs-launchagent.sh")
}

fn run(arguments: &[&str]) -> Output {
    Command::new("bash")
        .arg(script())
        .args(arguments)
        .output()
        .expect("run installer")
}

#[test]
fn render_is_zero_write_and_xml_escapes_exact_absolute_targets() {
    let temporary = TempDir::new().unwrap();
    let registry = temporary.path().join("registry & <private>.json");
    let root = temporary.path().join("derived & <private>");
    let agent_dir = temporary.path().join("LaunchAgents");
    let output = run(&[
        "--render",
        "--registry",
        registry.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
        "--agent-dir",
        agent_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let plist = String::from_utf8(output.stdout).unwrap();
    assert!(plist.contains("registry &amp; &lt;private&gt;.json"));
    assert!(plist.contains("derived &amp; &lt;private&gt;"));
    assert!(plist.contains("<key>RunAtLoad</key><true/>"));
    assert!(plist.contains("<key>KeepAlive</key><true/>"));
    assert!(plist.contains("<string>--registry</string>"));
    assert!(plist.contains("<string>--root</string>"));
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 0);
}

#[test]
fn invalid_or_conflicting_actions_fail_before_side_effects() {
    for arguments in [
        vec![
            "--render",
            "--start",
            "--registry",
            "/private/registry.json",
        ],
        vec!["--render", "--registry", "relative.json"],
        vec![
            "--render",
            "--registry",
            "/private/registry.json",
            "--root",
            "/",
        ],
        vec![
            "--render",
            "--registry",
            "/private/registry.json",
            "--periodic-seconds",
            "0",
        ],
        vec!["--render", "--registry", "/private/registry\n.json"],
    ] {
        let output = run(&arguments);
        assert!(!output.status.success(), "{arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("workvcs_launchagent_error="));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn ipc_readiness_requires_the_launchd_pid_and_rejects_invalid_responses() {
    let source = fs::read_to_string(script()).unwrap();
    let body = source
        .split_once("ipc_matches_launchd_pid() {\n")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0;
    let command = format!(
        "set -euo pipefail\nipc_matches_launchd_pid() {{\n{body}\n}}\nipc_matches_launchd_pid \"$1\" \"$2\""
    );
    let matching = r#"{"message":"{\"pid\":36110,\"worker_active\":true}"}"#;
    for (snapshot, response, expected) in [
        ("\tstate = running\n\tpid = 36110\n", matching, true),
        ("\tstate = running\n\tpid = 36111\n", matching, false),
        ("\tstate = running\n", matching, false),
        ("\tpid = 0\n", r#"{"message":"{\"pid\":0}"}"#, false),
        ("\tpid = 36110\n", "not-json", false),
        ("\tpid = 36110\n", r#"{"message":"not-json"}"#, false),
        ("\tpid = 36110\n", r#"{"message":"{}"}"#, false),
    ] {
        let result = Command::new("bash")
            .args(["-c", &command, "--", snapshot, response])
            .output()
            .unwrap();
        assert_eq!(result.status.success(), expected, "{snapshot:?} {response}");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn install_is_idempotent_and_preserves_conflicting_or_symlinked_plists() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

    let temporary = TempDir::new().unwrap();
    let binary = temporary.path().join("workvcs");
    let registry = temporary.path().join("registry.json");
    let root = temporary.path().join("derived");
    let agent_dir = temporary.path().join("LaunchAgents");
    // The fixture represents only the installed runtime gate. No launchctl
    // action is invoked: --install must leave loading to a separate operation.
    fs::write(&binary, "#!/bin/sh\nprintf 'action=runtime_status\\nexecutable_role=stable_installed\\nbuild_dirty=false\\n'\n").unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&registry, "{}").unwrap();
    let arguments = [
        "--install",
        "--binary",
        binary.to_str().unwrap(),
        "--registry",
        registry.to_str().unwrap(),
        "--root",
        root.to_str().unwrap(),
        "--agent-dir",
        agent_dir.to_str().unwrap(),
    ];
    let first = run(&arguments);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(String::from_utf8_lossy(&first.stdout).contains("launch_agent_start_requested=false"));
    let plist = agent_dir.join("io.workvcs.automation.plist");
    let before = fs::metadata(&plist).unwrap();
    let installed = fs::read(&plist).unwrap();
    assert_eq!(before.permissions().mode() & 0o777, 0o600);
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let second = run(&arguments);
    assert!(second.status.success());
    assert_eq!(fs::metadata(&plist).unwrap().ino(), before.ino());
    assert_eq!(fs::read(&plist).unwrap(), installed);

    let mut status_arguments = arguments;
    status_arguments[0] = "--status";
    let status = run(&status_arguments);
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains("launch_agent_installed=true"));
    assert!(
        status.contains("launch_agent_disabled=false")
            || status.contains("launch_agent_disabled=true")
    );
    assert!(status.contains("autostart_configured=false"));
    assert!(status.contains("autostart_enabled=false"));
    fs::set_permissions(&plist, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(!run(&arguments).status.success());
    assert_eq!(fs::read(&plist).unwrap(), installed);
    fs::set_permissions(&plist, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&agent_dir, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(!run(&status_arguments).status.success());
    fs::set_permissions(&agent_dir, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(!run(&status_arguments).status.success());
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();

    fs::write(&plist, b"unrelated user configuration").unwrap();
    assert!(!run(&arguments).status.success());
    assert_eq!(fs::read(&plist).unwrap(), b"unrelated user configuration");
    fs::remove_file(&plist).unwrap();
    let unrelated = temporary.path().join("unrelated.plist");
    fs::write(&unrelated, b"unrelated user configuration").unwrap();
    symlink(&unrelated, &plist).unwrap();
    assert!(!run(&arguments).status.success());
    assert!(
        fs::symlink_metadata(&plist)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read(&unrelated).unwrap(),
        b"unrelated user configuration"
    );
}
