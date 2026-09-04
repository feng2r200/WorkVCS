# Phase 4OH Recovery From Why Probe Evidence

Status: current read-only probe evidence
Date: 2026-09-04

This note records the Phase 4OH public CLI probe evidence. It is a provenance
entry, not an ADR, because the probe did not change WorkVCS behavior, schema,
CLI flags, tests, or release state.

## Scope

Phase 4OH tested whether the Resource basis fields added to Task and
Acceptance Criterion `why` closure chains in Phase 4OG are sufficient for a
human operator to recover a stale Resource-backed closeout without a separate
`verification show` lookup.

The probe used a temporary Store and left the repository unchanged. It did not
broaden ContextPacket behavior, Resource resolver behavior, relation-subject
traversal, evolution traversal, causal traversal, release operation, or V2
scope.

## Source State

- Starting branch: `main`
- Starting commit: `7845b16dbef5abcbe6914cb8c5f7d82552b0ed56`
- Starting ledger refresh: ADR-0494 / Phase 4OG
- Starting release gate matrix refresh: ADR-0494 / Phase 4OG
- Starting release flag positive hits: `0`

## Probe Evidence

Probe log directory:
`/tmp/workvcs-4oh-recovery-from-why-probe-20260904T011704Z`

Key result fields:

- `probe_execution_status=PASS`
- `probe_result=RECOVERY_SUPPORTED`
- `commands_exit_failures=0`
- `failed_assertions=0`
- `critical_failure=none`
- `task_show_done=true`
- `ac_status_before_closeout=verified`
- `ac_status_after_closeout=stale`
- `task_why_verification_id_present=true`
- `ac_why_verification_id_present=true`
- `task_ac_why_same_verification_id=true`
- `task_why_resource_basis=true`
- `ac_why_resource_basis=true`
- `task_why_basis_matches_verification=true`
- `ac_why_basis_matches_verification=true`
- `cache_refresh_from_why_id=true`
- `post_refresh_ac_status=verified`
- `post_refresh_ac_status_verified=true`
- `release_flag_positive_hits=0`

## Interpretation

The probe found no new implementation gap in the Phase 4OG Resource-backed
Task/Acceptance Criterion `why` closure path. The Task and Acceptance
Criterion `why` output exposed the same current Verification id and Resource
basis needed for a basis-aware cache refresh. After Task closeout advanced the
head and made the Acceptance Criterion stale, the operator could run the
recovery command using the `why`-discovered Verification id and restore the
Acceptance Criterion to `verified`.

This supports the existing Phase 4OG implementation evidence for the specific
Resource-backed Task/Acceptance Criterion closeout recovery path. It does not
change the remaining release gates: broader context/Resource resolver
maturity, full relation-subject traversal beyond the already proven direct
endpoint and Task/Acceptance Criterion closure slices, multi-hop/full
evolution traversal, broader causal traversal, and release-candidate operation
remain open.
