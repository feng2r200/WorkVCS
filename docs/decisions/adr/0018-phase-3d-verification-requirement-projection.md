# ADR-0018: Phase 3D Verification Requirement Projection

- **Status:** Accepted for implementation
- **Accepted by:** current Work request establishing the long-running
  implementation goal and authorizing continuous WorkVCS implementation from
  confirmed repository requirements.

## Context

Phase 3C introduced Acceptance Criteria and intentionally treated every
mandatory criterion as unverified because Verification Requirements,
Verification judgments, and effective projections were still deferred.

The confirmed domain and architecture require:

- stable AC-local Verification Requirement identity;
- immutable single-target Verification judgments;
- Verification targets through the canonical `verifies` Relation;
- distinct historical Verification result and current applicability;
- derived, non-materialized Acceptance Criterion effective status; and
- mandatory Task completion only when every mandatory Acceptance Criterion
  projects to `verified`.

Schema v0.1 already contains `verification_requirement_identity`,
`relation`, `relation_version`, `relation_membership_change`, and
`verification_basis`. No schema migration is required for the narrow slice.

## Decision

1. Phase 3D introduces only the Verification Requirement and effective
   Verification projection slice in `workvcs-core`.
2. The public surface remains Engine-owned and semantic. It must not expose
   SQLite handles, raw SQL, public generic Entity CRUD, or CLI-to-SQL
   shortcuts.
3. A Verification Requirement is stored as an Entity with
   `entity_kind = "verification_requirement"` and one row in
   `verification_requirement_identity`.
4. The identity row uses the owning Acceptance Criterion Entity id and a
   stable non-empty AC-local key. The `(owner_entity_id, local_key)` pair is
   unique.
5. Verification Requirement state uses canonical JSON with this Phase 3D
   shape:

   ```json
   {
     "statement": "The store reopens from persisted state."
   }
   ```

   The statement records the intent to prove. It does not prescribe a command,
   test framework, Resource adapter, or execution method.
6. Acceptance Criterion state now allows a deterministic
   `verification_requirements` array. Each entry records the stable AC-local
   key and the Verification Requirement Entity id. The array is canonicalized
   by local key so insertion order does not change Acceptance Criterion state
   bytes.
7. Creating a Verification Requirement is one atomic normal Work State commit.
   It writes the new Verification Requirement EntityVersion, the updated
   Acceptance Criterion EntityVersion, the identity row, two entity
   ChangeOperations, one ChangeSet, one WorkStateCommit, one primary parent,
   one event, and the Branch HEAD compare-and-swap update.
8. Revising a Verification Requirement preserves its Entity id and local key.
   It writes a new Verification Requirement EntityVersion through the existing
   entity transition storage path and does not rewrite the owning Acceptance
   Criterion unless the membership list changes.
9. A Verification is stored as an Entity with `entity_kind = "verification"`.
   Its Phase 3D state records result, method descriptor, and the basis summary
   needed for deterministic effective projection. A successful creation also
   writes exactly one defining `verifies` Relation from the Verification Entity
   to either an Acceptance Criterion Entity or a Verification Requirement
   Entity.
10. Phase 3D Verification basis is limited to a work-state-only basis:
    `verified_at_commit_id` and explicit semantic dependencies. Resource
    Basis, Artifact/Input Basis, ResourceObservation, adapter-backed drift
    comparison, and Evidence capture remain deferred.
11. The defining Verification closure is immutable in this slice: ordinary
    revision of a Verification Entity or its defining `verifies` Relation is
    not exposed. Re-verification creates a new Verification.
12. Because `verifies` is a confirmed Relation-backed target, this slice adds
    minimal relation membership write/replay support for relation creations
    produced by Engine semantic operations. It does not implement public
    generic Relation CRUD, relation deletion, relation update, merge replay, or
    custom relation algorithms.
13. Effective Acceptance Criterion status is derived at read/gate time, not
    materialized as a separate cache. The Phase 3D projection recognizes:
    `unverified`, `verified`, `failed`, `stale`, and `conflicted`.
14. Without Verification Requirements, one applicable passed Verification
    targeting the Acceptance Criterion may satisfy it. With Verification
    Requirements, every required Requirement must have an applicable passed
    Verification and no applicable failed Verification may defeat it.
15. Work-state-only applicability is computed conservatively from the recorded
    dependencies. If a recorded dependency is absent or at a different current
    EntityVersion, the Verification is stale. Resource-backed `unknown`
    applicability remains deferred until Resource Basis is implemented.
16. Task completion now uses the effective projection. Optional Acceptance
    Criteria do not block completion. Mandatory Acceptance Criteria must
    project to `verified`; `unverified`, `failed`, `stale`, and `conflicted`
    do not satisfy the gate.
17. Failed Verification Requirement or Verification creation/revision writes no
    partial authoritative history rows.
18. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3D does not add business Task, Acceptance Criterion, Verification
    Requirement, or Verification commands.
19. Runtime Session/Claim/Focus, `next` resolver, Merge, Federation, Resource
    adapters, ResourceObservation capture, Evidence capture, Artifact/Input
    Basis, Bundle, Checkpoint, schema migration, public generic Relation CRUD,
    public generic Entity CRUD expansion, projection materialization, a general
    verification policy DSL, AC waiver operations, and supersession-aware Task
    reopen remain deferred.

## Consequences

- WorkVCS can express multi-part required verification coverage while keeping
  stable historical references.
- Mandatory Acceptance Criteria can become satisfiable through immutable
  Work-State-backed Verification judgments.
- The first Relation-backed semantic operation is implemented only as much as
  Verification needs, while the broader relation API remains deferred.

## Implementation findings

- Phase 3D exposes that relation membership replay is a prerequisite for any
  correct Verification implementation because the confirmed target relation is
  `verifies` and WorkState digests already include relation mappings.
- Ordinary Acceptance Criterion revision must preserve the current
  `verification_requirements` list. Otherwise revision would act as an
  unconfirmed structure-removal operation and could re-enable direct
  Acceptance Criterion verification for criteria that require Requirement
  coverage.
- Resource-backed applicability cannot be implemented without Resource
  adapters and ResourceObservation comparison. Phase 3D therefore limits
  applicability to explicit Work-State dependencies instead of guessing
  Resource state.
- Evidence capture is still deferred. Phase 3D fixes the defining target
  relation and supports an empty defining Evidence relation set; later Evidence
  support can add fixed `evidenced_by` relations at Verification creation
  without changing existing Verification identity.
