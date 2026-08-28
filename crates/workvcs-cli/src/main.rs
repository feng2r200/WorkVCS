use clap::{ArgGroup, Parser, Subcommand};
use std::fmt::Write as _;
use std::path::PathBuf;
use workvcs_core::{
    BranchId, CommitId, Engine, HistoryEntry, HistoryQueryOptions, ReplayedState, Result,
    StoreInitOptions, WorkState, WorkVcsError,
};

#[derive(Debug, Parser)]
#[command(name = "workvcs")]
#[command(about = "WorkVCS v0.1 thin command shell")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long, default_value = "WorkVCS store")]
        display_name: String,
    },
    Doctor {
        #[arg(value_name = "STORE")]
        store: PathBuf,
    },
    #[command(group(
        ArgGroup::new("history-start")
            .required(true)
            .multiple(false)
            .args(["branch", "commit"])
    ))]
    History {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        branch: Option<String>,

        #[arg(long)]
        commit: Option<String>,

        #[arg(long)]
        limit: Option<usize>,
    },
    ShowAt {
        #[arg(value_name = "STORE")]
        store: PathBuf,

        #[arg(long)]
        commit: String,
    },
}

fn main() {
    match run(Cli::parse()) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<String> {
    match cli.command {
        Command::Init {
            store,
            display_name,
        } => {
            let engine = Engine::init(store, StoreInitOptions::new(display_name)?)?;
            let info = engine.store_info()?;
            Ok(format!(
                "initialized store_id={} schema_version={}\n",
                info.store_id, info.manifest.schema_version
            ))
        }
        Command::Doctor { store } => {
            let engine = Engine::open(store)?;
            let info = engine.store_info()?;
            Ok(format!(
                "ok store_id={} schema_version={} canonical_json_profile={}\n",
                info.store_id, info.manifest.schema_version, info.manifest.canonical_json_profile
            ))
        }
        Command::History {
            store,
            branch,
            commit,
            limit,
        } => {
            let engine = Engine::open(store)?;
            let mut options = match (branch, commit) {
                (Some(branch_id), None) => {
                    HistoryQueryOptions::from_branch(BranchId::parse_canonical(&branch_id)?)
                }
                (None, Some(commit_id)) => {
                    HistoryQueryOptions::from_commit(CommitId::parse_canonical(&commit_id)?)
                }
                _ => {
                    return Err(WorkVcsError::QueryInvalid(
                        "history requires exactly one of --branch or --commit".to_owned(),
                    ));
                }
            };
            if let Some(limit) = limit {
                options = options.with_limit(limit)?;
            }
            let history = engine.history(options)?;
            let mut output = format!(
                "start_commit_id={}\nentries={}\n",
                history.start_commit_id,
                history.entries.len()
            );
            for entry in &history.entries {
                render_history_entry(&mut output, entry);
            }
            Ok(output)
        }
        Command::ShowAt { store, commit } => {
            let engine = Engine::open(store)?;
            let state = engine.show_at(CommitId::parse_canonical(&commit)?)?;
            Ok(render_replayed_state(&state))
        }
    }
}

fn render_history_entry(output: &mut String, entry: &HistoryEntry) {
    let parent = entry
        .parent_commit_id
        .map(|commit_id| commit_id.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let _ = writeln!(
        output,
        "commit={} kind={} operation={} schema={} parent={} state_digest={}",
        entry.commit_id,
        entry.commit_kind,
        entry.operation_type,
        entry.operation_schema_version,
        parent,
        entry.state_digest
    );
}

fn render_replayed_state(state: &ReplayedState) -> String {
    let mut output = format!(
        "workspace_id={}\ncommit_id={}\nstate_digest={}\n",
        state.workspace_id, state.commit_id, state.state_digest
    );
    render_work_state(&mut output, &state.state);
    output
}

fn render_work_state(output: &mut String, state: &WorkState) {
    let _ = writeln!(output, "entities={}", state.entities().len());
    for (entity_id, entity_version_id) in state.entities() {
        let _ = writeln!(output, "entity={} version={}", entity_id, entity_version_id);
    }
    let _ = writeln!(output, "relations={}", state.relations().len());
    for (relation_id, relation_version_id) in state.relations() {
        let _ = writeln!(
            output,
            "relation={} version={}",
            relation_id, relation_version_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_exposes_phase2_thin_command_shells() {
        let command = Cli::command();
        let names = command
            .get_subcommands()
            .map(|command| command.get_name())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["init", "doctor", "history", "show-at"]);
    }

    #[test]
    fn history_requires_one_start_selector() {
        let missing = Cli::try_parse_from(["workvcs", "history", "store.sqlite"]);
        assert!(missing.is_err());

        let branch = BranchId::new_v7().to_string();
        let commit = CommitId::new_v7().to_string();
        let both = Cli::try_parse_from([
            "workvcs",
            "history",
            "store.sqlite",
            "--branch",
            &branch,
            "--commit",
            &commit,
        ]);
        assert!(both.is_err());
    }

    #[test]
    fn init_and_doctor_use_engine_store_boundary() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("workvcs.sqlite");

        let init = run(Cli::try_parse_from([
            "workvcs",
            "init",
            path.to_str().expect("path text"),
            "--display-name",
            "cli-store",
        ])
        .expect("parse init"))
        .expect("run init");
        assert!(init.starts_with("initialized store_id="));
        assert!(init.contains("schema_version=1"));

        let doctor =
            run(
                Cli::try_parse_from(["workvcs", "doctor", path.to_str().expect("path text")])
                    .expect("parse doctor"),
            )
            .expect("run doctor");
        assert!(doctor.starts_with("ok store_id="));
        assert!(doctor.contains("canonical_json_profile=workvcs-jcs-v1"));
    }
}
