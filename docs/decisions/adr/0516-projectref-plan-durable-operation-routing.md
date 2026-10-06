# ADR-0516: ProjectRef Plan Durable-Operation Routing

Status: Accepted, implemented, independently reviewed, and locally adopted
Date: 2026-10-06

## Context

ProjectRef registry v2 deliberately made legacy direct Store discovery fail
closed. `cognition capture` gained a durable intent, event, projection, and
receipt path, but `plan admit --cwd` and `plan evolve --cwd` still depended on
that legacy discovery path. The documented Plan carrier therefore existed for
registry v1 and explicit `STORE --branch`, while the installed registry-v2
control plane rejected it.

Routing Plan writes directly around the journal would restore functionality at
the cost of the durability boundary: a process failure after the Store commit
but before the caller received its result would have no control-plane receipt
or bounded recovery identity. Encoding Plan as cognition would be equally
incorrect because Goal, Plan, Task, Acceptance Criterion, and Verification
Requirement are Work-State objects, not cognition Records or Knowledge. A
second Plan queue would duplicate resolution, locking, projection, activation,
and recovery semantics.

The journal-admission marker also had only a coarse scope. Reinterpreting an
already installed v1 marker as authority for new Work-State mutations would
silently broaden operator authorization.

## Decision

The existing physical `capture-journal/v1` is the single durable-operation
protocol for registry-v2 project-routed writes. Its historical file and type
names remain for compatibility, while payload kinds distinguish semantics:

- `cognition_v2` and `legacy_cognition_v1` retain their current behavior;
- `plan_admit_v1` carries the exact canonical Plan-admission manifest; and
- `plan_evolve_v1` carries the exact canonical in-place or supersede manifest.

Plan payloads never enter cognition manifests and never carry a CaptureGroup.
The journal records a namespaced intent idempotency key, while the target Store
receives the manifest's original idempotency key and compare-and-swap guards.
The existing Store engines remain the sole semantic validators and atomic
Work-State writers.

For registry v2, `plan admit --cwd` and `plan evolve --cwd`:

1. parse the strict manifest before admission;
2. resolve exactly one bound ProjectRef;
3. require read routing plus the matching journal capability;
4. durably admit the typed intent under the registry/quiescence locks;
5. drive the same resolution, binding, `delivery_started`, target write, and
   `delivery_applied` state machine used by cognition recovery; and
6. report success only after the target result and durable receipt agree.

An explicit `STORE --branch` remains the expert direct boundary. Registry v1
keeps its existing bound-project behavior. Neither path creates a second
Work-State database.

`delivery_applied.result_objects` now accepts Goal, Plan, Task, Acceptance
Criterion, and Verification Requirement entity receipts in addition to the
existing cognition families. Plan evolution receipts may also contain the
typed `contains` and `supersedes` relation versions. A missing receipt after a
successful target commit is repaired by replaying the same manifest
idempotently; no duplicate Work-State objects or commits are created.
The journal validates each `delivery_applied` and `delivery_failed` event
against the admitted payload kind. Cognition cannot claim Plan results, Plan
cannot claim cognition results or CaptureGroup state, admission/evolution
receipt families cannot be interchanged, and each failure code has one exact
recovery action. Every Plan receipt carries an exact
`operation_payload_kind`; reconstruction also derives the complete expected
`local_id`/object-kind set from the admitted manifest. Create-Goal versus
existing-Goal admission and in-place versus supersede evolution therefore
cannot complete with one another's partial receipt shape.
Manifest aliases that the Plan engine canonicalizes, currently
`record.kind=unknown` to `question`, pass through the same domain parser before
placeholder sizing and expected-shape derivation. Preflight and the committed
Store result therefore use the same canonical receipt vocabulary.
The field is omitted for cognition receipts, preserving their historical
canonical bytes. No installed version-1 marker could authorize a Plan intent,
so there is no valid version-1 Plan-receipt family to reinterpret or migrate.

## Activation compatibility

Journal-admission activation version 2 declares a strictly sorted capability
set:

- `cognition_capture`;
- `plan_admit`; and
- `plan_evolve`.

The existing marker path remains
`journal-admission-activation-v1.json` because changing its location would
create two simultaneous authorities. A version-1 marker remains readable and
authorizes only `cognition_capture`. It never authorizes either Plan operation.

