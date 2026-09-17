use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use workvcs_core::{
    Engine, LocalObjectV2MigrationMode, LocalObjectV2MigrationOptions, LocalObjectV2MigrationResult,
};

#[derive(Debug, Parser)]
#[command(
    name = "workvcs-local-object-v2-cutover",
    about = "One-time WorkVCS local object v1 to v2 cutover tool"
)]
struct Args {
    #[arg(value_name = "STORE")]
    store: PathBuf,

    #[arg(long, help = "Apply the cutover after the default read-only preflight")]
    apply: bool,

    #[arg(long)]
    expected_local_objects: Option<usize>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let mut options = if args.apply {
        LocalObjectV2MigrationOptions::apply()
    } else {
        LocalObjectV2MigrationOptions::preflight()
    };
    if let Some(expected) = args.expected_local_objects {
        options = options.with_expected_local_objects(expected);
    }

    match Engine::migrate_local_object_store_v2(&args.store, options) {
        Ok(result) => {
            render_result(&result);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error_code={}", error.code());
            eprintln!("error_category={}", error.category());
            eprintln!("error={error}");
            ExitCode::FAILURE
        }
    }
}

fn render_result(result: &LocalObjectV2MigrationResult) {
    let mode = match result.mode {
        LocalObjectV2MigrationMode::Preflight => "preflight",
        LocalObjectV2MigrationMode::Applied => "applied",
        LocalObjectV2MigrationMode::AlreadyCurrent => "already_current",
    };
    println!("mode={mode}");
    println!("store_id={}", result.store_info.store_id);
    println!(
        "object_store_format_version={}",
        result.store_info.manifest.object_store_format_version
    );
    println!("local_objects={}", result.local_objects);
    println!(
        "preexisting_target_objects={}",
        result.preexisting_target_objects
    );
    println!("legacy_root={}", result.legacy_root.display());
    println!("target_root={}", result.target_root.display());
    println!(
        "migration_id={}",
        result
            .migration_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
}
