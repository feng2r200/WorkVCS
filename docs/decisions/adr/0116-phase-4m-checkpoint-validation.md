# ADR-0116: Phase 4M Checkpoint Validation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  checkpoint sequence after ADR-0115.

## Context

ADR-0115 added checkpoint creation and read support. Because the v0.1 physical
schema records ContentObject metadata but not checkpoint payload bytes, a
checkpoint remains useful only if tools can rebuild the expected checkpoint
payload from replay and compare it with the stored checkpoint metadata.

## Decision

1. Phase 4M adds `validate_checkpoint` to the Engine facade and
   `workvcs checkpoint validate` to the CLI.
2. Validation reloads the Checkpoint, replays its target WorkStateCommit,
   rebuilds the v1 canonical checkpoint payload, and compares workspace id,
   state digest, format version, content digest, content size, media type, and
   format metadata.
3. Validation updates `checkpoint_status` in place:
   - `usable` when the rebuilt values match the stored Checkpoint;
   - `invalid` when checkpoint metadata no longer matches replay.
4. A semantic mismatch is returned as a validation result with `valid=false`,
   not as a failed Engine operation. Storage, bootstrap, or replay failures still
   return errors.
5. This slice does not add checkpoint restore, checkpoint scheduling, eviction,
   bundle transport, or raw payload persistence.

## Consequences

- Checkpoint status is now actively refreshable from the replay authority.
- Tools can distinguish an unusable checkpoint from an Engine/runtime failure.
- Replay and WorkStateCommit remain authoritative; checkpoint validation never
  rewrites history.

## Implementation Findings

- No new contract ambiguity was found. The lack of raw payload persistence is
  the same Phase 4L finding and remains deferred.
