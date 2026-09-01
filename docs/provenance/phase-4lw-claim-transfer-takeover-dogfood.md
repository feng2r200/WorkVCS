# Phase 4LW Claim Transfer and Takeover Dogfood Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LW dogfoods Claim transfer and stale-gated takeover in realistic
continuation paths against the real local `agent_soul` project:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

The target project remained read-only. The WorkVCS Store and logs were written
only under:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lw-claim-transfer-takeover-dogfood
```

## Dogfood Run

Successful Store and log directory:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lw-claim-transfer-takeover-dogfood/.work-governance/runtime/dogfood/phase-4lw-20260901T020331Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lw-claim-transfer-takeover-dogfood/.work-governance/runtime/logs/phase-4lw/claim-transfer-takeover.20260901T020331Z
```

Scenario A, cooperative transfer, proved:

- one owner Session can claim the README review Task;
- a second active Session can receive the active Claim through
  `claim transfer`;
- the receiver's `claim guard` returns `allowed=true` and
  `reason=owned_exclusive_claim`;
- the receiver can save scoped context, run `verify` with Resource-backed
  evidence, get `ac status=verified`, and transition the Task to `done`; and
- both Sessions can be ended with SessionDiffs.

Scenario B, stale-gated takeover, proved:

- a taking Session blocked by another active exclusive Claim receives
  `reason=exclusive_claim_owned_by_other_session`;
- the stable stale-takeover hint fields identify the blocking Claim and
  previous owner Session;
- forced takeover is rejected while the previous owner remains active;
- explicit `session mark-stale` moves the previous owner to
  `potentially_stale`;
- `claim takeover --force` then replaces the Claim with rationale;
- the recovered guard returns `allowed=true` and
  `reason=owned_exclusive_claim`;
- the taking Session can save scoped context, run `verify` with
  Resource-backed evidence, get `ac status=verified`, and transition the Task
  to `done`; and
- closeout starts a fresh active Session before saving transition context.

Summary:

```text
phase4lw_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lw-claim-transfer-takeover-dogfood/.work-governance/runtime/dogfood/phase-4lw-20260901T020331Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lw-claim-transfer-takeover-dogfood/.work-governance/runtime/logs/phase-4lw/claim-transfer-takeover.20260901T020331Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
workspace_id=01a05ab5-1c3e-7f10-b09f-3dd033f64827
branch_id=01a05ab5-1c3e-7f10-b09f-3e0d46012002
goal_entity_id=01a05ab5-1c8c-72a3-8f56-01344e9d8681
plan_entity_id=01a05ab5-1ca2-7f43-a5c8-c2d2a926a192
resource_id=01a05ab5-1c58-7160-a219-845642bfc9f5
transfer_task_entity_id=01a05ab5-1cba-7c03-abcd-4a245ad90e2d
transfer_acceptance_criterion_entity_id=01a05ab5-1ce9-7e33-b6fa-f4c2a24b5ee9
transfer_verification_requirement_entity_id=01a05ab5-1d03-7ed1-be6c-ef324d3fa277
transfer_owner_session_id=01a05ab5-1d48-7801-b2ff-adfa8f2d7d88
transfer_receiver_session_id=01a05ab5-1d59-77e1-8f32-f7b9627e14f8
transfer_previous_claim_id=01a05ab5-1d6d-7d32-8edb-27f360ba2998
transfer_claim_id=01a05ab5-1d7f-7470-ae01-3c4616d6f4d0
transfer_context_packet_id=01a05ab5-1ddb-7582-8d96-cfa33b623039
transfer_verification_entity_id=01a05ab5-1df7-77d1-a62c-f3760d0289b3
transfer_owner_session_diff_id=01a05ab5-1e71-7c10-b657-b09cdae0e2a2
transfer_receiver_session_diff_id=01a05ab5-1e93-76e0-acf3-6cf5dbb6b8d0
takeover_task_entity_id=01a05ab5-1cd2-71b1-9c6d-3499e0725d27
takeover_acceptance_criterion_entity_id=01a05ab5-1d19-7c42-90f3-cd4bcd567d9e
takeover_verification_requirement_entity_id=01a05ab5-1d33-7862-a89d-59df4ca773a7
takeover_previous_owner_session_id=01a05ab5-1ea5-7633-9fc5-561da82e9f80
takeover_taking_session_id=01a05ab5-1eb7-7921-91df-3ab55ce6080a
takeover_previous_claim_id=01a05ab5-1ecb-7bf0-8537-e3b3aafbd4a7
takeover_hint_claim_id=01a05ab5-1ecb-7bf0-8537-e3b3aafbd4a7
takeover_hint_previous_session_id=01a05ab5-1ea5-7633-9fc5-561da82e9f80
takeover_claim_id=01a05ab5-1f19-7231-b09d-acea23299864
takeover_context_packet_id=01a05ab5-1f91-7ad0-aab4-b051a4a7f93b
takeover_verification_entity_id=01a05ab5-1fb2-7a33-a523-4142da231368
takeover_owner_session_diff_id=01a05ab5-203e-79b1-9258-3498779e0cc0
takeover_taker_session_diff_id=01a05ab5-2051-77a1-8d2c-cc84c00cc13f
closeout_viewer_session_id=01a05ab5-2096-7091-9461-02454856f211
closeout_viewer_session_diff_id=01a05ab5-211c-7573-898e-56aac4d57f3b
transition_context_packet_id=01a05ab5-20f6-7ca1-9262-638b1abbc442
final_head_commit_id=01a05ab5-2082-72a2-b392-bfad39f385cd
target_status_unchanged=PASS
```

