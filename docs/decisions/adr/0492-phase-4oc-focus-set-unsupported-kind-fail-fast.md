# ADR-0492: Phase 4OC Focus-Set Unsupported Kind Fail-Fast

Status: Accepted
Date: 2026-09-03

## Context

ADR-0031 requires unsupported Focus entity kinds to be rejected as structured
runtime errors instead of guessed. ADR-0207 keeps `session focus-set`
validation inside the existing runtime Engine path.

The Phase 4OC probe found that `session focus-set` accepted a current
`verification_requirement` entity as stored Session focus because the write
path only checked WorkState presence. Downstream `context` and `runnable`
commands then failed with `session_invalid` because runtime focused-scope
resolution supports only current Goal, Plan, and Task focus entities.

The corrected pre-change probe showed:

```text
log_dir=/tmp/workvcs-4oc-vr-focus-resource-context-probe-20260903T081318Z
phase4oc_corrected_probe_status=PASS
probe_result=GAP_FOUND
gap_kind=session_focus_contract_mismatch
commands_exit_failures=4
control_unfocused_full_vr_basis_hint=true
control_task_focus_brief_vr_basis_hint=true
session_focus_set_accepts_verification_requirement=true
context_vr_focus_returns_session_invalid=true
runnable_vr_focus_returns_session_invalid=true
failure_fragment=session invalid: focus entity is not a current Goal, Plan, or Task
```

## Decision

`session focus-set` now validates the requested focus entity kind before any
focus rows or focus event are written. After confirming the entity is present
at the active Branch head, the runtime checks the same current-head Goal,
Plan, and Task semantics used by focused runnable projection. If the current
entity is not a Goal, Plan, or Task, the command returns `SessionInvalid` with
the existing message shape:

```text
focus entity <entity_id> is not a current Goal, Plan, or Task at branch head <commit_id>
```

The persisted focus selection validation used when reading existing Session
state remains a presence check so historical runtime rows are not migrated or
rewritten by this slice.

## Non-Goals

- No support for Verification Requirement, Acceptance Criterion, Verification,
  Evidence, Record, Knowledge, or KnowledgeExposure as Session focus.
- No Store schema change, migration, or persisted Session row rewrite.
- No Session focus path structure validation change.
- No ContextPacket schema, snapshot schema, item field, CLI flag, or command
  surface change.
- No `context`, `runnable tasks`, `claim next`, `next`, Claim, Resource,
  verification, or `why` behavior change beyond rejecting the bad focus write.
- No broad context/Resource resolver, full relation-subject traversal,
  multi-hop/full evolution traversal, broader causal traversal, release,
  release-candidate, Push, tag, deployment, remote, cloud, V2, GUI/TUI,
  distributed collaboration, or Agent orchestration action.

## Evidence

Focused validation passed after implementation:

```text
log_dir=/tmp/workvcs-4oc-focus-set-unsupported-kind-20260903T082308Z/focused
core_vr_focus_reject=PASS
cli_vr_focus_reject=PASS
cargo_build_cli=PASS
```

The public CLI dogfood run used a temporary Store and the built
`target/debug/workvcs` binary:

```text
log_dir=/tmp/workvcs-4oc-focus-set-unsupported-kind-20260903T082308Z/dogfood
phase4oc_dogfood=PASS
commands_exit_failures=0
failed_assertions=0
focus_set_vr_exit=expected_failure
focus_set_vr_error_code=session_invalid
focus_set_vr_error_category=runtime
focus_set_vr_message_contains_supported_kind=true
session_after_reject_focus=none
context_unfocused_succeeded=true
task_focus_succeeded=true
context_task_focus_has_requirement=true
context_task_focus_has_resource_basis=true
context_task_focus_has_refresh_hint=true
```

Final validation passed:

```text
log_dir=/tmp/workvcs-4oc-focus-set-unsupported-kind-20260903T084201Z/final-validation
validation_status=PASS
validation_passes=16
validation_failures=0
```

Detailed evidence is recorded in
`docs/provenance/phase-4oc-focus-set-unsupported-kind-fail-fast.md`.

## Consequences

Operators can no longer persist a current but unsupported entity, such as a
Verification Requirement, as the active Session focus and then discover the
error only when running `context` or `runnable tasks`. The failing command now
returns a stable runtime error immediately, leaves the Session focus unchanged,
and preserves the existing valid Task-focus path for Resource-backed
Verification Requirement recovery hints.

The Session, Claim, Runnable, `claim next`, and `next` release gate remains
`Pass`. The Context resolver, packets, and `why` explanations gate remains
`Partial` because Phase 4OC only closes this concrete focus contract mismatch.
The overall release decision remains false.
