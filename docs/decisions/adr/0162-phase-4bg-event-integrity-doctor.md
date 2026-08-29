# ADR-0162: Phase 4BG Event Integrity Doctor

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Phase 4BF exposed Event provenance through read-only Engine and CLI queries.
Those queries validate Event payload JSON as fixed-point WorkVCS canonical JSON
before returning it. `doctor` already validated Branch heads, replayable
commits, and checkpoint status coverage, but it did not yet include Event
provenance rows.

## Decision

1. Store integrity validation now enumerates all Event ids.
2. Each Event row is validated through the same read-only Event query path used
   by public Engine consumers.
3. The integrity report exposes `checked_events`.
4. CLI `doctor` reports `checked_events` while preserving the existing output
   fields.
5. Non-canonical Event payload JSON is treated as an integrity failure.

## Non-Goals

- This slice does not add Event repair, migration, mutation APIs, replay from
  Events, or Event projection indexes.
- Invalid Events are not counted as a tolerated state. Unlike checkpoint
  usability, corrupt Event provenance fails `doctor`.

## Consequences

- `doctor` now covers the Event provenance preserved by Bundle export/import
  and visible through `event show/list`.
- Query and integrity validation share one Event row interpretation path.
