# ADR-0167: Phase 4BL ChangeSet Query

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Branch and commit inspection now expose enough provenance to locate the
ChangeSet behind a WorkStateCommit. The remaining gap in the CLI inspection
chain was reading the ChangeSet itself: operation type/schema version,
canonical payloads, rationale, origin session, and the associated commit.

## Decision

1. Add an Engine-level `changeset(changeset_id)` read API.
2. The snapshot exposes ChangeSet identity, workspace, operation metadata,
   canonical operation payload/rationale JSON, their raw-byte digests and
   sizes, origin session, creation timestamp, Event count, change operation
   count, and associated commit references.
3. Add a thin CLI `changeset show` command using stable key-value output.
4. Read-time validation enforces stored text, positive operation schema
   version, nonnegative timestamps, typed identifiers, digests, and
   fixed-point canonical JSON for the ChangeSet payload fields.

## Non-Goals

- This slice does not add change operation detail listing.
- This slice does not alter ChangeSet creation, replay, commit persistence,
  Event provenance, or persistence schema.

## Consequences

- CLI provenance navigation now has a direct path from branch to commit to
  ChangeSet to associated Events.
- Change operation drill-down remains a separate read-side tool slice.
