use std::process::Command;

fn main() {
    let commit = git_value(["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    let dirty = git_value(["status", "--porcelain"])
        .map(|value| if value.is_empty() { "false" } else { "true" })
        .unwrap_or("unknown");

    println!("cargo:rustc-env=WORKVCS_BUILD_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=WORKVCS_BUILD_GIT_DIRTY={dirty}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");
}

fn git_value<const N: usize>(args: [&str; N]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    Some(value.trim().to_owned())
}
