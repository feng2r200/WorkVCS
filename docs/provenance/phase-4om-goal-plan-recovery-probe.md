# Phase 4OM Goal Plan Recovery Probe Evidence

Status: current read-only probe evidence
Date: 2026-09-04

This note records the Phase 4OM public CLI probe evidence. It is a provenance
entry, not an ADR, because the probe did not change WorkVCS behavior, schema,
CLI flags, tests, or release state.

## Scope

Phase 4OM tested whether the Phase 4OK Plan `why` projection is sufficient for
a Goal-started closeout recovery workflow without adding Goal ancestor rollup.

The probe used a temporary Store and the current `main` CLI binary. It created
a Goal containing a Plan, the Plan directly containing a Task, and a
Resource-backed Verification Requirement for that Task. After Task closeout,
Plan completion, and Goal achievement, it checked whether a human operator can
start from Goal `why`, move one explicit hop to Plan `why`, recover the
Resource-backed Verification id and Resource basis, and run basis-aware cache
refresh.

The probe did not implement Goal-to-Plan-to-Task closure projection, full
relation-subject traversal, multi-hop/full evolution traversal, broader causal
traversal, broader ContextPacket or Resource resolver behavior, release
operation, Push, tag, deployment, V2, cloud/distributed collaboration, or Agent
orchestration.

## Source State

- Starting branch: `main`
- Starting commit: `c48c6d517174548f957fbed013d85d3f6a74fdd9`
- Starting ledger refresh: Phase 4OK / ADR-0495
- Starting release gate matrix refresh: Phase 4OK / ADR-0495
- Starting release flag positive hits: `0`
- Starting Work Governance state: `WORK_STATUS_READY`, `UNMANAGED_EMPTY`,
  `NO_ACTIVE_PLAN`

## Probe Evidence

Effective probe log directory:
`/tmp/workvcs-4om-goal-plan-recovery-probe-20260904T095537Z`

Key result fields:

- `phase4om_probe=PASS`
- `probe_result=RECOVERY_SUPPORTED_WITH_PLAN_HOP`
- `concrete_implementation_gap_found=false`
- `gap_kind=none`
- `commands_exit_failures=0`
- `failed_assertions=0`
- `task_why_closure_chains=1`
- `ac_why_closure_chains=1`
- `plan_why_closure_chains=1`
- `goal_why_closure_chains=0`
- `goal_primary_containment_relation_edges=1`
- `goal_why_contains_plan_id=true`
- `goal_why_contains_task_id=false`
- `goal_why_contains_verification_id=false`
- `plan_why_verification_id_matches=true`
- `plan_why_resource_id_matches=true`
- `cache_refresh_from_plan_why_id_applicability=applicable`
- `ac_status_after_plan_refresh=verified`
- `doctor_required_valid_exit=0`
- `release_flag_positive_hits=0`
- `repo_git_dirty_lines_after=0`

Independent evidence audit log:
`/tmp/workvcs-4om-goal-plan-recovery-probe-20260904T095537Z/evidence-audit`

Key audit result:

- `audit_result=PASS`

## Invalid Attempt Boundary

An earlier probe attempt at
`/tmp/workvcs-4om-goal-plan-recovery-probe-20260904T095105Z` used a stale
pre-rebuild `target/debug/workvcs` binary. It is retained as a failed attempt
record only and is not used as current behavioral evidence. The effective probe
rebuilt the CLI from commit `c48c6d517174548f957fbed013d85d3f6a74fdd9` before
creating a fresh Store.

## Interpretation

Phase 4OM found no concrete implementation gap that justifies Goal ancestor
rollup as the next minimal implementation slice. Goal `why` intentionally
continues to report `verification_closure_chains=0`, and it does not directly
expose the descendant Task or Verification id. However, Goal `why` exposes the
direct Plan through current primary containment. Querying that Plan's `why`
then exposes the direct Task's Verification closure and Resource basis, and the
operator can use the Plan-discovered Verification id with the existing
`verification cache-refresh --resource-content-from-basis` path to recover the
Acceptance Criterion to `verified`.

This supports keeping Goal ancestor rollup, broader relation traversal,
multi-hop/full evolution traversal, broader causal traversal, and broader
ContextPacket or Resource resolver behavior out of scope until a later real
dogfood continuation exposes a concrete workflow blockage.

Release state remains unchanged:

- `V0_1_DOGFOOD_COMPLETE=false`
- `V1_RELEASE_READY=false`
- `RELEASE_CANDIDATE_ALLOWED=false`
