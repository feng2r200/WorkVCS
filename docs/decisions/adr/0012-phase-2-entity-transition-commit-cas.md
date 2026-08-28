# ADR-0012: Phase 2 Entity Transition, Commit, and CAS

- **Status:** Accepted for implementation
- **Accepted by:** current Work request to merge Replay locally into `main`
  and start the Phase 2 Entity transition / Commit / CAS slice from the new
  `main`.

## Context

ADR-0011 added Genesis replay and intentionally left normal commits
unsupported until a minimal ChangeOperation apply vocabulary existed. The
frozen implementation contract orders Entity transition / Commit / CAS after
Replay and before query broadening or upper-domain work.

## Decision

1. This slice introduces the first normal semantic mutation kernel:
   `entity.transition` schema version 1.
2. The public entry remains the Engine facade. The slice does not create a
   business CLI command, public Entity CRUD API, public DAO, or public SQLite
   handle.
3. One Engine entity transition creates or updates one EntityVersion as
   canonical semantic JSON, records ObjectIdentity and Entity ownership when
   creating an Entity, records one ChangeSet, one entity ChangeOperation, one
   entity_membership_change, one normal WorkStateCommit, one primary
   commit_parent, and one provenance Event atomically.
4. Branch HEAD movement uses expected-head compare-and-swap. SQLite writer
   serialization does not replace logical CAS.
5. Normal replay applies `entity.transition` ChangeOperations from the primary
   parent and validates resulting WorkState digest. Events, branch current
   projections, and projection state are not replay truth.
6. Relation transitions, entity removal, merge replay, branch creation beyond
   Genesis, projection materialization/rebuild, query broadening, Task,
   Runtime, Verification, Federation, Checkpoint, Bundle, Doctor, migration
   chains, and business CLI behavior remain deferred.

## Consequences

- A Workspace can now produce a linear normal Commit after Genesis and replay
  that Commit from immutable history.
- CAS conflicts fail before history rows are written, preserving atomicity.
- Projection corruption or absence cannot redefine canonical WorkState.

## Implementation findings

- None so far.
