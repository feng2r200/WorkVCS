# ADR-0488: Phase 4NW Why Evidenced By Relation Evolution

Status: Accepted
Date: 2026-09-03

## Context

ADR-0036 defines Verification evidence closure: recording a Verification with
Evidence creates immutable `evidenced_by` Relations from the Verification to
the Evidence objects. ADR-0037 made current `evidenced_by` neighborhoods
visible in `why`, and ADR-0487 made the defining `verifies` Relation creation
visible as direct endpoint evolution for Verification endpoints.

Before this slice, a queried Verification endpoint could report the current
`evidenced_by` relation edge, but `why` did not expose the direct
`verification.record` Relation operation that created that edge.

The pre-change public CLI probe used current `main` commit
`1bcc48ec9d729c642232d7b6d552dce2d9a6f87a`, a temporary Store, one
Verification, and one Evidence object:

```text
log_dir=/tmp/workvcs-4nw-evidenced-by-endpoint-probe-20260903T021310Z
verification_record_status=0
verification_show_status=0
why_verification_evidenced_by_status=0
verification_relation_edges=1
verification_has_evidenced_by_edge=true
verification_evolution_change_operations=1
verification_has_evolution_subject_relation_kind_verifies=true
verification_has_evolution_subject_relation_kind_evidenced_by=false
why_verification_expected_two_evolution_status=1
message=query invalid: why evolution change operations 1 does not match expected 2
```

## Decision

`why --entity <Verification endpoint>` now projects direct
`verification.record` operations for current `evidenced_by` Relations attached
to that Verification through existing `evolution_change_operations` fields.

The operation subject detail includes:

- relation kind `evidenced_by`;
- relation version id;
- source Verification endpoint;
- target Evidence endpoint;
- relation state digest.

The implementation reuses current `verification_evidence_relations_at` replay
at the queried commit to resolve relation detail. It adds a guarded branch to
the existing direct relation endpoint evolution resolver instead of adding a
new relation-subject query surface.

## Non-Goals

- No `why --relation` command or subject support.
- No Evidence-subject evolution traversal.
- No Store schema change.
- No Verification, Evidence, Acceptance Criterion, or Verification Requirement
  semantics change.
- No CLI flag or public output field addition.
- No ContextPacket behavior change.
- No full relation-subject traversal beyond this direct Verification endpoint
  slice.
- No multi-hop/full evolution traversal or broader causal traversal.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed after implementation:

```text
log_dir=/tmp/workvcs-4nw-implementation-focused-20260903T021937Z
core_why_evidence_status=0
core_verification_evidence_closure_status=0
cli_evidenced_by_status=0
```

Final validation passed:

```text
log_dir=/tmp/workvcs-4nw-why-evidenced-by-relation-evolution-20260903T024218Z/final-validation
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
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nw-why-evidenced-by-relation-evolution-20260903T024218Z/dogfood
phase4nw_dogfood=PASS
verification_relation_edges=1
verification_relation_kind=evidenced_by
verification_evolution_change_operations=2
verification_evolution_match_expected=true
verification_evolution_subject_relation_kind_0=verifies
verification_evolution_subject_relation_kind_1=evidenced_by
verification_evidenced_by_source_matches_verification=true
verification_evidenced_by_target_matches_evidence=true
```

Detailed evidence is recorded in
`docs/provenance/phase-4nw-why-evidenced-by-relation-evolution.md`.

## Consequences

Operators can now use `why --entity <Verification> --relation-kind
evidenced_by` to inspect both the current Evidence edge and the direct
`verification.record` operation that created it, including relation id,
relation version, source Verification endpoint, target Evidence endpoint, and
state digest.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NW closes the concrete `evidenced_by` endpoint evolution gap
for queried Verification endpoints, but full relation-subject traversal beyond
direct endpoint slices, multi-hop/full evolution traversal, broader causal
traversal, broader context/Resource resolver maturity, and release-candidate
validation remain open.
