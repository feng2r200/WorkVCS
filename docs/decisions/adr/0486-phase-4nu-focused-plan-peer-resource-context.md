# ADR-0486: Phase 4NU Focused Same-Plan Peer Resource Context

Status: Accepted
Date: 2026-09-02

## Context

ADR-0481 made Resource basis recovery visible in brief ContextPacket
`verification_requirement` items for current Tasks. ADR-0483 extended the same
recovery information to focused `blocked_dependency` items when the blocking
prerequisite Task has a Resource-backed Verification Requirement.

The next current dogfood probe exposed a narrower focused-context gap. In a
temporary Store with two direct sibling Tasks under the same Plan, the sibling
Task was runnable and had a current-head Resource-backed Verification
Requirement. With no focus, full context showed both runnable Tasks and the
sibling `verification_requirement` item with Resource recovery detail. When the
Session was focused on the current Task, brief, normal, and full context all
omitted the sibling Resource-backed VR; full context reported zero omitted
items, so the absence was not a packet-budget artifact.

The pre-change probe showed:

```text
log_dir=/tmp/workvcs-4nu-broader-context-resource-probe-20260902T080858Z
task_count=2
plan_task_containment_relations=2
unfocused_runnable_candidates=2
sibling_focus_baseline_resource_hint=true
unfocused_full_has_sibling_resource_basis=true
unfocused_full_has_sibling_refresh_hint=true
current_brief_has_sibling_resource_basis=false
current_normal_has_sibling_resource_basis=false
current_full_has_sibling_resource_basis=false
current_full_has_sibling_refresh_hint=false
current_full_context_omitted_items=0
gap_observed=true
```

## Decision

When a ContextPacket is generated for `normal` or `full` profile and the active
Session focus is a Task, the context resolver now also inspects workspace-wide
runnable candidates and appends Resource-backed Verification Requirement items
for runnable peer Tasks that share the focused Task's direct parent Plan.

The peer recovery item reuses the existing `verification_requirement` category,
subject, and Resource basis summary renderer. The appended summary text starts
with focused peer provenance:

```text
same_plan_peer_task=<peer_task_entity_id>
parent_plan=<plan_entity_id>
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

The implementation keeps focus semantics unchanged for runnable selection,
`claim next`, `next`, and overview output. The workspace-wide runnable
projection is used only as a read-only context resolver input. Brief context
remains focused and does not include same-Plan peer Resource hints.

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
- No cross-Plan, ancestor/descendant, multi-hop, or full Resource traversal.
- No broad Resource resolver, Resource discovery, automatic refresh, background
  daemon, watcher, polling, or implicit re-observation.
- No `why` behavior change.
- No full relation-subject traversal, multi-hop/full evolution traversal, or
  broader causal traversal.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused implementation tests passed:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_same_plan_peer_resource_recovery_when_focused_on_task -- --exact --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_normal_exposes_same_plan_peer_resource_basis_recovery_hint -- --nocapture
1 passed; 0 failed
```

Adjacent 4NN and 4NP regressions passed:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_current_task_requirements -- --exact --nocapture
1 passed; 0 failed

cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements -- --exact --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_requirement -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_blocked_dependency -- --nocapture
1 passed; 0 failed
```

The post-change public CLI dogfood used a temporary Store and the newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nu-focused-plan-peer-resource-context-20260902T084000Z/post-dogfood
phase4nu_dogfood=PASS
unfocused_runnable_candidates=2
brief_has_same_plan_peer_hint=false
normal_has_same_plan_peer_hint=true
normal_has_peer_refresh_hint=true
full_has_same_plan_peer_hint=true
full_has_peer_refresh_hint=true
normal_verification_requirement_items=1
full_verification_requirement_items=1
```

Detailed evidence is recorded in
`docs/provenance/phase-4nu-focused-plan-peer-resource-context.md`.

## Consequences

A continuation operator focused on one Task can now use normal or full context
to notice a runnable same-Plan peer's Resource-backed Verification Requirement
and recover its basis-aware refresh path without clearing focus or separately
inspecting verification detail.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4NU closes one concrete focused same-Plan peer Resource
context gap, but broader context/Resource resolver maturity outside this
direct peer slice, full relation-subject traversal, multi-hop/full evolution
traversal, broader causal traversal, release-candidate validation, and release
authorization remain open.
