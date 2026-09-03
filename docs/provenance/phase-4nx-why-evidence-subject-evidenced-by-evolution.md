# Phase 4NX Why Evidence Subject Evidenced By Evolution Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4NX advances the Context resolver, packets, and `why` explanations
release gate by making direct `evidenced_by` Relation creation visible as
endpoint evolution from `workvcs why --evidence <Evidence> --relation-kind
evidenced_by`.

The slice is intentionally narrow. It adds direct `verification.record`
endpoint evolution only for current incoming `evidenced_by` Relations when the
queried subject is the target Evidence endpoint. It does not add `why
--relation`, full relation-subject traversal, Store schema changes, CLI flags,
ContextPacket behavior, release, Push, tag, deployment, remote/cloud/V2 scope,
GUI/TUI behavior, distributed collaboration, or Agent orchestration.

## Gap Proof

The pre-change gap used current `main`, a temporary local Store, and public CLI
commands.

Scenario:

```text
create Workspace
create Task
create Acceptance Criterion
create Evidence
record two Verifications targeting the Acceptance Criterion with the same Evidence
run workvcs why --evidence <Evidence> --relation-kind evidenced_by
```

Observed before implementation:

```text
log_dir=/tmp/workvcs-4nx-evidence-subject-evidenced-by-probe-20260903T051248Z
head_commit_id=c7917482d5fde79aa5c7cb085c86054a453b0a94
evidence_relation_edges=2
evidence_relation_0_kind=evidenced_by
evidence_relation_0_direction=incoming
evidence_relation_1_kind=evidenced_by
evidence_relation_1_direction=incoming
evidence_evolution_change_operations=0
evidence_evidenced_by_subject_detail_count=0
why_evidence_expected_evolution_exit_code=1
evidence_expected_evolution_error_code=query_invalid
evidence_expected_evolution_message=query invalid: why evolution change operations 0 does not match expected 2
probe_status=PASS
```

The current incoming `evidenced_by` relation edges were visible, but the direct
`verification.record` operations that created them were not visible through
`why` from the Evidence endpoint.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_evidence_neighborhood_phase3w.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0489-phase-4nx-why-evidence-subject-evidenced-by-evolution.md
docs/provenance/phase-4nx-why-evidence-subject-evidenced-by-evolution.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

The implementation lets Evidence subjects enter the existing direct relation
evolution scan after causal-anchor processing. It keeps Entity-only direct
Entity membership evolution unchanged and keeps Knowledge Exposure subjects
outside this direct relation pass. Existing relation subject-detail resolution
already knew how to match `evidenced_by` Relation endpoints against an Evidence
subject and render source Verification, target Evidence, relation version, and
state digest.

No Store schema, CLI flag, public output field, ContextPacket behavior,
Verification semantics, or Evidence semantics changed.

## Focused Validation

Focused tests passed after implementation:

```text
log_dir=/tmp/workvcs-4nx-implementation-focused-20260903T052730Z
cargo_fmt_status=0
core_why_evidence_status=0
core_verification_evidence_status=0
cli_evidence_subject_status=0
cli_verification_subject_status=0
focused_status=PASS
```

The first focused run failed before behavior validation because the core test
used a nonexistent helper field and the CLI test assumed cross-commit operation
ordering. A second focused run failed because the test tried to sort
`ChangeOperationSubject`, which has no `Ord` implementation. Both were test
harness issues; the final focused validation above passed after changing the
assertions to match relation/source sets rather than fixed ordering.

## Dogfood Proof

The public CLI dogfood uses a temporary local Store and the built
`target/debug/workvcs` binary.

```text
log_dir=/tmp/workvcs-4nx-why-evidence-subject-evidenced-by-evolution-20260903T052944Z/dogfood
phase4nx_dogfood=PASS
workspace_id=01a065be-c2c2-75a1-a1f7-322f355a555d
branch_id=01a065be-c2c2-75a1-a1f7-32528c5df2b4
task_id=01a065be-c2db-76e0-9df8-bd6825d0e798
criterion_id=01a065be-c2f3-76b3-8755-46e9ddd8c759
evidence_id=01a065be-c307-7c92-b762-8619a90a3539
first_verification_id=01a065be-c31a-7282-876f-ab22439a4df8
second_verification_id=01a065be-c332-7442-b349-6f6913ff4aa7
evidence_relation_edges=2
evidence_relation_0_kind=evidenced_by
evidence_relation_0_direction=incoming
evidence_relation_1_kind=evidenced_by
evidence_relation_1_direction=incoming
evidence_evolution_change_operations=2
evidence_evolution_match_expected=true
evidence_evolution_subject_relation_kind_count=2
evidence_evolution_target_evidence_id_count=2
evidence_evolution_sources_match=true
```

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4nx-why-evidence-subject-evidenced-by-evolution-20260903T054617Z/final-validation
git_status_status=0
git_diff_check_status=0
cargo_fmt_check_status=0
core_why_verification_status=0
core_verification_evidence_closure_status=0
core_why_evidence_neighborhood_status=0
core_why_task_scheduling_status=0
core_primary_containment_evolution_status=0
cli_why_evidence_subject_status=0
cli_why_evidenced_by_status=0
cli_why_verifies_status=0
cli_primary_containment_evolution_status=0
cli_task_scheduling_evolution_status=0
cargo_clippy_status=0
cargo_test_all_status=0
cargo_build_workvcs_cli_status=0
schema_validate_status=0
release_flag_scan_status=0
validation_status=PASS
status_0_count=18
```

## Interpretation

Phase 4NX closes the concrete `evidenced_by` Evidence endpoint evolution gap
shown by the current CLI probe. Operators can now see the direct
`verification.record` operations behind reused Evidence from the same `why
--evidence` surface that already showed incoming `evidenced_by` relation edges.

The proof is still bounded. It does not show `why --relation`, full
relation-subject traversal, multi-hop/full evolution traversal, broader causal
traversal, broader context/Resource resolver maturity, release-candidate
validation, or release authorization.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NX advances the gate by closing direct `evidenced_by` creation endpoint
evolution for queried Evidence endpoints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
