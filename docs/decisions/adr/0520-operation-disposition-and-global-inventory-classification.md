# ADR-0520: Operation Disposition and Global Inventory Classification

Status: Accepted; source implemented and locally validated
Date: 2026-10-08

## Context

ADR-0513 made Capture intents and events immutable and projections rebuildable.
ADR-0515 added deterministic terminal delivery failures. ADR-0519 then added a
read-only `project operation-recovery --list-open` inventory whose compatibility
definition is intentionally broad: any operation whose current recovery action
is not `none` is included, even when the required follow-up is a new operation,
binding repair, isolation, or owner restoration rather than recovery of the
listed Capture itself.

That contract exposes durable work, but it does not provide an authoritative
way to end an admitted operation that became obsolete before target delivery.
Leaving such an intent with no terminal event makes every future inventory
classify it again from the current registry. A later registry revision can also
make a previously completed or deterministic-terminal projection appear
effectively `pending_project`, so the global list can mix four materially
different cases:

1. a Capture that can still converge under the same immutable intent;
2. a completed Capture whose current binding receipt needs reconciliation;
3. a deterministic terminal Capture that requires a new operation; and
4. an undelivered Capture that later work or a withdrawn requirement made
   obsolete.

The responsible task needs enough runtime truth to classify those cases. The
user must not be required to continuously inspect journal history, but neither
the binary nor an Agent may infer authority to sweep, retarget, bootstrap, or
delete historical operations.

## Decision

### 1. Disposition is immutable operation authority, not delivery failure

The journal adds `operation_disposition_recorded` with one disposition:

- `superseded`, which requires the CaptureId of a later durable operation; or
- `abandoned`, which explicitly states that no successor operation is being
  named.

The event contains no free-form rationale, semantic payload, path, credential,
or tool output. `superseded` requires a distinct successor Capture that exists
under the same registry-derived journal set and was admitted later than the
target. `abandoned` forbids a successor. The event is terminal, append-only,
digest chained, and idempotently reusable only when its complete payload is
identical.

This is an operator judgment. WorkVCS validates structural safety and
concurrency, not business equivalence. Governance must establish that a later
operation actually replaces the earlier intent, or that authority withdrew the
requirement, before invoking the mutation.

### 2. Only pre-delivery operations are eligible

The explicit CLI mutation is:

```text
workvcs project operation-recovery
    --dispose superseded|abandoned
    --capture-id CAPTURE_ID
    [--successor-capture-id CAPTURE_ID]
    --expected-registry-digest DIGEST
    --expected-projection-digest DIGEST
    [--registry PATH]
```

The registry and reconstructed projection digests are mandatory compare-and-
swap guards. WorkVCS rechecks both before and under the existing registry and
journal quiescence locks. The operation is eligible only when immutable
authority contains no `delivery_started`, `delivery_applied`, `delivery_failed`,
canonical CaptureGroup receipt, secondary-reference receipt, or CaptureGroup
completion receipt. Resolution, binding-ready, or CaptureGroup-owner events are
not target delivery and may precede disposition.

Any later event after disposition is invalid. A completed, failed, partially
delivered, or otherwise target-bearing operation cannot be hidden by
disposition; its existing recovery or follow-up contract remains authoritative.
The command never opens or mutates a semantic Store, changes the registry,
repairs a binding, activates routing, or starts another Capture.

### 3. Terminal projections cannot be resurrected by registry drift

The derived recovery states add `superseded` and `abandoned`. Both have
`recovery_action=none`. Deterministic delivery-failure states and explicit
dispositions are immutable-intent outcomes, so current registry re-resolution
cannot rewrite their effective state to `pending_project`. Completed delivery
receipts remain target-relative under INV-107: a changed or stale current
binding may still produce a different effective state and follow-up action.

### 4. The global inventory exposes lifecycle and both state views

`--list-open` retains ADR-0519 compatibility: it returns operations whose
current `recovery_action` is not `none`. Existing filters, ordering, row limits,
redaction, digest locking, and read-only guarantees remain intact.

The same surface adds `--list-all`, mutually exclusive with `--list-open`, to
enumerate every supported durable operation through the same bounded,
digest-locked paging protocol. Explicitly superseded and abandoned operations
therefore leave the open queue but remain globally auditable.

Every inventory row adds:

- `projected_recovery_state`, derived only from immutable intent/events;
- `effective_recovery_state`, after current routing and binding validation;
- `operation_lifecycle`: `open`, `completed`, `terminal_failed`,
  `superseded`, or `abandoned`;
- `operation_disposition` and optional `successor_capture_id`; and
- bounded receipt/failure identifiers already exposed by per-operation status.

Summary counts distinguish projected state, effective state, lifecycle, and
recovery action. The inventory digest version advances and binds the requested
scope (`open` or `all`) plus the complete allowlisted row set. A digest from
one scope is never valid for the other.

### 5. No automatic historical reconciliation

This change does not apply any currently listed historical operation. It adds
runtime facts and one guarded per-Capture mutation so the responsible task can
make and verify its own decision. Listing remains non-authorizing. There is no
background worker, global batch apply, automatic successor inference,
retention change, deletion, migration, installation, activation, Push, release,
or deployment in this decision.

## Consequences

- Obsolete undelivered intents can reach an explicit terminal state without
  erasing their admission history.
- Deterministic failures remain terminal even when the registry changes.
- Existing `--list-open` consumers keep their inclusion contract, while
  `--list-all` gives audits a complete lifecycle view.
- Agents can own classification and follow-through using machine state rather
  than asking the user to monitor an undifferentiated backlog.
- A disposition made under incorrect business judgment remains visible and
  immutable; correction requires a new operation or a later ADR, not history
  rewriting.

## Validation

1. Projection reconstruction from intent and events is byte-equivalent before
   and after cache replacement.
2. Identical disposition replay is zero-write; conflicting disposition or any
   later event fails before installation.
3. Delivery-started, delivered, failed, referenced, and completed operations
   reject disposition.
4. Supersession rejects a missing, equal, or not-later successor; abandonment
   rejects a successor.
5. Registry and projection digest races fail closed before a new event.
6. `--list-open` preserves ADR-0519 rows and paging; `--list-all` includes
   completed, failed, superseded, and abandoned rows.
7. Both inventory formats retain the metadata allowlist and expose no semantic
   payload, raw idempotency key, free-form reason, path, or credential.
8. Formatting, strict lint, locked tests, schema validation, CLI smoke, and the
   ProjectRef acceptance matrix pass before any installation is considered.
