# ADR-0165: Phase 4BJ Commit Metadata Query

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

`history` now supports merge commits through a deterministic first-parent view,
and CLI history entries expose ChangeSet provenance. That is enough for linear
inspection, but it does not expose the full DAG shape of a specific
WorkStateCommit. Merge review and provenance debugging need a direct tool for
reading a commit node, its ChangeSet, and all persisted parent rows.

## Decision

1. Add an Engine-level `commit(commit_id)` read API returning commit metadata.
2. The snapshot includes workspace, commit, ChangeSet, commit kind, state
   digest, commit timestamp, operation type/schema version, ChangeSet creation
   timestamp, optional origin session, and all parents.
3. Add a thin CLI `commit show` command that renders the snapshot as stable
   key-value output.
4. Commit parent shape is validated at read time for current V0.1 commit kinds:
   genesis has no parents, normal has one ordinal-0 `primary` parent, and merge
   has ordinal-0 `primary` plus ordinal-1 `secondary` parents.

## Non-Goals

- This slice does not add graph traversal, alternate history output, or commit
  mutation commands.
- This slice does not alter merge replay, merge runtime behavior, Event
  provenance, or persistence schema.

## Consequences

- `history` remains the linear first-parent inspection tool.
- `commit show` becomes the exact DAG node inspection tool for parent and
  ChangeSet provenance.
