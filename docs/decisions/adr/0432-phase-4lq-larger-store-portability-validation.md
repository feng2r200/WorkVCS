# ADR-0432: Phase 4LQ Larger Store Portability Validation

Status: Accepted
Date: 2026-09-01

## Context

After Phase 4LP, the V1 readiness ledger still had a focus risk: the project
was proving many narrow smoke expectations while still lacking representative
Store-size dogfood evidence for the local Bundle portability path. The current
V1-local Bundle profile is same-Store directory export/import/apply only, so the
next useful validation is a larger copied-target fast-forward loop inside that
profile rather than a new external Store or packaged container design.

An initial small probe exposed a concrete implementation finding: Bundle
preflight reported `same_store_fast_forward_ready`, but `apply-dir` failed when
a Verification targeted a Verification Requirement created after the target
Store baseline copy. The importer wrote Verification basis rows before writing
the missing Commit closure, so fixed-point validation could not find the
Verification's `verified_at_commit_id`.

## Decision

Add `scripts/larger-store-portability-v0.1.sh` as an opt-in validation script.
It is not part of the default smoke path.

The script builds and uses the real local CLI, creates a copied-target same
Store scenario, and validates:

- nontrivial Task population;
- Acceptance Criterion, Verification Requirement, Verification, Evidence, and
  ContentObject import;
- `depends_on` and `ordered_before` scheduling relation import;
- Checkpoint creation, Bundle directory export, validation, preflight, and
  apply;
- target Branch head state digest;
- target Task, scheduling, VR, Verification, and AC status queries;
- post-apply target-local work followed by `restore` to the imported Bundle
  head; and
- source and target integrity/doctor checks.

The default workload is intentionally bounded:

```text
WORKVCS_LARGER_STORE_TASKS=48
WORKVCS_LARGER_STORE_BASELINE_TASKS=24
WORKVCS_LARGER_STORE_VERIFICATIONS=8
WORKVCS_LARGER_STORE_RELATION_PAIRS=16
```

The workload can be increased by overriding those environment variables. Large
command output is written to a run log; success output is a compact key-value
summary.

Fix the same-Store Bundle apply ordering so Relation versions and Commit
closure are imported before Verification basis rows. This preserves the
existing schema, Bundle profile, CLI spelling, and preflight contract while
ensuring `verification_basis.verified_at_commit_id` can refer to an imported
post-baseline Commit.

## Non-Goals

- No default smoke matrix expansion.
- No performance indexes or query-plan tuning.
- No packaged archive/container format.
- No external Store canonical DAG activation.
- No remote exchange, signing, compression, streaming, or cloud sync.
- No new CLI spelling.
- No schema change.
- No release-readiness or broad scale claim.

## Evidence

The initial probe failed before the fix:

```text
tasks=6
baseline_tasks=3
verification_requirements=2
relation_pairs=2
preflight_action=same_store_fast_forward_ready
apply_error=immutable import fixed-point validation failed: Commit 01a05a1b-0bb2-7cb1-a913-aa28fdfeaf9a is missing
missing_commit_role=first post-baseline vr create commit
```

The regression test
`bundle_apply_imports_verified_at_commit_before_verification_basis` now covers
that shape and passes with the existing verification object apply test.

The default opt-in validation run passed:

```text
larger_store_result=PASS
task_count=48
baseline_tasks=24
verification_requirements=8
verification_records=8
relation_pairs=16
relation_versions=32
payload_files=267
payload_references=749
imported_commits=80
imported_entity_versions=64
imported_relation_versions=48
imported_evidences=8
imported_content_objects=9
imported_verification_bases=8
updated_branch_heads=1
elapsed_seconds=22
```

## Consequences

The local copied-target Bundle portability path now has representative
dogfood evidence beyond narrow smoke expectations. The V1 readiness ledger can
move larger Store portability from missing to dogfood-proven for this bounded
local profile.

Performance maturity remains open. This run is not a substitute for larger or
varied workload profiling, index design, another real-project dogfood run, or
release readiness.
