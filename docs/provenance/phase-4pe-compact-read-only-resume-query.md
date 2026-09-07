# Phase 4PE Compact Read-only Resume Query Evidence

Status: current local evidence
Date: 2026-09-07

## Scope

Phase 4PE is the first local improvement after the Phase 4PD local V1
release-ready judgment. It follows the user's current requirement to prioritize
"less interruption, fewer tokens, less drift, and easier query" over broader
Context/Resource/why traversal.

The slice adds a compact read-only recovery query:

```text
workvcs resume STORE --session SESSION [--budget-items N] [scope flags] [--expected-state-digest HEX]
```

This advances operator discoverability and continuation recovery. It does not
change the V1 release gate decision, and it is not a release, tag, push,
deploy, remote, production, credential, or `/usr/local/bin/workvcs`
installation action.

## Requirement Basis

The user restated WorkVCS's core value as recording project goals and execution
steps to prevent long-task goal drift. WorkVCS must also support fast,
accurate, low-cost process understanding and quick recovery of:

```text
goal
current step
evidence
blockers
next step
```

The user's hard floor is that WorkVCS must not add extra Plan workload or
noticeable restriction, friction, interruption, or interference with normal
Codex execution.

The Phase 4PE review concluded that current main already satisfies bounded
local V1 release maturity and that post-V1 broader traversal should be
downgraded unless a concrete dogfood workflow proves a real recovery gap. The
highest-value next slice was therefore a compact read-only resume view over
existing context data.

## Implementation

Changed:

```text
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0496-phase-4pe-compact-read-only-resume-query.md
docs/provenance/phase-4pe-compact-read-only-resume-query.md
docs/provenance/v1-readiness-ledger.md
```

The CLI now exposes top-level `resume` help and parsing. Runtime behavior:

- opens the Store read-only through existing query paths;
- parses the Session id;
- accepts the same scope shorthands as `context`: `--scope-json`,
  `--scope-path`, and `--scope-path-prefix`;
- forces `ContextProfile::Brief`;
- calls existing `Engine::context_packet`;
- applies `--budget-items` only to compact output selection; and
- renders `resume_*` key-value fields plus selected `resume_item.<i>.*` rows.

Default output budget is 12 items. `--budget-items 0` fails before query output
with:

```text
resume budget items must be greater than zero
```

The compact output includes:

```text
session_id
lifecycle_state
workspace_id
branch_id
branch_name
head_commit_id
state_digest
focus_entity_id
resume_profile
resume_budget_items
resume_scope_json
resume_available_items
resume_items
resume_omitted_items
resume_omission_categories
resume_goal_plan_paths
resume_current_tasks
resume_readiness_items
resume_blockers
resume_acceptance_criteria
resume_verification_requirements
resume_failed_attempts
resume_resource_basis_items
resume_has_resource_basis
resume_next_action
resume_item.<i>.key
resume_item.<i>.priority
resume_item.<i>.category
resume_item.<i>.subject
resume_item.<i>.summary_json
resume_omission_category.<i>.category
resume_omission_category.<i>.omitted
```

`resume_next_action` is intentionally simple:

```text
resolve_blocked_dependency
continue_current_task
inspect_task_readiness
inspect_context
```

Blocked dependencies take precedence because they are the strongest immediate
anti-drift signal: continuing the current task is wrong if the current task is
blocked by unresolved prerequisite work.

## Validation

Focused validation:

```text
cargo fmt
cargo test -p workvcs-cli cli_resume -- --nocapture
cargo test -p workvcs-cli cli_exposes_thin_command_shells -- --nocapture
cargo test -p workvcs-cli cli_top_level_commands_have_help_summaries -- --nocapture
cargo test -p workvcs-cli cli_context_packet_renders_resource_basis_recovery_hint_summary -- --nocapture
cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_blocked_dependency -- --nocapture
```

Package/workspace validation:

```text
RUST_MIN_STACK=33554432 cargo test -p workvcs-cli
RUST_MIN_STACK=33554432 cargo test
git diff --check
```

The first unstacked `cargo test -p workvcs-cli` attempt aborted on an existing
large-test stack overflow. The same CLI package and full workspace tests pass
with `RUST_MIN_STACK=33554432`; the targeted `resume` tests pass without that
failure.

Log summary:

```text
log_dir=/tmp/workvcs-resume-slice-20260907T092600Z
```

## Read-only Proof

`cli_resume_shows_compact_read_only_recovery_context` creates a temporary
Store with:

- one Goal;
- one Plan;
- one focused dependent Task;
- one prerequisite Task;
- Goal -> Plan and Plan -> Task containment;
- an AC and VR on the prerequisite Task;
- a Resource-backed Verification with persisted baseline observation;
- a `depends_on` relation from dependent Task to prerequisite Task; and
- an active Session focused on the dependent Task.

The test then runs `workvcs resume` and proves:

- `resume_profile=brief`;
- `resume_next_action=resolve_blocked_dependency`;
- Goal and Plan descriptions are present;
- blocker text is present;
- `resource_basis=1` and `refresh_hint=` are present;
- output uses `resume_item.*`, not generic `context_item.*`; and
- output item count respects `--budget-items 12`.

It also proves `resume` has no visible write side effects:

- Branch head commit unchanged;
- Branch state digest unchanged;
- Branch history entry count unchanged;
- workspace event count unchanged;
- context-packet snapshot count unchanged; and
- active Session claim count unchanged.

## Interpretation

Phase 4PE directly serves the current product need: it gives Codex and human
operators a cheap recovery command for "what was the goal, where am I, what is
blocking, what evidence/recovery hint matters, and what should happen next?"
without asking them to run a new Plan or read a broad traversal dump.

The proof is bounded. It does not prove a new release candidate matrix,
broader traversal maturity, performance maturity, automatic recording,
background refresh, or system-level installation.

## Next Boundary

The user stated that after production readiness is confirmed, a later task
will add `workvcs` to `/usr/local/bin` as a system executable and overwrite it
on each update to guarantee availability. Phase 4PE records that as future
operational packaging work only. It is not executed here.
