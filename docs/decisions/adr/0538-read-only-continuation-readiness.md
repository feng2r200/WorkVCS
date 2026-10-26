# ADR-0538: Read-only Continuation Readiness Snapshot

Status: Accepted
Date: 2026-10-26

## Context

The continuation adapter needs one authoritative WorkVCS answer to whether an
existing ContinuationScope satisfies the native provider-protocol
prerequisites. Reading
`continuation scope-show` and independently joining Session, Claim, Branch,
Task, policy, and rotation state in Work Governance would duplicate WorkVCS
semantics and could combine facts from different SQLite snapshots.

The inspection boundary must not refresh a Branch projection, migrate a Store,
enroll a Project, repair runtime state, or otherwise turn observation into a
mutation. A non-ready domain state is expected inspection output. A malformed
identifier, invalid stored document, unsupported lifecycle value, or other
storage-integrity failure is not a readiness gap and must fail at its source.

## Decision

1. `Engine::continuation_readiness(scope_id)` is the native readiness
   aggregate. It is available only on an Engine opened by
   `Engine::open_readonly`; calling it on a writable Engine fails closed.
2. The aggregate executes all eligibility reads in one SQLite Deferred
   transaction on the read-only, `query_only` connection. It does not invoke
   currentness preview, projection refresh, schema migration, enrollment, or
   repair.
3. The typed snapshot reports:
   - the exact scope IDs, listener revision, carrier Claim, owner epoch, and
     immutable caller-selected Project association when one was admitted;
   - the resolved exact-Project policy revision, receipt, and enabled state;
   - listener lifecycle, role, thread identity, active Workspace and Branch;
   - carrier lifecycle, role, thread identity, and either ordinary `work`
     provenance or the exact committed `work_candidate` promotion;
   - the immutable Claim coordinates plus current ClaimRuntime activity;
   - Branch lifecycle, head commit/state digest, and stored projection
     currentness;
   - the current Task version, digest, and lifecycle status;
   - any active rotation state, listener revision, currentness digest,
     candidate, and abort-only fence;
   - ordered structured gaps and a digest of every reported eligibility fact.
4. `provider_protocol_ready` is true only when the native WorkVCS gap list is
   empty. It does not assert product membership or authorize dispatch. The
   top-level `product_membership_verified` and
   `product_dispatch_authorized` fields are always false and are included in
   `currentness_digest`; this native aggregate has no caller path that can set
   either field to true. Expected conditions such as a disabled policy,
   inactive or wrongly targeted Session, invalid role, aliased thread,
   released or mismatched Claim, inactive Branch, stale projection, absent or
   terminal Task, and any active nonterminal rotation are returned as explicit
   gaps.
5. A scope normally disappears atomically when its idle carrier Claim is
   released. Querying that retired scope returns `ContinuationNotFound`. If an
   observed scope still references an inactive ClaimRuntime, the snapshot is
   non-ready with `carrier_claim_released`; it is never promoted to ready.
6. An immutable `work_candidate` role is accepted as the current carrier only
   when one committed rotation result proves the exact candidate Session,
   resulting Claim, Workspace, Branch, Task, created target thread, and Project
   basis, and that thread has exactly one historical created-target occurrence.
7. A new Scope may explicitly associate a caller-selected Project only through
   `ContinuationScopeBindOptions::with_project_id`. The same write transaction
   verifies that exact Project has an enabled durable continuation policy and
   records the Project ID, policy revision, policy receipt digest, and source
   `continuation.scope_bound` in the existing immutable `scope_bound` event.
   No table, side ledger, schema migration, or caller-supplied readiness hint is
   introduced. Replaying the bind must match the original Project association;
   an old unbound Scope cannot be silently claimed and a bound Scope cannot be
   rebound to another Project. This proves only the explicit native association
   and the policy receipt present in WorkVCS. It does not prove that the caller,
   Workspace, Session, or Task belongs to a real product Project. A trusted
   product adapter must establish product membership and dispatch authorization
   independently before acting.
8. Readiness loads policy only for the Project carried by that immutable Scope
   binding. It never selects the sole policy in a Store and never treats the
   number of unrelated policies as Project evidence. An ordinary `work` Scope
   without a Project binding is non-ready with `project_binding_missing`. An
   exact committed `work_candidate` promotion may supply its already-verified
   rotation Project basis for an old unbound Scope. When a Scope is bound, an
   active rotation or committed promotion for another Project is an explicit
   mismatch and cannot report ready. Rotation admission rejects a new basis for
   a Project different from the bound Project before any rotation is written.
