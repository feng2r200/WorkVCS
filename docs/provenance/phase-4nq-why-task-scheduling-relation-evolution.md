# Phase 4NQ Why Task Scheduling Relation Evolution Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NQ advances the Context resolver, packets, and `why` explanations
release gate by making Task scheduling relations visible from
`workvcs why --entity <Task endpoint>`.

The slice is intentionally narrow. It adds current `depends_on` and
`ordered_before` relation edges for Task endpoints and direct
`task.scheduling_relation.create` endpoint evolution. It does not add
`why --relation`, change Store schema, alter scheduling or runnable semantics,
change mutation semantics, add packet/context behavior, release, Push, tag,
deployment, remote/cloud/V2 scope, GUI/TUI behavior, distributed collaboration,
or Agent orchestration.

## Gap Proof

The pre-change gap used a temporary local Store and public CLI commands.

Scenario:

```text
create prerequisite Task
create dependent Task
add depends_on relation from dependent -> prerequisite
run workvcs why --entity <dependent>
run workvcs why --entity <prerequisite>
```

Observed before implementation:

```text
log_dir=/tmp/workvcs-4nq-scheduling-why-probe.13rnVY
head_commit_id=01a0608c-c6af-7a50-9722-5c2746eefb66
dependent_id=01a0608c-c688-7dd3-9408-66dd39d4327c
prereq_id=01a0608c-c69b-78f0-9752-f16e7eac385f
relation_id=01a0608c-c6af-7a50-9722-5bfb55f20f99
relation_version_id=01a0608c-c6af-7a50-9722-5c00a8f655f3
dependent_relation_edges=0
dependent_evolution_change_operations=0
dependent_deferred_relation_families=0
prereq_relation_edges=0
prereq_evolution_change_operations=0
expected_relation_status=1
expected_evolution_status=1
expected_relation_error_head=error_code=query_invalid
expected_evolution_error_head=error_code=query_invalid
```

This contradicted the domain model in `docs/domain/relationships.md`, where
`depends_on` and `ordered_before` are canonical Work Graph Relations.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/src/history/task.rs
crates/workvcs-core/src/runtime/context.rs
crates/workvcs-core/tests/why_task_scheduling_relation_phase4nq.rs
crates/workvcs-cli/src/main.rs
```

The implementation adds two `WhyRelationKind` variants:

```text
TaskDependsOn -> task_depends_on
TaskOrderedBefore -> task_ordered_before
```

`explain_why` now reuses `task_scheduling_relations_at` to project current
Task scheduling relation edges. Each edge records the relation id, relation
version id, relation kind, direction from the queried endpoint, source/target
Task endpoints, no relation label, and the relation state digest.

Direct `task.scheduling_relation.create` operations are included in the
existing direct relation evolution pass. For queried Task endpoints, the
operation subject detail contains relation kind, relation version, source Task,
target Task, and state digest.

The CLI change is limited to rendering and accepting `task_depends_on` and
`task_ordered_before` as existing `--relation-kind` filter values. Relation
filters and limits still apply only to `relation_edges`; evolution operations
remain endpoint-scoped.

## Focused Validation

Focused tests failed before implementation:

```text
cargo test -p workvcs-core --test why_task_scheduling_relation_phase4nq
status=101
failure=E0599 no variant named TaskDependsOn / TaskOrderedBefore in WhyRelationKind

cargo test -p workvcs-cli cli_creates_structural_task_relations_with_actor_session
status=101
failure=QueryInvalid("why relation-kind filter \"task_depends_on\" is not supported")
```

Focused tests passed after implementation and formatting:

```text
cargo fmt --all -- --check
status=0

cargo test -p workvcs-core --test why_task_scheduling_relation_phase4nq
running 2 tests
2 passed; 0 failed

cargo test -p workvcs-cli cli_creates_structural_task_relations_with_actor_session
running 1 test
1 passed; 0 failed
```

## Dogfood Proof

The public CLI dogfood used a temporary local Store and the built
`target/debug/workvcs` binary.

```text
log_dir=/tmp/workvcs-4nq-after-dogfood.fECY7l
depends_commit_id=01a060a0-7d1d-7190-9eb2-fe692da580c6
depends_relation_id=01a060a0-7d1d-7190-9eb2-fe3aba5073ec
depends_relation_type=depends_on
dependent_relation_edges=1
dependent_relation_kind=task_depends_on
dependent_direction=outgoing
dependent_evolution_change_operations=1
dependent_evolution_subject_relation_kind=task_depends_on
prereq_relation_edges=1
prereq_direction=incoming
order_commit_id=01a060a0-7d8e-7650-9fdd-71515152104f
order_relation_id=01a060a0-7d8e-7650-9fdd-712f15751789
order_relation_type=ordered_before
earlier_relation_edges=1
earlier_relation_kind=task_ordered_before
earlier_direction=outgoing
earlier_evolution_change_operations=1
earlier_evolution_subject_relation_kind=task_ordered_before
later_relation_edges=1
later_relation_kind=task_ordered_before
later_direction=incoming
```

The dogfood commands used existing `why --entity`, `--relation-kind`,
`--direction`, `--source-entity-kind`, `--target-entity-kind`,
`--relation-limit`, `--expected-relation-edges`, and
`--expected-evolution-change-operations` flags.

## Interpretation

Phase 4NQ closes a concrete Task scheduling `why` endpoint gap: operators can
now see both the current scheduling relation edge and the direct create
operation through the same `why --entity` surface used for other endpoint
neighborhoods.

The proof is still bounded. It does not show full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, broader
context/Resource resolver maturity, release-candidate validation, or release
authorization.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NQ advances the gate by closing current Task scheduling relation edge
projection and direct scheduling create endpoint evolution for `depends_on`
and `ordered_before`.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
