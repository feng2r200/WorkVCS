# ADR-0163: Phase 4BH Merge First-Parent History

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Merge replay is implemented from the primary parent plus the recorded merge
ChangeSet, while Events remain provenance. The `history` query still rejected a
merge WorkStateCommit as deferred, which made a branch whose head is a completed
merge commit harder to inspect through the public Engine/CLI boundary.

## Decision

1. `history` now supports merge commits by validating that the commit has two
   parent rows.
2. Linear history inspection follows the ordinal-0 `primary` parent from a
   merge commit.
3. The secondary parent is retained in the DAG but is not traversed by this
   first-parent history query.

## Non-Goals

- This slice does not implement a graph traversal history view, merge replay
  changes, merge conflict resolution changes, or Event-based replay.
- This slice does not alter commit parent persistence or merge write paths.

## Consequences

- `history --branch` and `history --commit` can inspect branches after merge
  completion using a deterministic first-parent view.
- Secondary branch history remains available through its own branch/commit
  query rather than being folded into linear history output.
