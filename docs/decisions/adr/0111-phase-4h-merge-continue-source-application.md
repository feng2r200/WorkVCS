# ADR-0111: Phase 4H Merge Continue Source Application

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge lifecycle sequence after ADR-0110.

## Context

Phase 4A through 4G created merge attempts, classified items, recorded runtime
resolutions, froze those resolutions, and enabled replay of two-parent merge
commits. The next useful vertical slice is to make a frozen merge attempt
produce a durable merge commit and complete the merge attempt.

## Decision

1. Phase 4H adds `Engine::continue_merge` and CLI `workvcs merge continue`.
2. Continue requires an active merge attempt and frozen resolutions for every
   merge item.
3. Continue reuses the target and source branch heads captured when the merge
   attempt was started.
4. The target and source branch heads must still match those captured heads at
   continuation time.
5. The result is a `commit_kind = 'merge'` WorkStateCommit with exactly two
   parents: ordinal `0`/`primary` is the captured target head, and ordinal
   `1`/`secondary` is the captured source head.
6. `ours` resolutions produce no ChangeOperation and keep the target state for
   that item. If all items resolve to `ours`, continue still writes a two-parent
   no-op merge commit whose replayed state equals the primary parent state.
7. `theirs` resolutions produce replayable entity/relation membership
   ChangeOperations when the current replay vocabulary supports that transition.
8. The target Branch head advances to the new merge commit only after the
   merge commit and parents are written in the same transaction.
9. Continue marks `merge_runtime` completed, writes a completed
   `merge_attempt_outcome`, and records a `merge.completed` Event.
10. This slice does not implement custom merge materialization, entity removal,
    relation version update replay, restore, federation, or remote merge
    behavior.

## Consequences

- A source-side entity create/update can now be accepted into the target branch
  through the merge lifecycle.
- Completed merge attempts disappear from the default active merge list and are
  visible with `include_closed`.
- Future slices can extend the replay vocabulary and then enable additional
  merge transition forms without changing the merge attempt lifecycle.

## Implementation Findings

- Frozen merge item payloads already contain the target/source version inputs
  needed to build deterministic replayable ChangeOperations.
- The existing replay path required a small `merge.continue` ChangeSet allowlist
  addition because completed merge commits use a summary ChangeSet payload while
  each ChangeOperation carries the exact membership transition.
- Empty `merge.continue` ChangeSets are valid no-op transforms for all-ours
  merge commits; non-merge ChangeSets still require at least one ChangeOperation.