9. Stored Branch lifecycle values are validated against the native `active` and
   `closed` states. `closed` is a valid non-ready domain state; any unknown
   value is an integrity failure and does not produce a readiness snapshot.
10. `currentness_digest` uses the
   `workvcs.continuation.readiness.v1` domain and deterministic Serde JSON over
   the two fixed product-capability booleans, typed facts, and ordered gaps.
   Listener lifecycle/role/thread, Claim activity and exact coordinates, owner
   epoch, policy revision or revocation, Branch head/projection, Task
   version/status, carrier promotion, and active rotation changes therefore
   change the digest.
11. The CLI surface is
   `workvcs continuation readiness STORE --scope-id ID`. It opens the Store
   read-only and emits the snapshot as JSON. A valid but non-ready snapshot is
   successful command output; missing scope, invalid input, storage failure,
   or integrity failure uses the existing typed error and non-zero exit path.

## Consequences

- Work Governance and product adapters consume one typed WorkVCS result rather
  than reconstructing continuation eligibility from key-value output.
- Observation opens SQLite with `SQLITE_OPEN_READONLY`, enables `query_only`,
  and executes only reads in one Deferred transaction. It performs no SQL
  mutation, checkpoint, projection refresh, migration, enrollment, or repair;
  the main database, WAL, and other persistent Store members remain unchanged.
  SQLite implements the WAL-index by memory-mapping the `-shm` file, and reader
  bookkeeping may change that file's bytes or metadata during read-only access.
  `-shm` is therefore observed and reported separately rather than included in
  a claim that every Store-directory byte is invariant.
- The live Store is not opened with SQLite's `immutable` URI parameter. That
  parameter disables locking and change detection and can return incorrect
  results when the database changes, which conflicts with a currentness
  inspection of a live Store.
- Consumers may compare `currentness_digest` to detect any eligibility drift,
  but must still inspect `provider_protocol_ready` and `gaps` rather than
  treating digest equality as authorization.
- The snapshot reports native provider-protocol prerequisites only. The fixed
  false product-capability fields make explicit that it neither verifies real
  product membership nor authorizes conversation creation, messaging, Claim
  transfer, Push, deployment, or any other guarded effect.

## Validation obligations

- Exercise a valid temporary Store through the public Engine API and confirm
  that the baseline native snapshot is ready while both product-capability
  fields remain false.
- Inventory every Store-directory member before and after readiness inspection
  and a rejected write through the read-only Engine. Require identical non-SHM
  member sets and byte-, length-, and mtime-identical main database, WAL, and
  other persistent members. Record the optional `-shm` before/after observation
  separately, require it to be an ordinary file whenever present, and allow its
  presence, bytes, length, and mtime to change as part of normal WAL-index
  reader coordination.
- Copy a real main database and live WAL into a separate writable temporary
  directory without copying its `-shm`, then open that copy through the normal
  read-only Engine. Prove the readiness snapshot is unchanged, the main
  database and WAL remain unchanged, and the read-only SQLite open can create
  the missing ordinary `-shm` WAL-index. Do not use `immutable` or disable
  SQLite locking and change detection for this lifecycle test.
- Prove policy revocation, inactive/ended listener, released Claim retirement,
  and active rotation cannot report ready and change the digest where a scope
  remains addressable.
- Prove an exact committed `work_candidate` promotion is accepted without
  weakening the immutable role contract.
- Prove a bound Scope resolves only its exact Project policy; an ordinary
  unbound Scope remains non-ready with either one or multiple unrelated
  policies in the Store.
- Prove bind replay cannot claim an old unbound Scope or replace a bound
  Project, and rotation admission rejects a conflicting Project basis.
- Prove `closed` Branch state produces a normal non-ready gap while an unknown
  stored lifecycle value fails without returning ready JSON. Exercise the
  closed-Branch read against a real WAL Store and record its `-shm` observation
  under the same boundary.

## SQLite references

- [Write-Ahead Logging, sections 4, 5, and 7](https://sqlite.org/wal.html)
  defines the WAL as persistent database state, the conditions for read-only
  WAL access, and the memory-mapped `-shm` WAL-index.
- [Opening a New Database Connection](https://sqlite.org/c3ref/open.html)
  defines `SQLITE_OPEN_READONLY` and states that the `immutable` URI parameter
  disables locking and change detection.
