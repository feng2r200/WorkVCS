# ADR-0496: Phase 4PE Compact Read-only Resume Query

Status: Accepted
Date: 2026-09-07

## Context

The Phase 4PD ledger and release gate matrix record that the bounded local V1
release-maturity gate is satisfied, while release, tag, push, deploy, remote,
production, and credential operations remain separately authorized actions.
Phase 4PA also records that deeper nested Plan traversal, multi-hop structural
references, broader Context/Resource/why traversal, full relation-subject
traversal, multi-hop/full evolution traversal, and broader causal traversal
are deferred post-V1 unless later scope is explicitly reopened.

The next current user requirement re-centered WorkVCS on its core value:
recording project goals and execution steps so long Codex tasks can avoid
goal drift, and recovering "goal, current step, evidence, blockers, next step"
quickly with low token cost. The user also set a hard floor: WorkVCS must not
add extra Plan workload or noticeable execution friction.

Current `context --profile brief` already exposes the necessary underlying
items, including Session anchor, Branch overview, current Tasks, Goal/Plan
paths, acceptance and verification obligations, blocked dependencies, failed
attempts, and Resource-basis recovery hints. The gap is operator and Agent
friction: callers must still read the generic context packet shape and infer
which subset matters for fast continuation.

## Decision

Add a top-level read-only CLI view:

```text
workvcs resume STORE --session SESSION [--budget-items N] [scope flags] [--expected-state-digest HEX]
```

`resume` reuses the existing `Engine::context_packet` query with
`ContextProfile::Brief`. It does not call `next_work`, does not claim work,
does not save a `context-packet` snapshot, and does not write Branch, history,
event, Claim, or runtime state.

The command renders a compact key-value recovery summary:

- session, Branch, head commit, state digest, lifecycle, and focus;
- `resume_profile=brief`;
- output budget and scope;
- counts for Goal/Plan paths, current Tasks, readiness, blockers, acceptance
  criteria, verification requirements, failed attempts, and Resource basis
  items;
- `resume_next_action`, with blocked dependencies taking precedence over
  continuing current work;
- selected `resume_item.<i>.*` lines ordered for continuation recovery; and
- omitted category counts when the output budget hides lower-ranked items.

The default output budget is 12 items. The budget is applied only to the
compact resume rendering step, not to the underlying context packet query, so
critical counts and next-action selection can still inspect the full brief
packet. `--budget-items 0` is rejected with a `QueryInvalid` error.

## Non-Goals

- No Store schema, migration, ContextPacket schema, or snapshot schema change.
- No core runtime behavior change beyond using the existing read-only query.
- No new Plan/Goal/Task semantics, traversal semantics, dependency readiness,
  Claim, `next`, `claim next`, Resource, Verification, Handoff, Bundle,
  Checkpoint, Merge, or `why` behavior.
- No broader traversal, deeper nested traversal, multi-hop structural
  reference traversal, full relation-subject traversal, multi-hop/full
  evolution traversal, or broader causal traversal.
- No automatic transcript parsing, LLM semantic extraction, orchestration,
  daemon, watcher, background refresh, GUI/TUI, cloud sync, federation, or
  destructive compaction.
- No release, tag, push, deploy, remote, production, credential, or
  `/usr/local/bin/workvcs` installation action.

## Evidence

Implementation changed only:

```text
crates/workvcs-cli/src/main.rs
```

Focused validation passed:

```text
cargo fmt
cargo test -p workvcs-cli cli_resume -- --nocapture
cargo test -p workvcs-cli cli_exposes_thin_command_shells -- --nocapture
cargo test -p workvcs-cli cli_top_level_commands_have_help_summaries -- --nocapture
cargo test -p workvcs-cli cli_context_packet_renders_resource_basis_recovery_hint_summary -- --nocapture
cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_blocked_dependency -- --nocapture
```

Full local validation passed with the larger test stack required by an
existing large CLI test:

```text
RUST_MIN_STACK=33554432 cargo test -p workvcs-cli
RUST_MIN_STACK=33554432 cargo test
git diff --check
```

`cli_resume_shows_compact_read_only_recovery_context` proves the public CLI
view on a temporary Store containing a Goal, Plan, blocked dependent Task,
prerequisite Task, AC, VR, Resource-backed Verification, Session focus, and
dependency relation. The test asserts that `resume` surfaces Goal/Plan
descriptions, blocker state, Resource basis recovery text, and a blocker-first
next action, and that Branch head, state digest, history count, workspace
event count, context-packet snapshot count, and Session claim count are
unchanged after the command.

Detailed evidence is recorded in
`docs/provenance/phase-4pe-compact-read-only-resume-query.md` and
`/tmp/workvcs-resume-slice-20260907T092600Z/validation-summary.md`.

## Consequences

Continuation Agents now have a lower-token entrypoint for resuming a WorkVCS
Session without needing to inspect the full generic context packet shape. The
command directly serves the user's stated pain: quickly recover the goal,
current work, evidence/recovery hints, blockers, and next action while keeping
WorkVCS out of the normal execution path unless the operator asks for it.

Post-V1 broader traversal remains downgraded. Future work should only reopen
broader Context/Resource/why traversal when a concrete dogfood continuation
shows that the compact resume view or existing brief/normal/full context
cannot answer a real recovery question at acceptable token cost.

The user's future `/usr/local/bin/workvcs` installation and overwrite policy is
an operational packaging step. It remains outside this ADR and requires a
separate authorized run.
