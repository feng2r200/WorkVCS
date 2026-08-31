# Phase 4LQ Larger Store Portability Validation Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LQ corrects the previous risk of over-focusing on narrow CLI smoke
expectations by adding and running an opt-in, dogfood-scale local portability
validation. The validation stays inside the current V1-local Bundle profile:
same Store identity, copied target Store, directory Bundle export/import/apply,
and local restore.

This evidence does not claim performance maturity, release readiness, external
Store DAG activation, packaged Bundle containers, remote exchange, or cloud
sync.

## Script

The reusable script is:

```text
scripts/larger-store-portability-v0.1.sh
```

It is not called by the default smoke script. It writes full command output to
a temporary run log and emits only a compact key-value summary on success.

Default workload:

```text
WORKVCS_LARGER_STORE_TASKS=48
WORKVCS_LARGER_STORE_BASELINE_TASKS=24
WORKVCS_LARGER_STORE_VERIFICATIONS=8
WORKVCS_LARGER_STORE_RELATION_PAIRS=16
```

## Implementation Finding

A small probe first found a real bundle apply ordering blocker:

```text
probe_tasks=6
probe_baseline_tasks=3
probe_verifications=2
probe_relation_pairs=2
preflight_action=same_store_fast_forward_ready
apply_error=immutable import fixed-point validation failed: Commit 01a05a1b-0bb2-7cb1-a913-aa28fdfeaf9a is missing
```

The missing Commit was the first post-baseline `vr create` commit. The failure
was caused by `apply_bundle_import` importing Verification basis rows before
the missing Commit closure, while fixed-point validation requires
`verification_basis.verified_at_commit_id` to already exist.

The fix preserves the schema and CLI contract: Relation versions and Commit
closure are imported before Verification basis rows.

Regression coverage:

```text
cargo test -p workvcs-core --test bundle_verification_object_apply_phase4az
running 2 tests
test bundle_apply_imports_verified_at_commit_before_verification_basis ... ok
test bundle_apply_restores_verification_evidence_resource_and_basis ... ok
```

## Default Run

Command:

```bash
WORKVCS_LARGER_STORE_OUTPUT_ROOT=.work-governance/runtime/logs/phase-4lq \
  ./scripts/larger-store-portability-v0.1.sh
```

Result:

```text
larger_store_result=PASS
store_id=01a05a26-e977-7f02-badf-c73e0bcc8a5e
workspace_id=01a05a26-e9b0-7140-b908-f49026c4ae66
branch_id=01a05a26-e9b0-7140-b908-f4c7d6e8a92a
task_count=48
baseline_tasks=24
verification_requirements=8
verification_records=8
relation_pairs=16
relation_versions=32
source_head_commit_id=01a05a27-243d-7bf3-9155-e3ce5d050643
source_state_digest=a00659196d5fce030c0bf09c1654d323afbb0876ee06b3bb05fa2b222870de13
checkpoint_id=01a05a27-247a-7233-af5b-8e2d3cebd089
bundle_import_id=01a05a27-26fd-7db2-bdeb-fc5931ad96e2
bundle_digest=52101b8bec385abe58b6cdcfa8617a70f8bae11f1087b2938cebe7775cfaa8f5
payload_files=267
payload_references=749
imported_commits=80
imported_entity_versions=64
imported_relation_versions=48
imported_evidences=8
imported_content_objects=9
imported_verification_bases=8
updated_branch_heads=1
restore_commit_id=01a05a27-2f57-7da0-b276-aef755785bc4
elapsed_seconds=22
output_dir=.work-governance/runtime/logs/phase-4lq/larger-store-portability.0IQDKv
log_file=.work-governance/runtime/logs/phase-4lq/larger-store-portability.0IQDKv/run.log
```

Validated behaviors:

- copied-target preflight returned `same_store_fast_forward_ready`;
- apply imported missing commits, entity versions, relation versions,
  evidences, content objects, Verification bases, Checkpoint, and Checkpoint
  status;
- target Branch head matched the source state digest;
- target Task list, scheduling list, VR list, Verification list, and AC status
  checks passed;
- target-local post-apply work was restored to the imported Bundle head; and
- source integrity, target integrity, and target doctor checks passed.

## Remaining Gaps

This run proves a bounded local profile workload, not general scale maturity.
Before claiming performance readiness, run larger or more varied Stores and
collect query/runtime evidence that justifies any index or storage tuning.
