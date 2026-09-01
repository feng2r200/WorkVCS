# Phase 4MR Shared Claim Write-Mode Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MR dogfoods shared-Claim collaboration in a real WorkVCS repository
delivery slice that includes a repository documentation write.

The slice uses two active Sessions on the same WorkVCS Task:

- a reader Session that joins the Task with a shared Claim and inspects context;
- a writer Session that joins the same shared Claim set through `claim next`,
  checks the write gate, and performs the repository documentation write only
  after the reader releases its shared Claim.

This is dogfood and documentation evidence only. It does not change Rust code,
schema, CLI behavior, release operations, Push state, tags, deployment, or V2
scope. It also does not claim that WorkVCS V1 is release-ready or that V0.1
dogfood is complete.

## Dogfood Setup

- Source repository baseline: main commit
  `c1130e7fd5fdf95cc66931daa02394d74b0f1746`.
- Pre-write dogfood log directory:
  `/tmp/workvcs-4mr-shared-claim-write-mode-20260901T115851Z`.
- Dogfood Store:
  `/tmp/workvcs-4mr-shared-claim-write-mode-20260901T115851Z/store.workvcs`.
- Workspace id: `01a05cd6-2411-77e2-912d-90f8bfec4cde`.
- Branch id: `01a05cd6-2411-77e2-912d-912dc9181382`.
- Pre-write head commit id:
  `01a05cd6-24db-7fb0-98da-732cd9e0303d`.
- Task id: `01a05cd6-2461-7971-aa38-90b26c7723bb`.
- Task version id: `01a05cd6-24b8-7ae3-92e0-fe21fcebcc9a`.
- Acceptance Criterion id:
  `01a05cd6-24b8-7ae3-92e0-fe0ae45d2895`.
- Verification Requirement id:
  `01a05cd6-24db-7fb0-98da-72eb78ca1c61`.
- Reader Session id: `01a05cd6-24f7-7091-8cfe-158f259443f3`.
- Writer Session id: `01a05cd6-250d-7863-b02d-9ceab3cc3521`.
- Reader shared Claim id: `01a05cd6-254d-7070-a976-d5d387ec650f`.
- Writer shared Claim id: `01a05cd6-2570-7840-abcc-40847ed81484`.

## Command Coverage

The pre-write dogfood run first built the current CLI:

```text
cargo build -q -p workvcs-cli
```

It then exercised these process-boundary commands:

```text
init
workspace create
goal create
plan create
task create
task contain
ac create
vr create
session start
session focus-set
claim task --mode shared
claim next --mode shared
runnable tasks
context --profile brief
claim guard --action structural-task
task transition --session
branch head
claim release
claim guard
task show
```

## Pre-Write Coordination Evidence

Before the repository documentation write, the target provenance file did not
exist:

```text
target_doc_preexisting=no
```

The reader and writer Sessions both held active shared Claims on the same Task.
The writer saw the shared Claim through `runnable tasks` and `context` output.

While both shared Claims were active, `claim guard --action structural-task`
reported:

```text
shared_guard_allowed=false
shared_guard_reason=non_unique_shared_claim_set
active_claims=2
```

A real protected terminal Task transition from the writer Session was rejected:

```text
blocked-transition_status=1
blocked-transition_contains_non_unique_shared_claim_set=yes
head_unchanged_after_blocked_transition=yes
task_status_pre_write=pending
```

After the reader released its shared Claim, the writer Session became the
unique shared claimant:

```text
after_release_structural_guard_allowed=true
after_release_structural_guard_reason=unique_shared_claimant
after_release_terminal_guard_allowed=true
after_release_terminal_guard_reason=unique_shared_claimant
```

This is the point at which the repository documentation write became permitted
by the WorkVCS shared-Claim guard model for this slice.

## Repository Write

The writer Session's permitted write creates this Phase 4MR provenance file,
adds ADR-0459, updates the documentation index, and refreshes the V1 readiness
ledger and release gate matrix.

After adding the new files with intent-to-add, the post-write repository diff
listed the full documentation write set:

```text
M docs/README.md
A docs/decisions/adr/0459-phase-4mr-shared-claim-write-mode-dogfood.md
A docs/provenance/phase-4mr-shared-claim-write-mode-dogfood.md
M docs/provenance/v1-readiness-ledger.md
M docs/provenance/v1-release-gate-matrix.md
```

## Dogfood Findings

- A fresh isolated Git worktree did not initially have `target/debug/workvcs`;
  the dogfood harness now builds the CLI before probing command help or running
  the Store scenario.
- `session focus-set` for the target Task reports zero focus-path entries in
  this scenario. The first dogfood attempt incorrectly expected one entry and
  was corrected before the successful run.
- The first post-write `verify` attempt omitted `--evidence-content-role` and
  failed with `task_invalid`; the corrected retry added
  `--evidence-content-role log` and completed the Store closeout.
- The shared-Claim write gate is explicit: shared mode allows read/write
  coordination, but protected mutation is still blocked until the shared Claim
  set has exactly one active claimant.

These are dogfood harness and operator-boundary findings. They did not require
a WorkVCS product, schema, or CLI behavior change in this slice.

## Readiness Impact

Phase 4MI proved shared-Claim read-only collaboration against a real external
project. Phase 4MR extends that evidence into a real repository write-mode
delivery slice: a reader and writer share the same Task, the writer is blocked
while two shared Claims are active, and the writer is allowed only after the
reader releases its Claim.

This moves the release gate for Session, Claim, Runnable, `claim next`, and
`next` from missing write/read-write shared-Claim dogfood to bounded pass
evidence for the V1-local scope. It does not prove automatic ownership
arbitration, distributed collaboration, or remote multi-operator coordination.

## Post-Write Store Closeout

PASS:
`/tmp/workvcs-4mr-shared-claim-write-mode-20260901T115851Z`.

The writer Session recorded post-write verification evidence against the
Verification Requirement created in the dogfood Store:

```text
post-write-verify_status=0
post_write_verification_id=01a05cda-5da0-7bc0-98a5-67e0e71248b5
post_write_evidence_id=01a05cda-5d9c-70b2-8cdd-df70e186a727
ac_status_after_verify=verified
```

The same writer then closed the WorkVCS Task after the repository write:

```text
writer-task-done_status=0
writer_task_status=done
final_head_commit_id=01a05cda-5de5-7be1-af46-33d9ad434a09
final_task_version_id=01a05cda-5de5-7be1-af46-33fafe8de88a
```

The remaining Claim and both Sessions were closed:

```text
writer_claim_lifecycle=released
reader_session_lifecycle=ended
writer_session_lifecycle=ended
reader_session_diff_id=01a05cda-5e0e-75e3-b218-f3977e2c28a6
writer_session_diff_id=01a05cda-5e20-7642-8227-bcb2970476a8
```

Final Store inspection passed:

```text
final_history_entries=10
final_state_digest=5f5ea40f33ca192eb959b18256dc536c77fd7556a99cd85559cfd215a83bb180
integrity_valid_required=true
dogfood_complete=pass
```

## Final Validation

PASS:
`/tmp/workvcs-4mr-final-validation-20260901T120631Z`.

Final validation passed `git diff --check`, `cargo fmt --all -- --check`,
file and README link checks, ledger and release-gate matrix boundary checks,
and dogfood summary/raw evidence checks for `non_unique_shared_claim_set`,
`unique_shared_claimant`, post-write Store closeout, and the new documentation
write set.

Independent review found no blocker or high-severity issues. Its medium finding
was that this section still contained the earlier T-004 closeout placeholder;
this update resolves that documentation-sync finding. Its low finding was that
the active Plan file and index still needed explicit delivery handling; the
Plan file is part of the local delivery, while the active index is expected to
be removed by Plan closeout before commit.

Residual risk: the independent review did not rerun the full dogfood flow. This
evidence is bounded to one local WorkVCS repository delivery Store and does not
prove automatic shared-Claim arbitration, distributed collaboration, remote
multi-operator coordination, Push, tags, release operations, or V2 behavior.
