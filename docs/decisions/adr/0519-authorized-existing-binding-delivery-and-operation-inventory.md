# ADR-0519: Authorized Existing-Binding Delivery and Operation Inventory

Status: Accepted; source implemented and locally validated; adoption pending
Date: 2026-10-08

## Context

ADR-0513 deliberately made public registry-v2 cognition capture stop after
durable journal admission. Target delivery remained a separately explicit,
digest-locked `project operation-recovery` action. That separation established
the correct authority boundary for bootstrap, conflicting ownership, target
changes, and cross-project delivery.

The same boundary is unnecessarily expensive for the narrower case where the
caller already has delivery authority and the admitted operation resolves to
one unchanged, fully valid, non-shared ProjectRef binding. Each such checkpoint
currently requires a capture, a status call, digest extraction, an apply call,
and a final readback even though status can only recommend
`apply_binding_receipt`. Repeating that sequence across normal work creates
avoidable coordination cost and leaves otherwise routine captures in the
journal when the follow-up is missed.

The journal also lacks one bounded read-only inventory of open durable
operations. Operators must enumerate CaptureIds externally and run status once
per operation, repeatedly loading and validating the same control plane. A
clean unbound semantic owner is additionally surfaced by some read paths as
the generic `control_plane_invalid` error even though the registry may be
healthy and the resolver has produced a valid, recoverable `unbound` result.

The motivating live snapshot and its counting method are recorded in
[Authorized Existing-Binding Delivery Design and Change Plan](../../provenance/authorized-existing-binding-delivery-plan.md).
The snapshot is evidence of recurrence, not part of the normative contract.

## Decision

### 1. Default capture remains journal-only

The existing registry-v2 `capture` behavior remains the default. Without a new
explicit option it validates the semantic manifest, admits one immutable
target-neutral intent, reports `delivery_status=not_started`, and performs no
registry bootstrap or target Store mutation.

Registry migration, marker installation or refresh, binding repair, ProjectRef
bootstrap, target change, shared-binding isolation, rollback, cross-project
secondary delivery, historical batch recovery, installation, Push, release,
and deployment remain separate decisions and operations. Neither marker
presence nor this ADR supplies authority for them.

### 2. One explicit existing-binding continuation

Registry-v2 cognition capture adds:

```text
workvcs capture ... --deliver-existing-binding
```

The option means that the caller already holds authority for this operation to
deliver only to the exact existing binding selected at admission. It is a
per-invocation authority signal, not a persisted preference, background worker,
global Hook, or inference from control-plane health. Integrations may pass it
under a user-approved standing policy for the same target; the binary does not
create that policy.

The option is execution authority, not semantic content. It is not added to the
CaptureIntent, payload digest, CaptureGroup value, or admission idempotency
identity. A later invocation may therefore reuse an earlier journal-only intent
with the same idempotency key and explicitly request its eligible delivery;
omitting the option can never inherit that authority from the prior invocation.

The first implementation accepts the option only for target-neutral
`cognition_v2` with `capture_group=null`. It rejects registry-v1/legacy capture
and any `--capture-group` combination before admission. Typed Plan commands
retain their already synchronous journal-first receipt protocol.

After the normal semantic and activation preflight, WorkVCS MUST install the
immutable intent before attempting target delivery. Continuation is eligible
only when all of the following are true:

1. the admission result is `resolved` with exactly one primary ProjectRef;
2. that ProjectRef has one complete binding whose Store, Workspace, Branch,
   local identity, and full integrity validation all pass;
3. no other ProjectRef currently shares the exact Store/Workspace/Branch
   target;
4. the registry, routing activation, journal activation, and cognition
   capability still match the admitted snapshots; and
5. the recovery projection still names the just-admitted intent and has not
   acquired a conflicting event or terminal failure disposition; a matching
   fully completed receipt with `recovery_action=none` remains eligible for
   verified no-write success; and
6. when admission reuses an older intent, every existing `resolution_recorded`
   event names the current ProjectRef and every existing target-bearing event
   or receipt names the same ProjectRef/Store/Workspace/Branch tuple as the
   currently resolved binding; and
7. for each prior `resolution_recorded` event whose registry digest differs
   from the current admitted digest, later authority in the same event chain
   proves the complete current ProjectRef/Store/Workspace/Branch tuple.

A reused intent with a different prior ProjectRef or target tuple is not
eligible. Neither is an intent whose prior resolved state lacks proof of target
continuity after its recorded registry digest changed: a ProjectRef-only
`resolution_recorded` event is not evidence of a complete target. These rules
apply even where ordinary manual recovery could re-resolve a non-CaptureGroup
intent before canonical delivery. The fast-path check occurs before appending
a new resolution/binding/delivery event or opening a Store writable and returns
an instruction to start a new Capture for the current target. Standing
existing-binding authority never crosses an unproven historical target change.

