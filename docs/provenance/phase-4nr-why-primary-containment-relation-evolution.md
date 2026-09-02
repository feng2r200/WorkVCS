# Phase 4NR Why Primary Containment Relation Evolution Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NR advances the Context resolver, packets, and `why` explanations
release gate by making primary containment creation visible as direct endpoint
evolution from `workvcs why --entity <Goal/Plan/Task endpoint>`.

The slice is intentionally narrow. It adds direct
`primary_containment.create` endpoint evolution for queried Goal, Plan, and
Task endpoints that are source or target endpoints of the created containment
relation. It does not add `why --relation`, change Store schema, alter relation
identity or containment semantics, change Goal/Plan/Task mutation behavior, add
packet/context behavior, release, Push, tag, deployment, remote/cloud/V2 scope,
GUI/TUI behavior, distributed collaboration, or Agent orchestration.

## Gap Proof

The pre-change gap used a temporary local Store and public CLI commands.

Scenario:

```text
create Workspace
create Goal
create Plan
create Task
add Goal -> Plan primary containment
add Plan -> Task primary containment
run workvcs why --entity <Task>
```

Observed before implementation:

```text
log_dir=/tmp/workvcs-4nr-containment-why-probe.sxHZpI
branch_id=01a060bc-100e-7670-9c0e-9833eaa1bc65
head_commit_id=01a060bc-1079-7000-9790-cf3854a8ca92
goal_id=01a060bc-1023-7cb1-afce-0753f8b8085e
plan_id=01a060bc-1038-7610-8ef1-a77059ab42fe
task_id=01a060bc-104d-7e32-8cd3-e5ead0bbec1f
containment_relation_id=01a060bc-1079-7000-9790-cf0d5d4e699d
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

The current relation edge was visible, but the direct
`primary_containment.create` operation that created the endpoint neighborhood
was not visible through `why`.

## Implementation

Changed:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/tests/primary_containment_phase3k.rs
crates/workvcs-core/tests/why_structural_neighborhood_phase3t.rs
crates/workvcs-cli/src/main.rs
```

The implementation includes `primary_containment.create` in the existing
direct relation evolution operation pass. For queried Goal, Plan, or Task
endpoints, operation subject detail is resolved from the current primary
containment replay at the queried commit and records relation kind, relation
version, source endpoint, target endpoint, and state digest.

No CLI flag or output contract was added. The existing `why` fields,
`--relation-kind primary_containment`, and
`--expected-evolution-change-operations` assertion support are reused.

## Focused Validation

Focused tests failed before implementation:

```text
log_dir=/tmp/workvcs-4nr-failing-tests-20260902T061717Z

cargo test -p workvcs-core --test primary_containment_phase3k why_projects_primary_containment_create_as_endpoint_evolution
status=101
failure=assertion left 0 right 1

cargo test -p workvcs-cli cli_why_projects_primary_containment_create_as_endpoint_evolution
status=101
failure=QueryInvalid("why evolution change operations 0 does not match expected 1")
```

Focused tests passed after implementation and formatting:

```text
log_dir=/tmp/workvcs-4nr-focused-validation-20260902T062045Z

cargo fmt --all -- --check
status=0

cargo test -p workvcs-core --test primary_containment_phase3k why_projects_primary_containment_create_as_endpoint_evolution
running 1 test
1 passed; 0 failed

cargo test -p workvcs-cli cli_why_projects_primary_containment_create_as_endpoint_evolution
running 1 test
1 passed; 0 failed
```

## Dogfood Proof

The public CLI dogfood used a temporary local Store and the built
`target/debug/workvcs` binary.

```text
log_dir=/tmp/workvcs-4nr-after-dogfood.MyYclp
branch_id=01a060c8-a845-7e92-9602-a26b95a6e8a1
head_commit_id=01a060c8-a8bb-7e62-97f0-35f27f421bae
goal_id=01a060c8-a85e-75d0-a026-6362a43a4e5f
plan_id=01a060c8-a875-7c61-a515-4bea9339bd6a
task_id=01a060c8-a8a2-77c0-b5ea-493b807e5554
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

The dogfood commands used existing `why --entity`,
`--relation-kind primary_containment`, `--expected-relation-edges`, and
`--expected-evolution-change-operations` flags.

## Interpretation

Phase 4NR closes a concrete primary containment `why` endpoint evolution gap:
operators can now see the direct create operation behind Goal -> Plan and
Plan -> Task containment endpoint neighborhoods through the same `why --entity`
surface used for other endpoint explanations.

The proof is still bounded. It does not show full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, broader
context/Resource resolver maturity, release-candidate validation, or release
authorization.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NR advances the gate by closing direct primary containment create
endpoint evolution for queried Goal, Plan, and Task endpoints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
