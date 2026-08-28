# ADR-0017: Phase 3C Acceptance Criteria

- **Status:** Accepted for implementation
- **Accepted by:** current Work request authorizing Phase 3C on
  `work/phase-3c-acceptance-criteria`.

## Context

Phase 3A introduced Task entities with a stable semantic state shape and empty
`acceptance_criteria` and `child_order` arrays. Phase 3B added Task lifecycle
transitions while continuing to defer Acceptance Criteria and Verification.

The confirmed domain model defines Acceptance Criteria as stable, verifiable
success conditions owned by a Task. They have stable Task-local identity,
statement text, required/optional classification, and may later own stable
Verification Requirements. Every mandatory criterion must have an effective
`verified` projection before a Task may be completed.

The v0.1 physical schema already contains `acceptance_criterion_identity`,
allowing an Acceptance Criterion Entity to keep a stable owning Task and local
key without a schema migration.

## Decision

1. Phase 3C introduces only the Acceptance Criterion semantic slice in
   `workvcs-core`.
2. The public surface remains Engine-owned and Task-specific. It must not expose
   SQLite handles, raw SQL, public generic Entity CRUD, or a broad
   `SemanticOperation` framework. The earlier Phase 2 entity transition kernel
   remains available only for non-reserved entity kinds; Task and Acceptance
   Criterion changes must use their semantic Engine APIs.
3. An Acceptance Criterion is stored as an Entity with
   `entity_kind = "acceptance_criterion"` and one row in
   `acceptance_criterion_identity`.
4. The identity row uses the owning Task Entity id and a stable non-empty local
   key. The `(owner_entity_id, local_key)` pair is unique.
5. Acceptance Criterion state uses canonical JSON with this Phase 3C shape:

   ```json
   {
     "classification": "required",
     "statement": "The store opens after process restart.",
     "verification_requirements": []
   }
   ```

   `classification` is either `required` or `optional`.
   `verification_requirements` remains present as an empty array because
   Verification Requirements are deferred.
6. Task state now allows non-empty `acceptance_criteria`. Each entry records the
   stable Task-local key and the Acceptance Criterion Entity id. The array is
   canonicalized by local key so insertion order does not change Task state
   bytes. `child_order` remains an empty array.
7. Creating an Acceptance Criterion is one atomic normal Work State commit. It
   writes the new Acceptance Criterion EntityVersion, the updated Task
   EntityVersion, the identity row, two entity ChangeOperations, one ChangeSet,
   one WorkStateCommit, one primary parent, one event, and the Branch HEAD
   compare-and-swap update.
8. Revising an Acceptance Criterion preserves its Entity id and local key. It
   writes a new Acceptance Criterion EntityVersion through the existing entity
   transition storage path and does not rewrite the owning Task unless the Task
   membership list changes.
9. Failed Acceptance Criterion creation or revision writes no partial
   authoritative history rows.
10. Completion of a Task is blocked when any owned mandatory Acceptance
    Criterion lacks an effective `verified` projection. Because Phase 3C does
    not implement Verification or materialized effective projections, any
    mandatory criterion is treated as unverified. Optional criteria do not block
    completion.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3C does not add business Task or Acceptance Criterion commands.
12. VerificationRequirement, Verification, Evidence judgment closure,
    projection materialization, Runtime Session/Claim/Focus, `next` resolver,
    Merge, Federation, Resource adapters, Bundle, Checkpoint, schema migration,
    public generic Entity CRUD, and supersession-aware Task reopen remain
    deferred.

## Consequences

- WorkVCS can attach stable, versioned Acceptance Criteria to Tasks and revise
  them without breaking historical references.
- The first semantic completion gate is now enforced for mandatory criteria,
  while the actual Verification model remains a later slice.
- The implementation can use the existing multi-ChangeOperation replay path
  without changing schema v0.1.

## Implementation findings

- No new SQLite table or schema migration is required for this slice.
- The confirmed domain requires mandatory criteria to be verified before
  completion, but Verification projections are not implemented yet. Phase 3C
  therefore treats mandatory criteria as unverified and blocks `done` until a
  later Verification slice provides effective projection data.
- Independent review found that the earlier public entity transition entry
  could otherwise bypass the mandatory completion gate. Phase 3C closes that by
  rejecting public generic transitions for reserved semantic entity kinds
  `task` and `acceptance_criterion`.