The command then invokes the same recovery implementation used by explicit
`operation-recovery --apply`. It supplies the freshly derived registry and
projection guards internally, reacquires the established locks, performs the
same fresh writable-open validation, fixes target Branch guards before
mutation, and relies on the same Store idempotency key and durable receipt.
There is no second delivery engine or queue.

The routing/journal marker and capability recheck is a fast-path-only guard
executed under the established recovery lock window. It MUST NOT be added as a
precondition to the shared manual recovery engine. An already admitted
operation remains explicitly recoverable by digest-locked
`operation-recovery --apply` when admission/read markers are later inactive or
stale; otherwise activation damage could also disable recovery from that
damage.

Command success means the operation has a current verified delivery receipt
and its effective recovery action is `none`. Repeating the same idempotent
capture reuses the intent and completed receipt without a second semantic
mutation.

If admission succeeds but continuation is ineligible, races, or otherwise
returns an error, the intent remains durable and the command fails with
`capture_delivery_incomplete`. Machine output MUST include the CaptureId,
`journal_persisted=true`, the underlying cause code, and the next safe
status-first recovery action. A process or host loss may prevent any response.
If the operation is still open, the open-operation inventory exposes the
durable CaptureId and idempotency-key digest. If the target receipt was already
completed, repeating the caller-held idempotency key with the explicit option
returns the same receipt and CaptureId with zero new event, projection, Store
object, or commit write. The raw idempotency key is never rendered by the
inventory. The implementation MUST NOT hide a partial success, fall through to
another ProjectRef, bootstrap a project, substitute a target, or retry an
uncertain mutation blindly.

### 3. Read-only open-operation inventory

The canonical recovery surface adds a third mutually exclusive action:

```text
workvcs project operation-recovery --list-open
    [--registry PATH]
    [--project-ref PROJECT_REF]
    [--payload-kind KIND]
    [--recovery-action ACTION]
    [--limit N]
    [--after-capture-id CAPTURE_ID
     --expected-inventory-digest DIGEST]
```

`--status --capture-id ID` and digest-locked `--apply --capture-id ID` keep
their existing contracts. `project capture-recovery` remains an alias for the
same command and therefore exposes the same read-only inventory.

The inventory is strictly read-only. It loads one registry snapshot and
validates exactly once, with the complete integrity path, every distinct
referenced binding that affects classification of the matching operations. It
never validates such a binding twice in one invocation. It scans the distinct
supported journal aliases, reconstructs each operation from immutable
intent/event authority, and derives current recovery state without replacing a
projection. A missing or stale regular projection is reportable cache state;
invalid immutable authority, alias disagreement, or CaptureId collision fails
closed.

An operation is open when its current `recovery_action` is not `none`. This
includes separately gated bootstrap, repair, replacement, and terminal follow-
up actions; inclusion never authorizes the action. Results are ordered by
`created_at`, then CaptureId, oldest first. The default row limit is 100 and the
maximum is 1000. Summary counts cover every matching operation before the row
limit and are grouped by payload kind, effective recovery state, and recovery
action. Output reports truncation, `inventory_digest`, and
`next_after_capture_id` explicitly.

The inventory digest binds the registry digest, canonical filters, complete
ordered matching row set, authority digests, derived states/actions, and binding
classification. A following page supplies both the last returned CaptureId and
that digest. WorkVCS recomputes the complete inventory and fails closed on a
digest mismatch or a cursor absent from the matching set; it never silently
skips across a changing backlog. This permits complete enumeration beyond 1000
rows without claiming a transactionally frozen journal snapshot.

Each row is an allowlisted, bounded machine summary: CaptureId,
idempotency-key digest, payload kind, creation time, current ProjectRef or
`none`, effective recovery state/action, binding classification, projection
cache state/digest, and immutable authority digests plus bounded group/result
identifiers when applicable. Key-value output escapes every dynamic scalar;
JSON uses the same fields. Rows MUST NOT expose the raw idempotency key,
semantic payload, `value_reason`, provider or locator context, target path,
free-text diagnostic/cause, credentials, environment data, or raw tool output.

The inventory does not join `project health`. Health continues to answer
whether the selected control plane is safe to use; backlog presence is work to
reconcile, not proof that the control plane is unhealthy. No automatic
historical sweep, batch apply, or retention change is introduced.

### 4. Clean unbound ownership has a dedicated classification

A valid read-only resolver result with `resolution_status=unbound` is not
control-plane corruption. Read paths that require a bound target return the
stable `project_owner_unbound` error with the winning rank, bounded diagnostic
counts, `recoverable=true`, `retryable=false`, and
`recovery_action=admit_operation_then_authorize_project_bootstrap`.

