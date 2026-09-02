# ADR-0484: Phase 4NQ Why Task Scheduling Relation Evolution

Status: Accepted
Date: 2026-09-02

## Context

`docs/domain/relationships.md` defines Task scheduling relations as canonical
Work Graph Relations:

- `depends_on`: dependent -> prerequisite.
- `ordered_before`: earlier -> later.

Before this slice, `task depends-on` created a durable
`task.scheduling_relation.create` operation and `task scheduling-list` could
list the relation, but `workvcs why --entity <Task endpoint>` did not expose
the current scheduling relation edge or the direct create operation as endpoint
evolution.

The pre-change public CLI probe created a temporary Store with one
`depends_on` relation and then queried both Task endpoints:

```text
log_dir=/tmp/workvcs-4nq-scheduling-why-probe.13rnVY
dependent_relation_edges=0
dependent_evolution_change_operations=0
prereq_relation_edges=0
prereq_evolution_change_operations=0
expected_relation_status=1
expected_evolution_status=1
```

## Decision

`why --entity <Task endpoint>` now projects current Task scheduling relations
as normal `relation_edges`:

- `depends_on` is rendered as `task_depends_on`.
- `ordered_before` is rendered as `task_ordered_before`.
- Both endpoints are `entity` endpoints with `entity_kind=task`.
- The source endpoint keeps the domain direction (`dependent` or `earlier`) and
  reports `direction=outgoing`; the target endpoint reports
  `direction=incoming`.

Direct `task.scheduling_relation.create` operations are also projected through
the existing `evolution_change_operations` fields when the queried Task is the
source or target endpoint. The relation subject detail includes the scheduling
relation kind, relation version, source Task endpoint, target Task endpoint,
and relation state digest.

The implementation reuses the existing current-head
`task_scheduling_relations_at` replay path. It does not add a relation-subject
query surface.

## Non-Goals

- No `why --relation` command or subject support.
- No Store schema change.
- No Task scheduling, runnable, dependency, or ordering semantic change.
- No Task mutation semantic change.
- No ContextPacket schema or selection behavior change.
- No relation filter or limit semantic change beyond supporting the new
  relation-kind strings.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused tests failed before implementation:

```text
cargo test -p workvcs-core --test why_task_scheduling_relation_phase4nq
status=101
failure=no variant named TaskDependsOn / TaskOrderedBefore in WhyRelationKind

cargo test -p workvcs-cli cli_creates_structural_task_relations_with_actor_session
status=101
failure=why relation-kind filter "task_depends_on" is not supported
```

Focused validation passed after implementation:

```text
cargo fmt --all -- --check
status=0

cargo test -p workvcs-core --test why_task_scheduling_relation_phase4nq
2 passed; 0 failed

cargo test -p workvcs-cli cli_creates_structural_task_relations_with_actor_session
1 passed; 0 failed
```

The public CLI dogfood run used a temporary Store and a newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nq-after-dogfood.fECY7l
depends_relation_type=depends_on
dependent_relation_edges=1
dependent_relation_kind=task_depends_on
dependent_direction=outgoing
dependent_evolution_change_operations=1
dependent_evolution_subject_relation_kind=task_depends_on
prereq_relation_edges=1
prereq_direction=incoming
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

Detailed evidence is recorded in
`docs/provenance/phase-4nq-why-task-scheduling-relation-evolution.md`.

## Consequences

Operators can now use `why --entity` to inspect why a Task endpoint participates
in dependency or sibling-order scheduling relations, including the relation id,
relation version, direction, and direct create operation detail.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NQ closes a concrete Task scheduling endpoint gap, but full
relation-subject traversal beyond the direct endpoint slices, multi-hop/full
evolution traversal, broader causal traversal, broader context/Resource
resolver maturity, and release-candidate validation remain open.
