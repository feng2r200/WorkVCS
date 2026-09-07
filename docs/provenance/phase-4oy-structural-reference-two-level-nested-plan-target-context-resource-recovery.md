# Phase 4OY Structural Reference Two-Level Nested Plan Target Context Resource Recovery

Status: accepted evidence
Date: 2026-09-07

## Purpose

Phase 4OY checks and closes the next bounded structural-reference
Context/Resource resolver boundary after Phase 4OW: a focused Plan or Goal
directly `references` another Plan outside its containment path, that
referenced Plan contains a first nested Plan, that first nested Plan contains
a second nested Plan, and the second nested Plan directly contains a Task with
a current-head Resource-backed Verification Requirement.

The pre-change probe found a concrete recovery gap in ContextPacket output.
The implementation closes only this one-reference-hop plus two containment-hop
nested Plan target path for normal/full context packets. It does not recurse
through arbitrary Plan depth, follow multi-hop structural references, or
change Store schema, packet schema, public CLI flags, `why`, runnable,
`claim next`, structural reference semantics, ADR authority, or release state.

## Source State

- Repository: `/Users/example/Repositories/CLI/WorkVCS`
- Current main at pre-change probe start:
  `a341b8ca3493af68c767c2ca69dc2ec49553a9e0`
- Pre-change probe log dir:
  `/tmp/workvcs-4ox-structural-reference-two-level-nested-plan-target-context-resource-probe-20260907T055440Z`
- Post-change probe log dir:
  `/tmp/workvcs-4ox-structural-reference-two-level-nested-plan-target-context-resource-probe-20260907T055815Z`
- Implementation validation log dir:
  `/tmp/workvcs-4oy-implementation-validation-20260907T055808Z`

## Probe Shape

The probe created a temporary Store under `/tmp` with:

- one active Workspace and Branch;
- one referrer Goal with one direct referrer Plan;
- one separate owner Goal with one referenced Plan;
- one first nested Plan directly contained by the referenced Plan;
- one second nested Plan directly contained by the first nested Plan;
- one Task directly contained by the second nested Plan;
- one required Acceptance Criterion and Verification Requirement for that
  Task;
- one local-file Resource and current-head Resource-backed passed Verification
  for that Verification Requirement;
- one direct Plan-to-Plan structural reference from the referrer Plan to the
  referenced Plan;
- one direct Goal-to-Plan structural reference from the referrer Goal to the
  referenced Plan;
- one Session focused first on the referrer Plan, then on the referrer Goal.

## Pre-Change Evidence

The pre-change public CLI probe summary records:

- `probe_execution_status=PASS`
- `probe_result=STRUCTURAL_REFERENCE_TWO_LEVEL_NESTED_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_UNSUPPORTED`
- `commands_exit_failures=0`
- `why_reference_visible=true`
- `plan_context_has_plan_target=false`
- `plan_context_has_first_nested_plan=false`
- `plan_context_has_second_nested_plan=false`
- `plan_context_has_target_task=false`
- `plan_context_has_vr=false`
- `plan_context_has_resource_basis=false`
- `plan_context_has_refresh_hint=false`
- `goal_context_has_plan_target=false`
- `goal_context_has_first_nested_plan=false`
- `goal_context_has_second_nested_plan=false`
- `goal_context_has_target_task=false`
- `goal_context_has_vr=false`
- `goal_context_has_resource_basis=false`
- `goal_context_has_refresh_hint=false`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `repo_dirty=0`
- `release_positive_hits=0`

This proves direct Plan/Goal-to-Plan structural references and the two
Plan-to-Plan containment hops were accepted and visible through existing
reference and `why` surfaces, and basis-aware cache refresh already worked once
the Verification id was known, but focused ContextPacket output did not
recover the referenced Plan, first nested Plan, second nested Plan, nested
Task, or refresh hint.

## Implementation

The Phase 4OY implementation:

- reuses Phase 4OU through Phase 4OW structural-reference ContextOverview data;
- for normal/full context packets only, when the Session focus is a Plan or
  Goal, inspects direct structural references from that focused entity to
  Plans;
- preserves direct child Task recovery from the referenced Plan;
- preserves one-level child Plan recovery from the referenced Plan;
- additionally inspects one more level of child Plans directly contained by
  those one-level child Plans;
- reuses the existing Resource-backed Verification Requirement summary
  renderer for Tasks directly contained by those second-level child Plans;
- marks the summary with
  `structural_reference_two_level_nested_plan_task=...`, `referrer=...`,
  `referrer_kind=...`, `referenced_plan=...`, `first_nested_plan=...`,
  `second_nested_plan=...`, `relation_id=...`,
  `referenced_plan_containment_relation_id=...`,
  `nested_plan_containment_relation_id=...`,
  `plan_task_containment_relation_id=...`, `reference_direct=true`, and
  `referenced_plan_two_level_child=true`;
- preserves brief context behavior and all existing direct Task reference,
  direct Plan target, one-level nested Plan target, same-Plan peer,
  same-Goal peer, and blocked-dependency summary markers.

## Post-Change Evidence

The post-change public CLI probe summary records:

- `probe_execution_status=PASS`
- `probe_result=STRUCTURAL_REFERENCE_TWO_LEVEL_NESTED_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`
- `commands_exit_failures=0`
- `why_reference_visible=true`
- `plan_context_has_plan_target=true`
- `plan_context_has_first_nested_plan=true`
- `plan_context_has_second_nested_plan=true`
- `plan_context_has_target_task=true`
- `plan_context_has_vr=true`
- `plan_context_has_resource_basis=true`
- `plan_context_has_refresh_hint=true`
- `goal_context_has_plan_target=true`
- `goal_context_has_first_nested_plan=true`
- `goal_context_has_second_nested_plan=true`
- `goal_context_has_target_task=true`
- `goal_context_has_vr=true`
- `goal_context_has_resource_basis=true`
- `goal_context_has_refresh_hint=true`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `release_positive_hits=0`

Targeted validation also passed:

- `cargo fmt --check`
- `context_packet_summarizes_structural_reference_two_level_nested_plan_target_task_resource_recovery_when_focused_on_goal_or_plan`
- `context_packet_summarizes_structural_reference_nested_plan_target_task_resource_recovery_when_focused_on_goal_or_plan`
- `context_packet_summarizes_structural_reference_plan_target_task_resource_recovery_when_focused_on_goal_or_plan`
- `context_packet_summarizes_structural_reference_task_resource_recovery_when_focused_on_goal_or_plan`

## Conclusion

Phase 4OY closes the direct Plan/Goal structural-reference-to-Plan-target plus
two-level nested Plan child Task Context/Resource recovery gap for the tested
bounded shape. A continuation operator focused on the referrer Plan or Goal
can see the referenced Plan, both nested Plans, nested Task, Resource-backed
Verification Requirement, Resource basis, baseline observation, and
basis-aware refresh hint in normal/full ContextPacket output.

This does not prove deeper nested Plan traversal, multi-hop structural
references, broad Resource resolver maturity, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, or release
candidate readiness.

Release flags remain false:

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
