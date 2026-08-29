# ADR-0168: Phase 4BM ChangeSet Operation Query

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

`changeset show` exposes ChangeSet-level payloads and counts, but it does not
show the `change_operation` rows that identify the ordered entity/relation
subjects touched by the ChangeSet. Provenance inspection needs this drill-down
without changing replay or mutation behavior.

## Decision

1. Add an Engine-level `changeset_operations(changeset_id)` read API.
2. The result exposes workspace, ChangeSet id, and ordered change operation
   rows.
3. Each operation snapshot includes operation id, ordinal, typed subject
   identity, canonical operation payload JSON, raw-byte digest, and byte size.
4. Add a thin CLI `changeset operations` command using stable key-value output.

## Non-Goals

- This slice does not alter ChangeSet creation, replay, commit persistence, or
  persistence schema.
- This slice does not add filters or graph traversal over operations.

## Consequences

- CLI provenance navigation can now inspect both ChangeSet summary data and the
  ordered subject-level operations it contains.
- Subject ids remain typed inside the Engine facade and are rendered as
  canonical UUID text by the CLI.
