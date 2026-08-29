# ADR-0109: Phase 4F Merge Resolution Freeze

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4 merge lifecycle sequence after ADR-0108.

## Context

Merge item resolution runtime state is provisional and can be revised. The
schema also provides immutable `merge_resolution` rows for final choices. Before
`continue` can create a two-parent merge commit, the implementation needs a
small step that freezes one complete resolution set without advancing the
target Branch.

## Decision

1. Phase 4F adds `MergeFreezeResolutionsOptions` and
   `MergeFreezeResolutionsResult` through the Engine facade.
2. Freezing is allowed only for an active merge attempt.
3. Freezing requires every `merge_item` in the attempt to have a corresponding
   `merge_resolution_runtime` row.
4. Freezing copies runtime rows into `merge_resolution` and preserves each
   runtime row's resolution kind, custom payload, rationale, resolver session,
   and timestamp.
5. Freezing is single-use: an attempt with any existing frozen resolution rows
   rejects another freeze.
6. Once an item has a frozen resolution row, further runtime resolution updates
   for that item are rejected.
7. CLI `merge freeze` is a thin wrapper over the Engine operation.
8. This slice does not create a merge commit, update `merge_runtime` to
   completed, write `merge_attempt_outcome`, advance the target Branch, or replay
   merge commits.

## Consequences

- Later `continue` can rely on a complete immutable resolution set.
- Runtime resolution rows remain visible after freeze as the current working
  projection; the frozen table is the final source for continue.
- AUTO items are not implicitly frozen as `theirs`; this slice requires an
  explicit runtime resolution for every item until a later contract adds an
  auto-apply rule.

## Implementation Findings

- The existing schema is sufficient for freezing; no DDL change was required.
