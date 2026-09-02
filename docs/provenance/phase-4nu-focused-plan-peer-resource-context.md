# Phase 4NU Focused Same-Plan Peer Resource Context Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NU advances the Context resolver, packets, and `why` explanations
release gate by making Resource-backed Verification Requirement recovery
visible in `normal` and `full` ContextPacket output for runnable peer Tasks
that share the focused Task's direct parent Plan.

The slice is intentionally narrow. It does not change Store schema,
ContextPacket JSON fields, context packet snapshot schema, CLI flags, `verify`
semantics, Resource observation or cache-refresh semantics, runnable selection,
Claim behavior, `claim next`, `next`, `why` traversal, release state, Push
state, tags, remote state, deployment state, V2 scope, GUI/TUI behavior,
distributed collaboration, or Agent orchestration.

## Gap Proof

The pre-change public CLI probe used a temporary local Store under:

```text
log_dir=/tmp/workvcs-4nu-broader-context-resource-probe-20260902T080858Z
```

The probe created:

```text
one Goal
one Plan
two direct sibling Tasks under that Plan
one Resource-backed Verification Requirement on the sibling Task
one active Session on the same Branch
```

Observed before implementation:

```text
phase4nu_probe=PASS
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

The same sibling Resource-backed VR was visible when either the sibling Task
was focused or no focus was set. It disappeared when focus moved to the current
Task, even in full context with zero omitted items. That made the gap a focused
context resolver gap rather than a budget or renderer gap.

## Implementation

Changed:

```text
crates/workvcs-core/src/runtime/runnable.rs
crates/workvcs-core/src/runtime/context.rs
crates/workvcs-core/tests/context_profile_budget_phase4kx.rs
crates/workvcs-cli/src/main.rs
```

`RunnableTasksOptions` now has an internal `workspace_wide_scope` mode. The
default remains focus-aware, so existing `runnable tasks`, `claim next`, and
`next` behavior is unchanged. ContextPacket generation uses the workspace-wide
projection only when the requested profile is `normal` or `full` and the active
focus is a Task.

The context resolver then compares workspace-wide runnable candidates with the
focused Task's direct parent Plan. For runnable peer Tasks under the same
direct parent Plan, it validates the peer Task's AC/VR references and appends
only Resource-backed Verification Requirement items. The item uses the existing
`verification_requirement` category and subject, with `P2` priority, and
reuses the existing Resource basis recovery summary renderer.

The summary starts with:

```text
same_plan_peer_task=<peer_task_id>
parent_plan=<plan_id>
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

Brief context remains focused and does not include same-Plan peer Resource
hints.

## Focused Validation

The focused implementation tests passed:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_same_plan_peer_resource_recovery_when_focused_on_task -- --exact --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_normal_exposes_same_plan_peer_resource_basis_recovery_hint -- --nocapture
1 passed; 0 failed
```

Adjacent Resource-context regressions passed:

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

## Dogfood Proof

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
normal_context_items=7
normal_verification_requirement_items=1
normal_resource_basis_summary_count=1
full_context_items=7
full_verification_requirement_items=1
full_resource_basis_summary_count=1
```

## Final Validation

The final local validation matrix for the implemented 4NU slice passed with
logs preserved under:

```text
log_dir=/tmp/workvcs-4nu-focused-plan-peer-resource-context-20260902T084000Z/final-validation
phase4nu_validation=PASS
```

The matrix covered:

```text
git status --short --branch
cargo fmt --all -- --check
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_same_plan_peer_resource_recovery_when_focused_on_task -- --exact --nocapture
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_current_task_requirements -- --exact --nocapture
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements -- --exact --nocapture
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

The smoke log ended with:

```text
smoke status=0
```

## Interpretation

Phase 4NU closes one concrete focused same-Plan peer Resource recovery gap: a
focused operator can keep current-task focus while normal or full context still
surfaces a runnable same-Plan peer's Resource-backed Verification Requirement
and basis-aware refresh command.

The proof is bounded. It does not show cross-Plan Resource traversal,
ancestor/descendant Resource traversal, multi-hop context traversal, broad
Resource resolver maturity, `why` traversal, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, or release
candidate readiness.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NU advances the gate by closing one focused same-Plan runnable peer
Resource recovery gap beyond current-task and focused blocked-dependency
Resource-backed VR hints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
