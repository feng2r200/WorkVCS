# Phase 4OT Cross-Goal Dependency Context Resource Probe

Status: accepted evidence
Date: 2026-09-07

## Purpose

Phase 4OT checks a bounded cross-Goal Context/Resource resolver boundary left
open after Phase 4OS: a Task in one Goal is blocked by a prerequisite Task in a
different Goal, and that prerequisite has a current-head Resource-backed
Verification Requirement.

This is a read-only dogfood probe of current main. It does not change WorkVCS
behavior, schema, public CLI flags, tests, ADR authority, or release state.

## Source State

- Repository: `/Users/example/Repositories/CLI/WorkVCS`
- Current main at probe start:
  `846193310d759410b697aa6970eb1e93c38ca21d`
- Effective probe log dir:
  `/tmp/workvcs-4ot-cross-goal-dependency-context-resource-probe-20260907T044427Z`

## Probe Shape

The effective probe created a temporary Store under `/tmp` with:

- one active Workspace and Branch;
- one dependent Goal with one direct Plan and one dependent Task;
- one separate prerequisite Goal with one direct Plan and one prerequisite Task;
- one `depends_on` relation from the dependent Task to the prerequisite Task;
- one required Acceptance Criterion and Verification Requirement for the
  prerequisite Task;
- one local-file Resource and baseline ResourceObservation for the prerequisite
  Task;
- one current-head Resource-backed passed Verification for the prerequisite
  Task;
- one Session focused on the blocked dependent Task;
- one separate unfocused Session used to claim the runnable prerequisite Task.

## Evidence

The effective probe summary records:

- `probe_execution_status=PASS`
- `probe_result=CROSS_GOAL_DEPENDENCY_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`
- `gap_kind=none`
- `commands_exit_failures=0`
- `focus_dependent_task_supported=true`
- `focused_runnable_candidates_match_expected=true`
- `focused_candidate_dependent=true`
- `focused_candidate_blocked=true`
- `focused_brief_blocked_dependency_items=1`
- `focused_full_blocked_dependency_items=1`
- `focused_brief_has_prereq_task=true`
- `focused_full_has_prereq_task=true`
- `focused_brief_has_dependency_vr_key=true`
- `focused_full_has_dependency_vr_key=true`
- `focused_brief_has_resource_basis=true`
- `focused_brief_has_refresh_hint=true`
- `focused_brief_has_baseline=true`
- `focused_blocked_brief_context_items=6`
- `focused_blocked_full_context_items=7`
- `focused_blocked_brief_omitted_items=0`
- `focused_blocked_full_omitted_items=0`
- `claim_runnable_candidates_match_expected=true`
- `claim_candidate_0_prereq=true`
- `claim_candidate_0_runnable=true`
- `claim_candidate_1_dependent=true`
- `claim_candidate_1_blocked=true`
- `claim_next_selected=true`
- `claim_next_task_matches=true`
- `claim_next_has_vr_key=true`
- `claim_next_has_resource_basis=true`
- `claim_next_has_refresh_hint=true`
- `claim_next_has_baseline=true`
- `cache_refresh_applicability=applicable`
- `cache_refresh_resource_stamps=1`
- `doctor_required_valid_exit=0`
- `repo_git_dirty_lines_after=0`
- `release_flag_positive_hits=0`
- `cross_goal_blocked_dependency_context_supported=true`
- `cross_goal_claim_next_prereq_recovery_supported=true`

Key output snippets are preserved in:

- `probe/run/focused_blocked_context_key_snippets.txt`
- `probe/run/claim_next_key_snippets.txt`
- `probe/run/focused_runnable_tasks.out`
- `probe/run/claim_runnable_tasks.out`
- `probe/run/cache_refresh_from_verification.out`
- `probe/run/doctor.out`

## Conclusion

Current main already supports the tested cross-Goal Task-dependency
Context/Resource recovery path. A continuation operator focused on the blocked
dependent Task can see the different-Goal prerequisite Task's Resource-backed
Verification Requirement, Resource basis, baseline observation, and
basis-aware refresh hint in the existing `blocked_dependency` item. An
unfocused `claim next --context-profile full` selects the runnable prerequisite
Task and preserves the same recovery path.

This removes the direct cross-Goal Task-dependency evidence gap for this
bounded shape. It does not prove cross-Goal traversal outside explicit Task
dependencies, non-Task dependency Resource traversal, broad Resource resolver
maturity, full relation-subject traversal, multi-hop/full evolution traversal,
broader causal traversal, or release-candidate readiness.

Release flags remain false:

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
