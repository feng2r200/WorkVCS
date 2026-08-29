# ADR-0032: Phase 3R Structural References Relations

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed structural relation, Goal, Plan, Task, and
  multi-Plan reuse requirements.

## Context

The confirmed relation vocabulary defines `references` as a structural edge
from referrer to target: the referrer uses an entity without changing the
target's home scope. Primary containment remains tree/forest-like, while
multi-Plan reuse uses `references`, not another primary `contains` edge.

Goal and Plan entities can refer to existing work discovered or organized
later without changing that work's identity. A Goal may reference Plans and
other work. A Plan may reference Plans and Tasks. A Task may be referenced by
multiple Plans. The current implementation already has the generic Relation
identity/version/WorkState membership storage path, primary containment, and
Task scheduling relations, but no semantic API for structural references.

## Decision

1. Phase 3R introduces a narrow structural reference relation foundation in
   `workvcs-core`.
2. The public surface remains Engine-owned and semantic:
   `Engine::create_structural_reference` and
   `Engine::structural_references_at`.
3. The implemented canonical relation type is `references` with canonical
   direction `referrer -> target` and an empty discriminator.
4. Supported Phase 3R endpoint pairs are:

   ```text
   Goal -> Plan
   Goal -> Task
   Plan -> Plan
   Plan -> Task
   ```

5. `Task` is not a supported structural-reference referrer in Phase 3R.
   `Goal` is not a supported structural-reference target in Phase 3R.
   Goal replacement remains a future evolution/supersession relation, not a
   structural `references` edge.
6. RelationVersion state is the canonical empty object metadata for this
   foundation slice.
7. A successful creation writes one normal WorkStateCommit with a relation
   ChangeOperation, relation membership change, Relation identity,
   RelationVersion, ChangeSet, Event, primary parent, and Branch HEAD movement
   atomically.
8. Duplicate current or historical logical keys are rejected because relation
   removal/update and identity reuse after removal remain unimplemented.
9. Structural references are read back through replayed WorkState relation
   membership and validate relation type, empty discriminator, canonical
   fixed-point metadata, digest, endpoint kinds, endpoint currentness, and
   same-Workspace membership.
10. Structural references do not drive primary containment, dependency
    readiness, runnable scope, priority, manual order, context ranking, or
    `next` in Phase 3R.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3R does not add business relation, runnable, Goal, Plan, Task, or
    `next` commands.
12. Phase 3R does not implement relation removal/update, reference-driven
    context, Goal readiness hints, Goal/Plan/Task supersession, Task referrer
    semantics, Goal reference targets, sibling ordering, priority resolution,
    full `next`, claim-next, Merge, Federation, Resource adapters, Bundle,
    Checkpoint, migrations, or business CLI commands.

## Consequences

- Goals and Plans can now record non-owning references to existing Plans and
  Tasks without changing primary containment or Task identity.
- Multiple Plans can reference the same Task while the single primary parent
  rule remains intact.
- Later context, `why`, supersession, and full `next` slices can consume
  structural references through a semantic read API instead of raw relation
  table access.

## Implementation Findings

- The confirmed text does not require Task as a structural-reference referrer
  or Goal as a structural-reference target for this foundation slice. Phase 3R
  rejects those pairs instead of inferring broader semantics from the generic
  relation vocabulary.
- Existing relation storage is sufficient for this slice. No schema change or
  projection table is required.
- Adding a new versioned relation operation type also requires adding that
  operation type to replay's supported normal ChangeSet vocabulary. The
  underlying relation membership replay path was already generic once the
  operation type was admitted.
