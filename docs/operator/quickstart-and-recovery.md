# Local Operator Quickstart and Recovery

Status: Phase 4LC local V0.1 operator guide
Last updated: 2026-09-01

This guide is for a local operator or Agent using the current WorkVCS CLI from
this repository. It describes runnable local commands, not a packaged release.

## Current Boundary

WorkVCS currently runs as a local Rust CLI over a local SQLite Store. The Store
is the durable WorkVCS state; Git branches, Codex worktrees, remote pushes,
deployment, and plugin activation remain separate workflows.

Current V0.1 does not include a daemon, GUI, TUI, cloud sync, remote
collaboration, automatic transcript parsing, LLM extraction, or Agent
orchestration. A Session can be explicitly marked `potentially_stale`; Claim
takeover is stale-gated, explicit, and still requires `--force` plus a
rationale. Automatic stale detection remains Open.

## Build Or Install

From the repository root:

```bash
cargo test --workspace --quiet
cargo install --path crates/workvcs-cli --locked
workvcs --help
```

For one-off local use without installing:

```bash
cargo run -q -p workvcs-cli -- --help
```

The smoke script uses the one-off form internally, so it can validate the
workspace even when no `workvcs` binary has been installed.

## Baseline Validation

Run the schema validator and repository smoke before trusting a new local build:

```bash
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
```

Run integrity checks against any Store before and after risky local operations:

```bash
workvcs doctor "$STORE" --require-valid
workvcs store integrity "$STORE" --require-valid
```

## Minimal Work Loop

Create a Store and Workspace:

```bash
STORE=.workvcs/local.sqlite
workvcs init "$STORE" --display-name local-work
workvcs workspace create "$STORE" --display-name local-workspace
```

The CLI prints `key=value` lines. Capture the emitted `workspace_id`,
`branch_id`, and `genesis_commit_id` for subsequent commands.

Create work, start a Session, and claim it:

```bash
workvcs task create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --description "Implement the next bounded slice"
```

Capture the emitted `task_entity_id`, `task_entity_version_id`, and `commit_id`.
Use that `commit_id` as the next `HEAD_COMMIT_ID`.

```bash

workvcs session start "$STORE" \
  --workspace "$WORKSPACE_ID" \
  --branch "$BRANCH_ID"

workvcs claim task "$STORE" \
  --session "$SESSION_ID" \
  --task "$TASK_ENTITY_ID"
```

Inspect continuation context:

```bash
workvcs context "$STORE" --session "$SESSION_ID"
workvcs context "$STORE" --session "$SESSION_ID" --profile normal --budget-items 20
workvcs next "$STORE" --session "$SESSION_ID"
```

Record acceptance and verification when a slice has an explicit check:

```bash
workvcs ac create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --task "$TASK_ENTITY_ID" \
  --task-version "$TASK_ENTITY_VERSION_ID" \
  --local-key ac-validation \
  --statement "The bounded slice passes its validation command"
```

Capture the emitted `acceptance_criterion_entity_id`,
`acceptance_criterion_entity_version_id`, and `commit_id`. Use that `commit_id`
as the next `HEAD_COMMIT_ID`.

```bash
workvcs vr create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --criterion "$AC_ENTITY_ID" \
  --criterion-version "$AC_ENTITY_VERSION_ID" \
  --local-key vr-validation \
  --statement "Run the validation command and record its result"
```

Capture the emitted `verification_requirement_entity_id` and `commit_id`. Use
that `commit_id` as the next `HEAD_COMMIT_ID`.

```bash
workvcs verify "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --verification-requirement "$VR_ENTITY_ID" \
  --result passed \
  --method cli \
  --evidence-kind command_output \
  --evidence-content-role log \
  --evidence-content "validation command passed"
```

Capture the emitted `verification_entity_id` and `commit_id`. Use that
`commit_id` as the next `HEAD_COMMIT_ID`. Before marking the Task done, inspect
the current Task version at the new head:

```bash
workvcs task show "$STORE" \
  --branch "$BRANCH_ID" \
  --task "$TASK_ENTITY_ID"
```

Capture the current `task_entity_version_id`, then close the Task:

```bash
workvcs task transition "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --task "$TASK_ENTITY_ID" \
  --task-version "$CURRENT_TASK_ENTITY_VERSION_ID" \
  --status done \
  --outcome completed \
  --session "$SESSION_ID"
```

End a Session with a summary and create a focused handoff:

```bash
workvcs session end "$STORE" \
  --session "$SESSION_ID" \
  --summary-json '{"outcome":"completed bounded slice"}'
```

Capture the emitted `session_diff_id`.

