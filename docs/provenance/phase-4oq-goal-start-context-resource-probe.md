# Phase 4OQ Goal-start Context Resource Probe Evidence

Status: current local evidence
Date: 2026-09-07

## Scope

Phase 4OQ is a read-only current-main probe for the Goal-start
Context/Resource recovery boundary after current-task, focused
blocked-dependency, focused same-Plan peer, focused same-Goal cross-Plan peer,
and Plan-start Resource-backed Verification Requirement context evidence.

The probe checks whether an operator who starts from a focused Goal can see a
descendant runnable Task's Resource-backed Verification Requirement recovery
route in ContextPacket output without first focusing the Plan or Task.

The slice is intentionally narrow. It does not change WorkVCS behavior, Store
schema, ContextPacket JSON fields, context packet snapshot schema, CLI flags,
`verify` semantics, Resource observation or cache-refresh semantics, runnable
selection, Claim behavior, `claim next`, `next`, `why` traversal, release
state, Push state, tags, remote state, deployment state, V2 scope, GUI/TUI
behavior, distributed collaboration, or Agent orchestration.

## Current State Basis

The current-main basis before the probe was:

```text
git_branch=main
git_head=86524a204fc5239277086455fd73c7fa85e5efef
git_dirty_lines_before=0
```

The current V1 release gate matrix still recorded:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```

## Probe

The effective public CLI probe used the built
`target/debug/workvcs` binary, a temporary Store, and a temporary local file
Resource under:

```text
log_dir=/tmp/workvcs-4oq-goal-start-context-resource-probe-20260907T040620Z
```

The effective probe created:

```text
one Goal
one direct child Plan under that Goal
one runnable Task directly contained by that Plan
one Acceptance Criterion on that Task
one Verification Requirement on that Acceptance Criterion
one local-file Resource
one baseline ResourceObservation
one Resource-backed Verification targeting that Verification Requirement
one active Session
```

The probe used focused Task and focused Plan checks as controls, then focused
the same Session on the Goal and inspected `context --profile brief`, `normal`,
and `full`, plus focused `claim next --context-profile full`.

## Evidence

The effective probe passed:

```text
probe_execution_status=PASS
probe_result=GOAL_START_DIRECT_CONTEXT_RESOURCE_RECOVERY_SUPPORTED
gap_kind=none
commands_exit_failures=0
critical_failure=none
repo_git_dirty_lines_after=0
release_flag_positive_hits=0
doctor_required_valid_exit=0
```

The focused Task and Plan controls proved the Resource-backed Verification
Requirement recovery summary was present before judging the Goal boundary:

```text
task_focus_brief_has_vr_key=true
task_focus_brief_has_resource_basis=true
task_focus_brief_has_refresh_hint=true
plan_focus_full_has_vr_key=true
plan_focus_full_has_resource_basis=true
plan_focus_full_has_refresh_hint=true
plan_focus_full_context_items=8
plan_focus_full_omitted_items=0
```

The Goal-focus boundary was already supported by current main:

```text
focus_goal_exit=0
focus_goal_supported=true
goal_focus_brief_has_vr_key=true
goal_focus_normal_has_vr_key=true
goal_focus_full_has_vr_key=true
goal_focus_normal_has_resource_basis=true
goal_focus_full_has_resource_basis=true
goal_focus_normal_has_refresh_hint=true
goal_focus_full_has_refresh_hint=true
goal_focus_normal_context_items=8
goal_focus_full_context_items=8
goal_focus_normal_omitted_items=0
goal_focus_full_omitted_items=0
goal_start_direct_context_supported=true
```

The focused `claim next` path was also supported:

```text
goal_focus_claim_next_full_exit=0
goal_focus_claim_next_selected=true
goal_focus_claim_next_task=01a07a0b-ade3-7d81-86b0-d52f418faa4d
goal_focus_claim_next_has_vr_key=true
goal_focus_claim_next_has_resource_basis=true
goal_focus_claim_next_has_refresh_hint=true
goal_start_claim_next_recovery_supported=true
```

The same Verification id exposed by the context summary remained usable with
the existing basis-aware cache-refresh path:

```text
verification_id=01a07a0b-ae77-7d70-8ad7-dd28737ab6a1
cache_refresh_from_verification_exit=0
cache_refresh_applicability=applicable
cache_refresh_resource_stamps=1
```

## Interpretation

Phase 4OQ finds no concrete implementation gap for the Goal-start
Context/Resource recovery boundary in the tested direct Goal -> Plan -> Task
shape. Current main already lets a Goal-focused operator see the descendant
runnable Task's Resource-backed Verification Requirement, Resource basis,
baseline observation, and basis-aware refresh hint in normal and full context.
Brief context also contains the same direct Goal-start summary in this
one-Plan, one-Task probe.

This supports keeping a broader Resource resolver or context traversal change
out of scope until a future real workflow exposes a concrete blockage.

The proof remains bounded. It does not show nested Plan traversal, cross-Goal
traversal, multiple-Plan ranking, non-Task dependency Resource traversal,
broad Resource resolver maturity, `why` traversal beyond the existing direct
Task/Acceptance Criterion/direct-Plan closure projection, full
relation-subject traversal, multi-hop/full evolution traversal, broader
causal traversal, or release-candidate readiness.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OQ adds current-main read-only evidence that the Goal-start
Context/Resource recovery boundary is already covered for a direct
Goal-to-Plan-to-Task path with a Resource-backed Verification Requirement.

The Candidate release operation gate remains `Blocked` until a named
candidate commit has a fresh full validation matrix, a clean Git state, a
closed governance Plan where applicable, a refreshed release gate matrix, and
explicit release authorization.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
