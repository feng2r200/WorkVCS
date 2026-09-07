# Phase 4OP Plan-start Context Resource Probe Evidence

Status: current local evidence
Date: 2026-09-07

## Scope

Phase 4OP is a read-only current-main probe for the Plan-start
Context/Resource recovery boundary left open after the current-task,
focused blocked-dependency, focused same-Plan peer, and focused same-Goal
cross-Plan peer Resource-backed Verification Requirement context slices.

The probe checks whether an operator who starts from a focused Plan can see a
directly contained runnable Task's Resource-backed Verification Requirement
recovery route in ContextPacket output without first focusing the Task.

The slice is intentionally narrow. It does not change WorkVCS behavior, Store
schema, ContextPacket JSON fields, context packet snapshot schema, CLI flags,
`verify` semantics, Resource observation or cache-refresh semantics, runnable
selection, Claim behavior, `claim next`, `next`, `why` traversal, release
state, Push state, tags, remote state, deployment state, V2 scope, GUI/TUI
behavior, distributed collaboration, or Agent orchestration.

## Current State Basis

The short status check before the probe passed:

```text
log_dir=/tmp/workvcs-v1-short-status-20260907T034621Z
git_branch=main
git_head=4cff7acd4b0e5e43c720d9bfbc2b10f9bd63ac61
git_main_ref=4cff7acd4b0e5e43c720d9bfbc2b10f9bd63ac61
git_dirty_lines=0
workctl_status=WORK_STATUS_READY
workctl_layout_state=LAYOUT_READY
workctl_authority_state=UNMANAGED_EMPTY
workctl_contract_state=NO_ACTIVE_PLAN
release_flag_positive_hits=0
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
log_dir=/tmp/workvcs-4op-plan-start-context-resource-probe-20260907T035327Z
previous_failed_harness_log_dir=/tmp/workvcs-4op-plan-start-context-resource-probe-20260907T035159Z
```

The previous failed harness attempt stopped before a WorkVCS judgment because
the shell script used a read-only shell variable name. It is retained only as
a harness failure and is not used as behavioral evidence.

The effective probe created:

```text
one Goal
one direct child Plan
one runnable Task directly contained by that Plan
one Acceptance Criterion on that Task
one Verification Requirement on that Acceptance Criterion
one local-file Resource
one baseline ResourceObservation
one Resource-backed Verification targeting that Verification Requirement
one active Session
```

The probe used a focused Task as the control case, then focused the same
Session on the Plan and inspected `context --profile brief`, `normal`, and
`full`, plus focused `claim next --context-profile full`.

## Evidence

The effective probe passed:

```text
probe_execution_status=PASS
probe_result=PLAN_START_DIRECT_CONTEXT_RESOURCE_RECOVERY_SUPPORTED
gap_kind=none
commands_exit_failures=0
critical_failure=none
git_dirty_lines_before=0
repo_git_dirty_lines_after=0
release_flag_positive_hits=0
doctor_required_valid_exit=0
```

The Task-focus control proved the Resource-backed Verification Requirement
recovery summary was present:

```text
task_focus_brief_has_vr_key=true
task_focus_brief_has_resource_basis=true
task_focus_brief_has_refresh_hint=true
task_focus_context_items=7
task_focus_context_omitted_items=0
```

The Plan-focus boundary was already supported by current main:

```text
focus_plan_exit=0
focus_plan_supported=true
plan_focus_brief_has_vr_key=true
plan_focus_normal_has_vr_key=true
plan_focus_full_has_vr_key=true
plan_focus_normal_has_resource_basis=true
plan_focus_full_has_resource_basis=true
plan_focus_normal_has_refresh_hint=true
plan_focus_full_has_refresh_hint=true
plan_focus_normal_context_items=8
plan_focus_full_context_items=8
plan_focus_normal_omitted_items=0
plan_focus_full_omitted_items=0
plan_start_direct_context_supported=true
```

The focused `claim next` path was also supported:

```text
plan_focus_claim_next_full_exit=0
plan_focus_claim_next_selected=true
plan_focus_claim_next_task=01a079ff-e1c0-7283-8076-7edea3ddf636
plan_focus_claim_next_has_vr_key=true
plan_focus_claim_next_has_resource_basis=true
plan_focus_claim_next_has_refresh_hint=true
plan_start_claim_next_recovery_supported=true
```

The same Verification id exposed by the context summary remained usable with
the existing basis-aware cache-refresh path:

```text
verification_id=01a079ff-e253-7962-a4f3-2dc204f5acaf
cache_refresh_from_verification_exit=0
cache_refresh_applicability=applicable
cache_refresh_resource_stamps=1
```

## Interpretation

Phase 4OP finds no concrete implementation gap for the Plan-start
Context/Resource recovery boundary. Current main already lets a Plan-focused
operator see a directly contained runnable Task's Resource-backed
Verification Requirement, Resource basis, baseline observation, and
basis-aware refresh hint in normal and full context. Brief context also
contains the same direct Plan-start summary in this one-Task probe.

This supports keeping a broader Resource resolver or context traversal change
out of scope until a future real workflow exposes a concrete blockage.

The proof remains bounded. It does not show Goal-start Context/Resource
recovery, nested Plan traversal, cross-Goal traversal, non-Task dependency
Resource traversal, broad Resource resolver maturity, `why` traversal beyond
the existing direct Task/Acceptance Criterion/direct-Plan closure projection,
full relation-subject traversal, multi-hop/full evolution traversal, broader
causal traversal, or release-candidate readiness.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OP adds current-main read-only evidence that the Plan-start
Context/Resource recovery boundary is already covered for a directly contained
runnable Task with a Resource-backed Verification Requirement.

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
