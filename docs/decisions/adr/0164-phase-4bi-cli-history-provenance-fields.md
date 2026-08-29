# ADR-0164: Phase 4BI CLI History Provenance Fields

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The Engine `HistoryEntry` already carries ChangeSet id and commit/changeset
timestamps, but the CLI `history` rendering only printed commit kind,
operation, parent, and state digest. After Event provenance queries were added,
users needed a direct CLI path from history inspection to `event list
--changeset`.

## Decision

1. CLI history entries now include `changeset`.
2. CLI history entries now include `committed_at_us` and
   `changeset_created_at_us`.
3. The output remains a single key-value line per history entry.

## Non-Goals

- This slice does not change the Engine history model, event model, replay, or
  persistence schema.
- This slice does not add alternate history output formats.

## Consequences

- A CLI user can inspect history, copy the ChangeSet id, and query associated
  Event provenance without relying on the original mutation command output.
