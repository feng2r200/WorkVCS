# ADR-0479: Phase 4NL Why Relation-Subject Endpoint Evolution

Status: Accepted
Date: 2026-09-02

## Context

ADR-0477 made `workvcs why --entity <changed Entity>` project a queried
Entity's own direct first-parent evolution ChangeOperation even when that
Entity is not a causal anchor. ADR-0478 then made direct Entity operation
detail operation-local.

That still left a concrete relation-subject gap. When a Record-to-Record
Relation involving the queried Entity was removed, the resolved target commit
had `relation_edges=0`, no causal anchor ChangeSet for the queried Entity, and
no direct Entity-subject operation. The operation that changed the queried
Entity's neighborhood existed, but it was stored as
`ChangeOperationSubject::Relation(relation_id)`, so `why` skipped it.

## Decision

For Entity-subject `why` queries, first-parent direct evolution projection now
also admits operation-local Record relation remove and restore operations:

```text
target_history=first_parent_from_target_commit
workspace_filter=same_workspace
subject_filter=ChangeOperationSubject::Relation(relation_id)
operation_type_filter=record.relation.remove or record.relation.restore
version_source=relation_membership_change.before_relation_version_id or after_relation_version_id
endpoint_filter=query_entity_id is relation source or target endpoint
dedupe_key=operation_id
```

The relation detail is loaded from the operation's relation version, not from
the target commit's current WorkState membership. Removal therefore can still
render relation `subject_detail` even when the removed edge is no longer a
current `relation_edges` result.

For restore operations, the after relation version is used when present. For
remove operations, the before relation version is used. The projected
`subject_detail` includes the existing CLI fields for recognized Relation
operation subjects:

```text
subject_detail_kind=relation
subject_relation_kind=<record relation kind>
subject_relation_version_id=<operation-local relation version id>
subject_relation_source_entity_id=<source Record entity id>
subject_relation_target_entity_id=<target Record entity id>
subject_relation_state_digest=<relation state digest>
```

The CLI command and flag surface is unchanged. Existing
`--expected-evolution-change-operations` assertions now work for this direct
Record relation remove/restore endpoint slice.

## Non-Goals

- No Store schema change.
- No mutation semantics change.
- No CLI command or flag surface change.
- No relation create timeline.
- No Record-to-Knowledge, Knowledge-to-Knowledge, containment, verification,
  evidence, scope-link, or Knowledge Exposure relation-subject endpoint slice.
- No full relation-subject traversal beyond direct Record-to-Record
  remove/restore endpoint projection.
- No ChangeSet endpoint.
- No multi-hop or full evolution graph traversal.
- No broader causal traversal.
- No broader context or Resource resolver behavior change.
- No transcript parsing, LLM extraction, ranking, or Agent orchestration.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

Focused pre-change tests failed as expected:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax why_projects_removed_record_relation_as_endpoint_evolution --quiet
left: []
right: [Evolution]

cargo test -p workvcs-cli cli_why_projects_removed_record_relation_as_endpoint_evolution --quiet
QueryInvalid("why evolution change operations 0 does not match expected 1")
```

Focused validation passed after implementation:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax --quiet
4 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_removed_record_relation_as_endpoint_evolution --quiet
1 passed; 0 failed
```

The old-main/new-binary dogfood run proved the behavior change on one Store:

```text
old_commit=a7679f7e7ec182d5c03af19395aebc0483d8907c
old_expected_status=1
old_relation_edges=0
old_causal_anchor_changesets=0
old_evolution_change_operations=0
old_deferred_relation_families=0
old_expected_error=query invalid: why evolution change operations 0 does not match expected 1

new_branch=phase-4nl-why-relation-subject-evolution
new_relation_edges=0
new_causal_anchor_changesets=0
new_evolution_change_operations=1
new_expected_match=true
new_subject_family=relation
new_subject_detail_kind=relation
new_subject_relation_kind=record_supports
new_subject_relation_version_id_matches_removed_relation_version=true
new_source_entity_id_matches_finding=true
new_target_entity_id_matches_decision=true
new_deferred_relation_family_0=evolution
```

Detailed evidence is recorded in
`docs/provenance/phase-4nl-why-relation-subject-endpoint-evolution.md`.

## Consequences

`workvcs why --entity <Record endpoint>` can now explain a direct
Record-to-Record relation removal or restore that changed that endpoint's
neighborhood, even when the relation is absent from current `relation_edges`.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Relation-subject traversal beyond this direct
Record-to-Record remove/restore endpoint slice, multi-hop/full evolution graph
traversal, broader causal traversal, broader context/Resource resolver
maturity, release-candidate validation, and release authorization remain open.
