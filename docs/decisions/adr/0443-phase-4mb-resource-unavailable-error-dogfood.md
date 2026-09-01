# ADR-0443: Phase 4MB Resource Unavailable Error Dogfood

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger still listed Resource adapter contracts, glob
semantics, unavailable/error observation states, and automatic re-observation as
Open. Phase 4LZ had already made WorkVCS business errors machine-readable, and
Phase 4MA showed that dogfood should precede more CLI output expansion.

The next narrow Resource question was whether the already implemented
unavailable/error applicability stamp states are usable in a realistic recovery
loop. If they already work, adding more fields or adapter abstractions would be
premature.

## Decision

Accept Phase 4MB as dogfood-only evidence for explicit unavailable/error
Resource applicability stamps.

The dogfood records a baseline Resource observation from a read-only real local
file:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
```

It then records two applicability cache states for the Resource-backed
Verification:

```text
observation_status=unavailable -> applicability=unknown, reason_code=resource_unavailable
observation_status=error -> applicability=unknown, reason_code=resource_error
```

Both cache states leave the Acceptance Criterion effective status as `stale`.
Both cache details expose the Resource stamp with no observed fingerprint and
no observation id. An invalid unavailable stamp containing an observed
fingerprint fails with stable WorkVCS error metadata:

```text
error_code=task_invalid
error_category=task
retryable=false
message=task invalid: unavailable resource stamp must not include observed data
```

No code change is made in this slice.

## Non-Goals

- No Resource adapter contract implementation.
- No glob expansion semantics.
- No automatic adapter-backed re-observation.
- No new CLI output field.
- No release maturity claim for Resource workflows.
- No target-project mutation.

## Evidence

The passing dogfood run:

```text
phase4mb_dogfood_result=PASS
log_dir=/tmp/workvcs-4mb-resource-unavailable-error-dogfood-20260901T040501Z
store=/tmp/workvcs-4mb-resource-unavailable-error-dogfood-20260901T040501Z/store.sqlite
store_id=01a05b24-587f-7c12-b178-3c85e7722009
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_file=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
workspace_id=01a05b24-58b6-7720-aaed-b27d722f7498
branch_id=01a05b24-58b6-7720-aaed-b2a3d72dfce5
head_commit_id=01a05b24-597e-7b52-acb3-417775a228b5
goal_id=01a05b24-58cb-7270-96b9-a210a2465623
plan_id=01a05b24-58df-7a71-ac17-74a3dbcc6d76
task_id=01a05b24-58f3-76a1-ac5a-acb03cca1aea
criterion_id=01a05b24-591d-7e40-814d-f0dce621958a
verification_id=01a05b24-597e-7b52-acb3-4121812f3a53
evidence_id=01a05b24-5944-7660-b214-17600fe17672
resource_id=01a05b24-5956-7de1-b0e5-5b4d6449c0af
observation_id=01a05b24-5968-7982-800c-e39851531d71
baseline_fingerprint=a0ae4eccafb9eca86e82a5b6d9e6d23ba8891cdc6b0db3c01f33a8a9ccd3c4e1
pre_cache_ac_status=stale
unavailable_applicability=unknown
unavailable_reason_code=resource_unavailable
unavailable_ac_status=stale
error_applicability=unknown
error_reason_code=resource_error
error_ac_status=stale
invalid_unavailable_with_fingerprint_status=1
invalid_error_code=task_invalid
target_status_unchanged=true
```

The target project `git status --short` output was identical before and after
the dogfood run.

## Consequences

Explicit unavailable/error Resource stamp behavior is now dogfood-proven for a
real local file baseline. The V1 Resource row should no longer treat
unavailable/error observation states as an unproven Open item.

Resource adapter contracts, glob semantics, automatic re-observation, and
broader Resource resolver policy remain Open. Future Resource work should start
from one of those gaps, not from more stamp display fields.
