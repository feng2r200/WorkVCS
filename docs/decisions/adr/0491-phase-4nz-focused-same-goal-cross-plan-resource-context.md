# ADR-0491: Phase 4NZ Focused Same-Goal Cross-Plan Peer Resource Context

Status: Accepted
Date: 2026-09-03

## Context

ADR-0481 made Resource basis recovery visible in brief ContextPacket
`verification_requirement` items for current Tasks. ADR-0483 extended the same
recovery information to focused `blocked_dependency` items when the blocking
prerequisite Task has a Resource-backed Verification Requirement. ADR-0486 then
made the same recovery path visible in `normal` and `full` ContextPacket output
for runnable peer Tasks that share the focused Task's direct parent Plan.

The next current dogfood probe exposed the adjacent cross-Plan boundary. In a
temporary Store with one Goal, two direct child Plans, and one runnable Task
under each Plan, the non-focused Plan's Task had a current-head Resource-backed
Verification Requirement. With no focus, full context showed the cross-Plan
peer `verification_requirement` item with Resource recovery detail. When the
Session focused that peer Task, brief context also showed the same recovery
detail through the current-task path. When the Session focused the current
Task in the other Plan, `normal` and `full` context omitted the peer
Resource-backed VR; full context reported zero omitted items, so the absence was
not a packet-budget artifact. A focused `claim next --context-profile full`
packet selected the current Task and omitted the same peer recovery detail.

The pre-change probe showed:

```text
log_dir=/tmp/workvcs-4nz-current-context-why-gate-probe-20260903T070239Z
scenario=same_goal_different_plan_runnable_peer_resource_backed_vr
unfocused_runnable_candidates=2
unfocused_candidates_match_expected=true
context_unfocused_full_has_cross_plan_vr_key=true
context_unfocused_full_has_cross_plan_resource_basis=true
context_unfocused_full_has_cross_plan_refresh_hint=true
context_peer_focus_brief_has_cross_plan_vr_key=true
context_peer_focus_brief_has_cross_plan_resource_basis=true
context_peer_focus_brief_has_cross_plan_refresh_hint=true
context_current_focus_brief_has_cross_plan_vr_key=false
context_current_focus_normal_has_cross_plan_vr_key=false
context_current_focus_full_has_cross_plan_vr_key=false
context_current_focus_full_omitted_items=0
claim_next_packet_has_cross_plan_vr_key=false
claim_next_packet_has_cross_plan_resource_basis=false
claim_next_packet_has_cross_plan_refresh_hint=false
cross_plan_gap_reproduced=true
```

## Decision

When a ContextPacket is generated for `normal` or `full` profile and the active
Session focus is a Task whose direct primary-containment parent is a Plan, the
context resolver now also appends Resource-backed Verification Requirement
items for runnable peer Tasks whose direct parent Plan has the same direct Goal
parent as the focused Task's direct parent Plan.

The same-Goal peer recovery item reuses the existing
`verification_requirement` category, subject, priority, and Resource basis
summary renderer. The appended summary text starts with focused peer
provenance:

```text
same_goal_peer_task=<peer_task_entity_id>
parent_goal=<goal_entity_id>
focused_plan=<focused_plan_entity_id>
peer_plan=<peer_plan_entity_id>
peer_runnable=true
criterion=<acceptance_criterion_entity_id>
local_key=<verification_requirement_local_key>
```

It then appends the existing Resource basis recovery fragment:

```text
resource_basis=<count>
verification_id=<verification_entity_id>
resource_id=<first_basis_resource_id>
adapter=<adapter_kind>@<adapter_schema_version>
scope=<scope_kind>@<scope_schema_version>
baseline_observation_id=<first_basis_baseline_observation_id|none>
refresh_hint="verification cache-refresh --verification <verification_entity_id> --resource-content-from-basis"
```

The implementation keeps the ADR-0486 same-Plan summary unchanged:

```text
same_plan_peer_task=<peer_task_entity_id>
parent_plan=<plan_entity_id>
```

The implementation keeps focus semantics unchanged for runnable selection,
`claim next`, `next`, and overview output. The workspace-wide runnable
projection is still used only as a read-only context resolver input. Brief
context remains focused and does not include same-Plan or same-Goal peer
Resource hints.

The ContextPacket schema, snapshot schema, item category set, item subject set,
CLI command surface, and CLI flags are unchanged. The new information remains
summary text in existing packet items.

## Non-Goals

- No Store schema change.
- No ContextPacket JSON field addition or snapshot schema change.
- No CLI command, flag, or expectation flag addition.
- No change to `runnable tasks`, `claim next`, `next`, Claim, dependency, or
  task scheduling semantics.
- No brief-profile peer expansion.
- No ancestor, descendant, nested-Plan, multi-hop, or full Resource traversal.
- No broad Resource resolver, Resource discovery, automatic refresh, background
  daemon, watcher, polling, or implicit re-observation.
- No `why` behavior change.
- No full relation-subject traversal, multi-hop/full evolution traversal, or
  broader causal traversal.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed after implementation:

```text
log_dir=/tmp/workvcs-4nz-focused-tests-20260903T071229Z
core_new_test=PASS
cli_new_test=PASS
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nz-focused-same-goal-cross-plan-resource-context-20260903T071452Z/dogfood
phase4nz_dogfood=PASS
commands_exit_failures=0
unfocused_runnable_candidates=2
unfocused_candidates_match_expected=true
context_current_focus_brief_has_cross_plan_vr_key=false
context_current_focus_normal_has_cross_plan_vr_key=true
context_current_focus_normal_has_cross_plan_resource_basis=true
context_current_focus_normal_has_cross_plan_refresh_hint=true
context_current_focus_normal_same_goal_peer_hint_count=1
context_current_focus_full_has_cross_plan_vr_key=true
context_current_focus_full_has_cross_plan_resource_basis=true
context_current_focus_full_has_cross_plan_refresh_hint=true
context_current_focus_full_same_goal_peer_hint_count=1
context_current_focus_full_omitted_items=0
claim_next_selected=true
claim_next_context_omitted_items=0
claim_next_packet_has_cross_plan_vr_key=true
claim_next_packet_has_cross_plan_resource_basis=true
claim_next_packet_has_cross_plan_refresh_hint=true
claim_next_same_goal_peer_hint_count=1
```

Final validation passed:

```text
log_dir=/tmp/workvcs-4nz-focused-same-goal-cross-plan-resource-context-20260903T071452Z/final-validation
validation_status=PASS
```

Detailed evidence is recorded in
`docs/provenance/phase-4nz-focused-same-goal-cross-plan-resource-context.md`.

## Consequences

A continuation operator focused on one Task can now keep current-task focus
while normal or full context still surfaces a runnable peer Task's
Resource-backed Verification Requirement and basis-aware refresh command when
both Tasks are under direct sibling Plans contained by the same direct Goal.
The same behavior is visible in the context packet emitted by focused
`claim next --context-profile full`.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NZ closes one concrete focused same-Goal cross-Plan peer
Resource context gap, but broader context/Resource resolver maturity outside
these direct peer slices, full relation-subject traversal, multi-hop/full
evolution traversal, broader causal traversal, release-candidate validation,
and release authorization remain open.
