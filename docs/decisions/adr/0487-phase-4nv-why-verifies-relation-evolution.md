# ADR-0487: Phase 4NV Why Verifies Relation Evolution

Status: Accepted
Date: 2026-09-03

## Context

ADR-0018 defines Verification as an Entity whose judgment targets an Acceptance
Criterion or Verification Requirement through a canonical defining `verifies`
Relation. Phase 3U made current `verifies` neighborhoods visible in `why`.

Recent `why` endpoint evolution slices made direct relation creation visible for
Record, Knowledge, Task scheduling, and primary containment endpoints. Before
this slice, `workvcs why --entity <VerificationRequirement endpoint>` could
report the current `verifies` relation edge, but it did not expose the direct
`verification.record` relation operation that created that edge.

The pre-change public CLI probe used a temporary Store and queried a
Verification Requirement endpoint after recording a Verification against it:

```text
log_dir=/tmp/workvcs-4nv-plan-focus-resource-probe-20260903T011459Z/verifies-endpoint-probe
vr_relation_edges=1
vr_has_verifies_edge=true
vr_has_verifies_relation_id=true
vr_evolution_change_operations=0
expected_evolution_status=1
error_code=query_invalid
message=query invalid: why evolution change operations 0 does not match expected 1
```

## Decision

`why --entity <Verification/AcceptanceCriterion/VerificationRequirement
endpoint>` now projects the direct `verification.record` operation for the
defining `verifies` Relation through existing `evolution_change_operations`
fields when the queried Entity is the source Verification or the target AC/VR
endpoint.

The operation subject detail includes:

- relation kind `verifies`;
- relation version id;
- source Verification endpoint;
- target Acceptance Criterion or Verification Requirement endpoint;
- relation state digest.

The implementation reuses current `verification_relations_at` replay at the
queried commit to resolve the relation detail. It does not add a new CLI flag or
a relation-subject query surface.

## Non-Goals

- No `why --relation` command or subject support.
- No Store schema change.
- No Verification, Verification Requirement, Acceptance Criterion, or Evidence
  semantics change.
- No `evidenced_by` endpoint evolution.
- No ContextPacket behavior change.
- No full relation-subject traversal beyond this direct `verifies` endpoint
  slice.
- No multi-hop/full evolution traversal or broader causal traversal.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed after implementation:

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

The public CLI dogfood run used a temporary Store and a newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nv-why-verifies-relation-evolution-20260903T012528Z/dogfood-v3
phase4nv_dogfood=PASS
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

Detailed evidence is recorded in
`docs/provenance/phase-4nv-why-verifies-relation-evolution.md`.

## Consequences

Operators can now use `why --entity` to inspect why a Verification Requirement,
Acceptance Criterion, or Verification endpoint participates in Verification
creation, including the defining `verifies` relation id, relation version,
direction, and direct `verification.record` operation detail.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NV closes a concrete `verifies` endpoint evolution gap, but
`evidenced_by` endpoint evolution, full relation-subject traversal beyond direct
endpoint slices, multi-hop/full evolution traversal, broader causal traversal,
broader context/Resource resolver maturity, and release-candidate validation
remain open.
