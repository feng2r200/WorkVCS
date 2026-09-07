# Phase 4OS Nested Plan Context Resource Probe

Status: accepted evidence
Date: 2026-09-07

## Purpose

Phase 4OS checks the smallest adjacent Context/Resource resolver boundary left
open after Phase 4OR: a direct Goal or parent Plan focus that must traverse one
nested SubPlan to reach a runnable Task with a current-head Resource-backed
Verification Requirement.

This is a read-only dogfood probe of current main. It does not change WorkVCS
behavior, schema, public CLI flags, tests, ADR authority, or release state.

## Source State

- Repository: `/Users/example/Repositories/CLI/WorkVCS`
- Current main at probe start:
  `9c36b0e4f5eb9f1ace244765036ba930f95022d2`
- Effective probe log dir:
  `/tmp/workvcs-4os-nested-plan-context-resource-probe-20260907T043535Z`

## Probe Shape

The effective probe created a temporary Store under `/tmp` with:

- one active Workspace and Branch;
- one active Goal;
- one direct parent Plan under that Goal;
- one child SubPlan under the parent Plan;
- one runnable Task under the child SubPlan;
- one required Acceptance Criterion and Verification Requirement for that Task;
- one local-file Resource and baseline ResourceObservation;
- one current-head Resource-backed passed Verification;
- one active Session, first focused on the parent Plan and then focused on the
  Goal.

## Evidence

The effective probe summary records:

- `probe_execution_status=PASS`
- `probe_result=NESTED_PLAN_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`
- `gap_kind=none`
- `commands_exit_failures=0`
- `runnable_candidates_match_expected=true`
- `runnable_candidate_0_task=true`
- `runnable_candidate_0_runnable=true`
- `focus_parent_plan_supported=true`
- `parent_plan_focus_normal_has_vr_key=true`
- `parent_plan_focus_full_has_vr_key=true`
- `parent_plan_focus_full_has_resource_basis=true`
- `parent_plan_focus_full_has_refresh_hint=true`
- `parent_plan_focus_full_has_baseline=true`
- `parent_plan_focus_full_has_parent_plan_path=true`
- `parent_plan_focus_full_has_child_plan_path=true`
- `parent_plan_focus_normal_context_items=8`
- `parent_plan_focus_full_context_items=8`
- `parent_plan_focus_normal_omitted_items=0`
- `parent_plan_focus_full_omitted_items=0`
- `focus_goal_supported=true`
- `goal_focus_normal_has_vr_key=true`
- `goal_focus_full_has_vr_key=true`
- `goal_focus_full_has_resource_basis=true`
- `goal_focus_full_has_refresh_hint=true`
- `goal_focus_full_has_baseline=true`
- `goal_focus_full_has_parent_plan_path=true`
- `goal_focus_full_has_child_plan_path=true`
- `goal_focus_normal_context_items=8`
- `goal_focus_full_context_items=8`
- `goal_focus_normal_omitted_items=0`
- `goal_focus_full_omitted_items=0`
- `goal_focus_claim_next_selected=true`
- `goal_focus_claim_next_task_matches=true`
- `goal_focus_claim_next_has_vr_key=true`
- `goal_focus_claim_next_has_resource_basis=true`
- `goal_focus_claim_next_has_refresh_hint=true`
- `goal_focus_claim_next_has_baseline=true`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `doctor_required_valid_exit=0`
- `repo_git_dirty_lines_after=0`
- `release_flag_positive_hits=0`
- `nested_plan_plan_start_context_supported=true`
- `nested_plan_goal_start_context_supported=true`
- `nested_plan_goal_start_claim_next_recovery_supported=true`

Key output snippets are preserved in:

- `probe/run/parent_plan_focus_context_key_snippets.txt`
- `probe/run/goal_focus_context_key_snippets.txt`
- `probe/run/goal_focus_claim_next_key_snippets.txt`
- `probe/run/runnable_tasks.out`
- `probe/run/cache_refresh_from_verification.out`
- `probe/run/doctor.out`

## Conclusion

Current main already supports the tested one-level nested Plan
Context/Resource recovery path. A parent Plan-focused operator and a
Goal-focused operator can see the descendant runnable Task's Resource-backed
Verification Requirement, Resource basis fields, baseline observation, and
basis-aware refresh hint in normal and full context. Focused
`claim next --context-profile full` preserves the same recovery path for the
selected Task, and the discovered Verification id can drive
`verification cache-refresh --resource-content-from-basis`.

This removes the previously open one-level nested Plan evidence gap for this
bounded shape. It does not prove cross-Goal traversal, non-Task dependency
Resource traversal, broad Resource resolver maturity, full relation-subject
traversal, multi-hop/full evolution traversal, broader causal traversal, or
release-candidate readiness.

Release flags remain false:

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
