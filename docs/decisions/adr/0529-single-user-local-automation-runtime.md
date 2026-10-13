# ADR-0529: Single-user Local Automation Runtime

Status: Superseded in part by ADR-0531; legacy registration mode remains accepted
Date: 2026-10-13
Accepted by: The user's integrated WorkVCS post-V1 and Token-efficiency
authorization after independent review. The execution contract is persisted in
WorkVCS Plan `01a13391-7d89-7336-9cf3-e6d199896922`.

## Context

ADR-0528 intentionally stopped at explicit foreground import, federation and
immutable search generations. That preserves authority but leaves ordinary
local use dependent on repeated manual commands. A useful automatic layer must
reduce deterministic coordination work without becoming another canonical
state engine, hiding failures, or making local background execution an
installation side effect.

## Later supersession

ADR-0531 supersedes the per-ProjectRef registration requirement for the new
global-policy mode. This ADR continues to govern the legacy explicit-
registration format, daemon safety boundaries, watcher semantics, worker
lifecycle and the prohibition on canonical writes or automatic activation.

## Decision

WorkVCS adds one optional per-user local automation service with these fixed
boundaries:

1. Participation is by explicit ProjectRef registration. Preview is read-only;
   apply is compare-and-swap against the exact registry revision and digest.
2. The service is disabled by default. This delivery installs no LaunchAgent,
   startup item, login item, service unit, or privileged component.
3. A filesystem watcher is only a dirty-signal source. Its channel has capacity
   one; further events coalesce while event, error and coalescing counters stay
   inspectable. It never imports, indexes, mutates a Store, or establishes
   currentness inside the callback.
4. Reconciliation runs at startup, explicit manual request, watcher overflow,
   and a configured periodic anti-entropy interval. Lost or coalesced events
   converge through a complete scan.
5. Each reconciliation unit is a short-lived foreground-compatible worker. The
   resident process owns no model, Qdrant shard, canonical Store handle, or
   mutable generation.
6. Runtime coordination uses one owner-private directory, one instance lock,
   a versioned owner-private Unix socket, bounded frames, same-user peer
   verification, and an explicit status ledger. Worker streams are drained into
   bounded owner-private evidence with byte counts, full-stream digests and
   explicit truncation. Protocol mismatch, malformed frames, stale locks and
   worker failures are visible and fail closed.
7. Every worker re-resolves the registered ProjectRef and verifies Store,
   Workspace, Branch, registry revision/digest and source roots before derived
   work. Binding drift quarantines that registration; it is not repaired
   automatically.
8. Canonical and historical reconciliation are independent failure domains.
   A routing, Store-open or generation failure stays visible in
   `canonical_current` but cannot suppress revalidation of prior historical
   archives. Only successfully scanned roots classify deletion; a root or file
   observation failure retains its prior inventory and archive plus a separate
   failure record.

The service may read canonical Stores and registered local source roots and may
publish new immutable derived artifacts. It must not write Store semantics,
project registry bindings, KnowledgeExposure, source-status, source files,
archives, or existing generations. It never performs automatic cleanup.

## Registration and recovery

A registration records a stable id, exact ProjectRef and read identity,
canonical and reference source declarations, role policy, enabled/revoked
state, revision, and registry basis. Roots are absolute, non-overlapping after
realpath/inode checks, non-symlink at registration, and revalidated on use.
Revocation immediately removes the registration from new catalog rounds but
does not erase prior immutable evidence.

Crash recovery is deterministic: discard incomplete temporary publications,
retain last complete immutable generations and catalog rounds, mark affected
sources dirty or failed, then run a complete reconciliation. No failure is
converted to a successful or complete observation.

## Rejected alternatives

- A daemon enabled or installed by default.
- Long-lived Store, model, or vector-index processes.
- Watcher callbacks that directly import or mutate state.
- Automatic registry repair, binding reassignment, source deletion, archive
  replacement, generation cleanup, or canonical semantic capture.
- Silent best effort that labels a partial round complete.

## Validation obligations

Validation covers registration preview/apply conflicts; symlink, ownership,
permission and overlap faults; one-instance locking; peer and protocol checks;
startup/manual/overflow/periodic reconciliation; coalescing and lost-event full
scan; create/replace/rename/delete/permission transitions; worker crash,
timeout, exact stream evidence, malformed report, partial-status projection,
drain and restart; binding/Store failure with an independently retrievable
historical lane; immutable publication;
and byte-stable canonical Store/registry readback before and after real local
dogfood. Reviewer and Validator remain independent.
