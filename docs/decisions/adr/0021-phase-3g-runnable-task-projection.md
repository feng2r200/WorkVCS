# ADR-0021: Phase 3G Runnable Task Projection

- **Status:** Accepted for implementation
- **Accepted by:** current Work request establishing the long-running
  implementation goal and authorizing continuous WorkVCS implementation from
  confirmed repository requirements.

## Context

Phase 3A through Phase 3D introduced versioned Task, Task lifecycle,
Acceptance Criterion, Verification Requirement, and Verification projection
behavior. Phase 3E and Phase 3F introduced Runtime Session and Claim
coordination. The confirmed domain also requires deterministic runnable Task
resolution, but the complete `next` resolver still depends on surfaces that
are not implemented or not fully frozen.

The confirmed `next` precedence is:

```text
active Workspace and Work Branch
  -> active scope and Plan path
  -> executable Task descendants
  -> dependency readiness
  -> Task lifecycle eligibility
  -> priority
  -> explicit manual order
  -> Session and Claim coordination
```

The final stable tie-breaker among otherwise equal candidates is explicitly
Open in the confirmed baseline. Current Rust code also has no public semantic
relation creation API for `contains`, `depends_on`, or `ordered_before`, and
Task `child_order` remains fixed as an empty array.

## Decision

1. Phase 3G introduces only a read-only Runnable Task Projection foundation in
   `workvcs-core`. It does not implement complete `next` selection or
   claim-next.
2. The public surface remains Engine-owned and semantic. It must not expose
   SQLite handles, raw SQL, public generic projection CRUD, or CLI-to-SQL
   shortcuts.
3. The projection is evaluated for an active Session's active Workspace and
   active Branch. A missing or inactive Session is rejected.
4. The projection reads the active Branch head WorkState and interprets current
   Task entities through the existing Task semantic read boundary.
5. Projection is read-only. It creates no Event, ChangeSet, ChangeOperation,
   WorkStateCommit, Branch HEAD movement, Claim, Focus, or Task status change.
6. Phase 3G evaluates only implemented and confirmed dimensions: active
   Workspace/Branch, current Task membership at the active Branch head, Task
   lifecycle eligibility, Task priority value, and active exclusive Claim
   coordination.
7. `pending` and `in_progress` Tasks are lifecycle-eligible. `blocked`,
   `done`, `failed`, `cancelled`, and `superseded` Tasks are not runnable in
   this foundation slice.
8. An active exclusive Claim owned by the requesting Session leaves that Task
   runnable for the Session. An active exclusive Claim owned by another Session
   makes that Task claim-blocked for the Session. Released Claims do not block.
9. Results are ordered deterministically for reproducible display by confirmed
   implemented dimensions only: runnable tasks before non-runnable tasks, then
   stable Entity id bytes. The projection reports each Task's priority value,
   but Phase 3G does not interpret integer priority direction because the
   confirmed Rust implementation contract has not fixed that direction. This
   is not the confirmed final `next` tie-breaker.
10. The result must explicitly report deferred dimensions: active scope/Plan
    path, executable descendant traversal, dependency readiness, explicit
    manual order, and final equal-candidate tie-breaker.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3G does not add business runnable or `next` commands.
12. Failed projection writes no partial authoritative rows.

## Consequences

- WorkVCS can now show Agent-readable runnable status for Tasks in the active
  Session Branch without pretending the full `next` resolver is complete.
- Later relation, ordering, Plan path, complete `next`, and claim-next slices
  can replace deferred projection dimensions without changing the runtime
  ownership boundary.
- Claim coordination becomes visible to read-only scheduling logic while
  preserving the rule that claiming never changes Task status.

## Implementation findings

- The final equal-candidate stable tie-breaker for `next` is still Open.
  Phase 3G therefore returns an ordered projection list but does not select one
  authoritative next Task.
- Confirmed requirements establish priority as a scheduling dimension before
  manual order, but the Phase 3 implementation contract does not yet define
  whether larger or smaller integer values are higher priority. Phase 3G
  reports priority values without using the value direction as authority.
- Dependency readiness, executable descendants, active scope/Plan path, and
  manual order require relation/order/Plan-path semantics that are confirmed at
  the domain level but not yet implemented in the Rust semantic API. Phase 3G
  exposes them as deferred dimensions in result metadata.
- The projection uses active exclusive Claims from Phase 3F only. Shared
  Claims, stale takeover, force, and transfer remain deferred.
- Phase 3G tests cover `done`, `failed`, and `cancelled` as terminal
  lifecycle-ineligible Task statuses. `superseded` has no public
  supersession-aware semantic creation API in the current Rust surface, so its
  projection regression test is deferred to that future slice.
