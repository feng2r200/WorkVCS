# ADR-0493: Phase 4OE Task Closeout Why Closure Chain

Status: Accepted
Date: 2026-09-03

## Context

Phase 4NV, Phase 4NW, and Phase 4NX made the direct `verifies` and
`evidenced_by` endpoints explainable through `workvcs why`. That proved the
Verification Requirement, Verification, and Evidence endpoint sides, but it
left a Task closeout operator with repeated id plumbing: querying the closed
Task or its Acceptance Criterion did not expose the current VR-backed
Verification and Evidence closure chain.

The corrected pre-change public CLI probe showed:

```text
log_dir=/tmp/workvcs-4oe-task-closeout-why-chain-probe-20260903T100242Z
probe_execution_status=PASS
probe_result=GAP_FOUND
task_show_after_closeout_status=done
ac_status_after_verify=verified
task_why_has_ac=false
task_why_has_vr=false
task_why_has_verification=false
task_why_has_evidence=false
ac_why_has_vr=false
ac_why_has_verification=false
ac_why_has_evidence=false
control_vr_verifies=PASS
control_verification_evidenced_by=PASS
control_evidence_evidenced_by=PASS
```

The controls showed that the direct relation endpoint behavior was intact.
The gap was only the lack of an immediate Task/Acceptance Criterion projection
for the current closeout chain.

## Decision

`workvcs why --entity <Task>` and
`workvcs why --entity <AcceptanceCriterion>` now include
`verification_closure_chains` for current-head AC -> VR -> Verification ->
Evidence closure chains.

Each chain reports:

- Acceptance Criterion entity id, current version id, and local key.
- Verification Requirement entity id, current version id, and local key.
- Verification commit id, entity id, entity version id, and result.
- Evidence ids cited by the Verification.

The projection is derived read-only from current WorkState snapshots at the
queried commit. It does not add a relation-subject query surface and does not
change stored semantics.

## Non-Goals

- No Store schema, migration, snapshot schema, or ContextPacket change.
- No new CLI flag or `why --relation` subject.
- No broad relation traversal, full relation-subject traversal, multi-hop/full
  evolution traversal, or broader causal traversal.
- No broader context/Resource resolver behavior.
- No Verification, Evidence, Acceptance Criterion status, Task closeout,
  Resource observation, or cache-refresh semantic change.
- No release-candidate, release, Push, tag, deployment, remote, cloud, V2,
  GUI/TUI, distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed:

```text
log_dir=/tmp/workvcs-4oe-implementation-20260903T101500Z
cargo test -p workvcs-core --test why_verification_neighborhood_phase3u why_reports_vr_backed_verification_closure_from_task_and_criterion_after_task_closeout=PASS
cargo test -p workvcs-cli cli_why_projects_vr_backed_verification_closure_from_task_and_criterion=PASS
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4oe-task-closeout-why-closure-chain-20260903T105642Z
phase4oe_dogfood=PASS
commands_exit_failures=0
failed_assertions=0
task_show_done=true
ac_status_verified=true
task_why_verification_closure_chains=true
ac_why_verification_closure_chains=true
task_why_chain_has_ac=true
task_why_chain_has_vr=true
task_why_chain_has_verification=true
task_why_chain_has_evidence=true
ac_why_chain_has_vr=true
ac_why_chain_has_verification=true
ac_why_chain_has_evidence=true
control_vr_verifies_relation_edges=true
control_vr_verifies_evolution_ops=true
control_verification_evidenced_by_relation_edges=true
control_verification_evidenced_by_evolution_ops=true
control_evidence_evidenced_by_relation_edges=true
control_evidence_evidenced_by_evolution_ops=true
```

Final validation passed:

```text
log_dir=/tmp/workvcs-4oe-task-closeout-why-closure-chain-20260903T105642Z/final-validation
validation_status=PASS
validation_passes=14
validation_failures=0
```

Detailed evidence is recorded in
`docs/provenance/phase-4oe-task-closeout-why-closure-chain.md`.

## Consequences

A Task closeout operator can now query the closed Task or the relevant
Acceptance Criterion and see the current VR-backed Verification and cited
Evidence ids without manually walking through separate `why` endpoint queries.

The Context resolver, packets, and `why` explanations release gate remains
`Partial`. Phase 4OE closes one concrete Task/Acceptance Criterion closeout
explanation gap, but full relation-subject traversal beyond direct endpoint
slices, multi-hop/full evolution traversal, broader causal traversal, broader
context/Resource resolver maturity, and release-candidate validation remain
open.
