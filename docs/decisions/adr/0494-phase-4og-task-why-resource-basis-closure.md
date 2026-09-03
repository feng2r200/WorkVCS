# ADR-0494: Phase 4OG Task Why Resource Basis Closure

Status: Accepted
Date: 2026-09-03

## Context

Phase 4OE made current VR-backed Verification closure chains visible from
Task and Acceptance Criterion `why`. The follow-up Phase 4OF public CLI probe
showed that this was still incomplete for Resource-backed closeout recovery:
the same Task and Acceptance Criterion `why` output showed the AC, VR,
Verification, result, and Evidence ids, but omitted the Resource basis already
visible through `verification show`.

The corrected pre-change probe showed:

```text
log_dir=/tmp/workvcs-4of-next-gap-probe-20260903T112544Z
probe_execution_status=PASS
probe_result=GAP_FOUND
gap_kind=task_ac_why_omits_resource_basis_for_resource_backed_closeout
task_why_has_resource_basis=false
ac_why_has_resource_basis=false
verification_show_resource_basis=true
verification_show_resource_id=true
verification_show_observation_id=true
task_why_closure=true
ac_why_closure=true
task_why_has_verification=true
ac_why_has_verification=true
task_why_has_evidence=true
ac_why_has_evidence=true
```

The gap was not a missing Resource observation, cache-refresh, Verification,
or Evidence path. It was only the absence of Resource basis projection in the
existing Task/Acceptance Criterion `why` closure chain.

## Decision

`workvcs why --entity <Task>` and
`workvcs why --entity <AcceptanceCriterion>` now include the
Resource basis entries from each Resource-backed Verification already present
in `verification_closure_chains`.

Each closure chain now reports:

- Acceptance Criterion entity id, current version id, and local key.
- Verification Requirement entity id, current version id, and local key.
- Verification commit id, entity id, entity version id, and result.
- Evidence ids cited by the Verification.
- Resource basis entries copied from the Verification state: resource id,
  adapter kind/version, scope kind/version, canonical scope payload JSON,
  baseline observation id, and baseline fingerprint.

The projection remains read-only and uses the existing
`VerificationResourceBasis` state. It does not change stored semantics.

## Non-Goals

- No Store schema, migration, snapshot schema, or ContextPacket change.
- No new CLI flag, new command, or `why --relation` subject.
- No broad relation traversal, full relation-subject traversal, multi-hop/full
  evolution traversal, or broader causal traversal.
- No broader context/Resource resolver behavior.
- No Verification, Evidence, Acceptance Criterion status, Task closeout,
  Resource observation, applicability, or cache-refresh semantic change.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed:

```text
log_dir=/tmp/workvcs-4og-resource-basis-why-closure-20260903T113715Z
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_resource_basis_in_vr_backed_verification_closure_from_task_and_criterion=PASS
cargo test -p workvcs-cli tests::cli_why_projects_resource_basis_in_vr_backed_verification_closure=PASS
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4og-resource-basis-why-closure-20260903T113715Z
phase4og_dogfood=PASS
commands_exit_failures=0
failed_assertions=0
task_show_done=true
ac_status_verified=true
ac_status_after_closeout=stale
verify_resource_basis=true
verify_resource_observation=true
verification_show_resource_basis=true
task_why_resource_basis=true
ac_why_resource_basis=true
task_why_scope_payload=true
ac_why_scope_payload=true
task_why_baseline_observation=true
ac_why_baseline_observation=true
task_why_baseline_fingerprint=true
ac_why_baseline_fingerprint=true
release_flag_positive_hits=0
```

Final validation passed:

```text
log_dir=/tmp/workvcs-4og-resource-basis-why-closure-20260903T113715Z/final-validation
validation_status=PASS
validation_passes=14
validation_failures=0
```

Detailed evidence is recorded in
`docs/provenance/phase-4og-task-why-resource-basis-closure.md`.

## Consequences

A Resource-backed Task closeout operator can now query the closed Task or its
Acceptance Criterion and see the Resource basis needed to identify the
underlying resource, scope, baseline observation, and fingerprint without a
separate `verification show` lookup.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4OG closes one concrete Resource-backed Task/Acceptance
Criterion closeout explanation gap, but full relation-subject traversal beyond
direct endpoint slices, multi-hop/full evolution traversal, broader causal
traversal, broader context/Resource resolver maturity, and release-candidate
validation remain open.
