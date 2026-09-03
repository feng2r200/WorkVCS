# Phase 4NZ Focused Same-Goal Cross-Plan Peer Resource Context Evidence

Status: current local evidence
Date: 2026-09-03

## Scope

Phase 4NZ advances the Context resolver, packets, and `why` explanations
release gate by making Resource-backed Verification Requirement recovery
visible in `normal` and `full` ContextPacket output for runnable peer Tasks
whose direct parent Plans share the same direct parent Goal as the focused
Task's direct parent Plan.

The slice is intentionally narrow. It does not change Store schema,
ContextPacket JSON fields, context packet snapshot schema, CLI flags, `verify`
semantics, Resource observation or cache-refresh semantics, runnable selection,
Claim behavior, `claim next`, `next`, `why` traversal, release state, Push
state, tags, remote state, deployment state, V2 scope, GUI/TUI behavior,
distributed collaboration, or Agent orchestration.

## Gap Proof

The pre-change public CLI probe used a temporary local Store under:

```text
log_dir=/tmp/workvcs-4nz-current-context-why-gate-probe-20260903T070239Z
```

The probe created:

```text
one Goal
two direct child Plans under that Goal
one runnable Task under each Plan
one Resource-backed Verification Requirement on the cross-Plan peer Task
one active focused Session on the current Task
```

Observed before implementation:

```text
phase4nz_probe=PASS
scenario=same_goal_different_plan_runnable_peer_resource_backed_vr
commands_exit_failures=0
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
claim_next_selected=true
claim_next_packet_has_cross_plan_vr_key=false
claim_next_packet_has_cross_plan_resource_basis=false
claim_next_packet_has_cross_plan_refresh_hint=false
cross_plan_gap_reproduced=true
```

The same Resource-backed VR was visible when either the peer Task was focused
or no focus was set. It disappeared when focus moved to the current Task, even
in full context with zero omitted items and in the focused `claim next`
context packet. That made the gap a focused context resolver gap rather than a
budget or renderer gap.

## Implementation

Changed:

```text
crates/workvcs-core/src/runtime/context.rs
crates/workvcs-core/tests/context_profile_budget_phase4kx.rs
crates/workvcs-cli/src/main.rs
docs/decisions/adr/0491-phase-4nz-focused-same-goal-cross-plan-resource-context.md
docs/provenance/phase-4nz-focused-same-goal-cross-plan-resource-context.md
docs/provenance/v1-readiness-ledger.md
docs/provenance/v1-release-gate-matrix.md
```

`normal` and `full` ContextPacket generation already used a read-only
workspace-wide runnable projection when the active focus is a Task. Phase 4NZ
keeps that boundary and extends the peer filter by one direct containment
level: after preserving ADR-0486 same-Plan peer behavior, it also accepts
runnable peer Tasks whose direct parent Plan has the same direct parent Goal as
the focused Task's direct parent Plan.

The same-Goal peer summary starts with:

```text
same_goal_peer_task=<peer_task_id>
parent_goal=<goal_id>
focused_plan=<focused_plan_id>
peer_plan=<peer_plan_id>
peer_runnable=true
criterion=<acceptance_criterion_id>
local_key=<verification_requirement_local_key>
```

Then it appends:

```text
resource_basis=<count>
verification_id=<verification_id>
resource_id=<resource_id>
adapter=<kind>@<version>
scope=<kind>@<version>
baseline_observation_id=<observation_id|none>
refresh_hint="verification cache-refresh --verification <id> --resource-content-from-basis"
```

Brief context remains focused and does not include same-Plan or same-Goal peer
Resource hints.

No Store schema, CLI flag, public output field, snapshot schema, runnable
selection, Claim behavior, `claim next` behavior, `next` behavior, Resource
observation semantics, or `why` behavior changed.

## Focused Validation

Focused tests passed after implementation:

```text
log_dir=/tmp/workvcs-4nz-focused-tests-20260903T071229Z
core_new_test=PASS
cli_new_test=PASS
```

## Dogfood Proof

The post-change public CLI dogfood used a temporary Store and the newly built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4nz-focused-same-goal-cross-plan-resource-context-20260903T071452Z/dogfood
phase4nz_dogfood=PASS
commands_exit_failures=0
unfocused_runnable_candidates=2
unfocused_candidates_match_expected=true
context_unfocused_full_has_cross_plan_vr_key=true
context_peer_focus_brief_has_cross_plan_vr_key=true
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
claim_next_inspected_candidates=1
claim_next_context_omitted_items=0
claim_next_packet_has_cross_plan_vr_key=true
claim_next_packet_has_cross_plan_resource_basis=true
claim_next_packet_has_cross_plan_refresh_hint=true
claim_next_same_goal_peer_hint_count=1
why_current_primary_relation_edges=1
why_current_primary_evolution_change_operations=1
why_peer_primary_relation_edges=1
why_peer_primary_evolution_change_operations=2
why_peer_verifies_relation_edges=1
why_peer_verifies_evolution_change_operations=1
phase4nz_dogfood_assertion=PASS
```

## Final Validation

Final validation passed:

```text
log_dir=/tmp/workvcs-4nz-focused-same-goal-cross-plan-resource-context-20260903T071452Z/final-validation
validation_status=PASS
```

The matrix covered:

```text
git status --short --branch
cargo fmt --all -- --check
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_same_goal_cross_plan_peer_resource_recovery_when_focused_on_task -- --exact --nocapture
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_same_plan_peer_resource_recovery_when_focused_on_task -- --exact --nocapture
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_current_task_requirements -- --exact --nocapture
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements -- --exact --nocapture
cargo test -p workvcs-cli cli_context_normal_exposes_same_goal_cross_plan_peer_resource_basis_recovery_hint -- --nocapture
cargo test -p workvcs-cli cli_context_normal_exposes_same_plan_peer_resource_basis_recovery_hint -- --nocapture
cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_requirement -- --nocapture
cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_blocked_dependency -- --nocapture
scripts/validate-schema-v0.1.sh
release flag scan for accidental positive release-flag assignments
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --quiet
cargo build -p workvcs-cli
post-change public CLI dogfood
scripts/smoke-v0.1-cli-workflow.sh
workctl work status
git status --short --branch
```

## Interpretation

Phase 4NZ closes one concrete focused same-Goal cross-Plan peer Resource
recovery gap: a focused operator can keep current-task focus while normal or
full context still surfaces a runnable same-Goal peer Task's Resource-backed
Verification Requirement and basis-aware refresh command. The same recovery
summary is visible in the full context packet emitted by focused
`claim next --context-profile full`.

The proof is bounded. It does not show cross-Goal Resource traversal,
ancestor/descendant Resource traversal, nested-Plan traversal, multi-hop
context traversal, broad Resource resolver maturity, `why` traversal, full
relation-subject traversal, multi-hop/full evolution traversal, broader causal
traversal, or release-candidate readiness.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NZ advances the gate by closing one focused same-Goal cross-Plan
runnable peer Resource recovery gap beyond current-task, focused
blocked-dependency, and focused same-Plan peer Resource-backed VR hints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
