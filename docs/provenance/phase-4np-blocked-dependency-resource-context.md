# Phase 4NP Blocked Dependency Resource Context Evidence

Status: current local evidence
Date: 2026-09-02

## Scope

Phase 4NP advances the Context resolver, packets, and `why` explanations
release gate by making prerequisite Resource-backed Verification recovery
visible in brief `blocked_dependency` items when the current Session is focused
on a dependency-blocked Task.

The slice is intentionally narrow. It does not change Store schema,
ContextPacket JSON fields, CLI flags, `verify` semantics, cache-refresh
semantics, Resource observation policy, runnable or dependency scheduling
semantics, broader context ranking, `why` behavior, release state, Push state,
tags, remote state, deployment state, V2 scope, GUI/TUI behavior, or Agent
orchestration.

## Gap Proof

The pre-change gap used a temporary local Store and public CLI commands.

Scenario:

```text
create dependent Task
create prerequisite Task
add Acceptance Criterion and Verification Requirement to prerequisite Task
record a local-file Resource observation
record a Resource-backed Verification for the prerequisite VR
make dependent Task depend on prerequisite Task
focus a Session on the dependent Task
inspect workvcs context --profile brief --budget-items 20
```

Observed before implementation:

```text
log_dir=/tmp/workvcs-4np-blocked-dependency-resource-probe.KW8mNc
blocked_item_count=1
verification_requirement_item_count=0
prereq_vr_visible_in_focused_context=false
context_has_any_refresh_hint=false
blocked_dependency_has_resource_basis=false
blocked_dependency_has_refresh_hint=false
```

The focused context categories were:

```text
session_anchor
branch_overview
current_task
task_readiness
blocked_dependency
```

The pre-change `blocked_dependency` summary named only the dependent Task,
dependency Task, dependency status, dependency priority, and dependency
description:

```text
blocked dependency task=01a06064-c6d1-7222-8dfd-35e22b72b1bc dependency=01a06064-c6e8-7153-9307-e7c73972842b dependency_status=pending dependency_priority=0: Prerequisite has resource-backed verification
```

## Implementation

Changed:

```text
crates/workvcs-core/src/runtime/context.rs
crates/workvcs-core/tests/context_profile_budget_phase4kx.rs
crates/workvcs-cli/src/main.rs
```

The implementation preserves the existing blocked dependency identity and
status summary, then appends Resource recovery text only when the blocking
dependency Task has current-head Resource-backed Verifications through its own
Acceptance Criteria and Verification Requirements.

The appended text contains:

```text
dependency_resource_requirements=<count>
dependency_acceptance_criterion=<acceptance criterion id>
dependency_verification_requirement=<verification requirement id>
dependency_vr_local_key=<local key>
resource_basis=<count>
verification_id=<verification id>
resource_id=<resource id>
adapter=<kind>@<version>
scope=<kind>@<version>
baseline_observation_id=<observation id|none>
refresh_hint="verification cache-refresh --verification <id> --resource-content-from-basis"
```

The new helper validates the dependency Task's AC and VR references against
the current context snapshots. It counts all dependency VRs that have
Resource-backed Verifications and renders detail for the first deterministic
dependency Task AC/VR traversal item. Resource detail rendering reuses the
existing current-task VR summary helper, including the preference for a
basis-refreshable Verification when one is available.

## Focused Validation

Focused tests failed before implementation:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements -- --exact --nocapture
status=101
failure=assertion failed: blocker.summary.contains("dependency_resource_requirements=1")

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_blocked_dependency -- --nocapture
status=101
failure=assertion failed: summary_json.contains("dependency_resource_requirements=1")
```

Focused tests and adjacent regressions passed after implementation:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements -- --exact --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_blocked_dependency -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_current_task_requirements -- --exact --nocapture
1 passed; 0 failed

cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_explains_blocked_dependency_tasks -- --exact --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_requirement -- --nocapture
1 passed; 0 failed
```

## Dogfood Proof

The public CLI dogfood used a temporary local Store and a newly built
`target/debug/workvcs` binary.

```text
log_dir=/tmp/workvcs-4np-post-dogfood.NTxC4a
blocked_dependency_items=1
verification_requirement_items=0
has_dependency_resource_requirements=true
has_dependency_vr_id=true
has_dependency_vr_local_key=true
has_resource_basis=true
has_baseline_observation=true
has_basis_refresh_hint=true
```

The dogfood `blocked_dependency` summary was:

```text
blocked dependency task=01a06072-81fb-7ec3-a8fe-8642905197af dependency=01a06072-8210-74f0-b534-a6729e57f819 dependency_status=pending dependency_priority=0: Dogfood prerequisite has Resource-backed VR dependency_resource_requirements=1 dependency_acceptance_criterion=01a06072-8226-7270-933a-46e854671abe dependency_verification_requirement=01a06072-823d-76c1-b898-fbfb4fb999f9 dependency_vr_local_key=VR-dogfood-resource resource_basis=1 verification_id=01a06072-8277-7551-911e-94b199dc1df6 resource_id=01a06072-8251-76a0-9157-b5492c4936e7 adapter=local-file@1 scope=path@1 baseline_observation_id=01a06072-8262-7562-bcf5-0d98788e52b2 refresh_hint="verification cache-refresh --verification 01a06072-8277-7551-911e-94b199dc1df6 --resource-content-from-basis"
```

The context still reported `verification_requirement_items=0` because the
prerequisite Task was not the current focused Task. Phase 4NP intentionally
puts the recovery clue in the existing `blocked_dependency` item instead of
changing current-task selection or adding packet fields.

## Interpretation

Phase 4NP closes a concrete focused blocked-dependency Resource recovery gap:
the operator can see which prerequisite VR and verification cache-refresh
command to use from the continuation context itself.

The proof is still bounded. It does not show broader context ranking,
multi-dependency summarization beyond deterministic first-detail selection,
non-Task dependency Resource traversal, broad Resource resolver maturity,
`why` traversal, full relation-subject traversal, multi-hop/full evolution
traversal, or broader causal traversal.

## Release Gate Impact

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NP advances the gate by closing one focused blocked-dependency Resource
recovery gap beyond current-task Resource-backed VR hints.

The overall release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```
