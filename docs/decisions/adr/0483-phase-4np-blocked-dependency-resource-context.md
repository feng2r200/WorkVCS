# ADR-0483: Phase 4NP Blocked Dependency Resource Context

Status: Accepted
Date: 2026-09-02

## Context

ADR-0481 made Resource basis recovery visible in brief ContextPacket
`verification_requirement` items for current Tasks. A follow-up dogfood probe
exposed the next bounded context/Resource resolver gap: when a Session is
focused on a dependent Task blocked by a prerequisite Task, the brief packet
shows a `blocked_dependency` item for the prerequisite, but the prerequisite's
Resource-backed Verification Requirement does not appear as a separate current
Task VR item.

Before this slice, a focused dependent context reported one blocked dependency
and no recovery hint:

```text
blocked_item_count=1
verification_requirement_item_count=0
prereq_vr_visible_in_focused_context=false
context_has_any_refresh_hint=false
blocked_dependency_has_resource_basis=false
blocked_dependency_has_refresh_hint=false
```

The blocked dependency summary named only the dependent Task, dependency Task,
dependency status, dependency priority, and dependency description.

## Decision

Brief ContextPacket `blocked_dependency` item summaries now append a bounded
Resource-backed Verification Requirement recovery hint for the blocking
dependency Task when that dependency has current-head Resource-backed
Verifications through its own Acceptance Criteria and Verification
Requirements.

The appended summary text includes:

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

The implementation reuses the existing current-head Resource-backed
Verification grouping and the existing Resource basis summary renderer used by
current-task `verification_requirement` items. It validates the dependency
Task's AC/VR references before using them. If multiple dependency VRs have
Resource-backed Verifications, the summary counts all matching requirements
and renders details for the first deterministic dependency Task AC/VR traversal
item, while the nested Resource-basis renderer still prefers a
basis-refreshable Verification for the displayed command.

The ContextPacket schema, item category, item subject, CLI command surface, and
packet persistence format are unchanged. The recovery information remains
summary text.

## Non-Goals

- No Store schema change.
- No ContextPacket JSON field addition.
- No CLI command or flag surface change.
- No `verify` semantics change.
- No `verification cache-refresh` semantics change.
- No Resource observation, Resource binding, or applicability policy change.
- No runnable, dependency scheduling, or task selection semantics change.
- No broader context ranking or traversal change.
- No `why` behavior change.
- No broader causal, multi-hop, or relation-subject traversal change.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

Focused tests failed before implementation:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_blocked_dependency_requirements -- --exact --nocapture
status=101
failure=assertion failed: blocker.summary.contains("dependency_resource_requirements=1")

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_blocked_dependency -- --nocapture
status=101
failure=assertion failed: summary_json.contains("dependency_resource_requirements=1")
```

Focused validation passed after implementation:

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

The public CLI dogfood run used a temporary Store and a newly built binary:

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

Detailed evidence is recorded in
`docs/provenance/phase-4np-blocked-dependency-resource-context.md`.

## Consequences

A continuation operator focused on a dependency-blocked Task can now recover
the prerequisite Task's Resource-backed Verification path from the existing
brief `blocked_dependency` item, without first opening separate verification
detail.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Broader context/Resource resolver behavior beyond
current-task and focused blocked-dependency Resource recovery hints, full
relation-subject traversal beyond the direct create/remove/restore endpoint
slices, multi-hop/full evolution traversal, broader causal traversal,
release-candidate validation, and release authorization remain open.
