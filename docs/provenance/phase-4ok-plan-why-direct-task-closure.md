# Phase 4OK Plan Why Direct Task Closure Evidence

Status: current implementation and dogfood evidence
Date: 2026-09-04

This note records the Phase 4OK implementation evidence. It is provenance for
ADR-0495 and the V1 readiness ledger. It does not create a release-candidate
or release-ready claim.

## Scope

Phase 4OK extends the existing `verification_closure_chains` projection for
`workvcs why` from Task and Acceptance Criterion subjects to Plan subjects
only when the Plan directly contains the Task through current
`primary_containment`.

The slice does not implement Goal ancestor closure, full relation-subject
traversal, multi-hop/full evolution traversal, broader causal traversal,
broader ContextPacket or Resource resolver behavior, schema changes, CLI flag
changes, release operations, Push, tag, deployment, V2, cloud/distributed
collaboration, or Agent orchestration.

## Source State

- Implementation branch: `phase-4ok-plan-why-direct-task-closure`
- Implementation worktree:
  `/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ok-plan-why-direct-task-closure`
- Starting main commit:
  `ecd08737cd6ca9aa9f6967ddec6dd4be0443f879`
- Starting status log:
  `/tmp/workvcs-4ok-plan-direct-task-closure-20260904T025415Z`
- Starting Work Governance state: `WORK_STATUS_READY`, `UNMANAGED_EMPTY`,
  `NO_ACTIVE_PLAN`
- Starting release flag positive hits: `0`
- ADR authority path: `docs/decisions/adr/`
- Schema authority count in short status: `5`

## Current Evidence Basis

Phase 4OJ proved that the gap was concrete and limited:

```text
log_dir=/tmp/workvcs-4oj-plan-goal-closeout-why-probe-20260904T022737Z
probe_execution_status=PASS
probe_result=CONCRETE_GAP_FOUND
gap_kind=plan_goal_why_omits_descendant_resource_backed_closeout_closure
task_why_closure_chains=1
ac_why_closure_chains=1
task_why_resource_basis=true
ac_why_resource_basis=true
plan_why_closure_chains=0
goal_why_closure_chains=0
plan_why_contains_task_id=true
goal_why_contains_task_id=false
plan_primary_containment_relation_edges=2
goal_primary_containment_relation_edges=1
ancestor_closure_supported=false
doctor_required_valid=true
release_flag_positive_hits=0
```

Current code inspection before the change confirmed that
`why_verification_closure_chains` handled only `Task` and
`AcceptanceCriterion` subjects; `Plan` and `Goal` were no-op subjects for this
projection.

## Implementation Summary

Phase 4OK adds a small helper that preserves the existing Task closure-chain
logic. The Task subject path now calls that helper directly. The Plan subject
path reads current primary containment relations at the resolved `why` target
commit, selects only direct child Tasks of the queried Plan, and reuses the
same helper to project the Task's Acceptance Criterion -> Verification
Requirement -> Verification -> Evidence closure chain and Resource basis.

Goal remains outside this projection.

## Focused Validation

Focused validation log:
`/tmp/workvcs-4ok-focused-validation-20260904T031412Z`

Key result fields:

- `core_status=0`
- `cli_status=0`

Focused commands:

```text
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_resource_basis_in_vr_backed_verification_closure_from_direct_plan_task -- --exact
cargo test -p workvcs-cli cli_why_projects_phase4ok_plan_direct_task_resource_closure -- --exact
```

## Public CLI Dogfood

Dogfood log:
`/tmp/workvcs-4ok-plan-direct-task-closure-20260904T030424Z`

The dogfood used a temporary Store and the built `target/debug/workvcs`
binary. It created a Goal, Plan, directly contained Task, Acceptance
Criterion, Verification Requirement, local-file Resource, Resource-backed
Verification, Task closeout, Plan completion, and Goal achievement. It then
queried Task, Acceptance Criterion, Plan, and Goal `why` at the final Branch
head and used the Verification id surfaced by Plan `why` to run basis-aware
cache refresh.

Key result fields:

- `phase4ok_dogfood=PASS`
- `commands_exit_failures=0`
- `failed_assertions=0`
- `critical_failure=none`
- `branch_id=01a06a5f-e9cb-7bb2-af9d-d47eac6a239f`
- `final_head=01a06a5f-eb30-72a2-b257-428f54c607d1`
- `goal_id=01a06a5f-e9e4-7753-898a-6c4df53e3449`
- `plan_id=01a06a5f-e9fd-7f70-be16-c965d6c90fe6`
- `task_id=01a06a5f-ea2c-7bc0-bb3c-bcdc6d2d53eb`
- `criterion_id=01a06a5f-ea5d-76a1-b756-c51e93b79092`
- `requirement_id=01a06a5f-ea7a-7ae2-8bec-546205b0edd9`
- `verification_id=01a06a5f-eaad-7773-89a0-dd47f7555652`
- `resource_id=01a06a5f-ea91-73d3-811b-05d53b068d49`
- `task_show_done=true`
- `plan_show_completed=true`
- `goal_show_achieved=true`
- `ac_status_before_closeout_verified=true`
- `ac_status_after_closeout_stale=true`
- `task_why_closure_chains=true`
- `ac_why_closure_chains=true`
- `plan_why_closure_chains=true`
- `goal_why_closure_chains=true`
- `plan_why_verification_id_present=true`
- `plan_why_verification_id_matches=true`
- `plan_why_evidence_id_matches=true`
- `plan_why_resource_basis=true`
- `plan_why_resource_id_matches=true`
- `plan_why_observation_id_matches=true`
- `plan_why_fingerprint_matches=true`
- `cache_refresh_from_plan_why_id_applicable=true`
- `post_refresh_ac_status_verified=true`
- `doctor_required_valid=true`
- `release_flag_positive_hits=0`

## Final Validation

Final validation log:
`/tmp/workvcs-4ok-plan-direct-task-closure-20260904T030424Z/final-validation`

Key result fields:

- `validation_status=PASS`
- `validation_failures=0`

## Interpretation

Phase 4OK closes the Plan direct Task explanation gap proven by Phase 4OJ. A
Plan-level continuation operator can recover the direct Task's Resource-backed
Verification id and Resource basis from Plan `why`, then use the existing
basis-aware cache-refresh path without opening the Task, Acceptance Criterion,
or Verification detail first.

The dogfood intentionally keeps Goal outside the new closure projection:
Goal `why` still reports `verification_closure_chains=0`. This preserves the
no-go boundary against Goal ancestor rollup and multi-hop traversal.

Release state remains unchanged:

- `V0_1_DOGFOOD_COMPLETE=false`
- `V1_RELEASE_READY=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
