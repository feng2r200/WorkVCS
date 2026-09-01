# Phase 4MB Resource Unavailable Error Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MB is a dogfood-only slice. It validates existing Resource applicability
unavailable/error stamp handling against a real local file baseline and
intentionally avoids changing code or adding output fields.

The read-only external file used as the Resource baseline was:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
```

## Dogfood Evidence

The passing run:

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

The probes proved:

```text
missing cache before re-observation: ac status stale
unavailable stamp: applicability unknown, reason_code resource_unavailable
unavailable stamp detail: observation_status unavailable, observed_fingerprint none, observation_id none
unavailable AC projection: status stale
error stamp: applicability unknown, reason_code resource_error
error stamp detail: observation_status error, observed_fingerprint none, observation_id none
error AC projection: status stale
invalid unavailable stamp with observed_fingerprint: stable task_invalid error output
```

The target project status snapshot was identical before and after the run.

## Process Finding

The first temporary wrapper attempt failed before product behavior was
exercised:

```text
log_dir=/tmp/workvcs-4mb-resource-unavailable-error-dogfood-20260901T040319Z
missing key store_id in output:
initialized store_id=01a05b23-0330-7f13-ad7f-9026a0eac2d1 schema_version=1
```

The wrapper incorrectly treated the `init` banner as pure line-oriented
key-value output. The corrected run reads canonical Store fields from
`store info`. No repository state changed from the failed wrapper attempt.

## Validation

This slice made no code changes. Docs-only validation passed:

```text
git diff --check
cargo fmt --all -- --check
```

Initial independent review found one high closeout-wording issue in the Plan
and one medium operator-guide open-gap wording drift. Both findings were
corrected before commit. Final review status is reported by the live closeout
evidence rather than pre-written here.

## Boundary

This evidence supports explicit unavailable/error Resource applicability stamp
behavior and AC stale projection. It does not claim release maturity for
Resource adapter contracts, glob semantics, automatic adapter-backed
re-observation, or broader Resource resolver policy.