`project discover` remains non-zero because it cannot verify a target that does
not exist, but its output is inspectable and specific. `project health --cwd`
classifies the same clean state as `degraded`; `--require-healthy` still fails.
Malformed, stale, conflicting, ambiguous, or unsafe authority remains
`blocked`/`control_plane_invalid`. No read-only command calls `project ensure`,
creates a binding, or falls back to a weaker locator.

The machine-contract boundary is explicit:

| Surface | Clean missing/unbound result |
| --- | --- |
| Registry-v1 binding lookup | Existing `project_binding_not_found`; unchanged. |
| Registry-v2 read requiring a target | New top-level `project_owner_unbound` in key-value and JSON output. |
| `project health --cwd` | Successful `health=degraded` with `resolution_issue=project_owner_unbound`; `--require-healthy` fails with top-level `project_owner_unbound`. |
| Default v2 capture or operation-recovery status | Existing successful `resolution_status=unbound` and `recovery_action=apply_project_bootstrap`; no new top-level error. |
| Flagged capture after durable admission | Top-level `capture_delivery_incomplete` with `cause_error_code=project_owner_unbound`. |

The older `ownership_unbound` name remains a resolver/routing condition in
historical design vocabulary; ADR-0519 does not introduce it as a second
top-level ErrorCode. Operator documentation must preserve these scopes rather
than mapping every clean unbound observation to one command outcome.

### 5. Authority and compatibility

This decision narrows ADR-0513 only for an explicitly requested,
single-project, existing-binding continuation. The default admission-only
contract and every independent gate remain in force. It extends ADR-0518's
canonical recovery surface with a read-only inventory and does not rename
CaptureIds, journal paths, projections, events, receipts, or historical error
codes. It does not change the journal envelope or Store schema.

This ADR alone did not authorize source implementation, local Commit,
installation, live adoption, historical reconciliation, Push, release, or
deployment. Stage B source implementation and local Commit were separately
authorized and are evidenced by
[Authorized Existing-Binding Delivery Candidate Evidence](../../provenance/authorized-existing-binding-delivery-candidate.md).
Installation and the bounded canary remain Stage C; historical reconciliation
and all remote actions retain their later confirmation boundaries.

## Consequences

- Authorized routine checkpoints can become durable in the target Store with
  one public command while retaining journal-first ordering and exact recovery.
- Default and ambiguous cases remain conservative; no authorization is inferred
  from a healthy binding or activation marker.
- A failed continuation is an explicit durable partial result with a stable
  CaptureId, not a silent admission-only success or an unsafe retry.
- Operators gain one bounded view of the actual recovery backlog without
  repeatedly validating the same Store for every CaptureId.
- Clean unbound ownership becomes distinguishable from damaged control-plane
  authority without weakening fail-closed locator precedence.
- Default capture, exact operation-recovery status/apply, persistent formats,
  and their existing fields remain compatible. The registry-v2 clean-unbound
  discover/recall/health machine classification intentionally changes and its
  consumers must accept `project_owner_unbound`/`degraded`.

## Required verification before implementation may be called complete

1. Default `capture` remains Store-free and byte-stable.
2. Eligible `--deliver-existing-binding` installs the intent first, uses the
   existing recovery engine, records a verified receipt, and replays with zero
   duplicate semantic writes.
3. Adding or removing the option does not change admission idempotency; only an
   invocation that contains it may continue beyond admission. A fully completed
   matching replay performs no journal-event, projection, or Store write.
4. Unbound, conflicting, shared, invalid, or changed targets retain the intent
   but perform no unauthorized registry or Store mutation.
5. A reused open intent is eligible only when every prior resolved ProjectRef
   matches and every historical target tuple matches the current complete
   ProjectRef/Store/Workspace/Branch tuple. If a prior resolution used a
   different registry digest, later same-chain authority must prove that full
   tuple; mismatch or missing continuity proof creates no new event and
   requires a new Capture.
6. Commit-before-receipt and every existing injected recovery fault remain
   status-first and idempotently recoverable under the same CaptureId.
7. Inactive or stale markers block only the same-command fast path; manual
   digest-locked recovery of an already admitted operation retains its existing
   marker-independent behavior.
8. `--list-open` is byte-stable across registry, marker, journal, projection,
   and Store artifacts; it validates every classification-relevant distinct
   binding exactly once and reports deterministic filtered, digest-locked,
   cursor-pageable results.
9. Inventory rows obey the metadata allowlist and do not leak control-bearing
   or secret-bearing intent text in key-value or JSON output.
10. Invalid immutable authority and alias disagreement fail closed, while
   missing/stale projection caches do not become a second authority.
11. Clean unbound discovery reports `project_owner_unbound`, health reports
   `degraded`, and conflict/corruption remain blocked. Key-value and JSON forms,
   including `health --require-healthy`, preserve the scope table above.
12. Workspace format, lint, locked tests, ProjectRef acceptance validation,
   operator recovery audit, source-tree checks, and exact documentation/Skill
   consistency pass before any installation or live canary is considered.
