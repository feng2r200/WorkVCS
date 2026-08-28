# ADR-0016: Phase 3B Task Lifecycle Transition

- **Status:** Accepted for implementation
- **Accepted by:** current Work request authorizing the next Phase 3 slice after
  Phase 3A was merged locally into `main`.

## Context

Phase 3A introduced the first narrow Task semantic kernel: Tasks are stored as
`entity_kind = "task"` Entity versions, their semantic state is canonical JSON,
and `task_at` interprets a Task at a specific WorkStateCommit. That slice
created only `pending` Tasks and explicitly deferred lifecycle transitions.

The confirmed domain model separates Task execution status from open-semantic
outcome. It also defines the Task lifecycle vocabulary and distinguishes
non-terminal status from terminal status and explicit re-entry:

```text
non-terminal: pending | in_progress | blocked
terminal:     done | failed | cancelled | superseded
```

`done`, `failed`, and `cancelled` may return to a non-terminal status only
through an explicit rationale-bearing reopen or retry. `superseded` requires a
supersession-aware resolution and cannot use an ordinary reopen.

## Decision

1. Phase 3B introduces only a narrow Task lifecycle transition API in
   `workvcs-core`.
2. The public facade is Engine-owned and remains Task-specific. It must not
   expose SQLite handles, raw SQL, or public generic Entity CRUD.
3. The transition target is an existing Task Entity version at an expected
   Branch head. The caller supplies the Task Entity id, the expected current
   Task EntityVersion id, the next status, and optional next outcome.
4. A successful transition preserves Task identity, description, priority,
   empty `acceptance_criteria`, and empty `child_order`; it updates only
   `status` and `outcome`.
5. Outcome remains independent from status. A transition may set an outcome or
   clear it without implying Verification, Acceptance Criterion completion, or
   runtime claim state.
6. Standard transitions between non-terminal statuses, from non-terminal to
   ordinary terminal statuses `done`, `failed`, and `cancelled`, and direct
   atomic `pending` to `done` are allowed by this slice.
7. Entering `cancelled` from another status requires a non-empty rationale
   object, satisfying the confirmed provenance requirement for cancellation of
   active work.
8. Re-entering a non-terminal status from `done`, `failed`, or `cancelled`
   requires a non-empty rationale object. Re-entering from `superseded` is not
   implemented in Phase 3B because it requires supersession-aware resolution.
   Entering `superseded` is likewise deferred for the same reason.
9. Task lifecycle transition reuses the existing Entity transition kernel and
   therefore writes one atomic ChangeSet, one ChangeOperation, one EntityVersion,
   and one WorkStateCommit with the existing Branch HEAD compare-and-swap and
   expected EntityVersion checks.
10. Failed lifecycle transitions are structured errors and write no partial
   authoritative history rows.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3B does not add business Task CLI commands.
12. Acceptance Criteria, Verification, Runtime Session/Claim/Focus, `next`
    resolver, Merge, Federation, Resource adapters, Bundle, Checkpoint,
    migrations, a generic `SemanticOperation` framework, public generic Entity
    CRUD, supersession-aware reopen, and projection materialization remain
    deferred.

## Consequences

- WorkVCS can now update a Task's execution status and outcome while preserving
  immutable historical snapshots.
- Lifecycle changes remain normal Work State changes rather than runtime
  projections or side-channel updates.
- Future slices can add Acceptance Criteria, Verification-backed completion,
  Runtime claims, `next`, and supersession-aware resolution without changing
  this storage boundary.

## Implementation findings

- No new SQLite table or schema migration is required for this slice.
- The confirmed documents define terminal re-entry rules, and state that
  `superseded` requires supersession-aware resolution, but do not define a full
  transition graph for every ordinary status pair. Phase 3B therefore implements
  only those confirmed constraints and does not invent a stricter graph.
