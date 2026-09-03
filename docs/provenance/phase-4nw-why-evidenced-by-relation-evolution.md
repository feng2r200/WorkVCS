# Phase 4NW Why Evidenced By Relation Evolution Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4NW advances the Context resolver, packets, and `why` explanations
release gate by making direct `evidenced_by` Relation creation visible as
endpoint evolution from `workvcs why --entity <Verification> --relation-kind
evidenced_by`.

The slice is intentionally narrow. It adds direct `verification.record`
endpoint evolution only for current `evidenced_by` Relations when the queried
Entity is the source Verification. It does not add `why --relation`, Evidence
subject evolution traversal, Store schema changes, CLI flags, ContextPacket
behavior, release, Push, tag, deployment, remote/cloud/V2 scope, GUI/TUI
behavior, distributed collaboration, or Agent orchestration.

## Gap Proof

The pre-change gap used current `main`, a temporary local Store, and public CLI
commands.

Scenario:

```text
create Workspace
create Task
create Acceptance Criterion
create Evidence
record Verification targeting the Acceptance Criterion with that Evidence
run workvcs why --entity <Verification> --relation-kind evidenced_by
```

Observed before implementation:

```text
log_dir=/tmp/workvcs-4nw-evidenced-by-endpoint-probe-20260903T021310Z
head_commit_id=1bcc48ec9d729c642232d7b6d552dce2d9a6f87a
branch_id=01a0650a-a23e-7772-891d-86a5efa0170d
verification_id=01a0650a-a293-7cb1-9ccc-21252d4d8226
evidence_id=01a0650a-a280-7922-b2df-75e698f2c74f
evidenced_by_relation_id=01a0650a-a293-7cb1-9ccc-21a8b4198af0
evidenced_by_relation_version_id=01a0650a-a293-7cb1-9ccc-21b8b774d39d
verification_relation_edges=1
verification_has_evidenced_by_edge=true
verification_evolution_change_operations=1
verification_has_evolution_subject_relation_kind_verifies=true
verification_has_evolution_subject_relation_kind_evidenced_by=false
why_verification_expected_two_evolution_status=1
gap_observed=true
phase4nw_probe=PASS
failure=error_code=query_invalid; message=query invalid: why evolution change operations 1 does not match expected 2
```

The current `evidenced_by` relation edge was visible, but the direct
`verification.record` operation that created it was not visible through `why`
from the Verification endpoint.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/verification_evidence_closure_phase3v.rs
crates/workvcs-core/tests/why_evidence_neighborhood_phase3w.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0488-phase-4nw-why-evidenced-by-relation-evolution.md
docs/provenance/phase-4nw-why-evidenced-by-relation-evolution.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

The implementation adds `verification_evidence_relations_at` resolution to the
existing direct relation evolution subject-detail path. For queried
Verification endpoints, it resolves the direct `evidenced_by` Relation
operation from the current replayed relation version at the queried commit and
projects relation kind, relation version, source endpoint, target Evidence
endpoint, and state digest.

No CLI flag or output contract was added. The existing `why` fields,
`--relation-kind evidenced_by`, and `--expected-evolution-change-operations`
assertion support are reused.

## Focused Validation

Focused tests passed after implementation:

```text
log_dir=/tmp/workvcs-4nw-implementation-focused-20260903T021937Z

cargo test -p workvcs-core --test why_evidence_neighborhood_phase3w why_verification_subject_reports_evidenced_by_edges_to_evidence_endpoints --quiet
status=0

cargo test -p workvcs-core --test verification_evidence_closure_phase3v verification_creation_records_evidenced_by_closure_atomically --quiet
status=0

cargo test -p workvcs-cli cli_why_projects_evidenced_by_create_as_endpoint_evolution --quiet
status=0
```

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4nw-why-evidenced-by-relation-evolution-20260903T024218Z/final-validation
git_status_status=0
git_diff_check_status=0
cargo_fmt_check_status=0
core_why_verification_status=0
core_verification_evidence_closure_status=0
core_why_evidence_neighborhood_status=0
core_why_task_scheduling_status=0
core_primary_containment_evolution_status=0
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
status_0_count=17
```

## Dogfood Proof

The public CLI dogfood uses a temporary local Store and the built
`target/debug/workvcs` binary.

```text
log_dir=/tmp/workvcs-4nw-why-evidenced-by-relation-evolution-20260903T024218Z/dogfood
phase4nw_dogfood=PASS
workspace_id=01a06526-68ca-7e82-b38e-70c6a6438f20
branch_id=01a06526-68ca-7e82-b38e-70f3ac55a7e7
task_id=01a06526-68e3-7922-99e9-e83a27bf848b
criterion_id=01a06526-68fb-7c73-9974-b08c38d5fe0a
evidence_id=01a06526-6912-7323-a156-0dd9c0e19186
verification_id=01a06526-6926-7d93-a7db-27ea2cdd4c7f
verifies_relation_id=01a06526-6926-7d93-a7db-280f9e5bb8ad
evidenced_by_relation_id=01a06526-6926-7d93-a7db-286b6c6bbb27
evidenced_by_relation_version_id=01a06526-6926-7d93-a7db-28708838368a
verification_relation_edges=1
verification_relation_kind=evidenced_by
verification_evolution_change_operations=2
verification_evolution_match_expected=true
verification_evolution_subject_relation_kind_0=verifies
verification_evolution_subject_relation_kind_1=evidenced_by
verification_evidenced_by_source_matches_verification=true
verification_evidenced_by_target_matches_evidence=true
```

## Interpretation

Phase 4NW closes the concrete `evidenced_by` `why` endpoint evolution gap for
queried Verification endpoints. Operators can now see the direct
`verification.record` operation behind Evidence-backed Verification creation
through the same `why --entity` surface and assertion flags used for other
direct endpoint explanations.

The proof is still bounded. It does not show Evidence-subject evolution
traversal, full relation-subject traversal, multi-hop/full evolution traversal,
broader causal traversal, broader context/Resource resolver maturity,
release-candidate validation, or release authorization.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NW advances the gate by closing direct `evidenced_by` creation endpoint
evolution for queried Verification endpoints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
