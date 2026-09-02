# ADR-0478: Phase 4NK Why Operation-Local Entity Detail

Status: Accepted
Date: 2026-09-02

## Context

ADR-0477 made `workvcs why --entity <changed Entity>` project the queried
Entity's own direct first-parent evolution ChangeOperation even when that
Entity is not a causal anchor.

That exposed a narrower detail gap. When a queried Entity had multiple direct
Entity changes before the target commit, `why` could report multiple direct
evolution operations, but the projected Entity `subject_detail` still reused
the target commit's current Entity version and statement lookup for every
historical operation. An older operation could therefore appear to carry the
final Entity version rather than the version produced by that operation.

## Decision

For direct Entity-subject evolution operations projected by the queried-Entity
history path, `workvcs why` now builds Entity `subject_detail` from the
operation-local after-state:

```text
subject_entity_version_id=entity_membership_change.after_entity_version_id
subject_statement_json=statement_at(operation_commit_id, entity_id)
```

The same direct Entity filter from ADR-0477 is preserved:

```text
target_history=first_parent_from_target_commit
workspace_filter=same_workspace
subject_filter=ChangeOperationSubject::Entity(query_entity_id)
evolution_filter=entity_membership_change.before_entity_version_id is not null
dedupe_key=operation_id
```

Initial Entity creation operations remain outside this direct evolution slice
because they have no prior Entity version. A direct Entity operation that has a
prior version but no after version is still an evolution operation, but it does
not reuse the target-current Entity detail.

The CLI surface is unchanged. Existing
`evolution_change_operation.<i>.subject_entity_version_id` and
`subject_statement_json` fields now carry operation-local Entity detail for
this direct queried-Entity path.

## Non-Goals

- No Store schema change.
- No mutation semantics change.
- No CLI command or flag surface change.
- No initial Entity creation timeline.
- No relation-subject traversal.
- No ChangeSet endpoint.
- No multi-hop or full evolution graph traversal.
- No broader causal traversal.
- No broader context or Resource resolver behavior change.
- No transcript parsing, LLM extraction, ranking, or Agent orchestration.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

Focused pre-change tests failed as expected:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax why_reports_operation_local_entity_detail_for_multiple_direct_assumption_changes --quiet
left: final invalidated EntityVersionId
right: validated operation EntityVersionId

cargo test -p workvcs-cli cli_why_reports_operation_local_entity_detail_for_multiple_direct_assumption_changes --quiet
left: final invalidated version
right: validated operation version
```

Focused validation passed after implementation:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax why_reports_operation_local_entity_detail_for_multiple_direct_assumption_changes --quiet
cargo test -p workvcs-cli cli_why_reports_operation_local_entity_detail_for_multiple_direct_assumption_changes --quiet
cargo test -p workvcs-core --test why_record_relations_phase3ax --quiet
cargo test -p workvcs-core --test why_evolution_operation_projection_phase4ng --quiet
```

The old-main/new-binary dogfood run proved the behavior change on one Store:

```text
old_commit=7a4ad9fa00d3ac96a80b2124eca222b605146e6f
old_evolution_change_operations=2
old_op1_operation_id=01a05f79-cd4f-7992-82a7-e7d9c83e9288
old_op1_subject_entity_version_id=01a05f79-cd64-7cd0-85a5-62b1cd3da597
old_op1_actual_matches_final_invalidated=true
old_op1_actual_matches_operation_validated=false

new_branch=phase-4nk-why-operation-local-detail
new_evolution_change_operations=2
new_op1_operation_id=01a05f79-cd4f-7992-82a7-e7d9c83e9288
new_op1_subject_entity_version_id=01a05f79-cd4f-7992-82a7-e7e90d01fa80
new_op1_actual_matches_final_invalidated=false
new_op1_actual_matches_operation_validated=true
```

Detailed evidence is recorded in
`docs/provenance/phase-4nk-why-operation-local-entity-detail.md`.

## Consequences

`workvcs why --entity <changed entity>` now distinguishes multiple direct
first-parent Entity evolution operations more accurately. An older direct
Entity operation can show the Entity version produced by that operation instead
of inheriting the final target commit's Entity version.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Relation-subject traversal, multi-hop/full evolution
graph traversal, broader causal traversal, broader context/Resource resolver
maturity, release-candidate validation, and release authorization remain open.
