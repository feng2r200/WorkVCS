# Phase 4NV Why Verifies Relation Evolution Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4NV advances the Context resolver, packets, and `why` explanations
release gate by making the defining `verifies` Relation creation visible as
direct endpoint evolution from `workvcs why --entity
<Verification/AcceptanceCriterion/VerificationRequirement endpoint>`.

The slice is intentionally narrow. It adds direct `verification.record`
endpoint evolution only for the defining `verifies` Relation when the queried
Entity is the source Verification or the target Acceptance Criterion or
Verification Requirement endpoint. It does not add `why --relation`, change
Store schema, alter Verification/AC/VR/Evidence semantics, add `evidenced_by`
endpoint evolution, change ContextPacket behavior, release, Push, tag,
deployment, remote/cloud/V2 scope, GUI/TUI behavior, distributed collaboration,
or Agent orchestration.

## Gap Proof

The pre-change gap used a temporary local Store and public CLI commands.

Scenario:

```text
create Workspace
create Task
create Acceptance Criterion
create Verification Requirement
record Verification targeting the Verification Requirement
run workvcs why --entity <VerificationRequirement> --relation-kind verifies
```

Observed before implementation:

```text
log_dir=/tmp/workvcs-4nv-plan-focus-resource-probe-20260903T011459Z/verifies-endpoint-probe
branch_id=01a064d5-5fd4-7b81-accc-3331a4b82c58
head_commit_id=01a064d5-6098-7510-8767-5fb26e93d2b3
verification_requirement_id=01a064d5-6060-7fe3-b238-06e2807032d9
verification_id=01a064d5-6098-7510-8767-5f6387f6d6f0
verifies_relation_id=01a064d5-6098-7510-8767-5f8a6542308b
verifies_relation_version_id=01a064d5-6098-7510-8767-5f92838abe5a
vr_relation_edges=1
vr_has_verifies_edge=true
vr_has_verifies_relation_id=true
vr_evolution_change_operations=0
vr_has_evolution_relation_subject=false
vr_has_evolution_subject_relation_kind_verifies=false
expected_relation_status=0
expected_evolution_status=1
failure=error_code=query_invalid; message=query invalid: why evolution change operations 0 does not match expected 1
```

The current relation edge was visible, but the direct `verification.record`
operation that created the defining `verifies` relation was not visible through
`why` from the endpoint.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/task.rs
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_verification_neighborhood_phase3u.rs
crates/workvcs-core/tests/verification_evidence_closure_phase3v.rs
crates/workvcs-core/tests/why_evidence_neighborhood_phase3w.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0487-phase-4nv-why-verifies-relation-evolution.md
docs/provenance/phase-4nv-why-verifies-relation-evolution.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

The implementation includes `verification.record` in the existing direct
relation evolution operation pass. For queried Verification, Acceptance
Criterion, or Verification Requirement endpoints, operation subject detail is
resolved from current `verification_relations_at` replay at the queried commit
and records relation kind, relation version, source endpoint, target endpoint,
and state digest.

No CLI flag or output contract was added. The existing `why` fields,
`--relation-kind verifies`, and `--expected-evolution-change-operations`
assertion support are reused.

## Focused Validation

Focused tests passed after implementation and formatting:

```text
log_dir=/tmp/workvcs-4nv-why-verifies-relation-evolution-20260903T012528Z/focused

cargo test -p workvcs-core --test why_verification_neighborhood_phase3u
status=0

cargo test -p workvcs-cli cli_why_projects_verifies_create_as_endpoint_evolution
status=0

cargo test -p workvcs-core --test verification_evidence_closure_phase3v verification_creation_records_evidenced_by_closure_atomically
status=0

cargo test -p workvcs-core --test why_evidence_neighborhood_phase3w why_verification_subject_reports_evidenced_by_edges_to_evidence_endpoints
status=0
```

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4nv-why-verifies-relation-evolution-20260903T012528Z/final-validation
cargo_fmt_check_status=0
core_why_verification_status=0
core_verification_evidence_closure_status=0
core_why_evidence_neighborhood_status=0
core_why_task_scheduling_status=0
core_primary_containment_evolution_status=0
cli_why_verifies_status=0
cli_primary_containment_evolution_status=0
cli_task_scheduling_evolution_status=0
cargo_clippy_status=0
cargo_test_all_status=0
cargo_build_workvcs_cli_status=0
schema_validate_status=0
release_flag_scan_status=0
validation_status=PASS
```

## Dogfood Proof

The public CLI dogfood used a temporary local Store and the built
`target/debug/workvcs` binary.

```text
log_dir=/tmp/workvcs-4nv-why-verifies-relation-evolution-20260903T012528Z/dogfood-v3
phase4nv_dogfood=PASS
workspace_id=01a064e2-4922-7530-9ad3-2ede49073115
branch_id=01a064e2-4922-7530-9ad3-2f011f81cfd9
head_commit_id=01a064e2-497d-7f61-a611-197745e65a09
task_id=01a064e2-4939-78c0-a0c0-23f6d4616af6
criterion_id=01a064e2-494f-7943-ac6b-ec2140172129
verification_requirement_id=01a064e2-4967-7851-9101-a6e16e457bf2
verification_id=01a064e2-497d-7f61-a611-192cf7ab29a3
verifies_relation_id=01a064e2-497d-7f61-a611-19499971508d
verifies_relation_version_id=01a064e2-497d-7f61-a611-1954cdbff40e
vr_relation_edges=1
vr_relation_kind=verifies
vr_evolution_change_operations=1
vr_evolution_match_expected=true
vr_evolution_operation_type=verification.record
vr_evolution_subject_family=relation
vr_evolution_subject_detail_kind=relation
vr_evolution_subject_relation_kind=verifies
vr_evolution_source_matches_verification=true
vr_evolution_target_matches_requirement=true
vr_deferred_relation_families=1
vr_deferred_relation_family_0=evolution
```

The first dogfood wrapper attempt failed before proving product behavior because
its local key-value extraction returned an empty Branch id. The second attempt
proved the product path but failed after completion while printing the summary.
The successful third attempt above is the accepted dogfood evidence.

## Interpretation

Phase 4NV closes a concrete `verifies` `why` endpoint evolution gap: operators
can now see the direct `verification.record` operation behind Verification
Requirement-targeted Verification creation through the same `why --entity`
surface and assertion flags used for other endpoint explanations.

The proof is still bounded. It does not show `evidenced_by` endpoint evolution,
full relation-subject traversal, multi-hop/full evolution traversal, broader
causal traversal, broader context/Resource resolver maturity,
release-candidate validation, or release authorization.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NV advances the gate by closing direct `verifies` creation endpoint
evolution for queried Verification, Acceptance Criterion, and Verification
Requirement endpoints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
