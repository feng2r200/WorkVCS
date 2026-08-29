# ADR-0112: Phase 4I Restore Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge/restore capability sequence after ADR-0111.

## Context

The confirmed versioning design defines restore as an auditable operation that
reconstructs a selected historical WorkState without mutating the selected
commit, moving a Branch backward, deleting intervening history, or restoring
Runtime Coordination. Restore must create a new single-parent WorkStateCommit
from the current Branch head.

## Decision

1. Phase 4I adds `Engine::restore_work_state` and CLI `workvcs restore`.
2. Restore inputs are Branch, expected current head, target historical commit,
   and rationale JSON.
3. Restore replays both current head and target commit, requires both to belong
   to the same Workspace, and rejects no-op restores.
4. The restore ChangeSet is `workstate.restore` schema version `1`.
5. Restore computes deterministic entity/relation membership changes from the
   current WorkState to the target WorkState.
6. The result is a `commit_kind = 'normal'` WorkStateCommit with one
   `primary` parent: the expected current head.
7. The Branch head advances by compare-and-swap only after the restore commit,
   parent, ChangeOperations, and Event are written in the same transaction.
8. Restore does not read or recreate Session, Claim, Focus, MergeRuntime, or
   other Runtime Coordination rows.
9. This slice supports entity create/update/removal and relation create/removal
   in replayable restore ChangeOperations.
10. This slice defers relation version update restore, projection
    materialization, portable import/export, backup, federation, and remote
    behavior.

## Consequences

- A Branch can now be restored to an earlier WorkState while preserving
  intervening history.
- Historical replay can now apply entity removals, which restore requires for
  returning to states before an entity existed.
- Restore remains separate from historical Branch creation: it creates a new
  commit on the current Branch rather than checking out or rewinding the Branch.

## Implementation Findings

- The existing membership-change schema already supports entity removals; replay
  was the only missing piece for this restore slice.
- Restore ChangeSets use a summary payload like `merge.continue`, while each
  ChangeOperation carries the exact entity/relation membership transition.
