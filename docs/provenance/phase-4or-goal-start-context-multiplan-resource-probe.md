# Phase 4OR Goal-Start Multi-Plan Context Resource Probe

Status: accepted evidence
Date: 2026-09-07

## Purpose

Phase 4OR checks the smallest adjacent Context/Resource resolver boundary left
open after Phase 4OQ: a Goal-focused continuation where the Goal has multiple
direct child Plans, each with a runnable Task that has a current-head
Resource-backed Verification Requirement.

This is a read-only dogfood probe of current main. It does not change WorkVCS
behavior, schema, public CLI flags, tests, ADR authority, or release state.

## Source State

- Repository: `/Users/example/Repositories/CLI/WorkVCS`
- Current main at probe start:
  `5040379b00f5fc901ca76813a70a3f367da394d2`
- Effective probe log dir:
  `/tmp/workvcs-4or-goal-start-multiplan-context-resource-probe-20260907T042246Z`
- Earlier harness-only failed attempt:
  `/tmp/workvcs-4or-goal-start-multiplan-context-resource-probe-20260907T042033Z`

The earlier attempt misread the `workspace create` output field as
`commit_id` instead of the current public CLI field `genesis_commit_id`. It
therefore passed an empty head to `goal create` and is not behavioral evidence.

## Probe Shape

The effective probe created a temporary Store under `/tmp` with:

- one active Workspace and Branch;
- one active Goal;
- two direct child Plans under that Goal;
- one runnable Task under each Plan;
- one required Acceptance Criterion and Verification Requirement for each Task;
- one local-file Resource and baseline ResourceObservation for each Task;
- one current-head Resource-backed passed Verification for each Task;
- one active Session focused on the Goal.

The target Task was created before the peer Task so the stable Entity-id
ordering in the current runnable projection makes it the first selected
candidate. The probe does not rely on undefined integer priority direction.

## Evidence

The effective probe summary records:

- `probe_execution_status=PASS`
- `probe_result=GOAL_START_MULTIPLAN_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`
- `gap_kind=none`
- `commands_exit_failures=0`
- `goal_start_multiplan_context_supported=true`
- `goal_start_multiplan_claim_next_recovery_supported=true`
- `runnable_candidates_match_expected=true`
- `runnable_candidate_0_target=true`
- `runnable_candidate_1_peer=true`
- `runnable_candidate_0_runnable=true`
- `runnable_candidate_1_runnable=true`
- `focus_goal_supported=true`
- `goal_focus_normal_has_target_vr_key=true`
- `goal_focus_normal_has_peer_vr_key=true`
- `goal_focus_full_has_target_vr_key=true`
- `goal_focus_full_has_peer_vr_key=true`
- `goal_focus_full_target_has_resource_basis=true`
- `goal_focus_full_peer_has_resource_basis=true`
- `goal_focus_full_target_has_refresh_hint=true`
- `goal_focus_full_peer_has_refresh_hint=true`
- `goal_focus_full_target_has_baseline=true`
- `goal_focus_full_peer_has_baseline=true`
- `goal_focus_full_has_target_plan_path=true`
- `goal_focus_full_has_peer_plan_path=true`
- `goal_focus_normal_context_items=13`
- `goal_focus_full_context_items=13`
- `goal_focus_normal_omitted_items=0`
- `goal_focus_full_omitted_items=0`
- `goal_focus_claim_next_selected=true`
- `goal_focus_claim_next_task_matches=true`
- `goal_focus_claim_next_has_target_vr_key=true`
- `goal_focus_claim_next_has_resource_basis=true`
- `goal_focus_claim_next_has_refresh_hint=true`
- `goal_focus_claim_next_has_baseline=true`
- `cache_refresh_target_applicability=applicable`
- `cache_refresh_target_resource_stamps=1`
- `cache_refresh_peer_applicability=applicable`
- `cache_refresh_peer_resource_stamps=1`
- `doctor_required_valid_exit=0`
- `repo_git_dirty_lines_after=0`
- `release_flag_positive_hits=0`

Key output snippets are preserved in:

- `probe/run/goal_focus_context_key_snippets.txt`
- `probe/run/goal_focus_claim_next_key_snippets.txt`
- `probe/run/runnable_tasks.out`
- `probe/run/cache_refresh_from_target_verification.out`
- `probe/run/cache_refresh_from_peer_verification.out`
- `probe/run/doctor.out`

## Conclusion

Current main already supports the tested Goal-start multi-Plan
Context/Resource recovery path. A Goal-focused operator can see both direct
Plan descendants' runnable Task Resource-backed Verification Requirements,
Resource basis fields, baseline observations, and basis-aware refresh hints in
normal and full context. Focused `claim next --context-profile full` preserves
the same recovery path for the selected target Task, and each discovered
Verification id can drive
`verification cache-refresh --resource-content-from-basis`.

This removes the previously open multiple-Plan direct Goal-start evidence gap
for this bounded shape. It does not prove nested Plan traversal, cross-Goal
traversal, non-Task dependency Resource traversal, broad Resource resolver
maturity, full relation-subject traversal, multi-hop/full evolution traversal,
broader causal traversal, or release-candidate readiness.

Release flags remain false:

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
