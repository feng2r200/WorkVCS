# ADR-0481: Phase 4NN Context Resource Basis Packet Recovery

Status: Accepted
Date: 2026-09-02

## Context

Phase 4MN made Resource-basis refresh usable through
`verification cache-refresh --all-resource-backed --resource-content-from-basis`,
and Phase 4NA established explicit foreground operator-triggered refresh as
the V1-local re-observation scheduling policy. The next concrete
context/Resource resolver dogfood gap was narrower: `workvcs context --profile
brief` could show a current Task's Verification Requirement, but the item did
not tell a continuation operator which Resource-backed Verification to refresh
or which Resource basis made the requirement stale/recoverable.

The pre-change public CLI probe created a current Task with a Resource-backed
Verification targeting a Verification Requirement. Brief context output
included the `verification_requirement` item, but its `summary_json` contained
only `criterion`, `local_key`, and statement text. It omitted `resource_id`,
adapter/scope identity, `baseline_observation_id`, and the
`verification cache-refresh --resource-content-from-basis` recovery path.

## Decision

Brief ContextPacket `verification_requirement` items now append Resource basis
recovery detail when a current-head Verification with non-empty Resource basis
targets that Verification Requirement.

The appended summary fragment is deterministic and uses existing stored
Verification state:

```text
resource_basis=<count>
verification_id=<verification_entity_id>
resource_id=<first_basis_resource_id>
adapter=<adapter_kind>@<adapter_schema_version>
scope=<scope_kind>@<scope_schema_version>
baseline_observation_id=<first_basis_baseline_observation_id|none>
refresh_hint="verification cache-refresh --verification <verification_entity_id> --resource-content-from-basis"
```

When multiple current-head Resource-backed Verifications target the same
Verification Requirement, `resource_basis=<count>` counts all Resource basis
entries across those matching Verifications. The displayed `verification_id`
and first basis details prefer the first matching Verification whose Resource
basis entries all have persisted baseline observations, so the recovery hint is
immediately runnable. If no matching Verification is fully basis-refreshable,
the summary falls back to the first Resource-backed Verification for diagnosis.

The ContextPacket schema is unchanged. The Resource basis information remains
summary text on the existing `summary` field; no new packet item fields, Store
schema objects, public `ContextOverview` fields, CLI commands, or CLI flags are
introduced.

If the selected Resource-backed Verification has a basis entry without a
baseline observation, the summary reports
`refresh_hint=unavailable_missing_baseline_observation` instead of emitting a
command that cannot refresh from persisted basis.

## Non-Goals

- No Store schema change.
- No ContextPacket schema or snapshot schema change.
- No public `ContextOverview` shape or CLI overview output change.
- No Resource observation, applicability, cache refresh, or Branch-head
  mutation semantics change.
- No new CLI command, flag, output field, or expectation flag.
- No automatic refresh, background daemon, watcher, polling, or implicit
  re-observation.
- No `why` traversal change.
- No release-candidate, release, Push, tag, deployment, remote, or V2 action.

## Evidence

Pre-change CLI probe:

```text
context_items=6
context_item.4.category=verification_requirement
context_item.4.summary_json="verification requirement criterion=... local_key=VR-1: Refresh from the local file Resource basis"
has_resource_basis_in_context=false
```

Focused tests failed before implementation and passed after implementation:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_summarizes_resource_basis_recovery_for_current_task_requirements -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_brief_exposes_resource_basis_recovery_hint_for_requirement -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-cli cli_context_packet_renders_resource_basis_recovery_hint_summary -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_prefers_refreshable_resource_basis_for_requirement_summary -- --nocapture
1 passed; 0 failed

cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_includes_current_task_verification_obligations -- --nocapture
1 passed; 0 failed
```

The post-change public CLI dogfood run used
`/tmp/workvcs-phase4nn-dogfood.XS378N` and proved:

```text
phase4nn_dogfood=PASS
context_items=6
resource_id=01a0600e-4627-7012-953a-ade7055ab82f
baseline_observation_id=01a0600e-4638-7761-88e2-d5e3699c08ff
verification_id=01a0600e-464e-7561-be8a-fcce6e72b9f0
summary_json="verification requirement ... resource_basis=1 verification_id=01a0600e-464e-7561-be8a-fcce6e72b9f0 resource_id=01a0600e-4627-7012-953a-ade7055ab82f adapter=local-file@1 scope=path@1 baseline_observation_id=01a0600e-4638-7761-88e2-d5e3699c08ff refresh_hint=\"verification cache-refresh --verification 01a0600e-464e-7561-be8a-fcce6e72b9f0 --resource-content-from-basis\""
```

Detailed evidence is recorded in
`docs/provenance/phase-4nn-context-resource-basis-packet.md`.

The final local validation matrix passed with:

```text
phase4nn_validation=PASS
log_dir=/tmp/workvcs-phase4nn-validation2.Ft5ftR
```

## Consequences

Continuation operators can now recover a Resource-backed Verification
Requirement from brief context alone when the current Task already has a
Resource-backed Verification. The output tells them which Verification id to
refresh, which Resource basis is involved, and which explicit basis-aware
refresh command to run.

The Context resolver, packets, and `why` explanations release gate remains
`Partial` and blocking. Phase 4NN closes one concrete current-task
Resource-backed VR recovery hint gap, but broader context/Resource resolver
maturity, full relation-subject traversal, multi-hop/full evolution traversal,
broader causal traversal, release-candidate validation, and release
authorization remain open.
