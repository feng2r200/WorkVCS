# ADR-0110: Phase 4G Merge Commit Replay

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge lifecycle sequence after ADR-0109.

## Context

`merge continue` will eventually create a two-parent `WorkStateCommit`. Before
that operation can be safe, historical replay must understand merge commits.
The frozen design says replay reconstructs history from the recorded ChangeSet
along the primary-parent chain and never reruns the merge algorithm.

## Decision

1. Phase 4G enables `state_at` replay for `commit_kind = 'merge'`.
2. A merge commit must have exactly two parents.
3. Parent ordinal `0` must have role `primary`; parent ordinal `1` must have
   role `secondary`.
4. Both parents must replay to the same workspace as the merge commit.
5. The replayed merge state starts from the primary parent's WorkState and
   applies the merge commit's recorded ChangeSet.
6. The resulting WorkState digest must match `workstate_commit.state_digest`.
7. Unsupported ChangeSet operation types remain `ReplayUnsupported`.
8. This slice does not create merge commits, freeze new resolutions, advance
   Branch heads, or implement `merge continue`.

## Consequences

- Future `merge continue` can create replayable merge commits by recording the
  exact ChangeOperations needed to transform the target head into the merged
  state.
- The secondary parent is validated as provenance and ancestry, but replay does
  not derive state by diffing or rerunning merge classification.

## Implementation Findings

- The existing ChangeOperation replay path was already reusable for merge
  commits once parent validation was added.
