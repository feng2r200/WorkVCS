# ADR-0033: Phase 3S WorkState Diff Query

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed query, WorkState, history, and replay requirements.

## Context

The confirmed query semantics define `diff` as comparing Work State at commits
or Branch heads. Earlier Phase 2 query work implemented Branch HEAD, history,
and show-at, while leaving `diff`, `why`, `context`, `next`, projection
materialization, and merge semantics deferred.

Current implementation now has replayed WorkState mappings for Entity and
Relation memberships across generic Entity transitions, Task/Plan/Goal
semantic transitions, primary containment, scheduling relations, structural
references, and Verification relations. This is enough to provide a
deterministic WorkState-level diff without interpreting higher-level business
semantics.

## Decision

1. Phase 3S introduces a read-only WorkState diff query foundation in
   `workvcs-core`.
2. The public surface remains Engine-owned and semantic:
   `Engine::diff(WorkStateDiffOptions)`.
3. Diff inputs support either an explicit Commit id or the current head of a
   Branch. Branch-head selectors resolve to their Branch's current
   WorkStateCommit before replay.
4. Both selected WorkStates must belong to the same Workspace. Cross-Workspace
   diff is rejected as a structured query error.
5. The result reports the original selectors, resolved commit ids, Workspace
   id, state digests, entity version changes, and relation version changes.
6. Entity and Relation changes are classified as:

   ```text
   added    before absent, after present
   removed  before present, after absent
   updated  before and after present with different version ids
   ```

7. Changes are ordered deterministically by Entity id or Relation id.
8. Diff uses replayed WorkState mapping as authority. It does not read
   projection tables or Event rows as current state.
9. Diff is read-only. It creates no Event, ChangeSet, ChangeOperation,
   WorkStateCommit, Branch HEAD movement, projection row, runtime row, Entity
   version, or Relation version.
10. Phase 3S does not interpret semantic meaning of the changed objects. It
    does not compute field-level diffs, relation graph explanations, context,
    readiness, progress, `why`, `next`, claim-next, restore, merge, or
    projection materialization.
11. CLI remains the existing thin shell over Store/history smoke commands.
    Phase 3S does not add a CLI `diff` command.

## Consequences

- WorkVCS can now answer a basic current/historical WorkState difference query
  without mutating state or relying on derived projections.
- Later semantic diff, `why`, context, restore, and merge work can build on a
  deterministic object-version delta surface.
- Branch-head and Commit selectors are both represented in the result, so
  callers can distinguish a symbolic current-head query from a literal commit
  comparison.

## Implementation Findings

- `diff` can compare removals even before relation/entity removal operations
  exist by comparing a later WorkState to an earlier one. This is an output
  classification, not an implemented removal mutation.
- Replay and Commit lookup errors are preserved from the underlying replay
  boundary. Selector validation and cross-Workspace checks are query errors.
- Priority direction and final `next` tie-breakers remain unfrozen. Phase 3S
  therefore deliberately avoids using priority or manual order as scheduling
  authority.