```bash
workvcs handoff create "$STORE" \
  --branch "$BRANCH_ID" \
  --head "$HEAD_COMMIT_ID" \
  --session "$SESSION_ID" \
  --session-diff "$SESSION_DIFF_ID" \
  --focus "$TASK_ENTITY_ID" \
  --statement "Continue from this task and inspect context before claiming work"
```

Capture the emitted `record_entity_id` as `HANDOFF_RECORD_ID` and the emitted
`commit_id` as `HANDOFF_COMMIT_ID`.

Show the handoff before using it:

```bash
workvcs handoff show "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --handoff "$HANDOFF_RECORD_ID"
```

Review the Handoff focus link from either endpoint:

```bash
workvcs why "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --entity "$HANDOFF_RECORD_ID"

workvcs why "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --entity "$TASK_ENTITY_ID"
```

For a focused Handoff, the Handoff Record reports
`scope_link.0.direction=outgoing` and the focused Task reports
`scope_link.0.direction=incoming`. Stored `relation_edges` remain separate from
these read-only scope links; existing relation filters and limits apply to
`relation_edges`, not to scope links.

Start a continuation Session, then consume the Handoff into that Session's
focus:

```bash
workvcs session start "$STORE" \
  --workspace "$WORKSPACE_ID" \
  --branch "$BRANCH_ID"
```

Capture the emitted `session_id` as `CONTINUATION_SESSION_ID`.

```bash
workvcs handoff consume "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --handoff "$HANDOFF_RECORD_ID" \
  --session "$CONTINUATION_SESSION_ID"
```

Inspect Context and `next` after consuming the Handoff. The Session focus should
match the Handoff focus before work is claimed.

## Common Recovery Actions

When a Store fails integrity or doctor checks, stop using it as an authority
until the failure is understood:

```bash
workvcs doctor "$STORE" --require-valid
workvcs store integrity "$STORE" --require-valid
```

When verification applicability is stale after a branch head advances, refresh
the cache for the new head instead of manually editing observations:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID"
```

When an active Claim blocks another active Session and the claimant can hand
work over, transfer the Claim:

```bash
workvcs claim transfer "$STORE" \
  --from-session "$SOURCE_SESSION_ID" \
  --to-session "$TARGET_SESSION_ID" \
  --claim "$CLAIM_ID"

workvcs claim guard "$STORE" \
  --session "$TARGET_SESSION_ID" \
  --task "$TASK_ENTITY_ID"
```

If a focused Handoff continuation is blocked by another active Session's Claim,
first consume the Handoff, inspect the guard, then recover with the same
stale-gated takeover chain:

```bash
workvcs handoff consume "$STORE" \
  --commit "$HANDOFF_COMMIT_ID" \
  --handoff "$HANDOFF_RECORD_ID" \
  --session "$CONTINUATION_SESSION_ID"

workvcs claim guard "$STORE" \
  --session "$CONTINUATION_SESSION_ID" \
  --task "$TASK_ENTITY_ID"

workvcs session mark-stale "$STORE" \
  --session "$PREVIOUS_OWNER_SESSION_ID" \
  --rationale "previous owner cannot continue"

workvcs claim takeover "$STORE" \
  --session "$CONTINUATION_SESSION_ID" \
  --claim "$CLAIM_ID" \
  --force \
  --rationale "handoff recovery"
```

When an operator must override a blocked active Claim, first mark the previous
owning Session as `potentially_stale`, then use forced takeover with a rationale
and inspect the guard afterward:

```bash
workvcs session mark-stale "$STORE" \
  --session "$PREVIOUS_SESSION_ID" \
  --rationale "operator recovery: previous session cannot continue"

workvcs claim takeover "$STORE" \
  --session "$TAKING_SESSION_ID" \
  --claim "$CLAIM_ID" \
  --force \
  --rationale "operator recovery: previous session cannot continue"

workvcs claim guard "$STORE" \
  --session "$TAKING_SESSION_ID" \
  --task "$TASK_ENTITY_ID"
```

When a Session is ended, start a new Session rather than attempting to mutate
the ended runtime row:

```bash
workvcs session start "$STORE" \
  --workspace "$WORKSPACE_ID" \
  --branch "$BRANCH_ID"
```

When the repository smoke fails, keep the temporary Store directory reported by
the script, rerun the failing command with `workvcs ... --help` open for that
subcommand, and only update expectations after the command output proves the
intended state transition.

## Still Open For V1

- The documented loop has been dogfooded for one implementation closeout, but
  has not yet been repeated on another real project.
- Resource path/glob normalization and adapter-backed re-observation remain
  open.
- Context packets still need more V1 categories and persistence decisions.
- `why` does not yet expose the Handoff focus link as a relation.
- Automatic stale detection remains open.
- Larger Store validation has not yet been run.
