# Phase 4NO Why Relation Create Endpoint Evolution Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NO advances the Context resolver, packets, and `why` explanations
release gate by making direct relation create operations visible as endpoint
evolution for Entity-subject `why` queries on current Record-to-Record,
Record-to-Knowledge, and Knowledge-to-Knowledge relation shapes.

The slice is intentionally narrow. It does not change Store schema, stored
Relation semantics, mutation semantics, CLI flags, relation filters or limits,
non-relation Entity create behavior, release state, Push state, tags, remote
state, deployment state, V2 scope, containment or verification
relation-subject traversal, Knowledge Exposure relation-subject traversal,
full relation-subject traversal, multi-hop/full evolution traversal, broader
causal traversal, broader context/Resource resolver behavior, GUI/TUI
behavior, or Agent orchestration.

## Gap Proof

The pre-change gap used a temporary local Store and public CLI commands.

Record-to-Record creation:

```text
create Decision Record
create Finding Record
create supports relation from Finding Record to Decision Record
query why at the relation create commit for the Decision endpoint
```

Observed before implementation:

```text
pre_change_relation_edges=1
pre_change_evolution_change_operations=0
pre_change_deferred_relation_families=0
pre_change_expected_evolution_1_status=1
pre_change_expected_evolution_1_error=error_code=query_invalid ... why evolution change operations 0 does not match expected 1
```

Focused failing tests captured the same gap:

```text
cargo test -p workvcs-core --test why_record_relations_phase3ax why_projects_created_record_relation_as_endpoint_evolution
status=101
failure=deferred_relation_families left [] right [Evolution]

cargo test -p workvcs-core --test why_knowledge_relation_endpoint_evolution_phase4nm why_projects_created_record_knowledge_relation_as_knowledge_endpoint_evolution
status=101
failure=deferred_relation_families left [] right [Evolution]

cargo test -p workvcs-cli cli_why_projects_created_record_relation_as_endpoint_evolution
status=101
failure=why evolution change operations 0 does not match expected 1
```

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/why_record_relations_phase3ax.rs
crates/workvcs-core/tests/why_knowledge_relation_endpoint_evolution_phase4nm.rs
crates/workvcs-cli/src/main.rs
```

The implementation keeps the existing causal-anchor ChangeSet path and direct
Entity-subject history scan. It broadens only the direct relation-subject
operation-type classifier to recognize:

```text
operation_subject=ChangeOperationSubject::Relation(relation_id)
operation_type=record.relation.create
  relation shape=Record-to-Record or Record-to-Knowledge
operation_type=knowledge.relation.create
  relation shape=Knowledge-to-Knowledge
membership_table=relation_membership_change
```

The existing relation-membership lookup already chooses the after relation
version before the before relation version. For create operations this selects
the operation-local after relation version. The existing relation detail loader
then projects the operation only when the queried Entity is a source or target
endpoint of the relation.

Because create is now part of the direct endpoint relation history, remove and
restore counts increase for these recognized shapes:

```text
removed relation endpoint history: remove, create
restored relation endpoint history: restore, remove, create
```

## Focused Validation

The post-change focused validation passed:

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

## Dogfood Proof

The old-main/new-binary CLI dogfood used public commands and temporary local
Stores. The old binary first proved the pre-change Record-to-Record create gap:

```text
old_rr_relation_edges=1
old_rr_evolution_change_operations=0
old_rr_expected_evolution_1_status=1
old_rr_expected_evolution_1_error=error_code=query_invalid ... why evolution change operations 0 does not match expected 1
```

The Phase 4NO binary then proved relation create endpoint evolution for the
same Record-to-Record shape and for the other two recognized relation families:

```text
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

The dogfood temporary directory was:

```text
/tmp/workvcs-4no-dogfood.sqdWiN
```

## Interpretation

Phase 4NO removes the concrete direct relation-create endpoint evolution gap
for recognized Record-to-Record, Record-to-Knowledge, and
Knowledge-to-Knowledge relation shapes. The projected operation remains bounded
to first-parent history, same workspace, operation-local relation membership
versions, recognized relation shapes, and endpoint membership for the queried
Entity.

The proof is still bounded. It does not show full relation-subject traversal,
ChangeSet endpoints, Knowledge Exposure relation endpoint evolution,
multi-hop/full evolution graph traversal, broader causal traversal, or broader
context/Resource resolver maturity.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NO advances the gate by closing direct relation-create endpoint
evolution gaps for the recognized relation shapes already covered by direct
remove/restore endpoint evolution slices.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
