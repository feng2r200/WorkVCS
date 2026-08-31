# Phase 4LE Dogfood and Stale-Gated Takeover Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LE corrects the Phase 4K risk of over-focusing on smoke expectations by
using WorkVCS itself for a real implementation-slice closeout while adding a
real recovery policy: forced Claim takeover requires the previous owning Session
to be explicitly `potentially_stale`.

This evidence claims one local WorkVCS-managed implementation closeout. It does
not claim release readiness, automatic stale detection, repeated recovery
dogfood, or another-project use.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4le-stale-gated-takeover
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The local ignored Store used for the dogfood run was:

```text
.work-governance/runtime/dogfood/phase-4le-full-20260901024049.sqlite
```

Captured IDs:

```text
workspace_id=01a0591f-cb55-7871-b2ee-1f78f202f25c
branch_id=01a0591f-cb55-7871-b2ee-1fa3041503a8
task_entity_id=01a0591f-cb70-7682-8f9d-46fff869e900
task_entity_version_id=01a0591f-cb6f-7b12-8df2-a98b4a57e5ca
session_id=01a0591f-cb87-78d3-a35b-770e9834b89b
claim_id=01a0591f-cb9a-7df1-b394-a72ea5378b16
finding_record_id=01a0591f-cbd3-77a0-aac0-9b91dea61309
finding_commit_id=01a0591f-cbd3-77a0-aac0-9b65bcb47e26
```

The run created a real WorkVCS Task for "Phase 4LE stale-gated Claim takeover
implementation closeout", started a Session, claimed that Task, inspected
Context and `next`, recorded a Finding, created AC/VR, recorded Verification,
transitioned the Task to done, ended the Session, and authored/read a focused
Handoff.

## Dogfood Finding

The first guide-following attempt used:

```text
workvcs context STORE --session SESSION --profile handoff --budget-items 20
```

Current CLI rejected it:

```text
context profile "handoff" is not in the CLI vocabulary
```

The command succeeded when changed to the current vocabulary:

```text
workvcs context STORE --session SESSION --profile normal --budget-items 20
```

Observed result:

```text
normal_context_result=passed
normal_context_profile=normal
normal_context_items=5
```

The full closeout also found two operator ergonomics details:

1. `verify` with inline evidence content requires an explicit
   `--evidence-content-role`;
2. Task closeout after AC/VR/Verification requires reading the current Task
   version before `task transition`, because verification-linked history has
   advanced the Task version.

The operator guide was corrected to use `normal` until a dedicated handoff
profile is implemented, and to show `--evidence-content-role log` in the verify
example.

## Stale-Gated Takeover Validation

Targeted validation after the implementation:

```text
cargo test -q -p workvcs-core --test claim_runtime_phase3f force_takeover_claim_requires_rationale_and_records_prior_claim
cargo test -q -p workvcs-cli cli_runs_claim_transfer_and_force_takeover_workflow
scripts/smoke-v0.1-cli-workflow.sh
```

Result:

```text
targeted4_rc=0
claim_transfer_id=01a05923-e11b-7350-82e5-f1d17d2ac76b
claim_takeover_id=01a05923-ec25-7a92-a176-4c8beacab5b8
stale_session_id=01a05923-f1c3-7da3-9e05-ef0fb43b77d5
```

The focused tests prove that a takeover attempt against an active previous owner
is rejected, the previous owner can then be explicitly marked
`potentially_stale`, and a later forced takeover records
`previous_session_lifecycle_state=potentially_stale`.

The smoke workflow proves the same recovery order through the CLI and updates
Store integrity expectations to `checked_events=41`. WorkStateCommit,
ChangeSet, and schema counts remain unchanged by the runtime recovery events.

## Dogfood Closeout

Full closeout command summary:

```text
dogfood_full_result=passed
ac_entity_id=01a0591f-cbe8-71c1-ac3f-450495faa079
vr_entity_id=01a0591f-cc01-7c11-ae1a-9de5e486d2ac
verification_entity_id=01a0591f-cc1a-75f0-b958-ba2288c4e0c0
task_done_version_id=01a0591f-cc49-7121-91af-b60e6da36c49
session_diff_id=01a0591f-cc5e-7130-8dd2-04c0ad260617
handoff_record_id=01a0591f-cc7a-7583-8e3d-ea27c2509fe8
handoff_commit_id=01a0591f-cc7a-7583-8e3d-e9fa204c8d7e
handoff_show_session_match=true
handoff_show_focus_match=true
```

## Remaining Gap

The dogfood run covered one local implementation closeout. It did not yet
exercise a real blocked handoff where another Session must mark the previous
Session stale and take over its Claim. It also still required manual key-value
ID capture between commands.
