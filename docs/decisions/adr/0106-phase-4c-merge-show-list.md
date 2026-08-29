# ADR-0106: Phase 4C Merge Show and List

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge lifecycle sequence after ADR-0104 and ADR-0105.

## Context

Merge start and abort now persist attempts, runtime state, outcomes, and events.
Tools need a read-side surface to inspect a merge attempt by id and to find the
active merge attempts for a workspace or target Branch before later resolution
and continue operations are implemented.

## Decision

1. Phase 4C adds `MergeAttemptSnapshot`, `MergeListOptions`, and
   `MergeListResult` through the Engine facade.
2. `Engine::merge_attempt` returns one merge attempt with captured base/head
   ids, runtime state, and optional immutable outcome detail.
3. `Engine::merge_attempts` lists merge attempts by workspace, optionally
   narrowed to one target Branch.
4. Merge list defaults to active attempts only; callers must opt into closed
   attempts with `include_closed`.
5. The CLI adds `workvcs merge show` and `workvcs merge list` as thin wrappers.
6. This slice does not classify merge items, resolve conflicts, create frozen
   resolutions, create a two-parent commit, update target Branch head, or
   implement federation/remote merge behavior.

## Consequences

- Tooling can discover active merge attempts without keeping only command output
  from `merge start`.
- Closed aborted attempts remain queryable when requested but do not clutter the
  default active work view.
- Later resolve/continue slices can reuse the same snapshot shape.

## Implementation Findings

- The read-side can derive active versus closed state from
  `merge_attempt_outcome` while still validating `merge_runtime.runtime_json`.
- Completed merge snapshots are represented in the read model vocabulary even
  though the completed outcome producer remains deferred to a later slice.
