# Phase 4MS Handoff Consumption Write-Mode Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MS dogfoods focused Handoff consumption in a real WorkVCS repository
delivery slice that includes a repository documentation write.

The slice uses a source Session and a separate continuation Session:

- the source Session focuses the documentation Task, ends with a SessionDiff,
  and authors a focused Handoff;
- the continuation Session starts with no focus, consumes the Handoff through
  `handoff consume`, and receives the focused Task without manual focus-copy;
- the continuation Session claims the focused Task, verifies the repository
  write, and closes the WorkVCS Task.

This is dogfood and documentation evidence only. It does not change Rust code,
schema, CLI behavior, release operations, Push state, tags, deployment, or V2
scope. It also does not claim that WorkVCS V1 is release-ready or that V0.1
dogfood is complete.

## Dogfood Setup

- Source repository baseline: main commit
  `ef025c7b1eaaacb9100113d86dd15c9639d5afa2`.
- Inspection log directory:
  `/tmp/workvcs-4ms-inspection-20260901T122229Z`.
- Pre-write dogfood log directory:
  `/tmp/workvcs-4ms-handoff-consumption-write-mode-20260901T122825Z`.
- Dogfood Store:
  `/tmp/workvcs-4ms-handoff-consumption-write-mode-20260901T122825Z/store.workvcs`.
- Workspace id: `01a05cf1-3054-7291-a202-893ab8dbaf26`.
- Branch id: `01a05cf1-3054-7291-a202-896d45491311`.
- Pre-write head commit id:
  `01a05cf1-3164-7d62-bc5f-091ad62d7145`.
- Goal id: `01a05cf1-3069-7852-99e4-e4054b66c488`.
- Plan id: `01a05cf1-307e-7c90-beb1-c34ff1c1c8d3`.
- Task id: `01a05cf1-3092-7ae0-875b-13164d6268c1`.
- Task version id: `01a05cf1-30db-7e91-869b-a129c8d101f5`.
- Acceptance Criterion id:
  `01a05cf1-30db-7e91-869b-a105acc37fdc`.
- Verification Requirement id:
  `01a05cf1-30f5-7a21-a854-c59040f17a68`.
- Source Session id: `01a05cf1-3120-7893-8f51-404256788515`.
- Source SessionDiff id: `01a05cf1-3146-7103-9ce8-87fc555e9105`.
- Handoff Record id: `01a05cf1-3164-7d62-bc5f-094d87f3d01b`.
- Handoff commit id: `01a05cf1-3164-7d62-bc5f-091ad62d7145`.
- Continuation Session id: `01a05cf1-31a2-7e00-8dbf-b7313ec04147`.
- Continuation Claim id: `01a05cf2-4ef9-7432-89b2-f930a08dba62`.

## Command Coverage

The inspection run first built the current CLI:

```text
cargo build -q -p workvcs-cli
```

It then exercised or inspected these process-boundary commands:

```text
handoff create
handoff show
handoff consume
session start
session focus-set
session show
session end
claim next
claim guard
runnable tasks
context
next
verify
task transition
store integrity
doctor
```

## Pre-Write Handoff Consumption Evidence

Before the repository documentation write, the target provenance file did not
exist:

```text
target_doc_preexisting=no
```

The WorkVCS Store created a pending documentation Task with AC/VR obligations.
Before post-write verification, that Acceptance Criterion was still
unverified:

```text
ac-status-before_status=0
task_status_prewrite=pending
```

The source Session focused the Task, ended, and produced a SessionDiff:

```text
source_session_id=01a05cf1-3120-7893-8f51-404256788515
source_session_diff_id=01a05cf1-3146-7103-9ce8-87fc555e9105
```

The source Session then authored a focused Handoff over that ended
SessionDiff:

```text
handoff_record_id=01a05cf1-3164-7d62-bc5f-094d87f3d01b
handoff_commit_id=01a05cf1-3164-7d62-bc5f-091ad62d7145
handoff_scope_recognized=true
```

The continuation Session initially had no focus. `handoff consume` then applied
the Handoff focus to that separate active Session:

```text
continuation-session-before-consume_status=0
handoff-consume_status=0
consume_focus_entity_id=01a05cf1-3092-7ae0-875b-13164d6268c1
consume_session_matches_expected=true
consume_lifecycle_state_matches_expected=true
```

After consumption, `session show` and `context` confirmed the continuation
Session was focused on the documentation Task:

```text
continuation-session-after-consume_status=0
continuation-context_status=0
context_contains_focus_task=yes
```

The same continuation Session then saw the Task as runnable, selected it
through `next`, claimed it, and passed the Claim guard:

