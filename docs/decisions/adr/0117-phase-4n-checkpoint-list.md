# ADR-0117: Phase 4N Checkpoint List

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  checkpoint sequence after ADR-0116.

## Context

ADR-0115 and ADR-0116 allow tools to create, read, and validate individual
checkpoints. The next needed tool surface is a narrow way to discover existing
checkpoints for a WorkStateCommit without introducing checkpoint scheduling or a
new recovery path.

## Decision

1. Phase 4N adds `CheckpointListOptions::for_commit` and an Engine API that
   lists checkpoints associated with one commit.
2. The list result returns the requested commit id and ordered
   `CheckpointSnapshot` entries.
3. CLI support is limited to `workvcs checkpoint list --commit <commit-id>`.
4. Listing is read-only. It does not replay, validate, create, delete, schedule,
   evict, or select checkpoints automatically.

## Consequences

- Tools can inspect the checkpoint inventory for a commit and choose a
  checkpoint to validate or display.
- Checkpoint scheduling and recovery policy remain deferred.

## Implementation Findings

- No new contract ambiguity was found.
