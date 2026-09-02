# ADR-0482: Phase 4NO Why Relation Create Endpoint Evolution

Status: Accepted
Date: 2026-09-02

## Context

ADR-0479 and ADR-0480 made direct relation remove and restore operations
visible as endpoint evolution for Record-to-Record, Record-to-Knowledge, and
Knowledge-to-Knowledge relation shapes.

The next dogfood probe exposed the adjacent create gap. After a supports
relation was created, `workvcs why --entity <Decision endpoint>` showed the
new current relation edge, but still reported `evolution_change_operations=0`.
A script that expected the relation's create ChangeOperation to explain the
endpoint-neighborhood change failed with `query_invalid`.

## Decision

For Entity-subject `why` queries, first-parent direct relation-subject evolution
projection now also recognizes relation create operations:

```text
record.relation.create
  - Record-to-Record relation versions
  - Record-to-Knowledge relation versions

knowledge.relation.create
  - Knowledge-to-Knowledge supersedes relation versions
```

The projection reuses the existing `relation_membership_change` lookup. For
create operations, the after relation version is present and becomes the
operation-local version used for detail rendering. The operation is projected
only when the queried Entity is one of the relation endpoints.

For create, remove, and restore relation operations, `why` now presents the
direct endpoint evolution history for these recognized relation shapes. A
removed relation therefore reports remove plus the earlier create operation;
a restored relation reports restore, remove, and create in first-parent order.

The CLI command and flag surface is unchanged. Existing
`--expected-evolution-change-operations` assertions now work for direct
relation create endpoint slices.

## Non-Goals

- No Store schema change.
- No mutation semantics change.
- No CLI command or flag surface change.
- No relation filter or relation limit behavior change.
- No non-relation Entity create evolution change.
- No containment, verification, evidence, scope-link, Knowledge Exposure, or
  ChangeSet endpoint relation-subject slice.
- No full relation-subject traversal beyond direct Record-to-Record,
  Record-to-Knowledge, and Knowledge-to-Knowledge create/remove/restore
  endpoint projection.
- No multi-hop or full evolution graph traversal.
- No broader causal traversal.
- No broader context or Resource resolver behavior change.
- No transcript parsing, LLM extraction, ranking, or Agent orchestration.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

The pre-change public CLI probe reproduced the create gap:

```text
pre_change_relation_edges=1
pre_change_evolution_change_operations=0
pre_change_deferred_relation_families=0
pre_change_expected_evolution_1_status=1
pre_change_expected_evolution_1_error=why evolution change operations 0 does not match expected 1
```

Focused tests failed before implementation:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax why_projects_created_record_relation_as_endpoint_evolution
left: []
right: [Evolution]

cargo test -p workvcs-core --test why_knowledge_relation_endpoint_evolution_phase4nm why_projects_created_record_knowledge_relation_as_knowledge_endpoint_evolution
left: []
right: [Evolution]

cargo test -p workvcs-cli cli_why_projects_created_record_relation_as_endpoint_evolution
QueryInvalid("why evolution change operations 0 does not match expected 1")
```

Focused validation passed after implementation:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax
5 passed; 0 failed

cargo test -p workvcs-core --test why_knowledge_relation_endpoint_evolution_phase4nm
7 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_created_record_relation_as_endpoint_evolution
1 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_removed_record_relation_as_endpoint_evolution
1 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_removed_record_knowledge_relation_as_endpoint_evolution
1 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_removed_knowledge_relation_as_endpoint_evolution
1 passed; 0 failed
```

The old-main/new-binary dogfood run used public commands and temporary Stores:

```text
old_rr_relation_edges=1
old_rr_evolution_change_operations=0
old_rr_expected_evolution_1_status=1

new_rr_relation_edges=1
new_rr_evolution_change_operations=1
new_rr_operation_type=record.relation.create
new_rr_subject_relation_kind=record_supports

new_rk_evolution_change_operations=1
new_rk_operation_type=record.relation.create
new_rk_subject_relation_kind=record_supports

new_kk_evolution_change_operations=1
new_kk_operation_type=knowledge.relation.create
new_kk_subject_relation_kind=knowledge_supersedes
```

Detailed evidence is recorded in
`docs/provenance/phase-4no-why-relation-create-endpoint-evolution.md`.

## Consequences

`workvcs why --entity <Record or Knowledge endpoint>` can now explain direct
relation creation that changed the endpoint's neighborhood, using the same
operation-local relation detail projection already used for remove and restore.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Full relation-subject traversal beyond the direct
create/remove/restore endpoint slices, multi-hop/full evolution graph
traversal, broader causal traversal, broader context/Resource resolver
maturity, release-candidate validation, and release authorization remain open.
