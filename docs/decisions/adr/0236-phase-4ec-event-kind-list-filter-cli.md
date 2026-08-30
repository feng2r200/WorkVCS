# ADR-0236: Phase 4EC Event Kind List Filter CLI

Status: Accepted

Date: 2026-08-30

## Context

Event list can query events by ChangeSet, Session, or Workspace and can limit
the number of rendered rows. It cannot narrow results by event kind even though
the event kind is already part of every Event snapshot and list row.

This slice adds a read-side CLI filter only. It does not change the Event
journal, event payloads, storage schema, replay, or the Engine event query API.

## Decision

1. Add `--kind KIND` to `workvcs event list`.
2. Keep the existing requirement that exactly one target selector is supplied:
   `--changeset`, `--session`, or `--workspace`.
3. Preserve the existing `--limit` validation that the limit must be greater
   than zero.
4. When `--kind` and `--limit` are used together, load the selected target,
   filter by event kind, then truncate the filtered result to the requested
   limit.

## Non-Goals

- This slice does not add event kind vocabulary validation.
- This slice does not add time range, payload, digest, or subject filters.
- This slice does not change Event journal writing or payload canonicalization.
- This slice does not change SQLite schema, replay, or core snapshot APIs.

## Consequences

- CLI users can inspect a focused event stream without external post-processing.
- Event debugging workflows can combine target selection with kind and limit.
- The command remains a thin read-only wrapper over existing Engine snapshots.
