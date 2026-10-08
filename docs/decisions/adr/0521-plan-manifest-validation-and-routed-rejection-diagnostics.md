# ADR-0521: Plan Manifest Validation and Routed Rejection Diagnostics

Status: Accepted; source implemented and locally validated
Date: 2026-10-08

## Context

ADR-0516 intentionally persists a registry-v2 Plan operation before the target
Store performs pure typed-manifest validation. A deterministic manifest error
therefore becomes the immutable terminal state `plan_manifest_rejected` with
zero target Store writes. This journal-first ordering is an audit and recovery
property, not an implementation accident.

During ADR-0520 work, a Plan manifest used the unsupported Record kind
`constraint`. WorkVCS correctly wrote no target Store state and recorded a
terminal failure, but the command returned only:

```text
durable Plan delivery ended in plan_manifest_rejected:
start_new_plan_operation_with_corrected_manifest
```

The response omitted the durable CaptureId and the validation cause already
known during that invocation. There was also no public zero-write command for
an Agent to validate the strict manifest before choosing durable admission.
Those gaps force avoidable failed intents and can push diagnosis back to the
user even though the responsible task has enough local evidence to act.

## Decision

### 1. Preserve journal-first terminal audit

`plan admit --cwd` and `plan evolve --cwd` keep the ADR-0516 sequence. The
command parses the strict JSON shape, durably admits the typed operation, and
then performs target-aware Plan preflight. If the caller skips validation or a
race/target conflict occurs, the immutable terminal event remains the source
of truth. Existing terminal CaptureIds are never deleted or silently retried.

### 2. Add an optional read-only manifest validator

The Plan CLI adds:

```text
workvcs plan validate --operation admit|evolve --manifest PATH
```

Validation invokes the same typed manifest parser and side-effect-free domain
validation used by routed delivery. It does not resolve a project, open a
Store, read or write a registry, inspect an activation marker, admit a journal
intent, or compare target-state guards. Success returns the operation kind,
mode when applicable, payload digest, and an idempotency-key digest; it never
renders the raw idempotency key or manifest content. Failure returns the normal
specific typed error and creates no durable operation.

This command validates intrinsic manifest semantics only. A valid result does
not prove that expected head/state/entity guards are still current and grants
no delivery authority.

### 3. Routed terminal rejection is Capture-aware and actionable

When the same `plan admit --cwd` or `plan evolve --cwd` invocation reaches a
durable `delivery_failed`, it returns structured
`capture_delivery_incomplete` rather than a generic
`control_plane_invalid`. The error includes:

- the exact durable CaptureId;
- `journal_persisted=true`;
- the terminal failure code;
- its canonical recovery action; and
- the bounded validation or target-conflict detail already produced by that
  invocation.

The detail is not added to the immutable event or global inventory. It may
echo a value from the caller-supplied manifest, exactly as a local validation
error can; it is therefore limited to the direct command response. A later
status/replay without the original in-memory preflight detail still reports the
stable failure code and recovery action from journal authority.

## Consequences

- Responsible Agents can prevent routine schema mistakes without producing a
  failed journal intent.
- If durable admission still terminalizes, the caller can recover by CaptureId
  and correct the exact error without user monitoring.
- Journal-first durability, target CAS checks, terminal monotonicity, and
  redacted inventory remain unchanged.
- Validation is not a substitute for admission, target preflight, or user
  authority.

## Validation

1. Valid admission and evolution manifests report stable read-only validation
   metadata with zero registry, marker, journal, projection, and Store writes.
2. Unsupported Record kinds and other intrinsic manifest failures return the
   same typed error as delivery preflight and leave no Capture.
3. Routed invalid manifests still create one terminal
   `plan_manifest_rejected` operation with zero target Store writes.
4. The routed error is `capture_delivery_incomplete` and includes the exact
   CaptureId, journal flag, terminal code, canonical recovery action, and
   same-invocation cause.
5. Exact replay does not create another intent or event and retains stable
   terminal status.
