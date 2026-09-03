# ADR-0489: Phase 4NX Why Evidence Subject Evidenced By Evolution

Status: Accepted
Date: 2026-09-03

## Context

ADR-0036 defines Verification evidence closure: recording a Verification with
Evidence creates immutable `evidenced_by` Relations from the Verification to
the Evidence objects. ADR-0037 made current `evidenced_by` neighborhoods
visible in `why`, and ADR-0488 made the direct `evidenced_by` Relation creation
visible as endpoint evolution when querying the source Verification endpoint.

Before this slice, a queried Evidence endpoint could report the current
incoming `evidenced_by` relation edges, but `why` did not expose the direct
`verification.record` Relation operations that created those edges.

The pre-change public CLI probe used current `main` commit
`c7917482d5fde79aa5c7cb085c86054a453b0a94`, a temporary Store, one Evidence
object reused by two Verifications, and the current public CLI:

```text
log_dir=/tmp/workvcs-4nx-evidence-subject-evidenced-by-probe-20260903T051248Z
why_first_verification_control_status=0
why_evidence_actual_status=0
why_evidence_expected_evolution_exit_code=1
first_verification_control_relation_edges=1
first_verification_control_evolution_change_operations=2
first_verification_control_subject_relation_kind_1=evidenced_by
evidence_relation_edges=2
evidence_relation_0_kind=evidenced_by
evidence_relation_0_direction=incoming
evidence_relation_1_kind=evidenced_by
evidence_relation_1_direction=incoming
evidence_evolution_change_operations=0
evidence_evidenced_by_subject_detail_count=0
evidence_expected_evolution_error_code=query_invalid
evidence_expected_evolution_message=query invalid: why evolution change operations 0 does not match expected 2
probe_status=PASS
```

## Decision

`why --evidence <Evidence endpoint>` now projects direct
`verification.record` operations for current incoming `evidenced_by` Relations
attached to that Evidence through existing `evolution_change_operations` fields.

The operation subject detail includes:

- relation kind `evidenced_by`;
- relation version id;
- source Verification endpoint;
- target Evidence endpoint;
- relation state digest.

The implementation lets Evidence subjects enter the existing direct relation
evolution scan. Entity subject evolution remains unchanged, and Knowledge
Exposure subjects still do not enter this direct relation evolution pass.

## Non-Goals

- No `why --relation` command or subject support.
- No Store schema change.
- No Verification, Evidence, Acceptance Criterion, or Verification Requirement
  semantics change.
- No CLI flag or public output field addition.
- No ContextPacket behavior change.
- No full relation-subject traversal beyond this direct Evidence endpoint
  slice.
- No multi-hop/full evolution traversal or broader causal traversal.
- No broader context/Resource resolver behavior change.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed after implementation:

```text
log_dir=/tmp/workvcs-4nx-implementation-focused-20260903T052730Z
cargo_fmt_status=0
core_why_evidence_status=0
core_verification_evidence_status=0
cli_evidence_subject_status=0
cli_verification_subject_status=0
focused_status=PASS
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nx-why-evidence-subject-evidenced-by-evolution-20260903T052944Z/dogfood
phase4nx_dogfood=PASS
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

Final validation passed:

```text
log_dir=/tmp/workvcs-4nx-why-evidence-subject-evidenced-by-evolution-20260903T054617Z/final-validation
validation_status=PASS
```

Detailed evidence is recorded in
`docs/provenance/phase-4nx-why-evidence-subject-evidenced-by-evolution.md`.

## Consequences

Operators can now use `why --evidence <Evidence> --relation-kind evidenced_by`
to inspect both the current incoming Verification edges and the direct
`verification.record` operations that created them, including relation id,
relation version, source Verification endpoint, target Evidence endpoint, and
state digest.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NX closes the concrete Evidence endpoint side of the
`evidenced_by` direct relation evolution gap, but full relation-subject
traversal beyond direct endpoint slices, multi-hop/full evolution traversal,
broader causal traversal, broader context/Resource resolver maturity, and
release-candidate validation remain open.