## Recovery Finding

The first run failed when it attempted to save a ContextPacket from an ended
Session:

```text
context_transition_show=FAIL rc=1
session invalid: session 01a05ab2-f422-7910-8523-5633727f06e7 is not active
```

The successful run records `context_rejects_ended_transfer_owner=PASS` and
uses a new active closeout-viewer Session before saving the transition
ContextPacket. This is a useful operator boundary, not a code blocker.

## Target Project Evidence

The run captured before/after target `git status --porcelain=v1` and
`git diff --name-status` snapshots, then compared both pairs byte-for-byte.
The comparisons passed.

The target project remained dirty because it was already dirty before this
slice. That dirty state is unrelated to Phase 4LW.

## Findings

- Cooperative `claim transfer` works as a real continuation handoff between
  active Sessions.
- Stale-gated `claim takeover --force` works as an explicit recovery path after
  `claim guard` hints and `session mark-stale`.
- `verify` remains practical for Resource-backed evidence when the operator can
  provide the file path and scope.
- Context remains active-Session-scoped. After Session closeout, start a fresh
  active continuation Session before resolving context.
- This evidence does not prove automatic stale detection, shared-mode maturity,
  adapter-backed re-observation, remote coordination, or broad multi-project
  maturity.

## Independent Review

Independent review initially found one Medium inconsistency: the Handoff row in
the V1 readiness ledger still treated Claim transfer/takeover realistic
continuation repetition as an open gap. The ledger row was corrected to cite
Phase 4LW and leave broader Handoff consumption across varied project and
write-mode flows as the remaining Handoff gap.

The follow-up review confirmed that Medium was closed and found no
blocker/high findings. It raised one further Medium documentation-consistency
issue: this provenance file needed to record the independent-review closeout
explicitly instead of leaving that detail only in the plan. This section closes
that documentation gap.

## Validation

The final pre-commit validation matrix passed after the independent-review
ledger fix:

```text
git_diff_check=PASS log=/tmp/workvcs-4lw-final-validation-20260901T023000Z/git_diff_check.log
cargo_fmt=PASS log=/tmp/workvcs-4lw-final-validation-20260901T023000Z/cargo_fmt.log
schema=PASS log=/tmp/workvcs-4lw-final-validation-20260901T023000Z/schema.log
cargo_clippy=PASS log=/tmp/workvcs-4lw-final-validation-20260901T023000Z/cargo_clippy.log
cargo_test=PASS log=/tmp/workvcs-4lw-final-validation-20260901T023000Z/cargo_test.log
cli_smoke=PASS log=/tmp/workvcs-4lw-final-validation-20260901T023000Z/cli_smoke.log
```

Summary path:

```text
/tmp/workvcs-4lw-final-validation-20260901T023000Z/summary.log
```
