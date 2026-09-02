# Phase 4NM Why Knowledge Relation Endpoint Evolution Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NM advances the Context resolver, packets, and `why` explanations
release gate by making direct Record-to-Knowledge and Knowledge-to-Knowledge
relation remove/restore operations visible as endpoint evolution for
Entity-subject `why` queries on Knowledge endpoints.

The slice is intentionally narrow. It does not change Store schema, stored
Relation semantics, mutation semantics, CLI flags, release state, Push state,
tags, remote state, deployment state, V2 scope, relation create timeline,
containment or verification relation-subject traversal, Knowledge Exposure
relation-subject traversal, full relation-subject traversal, multi-hop/full
evolution traversal, broader causal traversal, broader context/Resource
resolver behavior, GUI/TUI behavior, or Agent orchestration.

## Gap Proof

The pre-change gap used temporary local Stores and public CLI commands.

Record-to-Knowledge removal:

```text
create Knowledge
create Finding Record
create supports relation from Finding Record to Knowledge
remove the Record-to-Knowledge relation
query why at the removal commit for the Knowledge endpoint
```

Observed before implementation:

```text
rk_relation_edges=0
rk_causal_anchor_changesets=0
rk_evolution_change_operations=0
rk_deferred_relation_families=0
rk_expected_status=1
rk_stderr=error_code=query_invalid ... why evolution change operations 0 does not match expected 1
```

Knowledge-to-Knowledge removal:

```text
create prior Knowledge
create replacement Knowledge
supersede the prior Knowledge
create supersedes relation from replacement Knowledge to prior Knowledge
remove the Knowledge-to-Knowledge relation
query why at the removal commit for the replacement Knowledge endpoint
```

Observed before implementation:

```text
kk_relation_edges=0
kk_causal_anchor_changesets=0
kk_evolution_change_operations=0
kk_deferred_relation_families=0
kk_expected_status=1
kk_stderr=error_code=query_invalid ... why evolution change operations 0 does not match expected 1
```

Focused failing tests captured the same gap:

```text
cargo test -p workvcs-core --test why_knowledge_relation_endpoint_evolution_phase4nm --quiet
core_status=101
4 failed because deferred_relation_families was [] instead of [Evolution]

cargo test -p workvcs-cli cli_why_projects_removed_ --quiet
cli_status=101
new RK/KK CLI tests failed with QueryInvalid("why evolution change operations 0 does not match expected 1")
```

## Implementation

Changed:

```text
crates/workvcs-core/src/history/record.rs
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_knowledge_relation_endpoint_evolution_phase4nm.rs
crates/workvcs-cli/src/main.rs
```

The implementation keeps the existing causal-anchor ChangeSet path and direct
Entity-subject history scan. It broadens the direct relation-subject branch to
recognize:

```text
operation_subject=ChangeOperationSubject::Relation(relation_id)
operation_type=record.relation.remove or record.relation.restore
  relation shape=Record-to-Record or Record-to-Knowledge
operation_type=knowledge.relation.remove or knowledge.relation.restore
  relation shape=Knowledge-to-Knowledge
membership_table=relation_membership_change
```

The branch reads the operation-local relation version from
`relation_membership_change`: after version first, otherwise before version.
It then loads existing relation-version detail for Record-to-Record,
Record-to-Knowledge, or Knowledge-to-Knowledge relations and projects the
operation only when the queried Entity is one of the relation endpoints.

## Focused Validation

The post-change focused validation passed:

```text
cargo test -p workvcs-core --test why_knowledge_relation_endpoint_evolution_phase4nm --quiet
4 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_removed_ --quiet
3 passed; 0 failed

cargo test -p workvcs-core --test why_record_relations_phase3ax why_projects_ --quiet
2 passed; 0 failed
```

## Dogfood Proof

The post-change CLI dogfood used public commands and temporary local Stores.

Record-to-Knowledge:

```text
rk_remove_relation_edges=0
rk_remove_evolution_change_operations=1
rk_remove_expected_match=true
rk_remove_operation_type=record.relation.remove
rk_remove_subject_relation_kind=record_supports
rk_restore_relation_edges=1
rk_restore_evolution_change_operations=2
rk_restore_expected_match=true
rk_restore_operation_0_type=record.relation.restore
rk_restore_operation_1_type=record.relation.remove
```

Knowledge-to-Knowledge:

```text
kk_remove_relation_edges=0
kk_remove_evolution_change_operations=1
kk_remove_expected_match=true
kk_remove_operation_type=knowledge.relation.remove
kk_remove_subject_relation_kind=knowledge_supersedes
kk_restore_relation_edges=1
kk_restore_evolution_change_operations=2
kk_restore_expected_match=true
kk_restore_operation_0_type=knowledge.relation.restore
kk_restore_operation_1_type=knowledge.relation.remove
```

## Interpretation

Phase 4NM removes the concrete direct relation-subject endpoint evolution gap
for Knowledge endpoints of Record-to-Knowledge and Knowledge-to-Knowledge
remove/restore operations. The projected operation is still bounded to
first-parent history, same workspace, operation-local relation membership
versions, recognized relation shapes, and endpoint membership for the queried
Entity.

The proof is still bounded. It does not show full relation-subject traversal,
ChangeSet endpoints, Knowledge Exposure relation endpoint evolution,
multi-hop/full evolution graph traversal, broader causal traversal, or broader
context/Resource resolver maturity.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NM advances the gate by closing direct Record-to-Knowledge and
Knowledge-to-Knowledge remove/restore endpoint evolution gaps.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
