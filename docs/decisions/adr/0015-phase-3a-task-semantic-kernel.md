# ADR-0015: Phase 3A Task Semantic Kernel

- **Status:** Accepted for implementation
- **Accepted by:** current Work request confirming the next stage content and
  authorizing implementation.

## Context

Phase 2 closed the first Store/history vertical slice: canonical values,
SQLite bootstrap/open, Workspace Genesis, replay, Entity transition,
Commit/CAS, query/show-at/history, doctor integrity, and concurrency/integrity
tests are implemented on `main`.

V1 requires versioned Task state, but the confirmed logical schema places Task
inside the unified Entity/EntityVersion family. Schema v0.1 has no dedicated
Task table and does not require one. A Task implementation should therefore
prove the first real V1 semantic object on top of existing immutable history
without reopening schema or exposing generic Entity CRUD.

## Decision

1. Phase 3A introduces only a narrow Task semantic kernel in `workvcs-core`.
2. A Task is represented as an Entity whose immutable `entity_kind` is `task`.
   Its versioned semantic state is stored in `entity_version.state_json` as
   WorkVCS canonical semantic JSON.
3. The canonical Task state shape for this slice is:

   ```json
   {
     "acceptance_criteria": [],
     "child_order": [],
     "description": "...",
     "outcome": null,
     "priority": 0,
     "status": "pending"
   }
   ```

4. `status` is limited to the confirmed Task lifecycle vocabulary:
   `pending`, `in_progress`, `blocked`, `done`, `failed`, `cancelled`, and
   `superseded`. Phase 3A creates only `pending` Tasks.
5. `outcome` is independent from `status`. Phase 3A creates Tasks with
   `outcome = null` and does not implement lifecycle completion.
6. `priority` is a JSON safe integer. Phase 3A accepts a caller-provided
   priority but does not implement `next` resolution or manual ordering.
7. `acceptance_criteria` and `child_order` are present as empty arrays to keep
   the Task state shape stable, but Phase 3A does not implement Acceptance
   Criterion identity, Verification Requirement identity, child Task ordering,
   or Task decomposition operations.
8. Core exposes narrow Engine-owned Task APIs for create and readback by
   WorkStateCommit. They do not expose SQLite handles, raw SQL, or public
   generic Entity CRUD.
9. Task create reuses the existing internal entity transition kernel and
   therefore writes one atomic ChangeSet, one ChangeOperation, one
   EntityVersion, and one WorkStateCommit with the existing Branch HEAD CAS
   behavior.
10. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3A does not add business Task CLI commands.
11. Task lifecycle transitions, Acceptance Criteria, Verification,
    Runtime Session/Claim/Focus, `next` resolver, Merge, Federation, Resource
    adapters, Bundle, Checkpoint, migrations, a generic `SemanticOperation`
    framework, public generic Entity CRUD, and projection materialization
    remain deferred.

## Consequences

- WorkVCS can create and replay the first real V1 semantic object while still
  using the proven Phase 2 Store/history path.
- Task readback is explicit semantic interpretation over a replayed Work State,
  not a competing projection authority.
- Future lifecycle, AC, Verification, Runtime, and `next` slices can extend the
  Task semantic surface without changing this slice's storage boundary.

## Implementation findings

- The implementation did not require any schema change, Task-specific table,
  Task-specific `object_kind`, or public generic Entity CRUD.
- The semantic Task create API is persisted as the existing
  `entity.transition` ChangeSet/ChangeOperation shape for this slice. A future
  semantic operation vocabulary can wrap or refine that surface, but Phase 3A
  does not introduce a generic `SemanticOperation` framework.
