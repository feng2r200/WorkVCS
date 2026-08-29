# ADR-0104: Phase 4A Merge Start Foundation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge/restore capability gap.

## Context

WorkVCS already has Branch history, WorkState replay, branch fork, and runtime
session coordination. The schema also includes merge attempt and runtime tables,
but the Engine has no operation that records a merge attempt boundary. Without
that boundary, later merge classification, resolution, and restore slices would
not have a stable runtime anchor.

## Decision

1. Phase 4A adds a merge-start operation through the Engine facade.
2. `Engine::start_merge` records one `merge_attempt`, one `merge_runtime` row,
   and one runtime `merge.started` event.
3. The operation captures target Branch, source Branch, their current heads, and
   their nearest common merge base at start time.
4. The operation may attach an active origin session from the same workspace and
   updates that session activity timestamp.
5. The CLI adds only `workvcs merge start` as a thin wrapper over the Engine API.
6. This slice does not classify merge items, resolve conflicts, create a
   two-parent commit, update target branch head, complete/abort merge attempts,
   or implement federation/remote merge behavior.

## Consequences

- Later merge slices have a durable runtime object and start-time branch-head
  snapshot to build on.
- A target branch can have only one active merge attempt until a later slice
  introduces an explicit outcome path.
- Starting a merge is metadata/runtime work only; it does not mutate WorkState.

## Implementation Findings

- The existing `merge_attempt` and `merge_runtime` schema supports this first
  boundary directly; no schema change was needed.
- Merge-base discovery can reuse existing commit parent edges and workspace
  ownership validation for this slice.
