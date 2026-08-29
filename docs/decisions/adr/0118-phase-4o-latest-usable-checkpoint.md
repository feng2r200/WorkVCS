# ADR-0118: Phase 4O Latest Usable Checkpoint

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  checkpoint sequence after ADR-0117.

## Context

After checkpoint creation, validation, and listing, tools still need a narrow
selection primitive for the common case: find the latest checkpoint for a commit
that is currently marked usable. Selection must not become checkpoint scheduling
or change replay authority.

## Decision

1. Phase 4O adds an Engine API to select the latest usable Checkpoint for one
   WorkStateCommit.
2. Selection reads `checkpoint` joined to `checkpoint_status` and returns the
   newest row whose `usability_state` is `usable`.
3. CLI support is limited to `workvcs checkpoint latest --commit <commit-id>`.
4. Selection is read-only. It does not validate, create, delete, schedule,
   evict, or restore checkpoints.

## Consequences

- Tools can pick a candidate checkpoint before validation or display.
- Invalid checkpoints are ignored by the latest-usable selector.
- Replay and WorkStateCommit remain authoritative.

## Implementation Findings

- No new contract ambiguity was found.
