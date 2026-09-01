# ADR-0475: Phase 4NH Why Evolution Subject Detail Projection

Status: Accepted
Date: 2026-09-02

## Context

ADR-0474 projects direct ChangeOperation Entity and Relation subjects for
already-detected causal-anchor ChangeSets in `why`. That removes the need to
leave `why` just to run `changeset operations`, but the projected UUIDs are
still not self-explanatory.

The Phase 4NH dogfood compares the same Store through the previous main binary
and the new implementation. The previous output renders three evolution
operations but does not include `subject_statement_json` or
`subject_relation_kind`. The new output keeps the same three operations and
adds readable subject detail for current recognized operation subjects.

## Decision

`WhyEvolutionChangeOperation` now includes optional `subject_detail`.

For current recognized Entity subjects, the detail includes:

```text
subject_detail_kind=entity
subject_entity_kind=<entity_kind>
subject_entity_version_id=<entity_version_id>
subject_statement_json=<statement_json>
```

`subject_statement_json` is rendered for current Record and Knowledge subjects.
Other current Entity subjects still report kind and version without inventing a
statement.

For current recognized Relation subjects, the detail includes:

```text
subject_detail_kind=relation
subject_relation_kind=<relation_kind>
subject_relation_version_id=<relation_version_id>
subject_relation_label=<label>
subject_relation_source_kind=<entity|evidence|knowledge_exposure>
subject_relation_source_entity_kind=<entity_kind>
subject_relation_source_entity_id=<entity_id>
subject_relation_target_kind=<entity|evidence|knowledge_exposure>
subject_relation_target_entity_kind=<entity_kind>
subject_relation_target_entity_id=<entity_id>
subject_relation_state_digest=<state_digest>
```

Evidence and knowledge exposure endpoints use the matching
`subject_relation_*_evidence_id` or `subject_relation_*_exposure_id` fields.
The relation detail is built from existing current WorkState projections for
the relation families already recognized by `why`.

The detail is best-effort and read-only. If a ChangeOperation subject is not
current in the target WorkState or is not a recognized `why` subject family,
`subject_detail` is omitted while the existing Phase 4NG operation fields
remain unchanged.

## Non-Goals

- No Store schema change.
- No new stored Relation rows.
- No mutation semantics change.
- No full evolution traversal.
- No broader causal traversal.
- No context or Resource resolver behavior change.
- No payload parsing or semantic inference from ChangeOperation payloads.
- No transcript parsing, LLM extraction, ranking, or Agent orchestration.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

Focused validation passed:

```text
cargo fmt --all -- --check
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng
cargo test -p workvcs-cli cli_supersedes_decision_record_atomically -- --nocapture
```

The dogfood run proved old-main/new-binary compatibility on the same Store:

```text
old_evolution_change_operations=3
old_subject_statement_json_present=false
old_subject_relation_kind_present=false
new_evolution_change_operations=3
new_evolution_match=true
new_op0_subject_detail_kind=entity
new_op0_subject_entity_kind=record
new_op0_subject_statement_json="Use optimistic writes"
new_op1_subject_detail_kind=relation
new_op1_subject_relation_kind=record_supersedes
new_op2_subject_relation_kind=record_derived_from
```

Focused validation and dogfood details are recorded in
`docs/provenance/phase-4nh-why-evolution-subject-detail.md`.

## Consequences

`workvcs why` can now answer the next practical continuation question in one
command: after it reports the causal-anchor ChangeSet and direct
ChangeOperation subjects, the same output explains current recognized subjects
with statements or relation kind/source/target details.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Full evolution traversal, broader causal traversal,
broader context/Resource resolver maturity, release-candidate validation, and
release authorization remain open.
