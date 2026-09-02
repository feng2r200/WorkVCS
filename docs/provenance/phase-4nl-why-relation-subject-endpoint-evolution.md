# Phase 4NL Why Relation-Subject Endpoint Evolution Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NL advances the Context resolver, packets, and `why` explanations
release gate by making direct Record-to-Record relation remove/restore
operations visible as endpoint evolution for Entity-subject `why` queries.

The slice targets the concrete post-4NK residual risk: when a queried Record
Entity is the source or target endpoint of a removed or restored Record
Relation, the operation that changed the endpoint neighborhood is a Relation
subject operation. `workvcs why --entity <endpoint>` must be able to project
that direct operation and render operation-local relation detail.

The slice is intentionally narrow. It does not change Store schema, stored
Relation semantics, mutation semantics, CLI flags, release state, Push state,
tags, remote state, deployment state, V2 scope, relation create timeline,
Record-to-Knowledge or Knowledge relation-subject traversal, containment or
verification relation-subject traversal, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, broader
context/Resource resolver behavior, GUI/TUI behavior, or Agent orchestration.

## Gap Proof

The pre-change gap used a temporary local Store:

```text
create Decision Record: Use serialized writes
create Finding Record: Concurrent write tests require serialization
create supports relation from Finding to Decision
remove the supports relation
query why at the removal commit for the Decision endpoint
```

The existing implementation skipped the relation-subject remove operation:

```text
relation_edges=0
causal_anchor_changesets=0
evolution_change_operations=0
deferred_relation_families=0
```

Focused failing tests captured the same gap:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax why_projects_removed_record_relation_as_endpoint_evolution --quiet
core_status=101
left: []
right: [Evolution]

cargo test -p workvcs-cli cli_why_projects_removed_record_relation_as_endpoint_evolution --quiet
cli_status=101
QueryInvalid("why evolution change operations 0 does not match expected 1")
```

## Implementation

Changed:

```text
crates/workvcs-core/src/history/record.rs
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_record_relations_phase3ax.rs
crates/workvcs-cli/src/main.rs
```

The implementation keeps the existing causal-anchor ChangeSet path and direct
Entity-subject history scan. It adds a direct relation-subject branch only for:

```text
operation_subject=ChangeOperationSubject::Relation(relation_id)
operation_type=record.relation.remove or record.relation.restore
membership_table=relation_membership_change
```

The branch reads the operation-local relation version from
`relation_membership_change`: after version first, otherwise before version.
It then reuses the existing Record relation version loader to obtain kind,
label, source endpoint, target endpoint, and state digest. The operation is
projected only when the queried Entity is one of the relation endpoints.

## Focused Validation

The post-change focused validation passed:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax --quiet
cargo test -p workvcs-cli cli_why_projects_removed_record_relation_as_endpoint_evolution --quiet
```

Observed results:

```text
core_why_record_relations_tests=4_passed
cli_removed_relation_endpoint_evolution_test=1_passed
```

The core validation includes both new relation-subject endpoint paths:

```text
why_projects_removed_record_relation_as_endpoint_evolution=passed
why_projects_restored_record_relation_as_endpoint_evolution=passed
```

## Dogfood Proof

The old-main/new-binary dogfood run built:

```text
old_commit=a7679f7e7ec182d5c03af19395aebc0483d8907c
new_branch=phase-4nl-why-relation-subject-evolution
```

It used the old main binary to create one temporary Store and queried the same
removal commit with both binaries.

Observed output:

```text
phase4nl_dogfood=true
old_expected_status=1
old_relation_edges=0
old_causal_anchor_changesets=0
old_evolution_change_operations=0
old_deferred_relation_families=0
old_expected_error=query invalid: why evolution change operations 0 does not match expected 1

new_relation_edges=0
new_causal_anchor_changesets=0
new_evolution_change_operations=1
new_expected_match=true
new_operation_id_matches_removed_operation=true
new_subject_family=relation
new_subject_object_id_matches_relation_id=true
new_subject_detail_kind=relation
new_subject_relation_kind=record_supports
new_subject_relation_version_id_matches_relation_version=true
new_source_entity_id_matches_finding=true
new_target_entity_id_matches_decision=true
new_deferred_relation_families=1
new_deferred_relation_family_0=evolution
```

## Interpretation

Phase 4NL removes the concrete post-4NK direct relation-subject endpoint
evolution gap for Record-to-Record relation removal and restore operations.
The projected operation is still bounded to first-parent history, same
workspace, recognized Record relation version detail, and endpoint membership
for the queried Entity.

The proof is still bounded. It does not show full relation-subject traversal,
Record-to-Knowledge or Knowledge relation-subject endpoint evolution,
ChangeSet endpoints, multi-hop/full evolution graph traversal, broader causal
traversal, or broader context/Resource resolver maturity.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NL advances the gate by closing one concrete direct Record relation
remove/restore endpoint evolution gap.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
