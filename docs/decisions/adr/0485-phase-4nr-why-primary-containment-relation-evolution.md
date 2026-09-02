# ADR-0485: Phase 4NR Why Primary Containment Relation Evolution

Status: Accepted
Date: 2026-09-02

## Context

`docs/domain/relationships.md` defines primary containment as the canonical
parent-child Work Graph relation for Goal, Plan, and Task organization.
Containment is tree/forest-like in one Work State: an Entity has at most one
primary containment parent, and adding containment must not create cycles.

Before this slice, `task contain` created a durable
`primary_containment.create` operation and `workvcs why --entity <Goal/Plan/Task
endpoint>` could report the current `primary_containment` relation edge.
However, `why` did not expose the direct create operation as endpoint evolution
for the queried Goal, Plan, or Task endpoint.

The pre-change public CLI probe created a temporary Store with Goal -> Plan and
Plan -> Task primary containment and then queried the contained Task endpoint:

```text
log_dir=/tmp/workvcs-4nr-containment-why-probe.sxHZpI
relation_edges=1
relation.0.relation_kind=primary_containment
relation.0.direction=incoming
relation.0.source_entity_kind=plan
relation.0.target_entity_kind=task
evolution_change_operations=0
deferred_relation_families=0
expected_evolution_status=1
expected_evolution_error_head=error_code=query_invalid
```

## Decision

`why --entity <Goal/Plan/Task endpoint>` now projects direct
`primary_containment.create` operations through the existing
`evolution_change_operations` fields when the queried endpoint is the source or
target of the created containment relation.

The operation subject detail includes:

- relation kind `primary_containment`;
- relation version id;
- source Entity endpoint and kind;
- target Entity endpoint and kind;
- relation state digest.

The implementation reuses the existing `primary_containment_relations_at`
replay path at the queried commit to recover relation detail. It does not add a
relation-subject query surface.

## Non-Goals

- No `why --relation` command or subject support.
- No Store schema change.
- No relation identity, containment identity, or containment semantic change.
- No Goal, Plan, Task, or ContextPacket mutation behavior change.
- No full relation-subject traversal beyond this direct endpoint slice.
- No multi-hop/full evolution traversal or broader causal traversal.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused tests failed before implementation:

```text
cargo test -p workvcs-core --test primary_containment_phase3k why_projects_primary_containment_create_as_endpoint_evolution
status=101
failure=assertion left 0 right 1 for expected evolution_change_operations

cargo test -p workvcs-cli cli_why_projects_primary_containment_create_as_endpoint_evolution
status=101
failure=QueryInvalid("why evolution change operations 0 does not match expected 1")
```

Focused validation passed after implementation:

```text
log_dir=/tmp/workvcs-4nr-focused-validation-20260902T062045Z
cargo fmt --all -- --check
status=0

cargo test -p workvcs-core --test primary_containment_phase3k why_projects_primary_containment_create_as_endpoint_evolution
1 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_primary_containment_create_as_endpoint_evolution
1 passed; 0 failed
```

The public CLI dogfood run used a temporary Store and a newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nr-after-dogfood.MyYclp
goal_plan_relation_id=01a060c8-a88c-79f3-ae9a-222fea136c6c
plan_task_relation_id=01a060c8-a8bb-7e62-97f0-35cf88c33798
goal_relation_edges=1
goal_direction=outgoing
goal_evolution_change_operations=1
goal_evolution_subject_relation_kind=primary_containment
plan_child_relation_edges=1
plan_child_direction=incoming
plan_child_evolution_change_operations=1
task_relation_edges=1
task_direction=incoming
task_evolution_change_operations=1
task_evolution_subject_relation_kind=primary_containment
```

Detailed evidence is recorded in
`docs/provenance/phase-4nr-why-primary-containment-relation-evolution.md`.

## Consequences

Operators can now use `why --entity` to inspect why a Goal, Plan, or Task
endpoint participates in primary containment creation, including the relation
id, relation version, direction, and direct create operation detail.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NR closes a concrete primary containment endpoint evolution
gap, but full relation-subject traversal beyond the direct endpoint slices,
multi-hop/full evolution traversal, broader causal traversal, broader
context/Resource resolver maturity, and release-candidate validation remain
open.