```text
runnable_after_consume_task=01a05cf1-3092-7ae0-875b-13164d6268c1
next_selected_after_consume=true
continuation_claim_id=01a05cf2-4ef9-7432-89b2-f930a08dba62
post_claim_guard_allowed=true
post_claim_guard_reason=owned_exclusive_claim
```

The Task could not be completed before VR-backed evidence was recorded:

```text
blocked-preverify-transition_status=1
blocked_preverify_transition_contains_unverified=yes
head_unchanged_after_blocked_transition=yes
```

## Dogfood Findings

- The first run expected `handoff consume --expected-lifecycle-state ended`,
  but current CLI behavior checks the target continuation Session lifecycle;
  the correct expectation for this scenario is `active`.
- The second run proved `handoff consume` succeeded, but the harness initially
  checked a stale field name. Current output uses
  `consume_session_matches_expected`, not `session_matches_expected`.
- The successful run continued from the already-consumed Handoff state instead
  of recreating a third Store, preserving the actual transition that mattered.
- The first post-write integrity attempt passed an unsupported `--branch`
  option to `store integrity`. The corrected check uses the current whole-store
  command shape, which passed with `valid_required=true`.
- `handoff consume` removed manual focus-copy for this write-mode continuation.
  The continuation Session still needs an explicit Session, Claim, verification,
  and Task transition. That remains the intended V1-local operator model.

These are dogfood harness and operator-boundary findings. They did not require
a WorkVCS product, schema, or CLI behavior change in this slice.

## Readiness Impact

Phase 4LF proved focused Handoff consumption in a local continuation loop.
Phase 4LG proved blocked Handoff recovery through explicit stale marking and
takeover. Phase 4LV and Phase 4LW added real external-project read-only and
continuation evidence. Phase 4MS extends the Handoff evidence into a real
write-mode repository delivery slice: the continuation Session consumes the
focused Handoff, claims the focused Task, performs the documentation write, and
closes the Store obligation.

This moves the release gate for Handoff consumption from missing write-mode
Handoff consumption dogfood to bounded pass evidence for the V1-local scope. It
does not prove remote Handoff, cross-Store synchronization, automatic Agent
orchestration, automatic takeover, or distributed collaboration.

## Post-Write Store Closeout

PASS:
`/tmp/workvcs-4ms-handoff-consumption-write-mode-20260901T122825Z`.

After this repository documentation write, the post-write Git diff recorded the
expected documentation write set:

```text
M docs/README.md
A docs/decisions/adr/0460-phase-4ms-handoff-consumption-write-mode-dogfood.md
A docs/provenance/phase-4ms-handoff-consumption-write-mode-dogfood.md
M docs/provenance/v1-readiness-ledger.md
M docs/provenance/v1-release-gate-matrix.md
```

The continuation Session recorded VR-backed post-write verification evidence:

```text
post-write-verify_status=0
post_write_verification_id=01a05cf5-92a3-7c13-b8d4-ae9dc06efa34
post_write_evidence_id=01a05cf5-929f-7381-ab88-b713ce93b748
post_write_verify_commit_id=01a05cf5-92a3-7c13-b8d4-aee09553ef32
ac_status_after_verify=verified
```

The same continuation Session then completed the focused Task:

```text
continuation-task-done_status=0
continuation_task_status=done
final_head_commit_id=01a05cf5-92e1-7ea2-a28b-cb6f16e84591
final_task_version_id=01a05cf5-92e1-7ea2-a28b-cb82f3d0373e
```

The continuation Claim and Session were closed:

```text
continuation_claim_lifecycle=released
continuation_session_lifecycle=ended
continuation_session_diff_id=01a05cf5-9323-70f2-960d-e41008181c26
```

Final Store inspection passed:

```text
final_history_entries=11
integrity_valid_required=true
doctor_valid_required=true
phase4ms_post_write_dogfood=pass
dogfood_complete=pass
```

## Final Validation

PASS:
`/tmp/workvcs-4ms-final-validation-20260901T123956Z`.

T-004 final validation passed `git diff --check`, `cargo fmt --all -- --check`,
required-document existence checks, README/ADR/provenance/ledger/matrix anchor
checks, dogfood summary anchors, raw Handoff consumption anchors, blocked
pre-verification transition anchors, post-write Verification anchors, corrected
Store integrity/doctor anchors, and the recorded unsupported `store integrity
--branch` finding.

Independent review found no blocker, high, or medium findings. The only low
finding was that this section still carried a stale pending closeout marker;
this update resolves that marker without widening the V1-local Handoff scope.
