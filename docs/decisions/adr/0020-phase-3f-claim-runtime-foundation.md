# ADR-0020: Phase 3F Claim Runtime Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current Work request establishing the long-running
  implementation goal and authorizing continuous WorkVCS implementation from
  confirmed repository requirements.

## Context

Phase 3E introduced Session Runtime Foundation, including stable Session
occurrences, active SessionRuntime projection, Focus, SessionDiff, and runtime
provenance Events. The confirmed domain requires Claims to coordinate
concurrent Task work without becoming versioned Work State or a generic
authorization lock.

Schema v0.1 already contains the Claim-related tables needed for a narrow
foundation slice:

- `object_identity` supports the `claim` object family;
- `claim` stores one stable ownership/mode occurrence for a Session, Workspace,
  Branch, Task, mode, and creation time;
- `claim_runtime` stores the active current projection for that occurrence; and
- `event` records runtime provenance independently from ChangeSet and
  WorkStateCommit.

No schema migration is required for this slice.

## Decision

1. Phase 3F introduces only the Claim Runtime Foundation in `workvcs-core`.
   It does not implement shared Claims, mode change, takeover, force
   provenance, transfer, stale-session recovery, branch switching, claim-next,
   next resolver, MergeRuntime, Resource/Evidence capture, Handoff, Federation,
   Bundle, Checkpoint, schema migration, or business CLI commands.
2. The public surface remains Engine-owned and semantic. It must not expose
   SQLite handles, raw SQL, public generic Runtime CRUD, or CLI-to-SQL
   shortcuts.
3. A Claim is created as a stable ObjectIdentity-backed occurrence with
   `object_kind = "claim"` and one row in `claim`.
4. Claim creation creates one active `claim_runtime` row and writes a
   `claim.created` Event in one `BEGIN IMMEDIATE` transaction.
5. Claim creation validates that the Session is active, that the Session active
   Branch belongs to the active Workspace, and that the Branch is active.
6. Claim creation validates that the claimed Task Entity is present at the
   active Branch head and is a Task by using the existing Task semantic read
   boundary.
7. Phase 3F implements default exclusive Claims only. Exclusive is the default
   confirmed mode; shared Claims require an explicit later operation surface.
8. For one Workspace, Branch, and Task, Phase 3F rejects Claim creation when any
   active ClaimRuntime already exists for that target. This preserves the
   active set invariant for the implemented exclusive-only subset.
9. Claim creation does not change Task status, does not create a ChangeSet,
   ChangeOperation, WorkStateCommit, or Branch HEAD movement, and does not imply
   TaskStart.
10. Claim read returns a deterministic projection: Claim id, lifecycle state,
    owning Session id, Workspace, Branch, Task Entity, mode, creation time, and
    last activity time when active.
11. If `claim_runtime` exists, the Claim projects as `active` in Phase 3F. If
    `claim_runtime` is absent and the Claim occurrence exists, it projects as
    `released`. Missing occurrence is not found.
12. Claim release is a dedicated Engine semantic API. It validates the Claim
    exists, is active, and is owned by the releasing Session.
13. Claim release removes only the active `claim_runtime` row, updates the
    owning Session activity, writes a `claim.released` Event, and preserves the
    stable Claim occurrence.
14. Normal SessionEnd releases active ClaimRuntime rows owned by the Session as
    part of the existing atomic cleanup requirement. Phase 3F adds coverage
    through public Claim APIs.
15. Failed Claim creation or release writes no partial authoritative runtime or
    provenance rows.
16. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3F does not add business Claim commands.

## Consequences

- WorkVCS gains a minimal Claim coordination surface that composes with active
  Sessions and existing Task semantic history without redefining Task
  lifecycle.
- Later shared Claim, transfer, takeover, stale recovery, claim-next, and next
  resolver slices can build on stable Claim occurrence and ClaimRuntime
  projection without changing the storage ownership boundary.
- Runtime provenance remains queryable through Events without implying an empty
  ChangeSet or Commit.

## Implementation findings

- The confirmed logical schema leaves the exact index, transaction, or lock
  mechanism for active Claim-set enforcement Open. Phase 3F uses one
  `BEGIN IMMEDIATE` transaction and a conflict query against active
  `claim_runtime` joined to `claim`; no schema-level unique index is added.
- Schema v0.1 has `claim` and `claim_runtime` but no `claim_diff` or release
  detail table. Phase 3F therefore projects `released` from the preserved
  Claim occurrence plus absence of an active `claim_runtime` row, with the
  release fact recorded as an immutable Event.
- Shared Claim mode is confirmed domain vocabulary, but supporting it safely
  requires an explicit operation surface and tests for coexistence rules.
  Phase 3F stores only `exclusive` mode through public APIs.
