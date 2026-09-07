# Phase 4OV Structural Reference Plan Target Context Resource Probe

Status: accepted evidence
Date: 2026-09-07

## Purpose

Phase 4OV checks a bounded structural-reference Context/Resource resolver
boundary left open after Phase 4OU: a focused Plan or Goal directly
`references` another Plan outside its containment path, and that referenced
Plan directly contains a Task with a current-head Resource-backed Verification
Requirement.

The pre-change probe found a concrete recovery gap in ContextPacket output.
The implementation closes only the one-reference-hop Plan target plus direct
contained Task path for normal/full context packets. It does not recurse
through referenced Plans, follow multi-hop references, or change Store schema,
packet schema, public CLI flags, `why`, runnable, `claim next`, structural
reference semantics, or release state.

## Source State

- Repository: `/Users/example/Repositories/CLI/WorkVCS`
- Current main at pre-change probe start:
  `bcb8b07fe5ca3c0ffa4683cb369c3484b1744a3d`
- Pre-change probe log dir:
  `/tmp/workvcs-4ov-structural-reference-plan-target-context-resource-probe-20260907T052344Z`
- Post-change probe log dir:
  `/tmp/workvcs-4ov-structural-reference-plan-target-context-resource-probe-20260907T052625Z`

## Probe Shape

The probe created a temporary Store under `/tmp` with:

- one active Workspace and Branch;
- one referrer Goal with one direct referrer Plan;
- one separate owner Goal with one referenced Plan;
- one Task directly contained by the referenced Plan;
- one required Acceptance Criterion and Verification Requirement for that
  Task;
- one local-file Resource and current-head Resource-backed passed Verification
  for that Verification Requirement;
- one direct Plan-to-Plan structural reference;
- one direct Goal-to-Plan structural reference;
- one Session focused first on the referrer Plan, then on the referrer Goal.

## Pre-Change Evidence

The pre-change public CLI probe summary records:

- `probe_execution_status=PASS`
- `probe_result=STRUCTURAL_REFERENCE_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_UNSUPPORTED`
- `commands_exit_failures=0`
- `why_reference_visible=true`
- `plan_context_has_plan_target=false`
- `plan_context_has_target_task=false`
- `plan_context_has_vr=false`
- `plan_context_has_resource_basis=false`
- `plan_context_has_refresh_hint=false`
- `goal_context_has_plan_target=false`
- `goal_context_has_target_task=false`
- `goal_context_has_vr=false`
- `goal_context_has_resource_basis=false`
- `goal_context_has_refresh_hint=false`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `repo_dirty=0`
- `release_positive_hits=0`

This proves direct Plan/Goal-to-Plan structural references were accepted and
visible through existing reference and `why` surfaces, and basis-aware cache
refresh already worked once the Verification id was known, but focused
ContextPacket output did not recover the referenced Plan target, its directly
contained Resource-backed Task, or the refresh hint.

## Implementation

The Phase 4OV implementation:

- reuses Phase 4OU structural-reference ContextOverview data;
- for normal/full context packets only, when the Session focus is a Plan or
  Goal, inspects direct structural references from that focused entity to
  Plans;
- validates the referenced Plan exists in the current overview;
- inspects only Tasks directly contained by that referenced Plan;
- reuses the existing Resource-backed Verification Requirement summary
  renderer for those direct child Tasks;
- marks the summary with `structural_reference_plan_task=...`,
  `referrer=...`, `referrer_kind=...`, `referenced_plan=...`,
  `relation_id=...`, `containment_relation_id=...`,
  `reference_direct=true`, and `referenced_plan_direct_child=true`;
- preserves brief context behavior and all existing direct Task reference,
  same-Plan peer, same-Goal peer, and blocked-dependency summary markers.

## Post-Change Evidence

The post-change public CLI probe summary records:

- `probe_execution_status=PASS`
- `probe_result=STRUCTURAL_REFERENCE_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`
- `commands_exit_failures=0`
- `why_reference_visible=true`
- `plan_context_has_plan_target=true`
- `plan_context_has_target_task=true`
- `plan_context_has_vr=true`
- `plan_context_has_resource_basis=true`
- `plan_context_has_refresh_hint=true`
- `goal_context_has_plan_target=true`
- `goal_context_has_target_task=true`
- `goal_context_has_vr=true`
- `goal_context_has_resource_basis=true`
- `goal_context_has_refresh_hint=true`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `release_positive_hits=0`

Targeted core regression tests include the new Plan target recovery path and
the Phase 4OU direct Task structural-reference path.

## Conclusion

Phase 4OV closes the direct Plan/Goal structural-reference-to-Plan-target
Context/Resource recovery gap for the tested bounded shape. A continuation
operator focused on the referrer Plan or Goal can see the referenced Plan's
direct child Task, Resource-backed Verification Requirement, Resource basis,
baseline observation, and basis-aware refresh hint in normal/full
ContextPacket output.

This does not prove nested referenced Plan traversal, multi-hop structural
references, broad Resource resolver maturity, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, or release
candidate readiness.

Release flags remain false:

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