The existing digest-locked apply command may atomically replace a version-1
marker for the exact same registry ID, revision, and digest only when the old
marker digest is supplied and its effective capabilities are a strict subset
of the version-2 candidate. This is a capability refresh, not a registry
refresh. Wrong lineage, changed registry snapshot, invalid marker, missing old
digest, capability removal, or lateral replacement fails closed. Read routing
must already be exact, and the registry-then-quiescence lock order is
unchanged.

## Failure and authority boundaries

Before a first target write, the route runs a side-effect-free typed manifest
validation and explicit current-snapshot guard comparison. Only failures
proved at those stages become durable terminal results: malformed or
semantically impossible manifest content becomes `plan_manifest_rejected`,
while head, state, Plan-version/status/digest, Goal-version/status, and
relation-version/parent mismatches, including an existing entity of the wrong
Goal/Plan kind, become `plan_target_conflict`. Terminal disposition is closed
before any receipt materialization or other fallible delivery work, so it
cannot be masked by a placeholder-receipt error. The target Store head remains
unchanged. A previously committed idempotent result is looked up before
comparing the now-advanced head, so commit-before-receipt recovery remains
valid.

Once a `delivery_failed` event is authoritative for one `delivery_started`
identity, another failure or a `delivery_applied` receipt cannot replace it.
Only a later authorized resolution/binding change that clears the entire
delivery attempt may start a different target attempt. Terminal classification
is therefore monotonic in the immutable event chain, not merely a CLI policy.

Receipt size is measured with the same typed payload and a fixed
maximum-length RFC 3339 timestamp envelope, not a second wall-clock sample.
The `Timestamp` scalar admits at most nine fractional digits, making that
nanosecond envelope a type-level maximum for later append rather than an
estimate.
The boundary accepts event sizes at the limit and rejects only the first byte
above it. An oversized result leaves `plan_receipt_too_large` and performs zero
target Store writes. Engine execution, storage, integrity, transaction commit,
and receipt-reconstruction failures are never reclassified from an error code
alone; they remain pending or indeterminate for status-first recovery.

Activation does not authorize unrelated ProjectRef bootstrap, release,
deployment, or any other Store mutation.

The public recovery command retains its compatibility name
`project capture-recovery`; status now exposes `payload_kind`, and the same
exact registry/projection digest locks recover cognition and Plan intents.

## Consequences

- Registry-v2 Plan admission and evolution now have the same durable recovery
  boundary as routed cognition without conflating their domain semantics.
- One event/projection/receipt implementation owns target routing and the
  commit-before-receipt window.
- Command output distinguishes reuse of the journal admission from reuse of
  the target Store result. A retry that had written only `delivery_started`
  therefore reports target `created`, while commit-before-receipt replay
  reports target `reused`. Once the receipt is durable, output reconstruction
  uses only the read-only result lookup and does not invoke the mutating Plan
  engine again.
- Existing version-1 markers keep their original authority and require an
  explicit compare-and-swap refresh before Plan writes become possible.
- Existing explicit-Store and registry-v1 callers remain compatible.
- Physical `capture-*` names are historical compatibility names; new
  documentation describes the abstraction as the durable-operation journal.

## Verification

Isolated tests cover version-1 marker compatibility, explicit same-snapshot
capability refresh, absent-gate zero-write behavior, routed Plan admission,
idempotent replay, in-place evolution, supersede evolution, Goal/Plan/Task
recall, delivery-started retry versus target-result reuse,
target-commit-before-receipt recovery, exact limit-minus-one/limit/limit-plus-
one receipt envelope boundaries plus actual maximum-precision append, exact
operation/manifest receipt shapes, typed receipt/failure rejection, wrong-kind
Goal/Plan guards, canonical `unknown`-to-`question` receipt convergence for
admission and evolution, terminal-failure monotonicity, read-only post-receipt
result projection, and durable terminalization of explicitly proven stale
guards plus pure manifest validation errors.
Existing cognition, CaptureGroup, registry migration, activation fault, direct
Plan, and Plan conflict suites remain regression gates. Full repository
validation and local adoption evidence, including the exact package, marker
refresh, and live routed admission/evolution receipts, are recorded in
[`projectref-plan-durable-operation-routing.md`](../../provenance/projectref-plan-durable-operation-routing.md).
