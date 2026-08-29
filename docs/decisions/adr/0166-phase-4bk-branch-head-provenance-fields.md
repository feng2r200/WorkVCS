# ADR-0166: Phase 4BK Branch Head Provenance Fields

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

`branch head` and `branch list` are the natural entry points for inspecting a
workspace. Before this slice, they exposed the head commit id and state digest,
but not the ChangeSet and operation metadata needed to jump directly into
`commit show` or `event list --changeset`.

## Decision

1. Extend `BranchHead` with metadata from the current head commit and its
   ChangeSet.
2. `branch head` now renders the head ChangeSet id, commit kind, operation
   type/schema version, commit timestamp, and ChangeSet creation timestamp.
3. `branch list` renders the same head provenance fields for each branch.

## Non-Goals

- This slice does not alter branch lifecycle, branch movement, replay,
  projections, merge runtime, or persistence schema.
- This slice does not add alternate branch output formats.

## Consequences

- A CLI user can move from workspace/branch inspection to commit or event
  provenance without first running a history query.
- The BranchHead Engine snapshot remains a read-only facade over stored branch
  and head commit state.
