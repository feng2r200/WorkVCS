# ADR-0495: Phase 4OK Plan Why Direct Task Closure

Status: Accepted
Date: 2026-09-04

## Context

Phase 4OE made current VR-backed Verification closure chains visible from
Task and Acceptance Criterion `why`. Phase 4OG added Resource basis fields to
those same closure chains. Phase 4OH then proved that a human operator can use
Task or Acceptance Criterion `why` to recover a stale Resource-backed closeout
without a separate `verification show` lookup.

The follow-up Phase 4OJ read-only probe found a remaining direct containment
gap for Plan queries. A temporary Store had one Goal, one Plan, one directly
contained Task, one Acceptance Criterion, one Verification Requirement, one
Resource, and one Resource-backed Verification. After Task, Plan, and Goal
closeout, Task and Acceptance Criterion `why` exposed the Resource-backed
closure chain, but Plan `why` did not:

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
release_flag_positive_hits=0
```

The evidence showed that Plan `why` already saw the direct contained Task
through current primary containment relation edges. The gap was limited to the
existing verification closure projection, which only handled Task and
Acceptance Criterion subjects.

## Decision

`workvcs why --entity <Plan>` now includes VR-backed Verification closure
chains from Tasks directly contained by that Plan through current
`primary_containment` relations.

The Plan projection reuses the existing Task closure-chain construction. Each
chain reports the same fields already available from Task and Acceptance
Criterion `why`:

- Acceptance Criterion entity id, current version id, and local key.
- Verification Requirement entity id, current version id, and local key.
- Verification commit id, entity id, entity version id, and result.
- Evidence ids cited by the Verification.
- Resource basis entries copied from the Verification state: resource id,
  adapter kind/version, scope kind/version, canonical scope payload JSON,
  baseline observation id, and baseline fingerprint.

The projection is read-only and uses current WorkState containment at the
resolved `why` target commit. It does not change stored semantics.

## Non-Goals

- No Goal-to-Plan-to-Task or other ancestor rollup.
- No full relation-subject traversal.
- No multi-hop/full evolution traversal.
- No broader causal traversal.
- No broader ContextPacket or Resource resolver behavior.
- No Store schema, migration, snapshot schema, or CLI flag change.
- No Verification, Evidence, Acceptance Criterion status, Task/Plan/Goal
  closeout, Resource observation, applicability, or cache-refresh semantic
  change.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed:

```text
log_dir=/tmp/workvcs-4ok-focused-validation-20260904T031412Z
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_resource_basis_in_vr_backed_verification_closure_from_direct_plan_task -- --exact=PASS
cargo test -p workvcs-cli cli_why_projects_phase4ok_plan_direct_task_resource_closure -- --exact=PASS
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4ok-plan-direct-task-closure-20260904T030424Z
phase4ok_dogfood=PASS
commands_exit_failures=0
failed_assertions=0
critical_failure=none
task_show_done=true
plan_show_completed=true
goal_show_achieved=true
ac_status_before_closeout_verified=true
ac_status_after_closeout_stale=true
task_why_closure_chains=true
ac_why_closure_chains=true
plan_why_closure_chains=true
goal_why_closure_chains=true
plan_why_verification_id_present=true
plan_why_verification_id_matches=true
plan_why_evidence_id_matches=true
plan_why_resource_basis=true
plan_why_resource_id_matches=true
plan_why_observation_id_matches=true
plan_why_fingerprint_matches=true
cache_refresh_from_plan_why_id_applicable=true
post_refresh_ac_status_verified=true
doctor_required_valid=true
release_flag_positive_hits=0
```

Final validation passed:

```text
log_dir=/tmp/workvcs-4ok-plan-direct-task-closure-20260904T030424Z/final-validation
validation_status=PASS
validation_failures=0
```

Detailed evidence is recorded in
`docs/provenance/phase-4ok-plan-why-direct-task-closure.md`.

## Consequences

A Plan-level continuation operator can now query the Plan and see the current
direct Task's VR-backed closeout closure, including Resource basis, without
first querying the Task or Acceptance Criterion. The Phase 4OK dogfood also
proves that the exposed Verification id can drive the existing basis-aware
cache-refresh recovery path after closeout stales the Acceptance Criterion.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4OK closes one concrete Plan-to-direct-Task closeout
explanation gap, but Goal ancestor rollup, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, broader
context/Resource resolver maturity, and release-candidate validation remain
open.
