# Phase 4OU Structural Reference Context Resource Probe

Status: accepted evidence
Date: 2026-09-07

## Purpose

Phase 4OU checks a bounded structural-reference Context/Resource resolver
boundary left open after Phase 4OT: a focused Plan or Goal directly
`references` a Task outside its containment path, and that referenced Task has
a current-head Resource-backed Verification Requirement.

The pre-change probe found a concrete recovery gap in ContextPacket output.
The implementation closes only the direct Plan/Goal-to-Task structural
reference path for normal/full context packets. It does not change Store
schema, packet schema, public CLI flags, `why`, runnable, `claim next`,
structural reference semantics, or release state.

## Source State

- Repository: `/Users/example/Repositories/CLI/WorkVCS`
- Current main at pre-change probe start:
  `92b0e5c2424fe87e3cc5a573386a8c721b2221c0`
- Pre-change probe log dir:
  `/tmp/workvcs-4ou-structural-reference-context-resource-probe-20260907T050012Z`
- Post-change probe log dir:
  `/tmp/workvcs-4ou-structural-reference-context-resource-probe-20260907T050627Z`

## Probe Shape

The probe created a temporary Store under `/tmp` with:

- one active Workspace and Branch;
- one referrer Goal with one direct referrer Plan;
- one separate owner Goal with one direct owner Plan and one referenced Task;
- one required Acceptance Criterion and Verification Requirement for the
  referenced Task;
- one local-file Resource and current-head Resource-backed passed Verification
  for that Verification Requirement;
- one direct Plan-to-Task structural reference;
- one direct Goal-to-Task structural reference;
- one Session focused first on the referrer Plan, then on the referrer Goal.

## Pre-Change Evidence

The pre-change public CLI probe summary records:

- `probe_execution_status=PASS`
- `probe_result=STRUCTURAL_REFERENCE_CONTEXT_RESOURCE_RECOVERY_UNSUPPORTED`
- `commands_exit_failures=0`
- `why_reference_visible=true`
- `plan_context_has_target=false`
- `plan_context_has_vr=false`
- `plan_context_has_resource_basis=false`
- `plan_context_has_refresh_hint=false`
- `goal_context_has_target=false`
- `goal_context_has_vr=false`
- `goal_context_has_resource_basis=false`
- `goal_context_has_refresh_hint=false`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `repo_dirty=0`
- `release_positive_hits=0`

This proves structural references were visible through existing reference and
`why` surfaces, and basis-aware cache refresh already worked once the
Verification id was known, but Plan/Goal-focused ContextPacket output did not
recover the referenced Task's Resource-backed Verification Requirement or
refresh hint.

## Implementation

The Phase 4OU implementation:

- loads current-head structural reference snapshots while resolving
  ContextOverview and validates their workspace and commit anchors;
- for normal/full context packets only, when the Session focus is a Plan or
  Goal, inspects direct structural references from that focused entity to
  Tasks;
- reuses the existing Resource-backed Verification Requirement summary
  renderer for those referenced Tasks;
- marks the summary with `structural_reference_task=...`,
  `referrer=...`, `referrer_kind=...`, `relation_id=...`, and
  `reference_direct=true`;
- preserves brief context behavior and all existing same-Plan and same-Goal
  peer summary markers.

## Post-Change Evidence

The post-change public CLI probe summary records:

- `probe_execution_status=PASS`
- `probe_result=STRUCTURAL_REFERENCE_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`
- `commands_exit_failures=0`
- `why_reference_visible=true`
- `plan_context_has_target=true`
- `plan_context_has_vr=true`
- `plan_context_has_resource_basis=true`
- `plan_context_has_refresh_hint=true`
- `goal_context_has_target=true`
- `goal_context_has_vr=true`
- `goal_context_has_resource_basis=true`
- `goal_context_has_refresh_hint=true`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `release_positive_hits=0`

The run log also records `status=0` for the `doctor --require-valid` step.

Targeted core regression tests:

- `context_packet_summarizes_structural_reference_task_resource_recovery_when_focused_on_goal_or_plan`
- `context_packet_summarizes_same_plan_peer_resource_recovery_when_focused_on_task`
- `context_packet_summarizes_same_goal_cross_plan_peer_resource_recovery_when_focused_on_task`
- `context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements`

## Conclusion

Phase 4OU closes the direct Plan/Goal structural-reference-to-Task
Context/Resource recovery gap for the tested bounded shape. A continuation
operator focused on the referrer Plan or Goal can see the referenced Task's
Resource-backed Verification Requirement, Resource basis, baseline observation,
and basis-aware refresh hint in normal/full ContextPacket output.

This does not prove structural-reference Plan targets, multi-hop structural
references, broad Resource resolver maturity, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, or release
candidate readiness.

Release flags remain false:

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
