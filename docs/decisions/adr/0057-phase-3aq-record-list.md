# ADR-0057: Phase 3AQ Record List Projection

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed explicit semantic Record boundary.

## Context

Phase 3AL through 3AP made Finding, Assumption, ordinary Decision, Question,
and Risk Records writable as explicit semantic `record` entities. A local Agent
can persist these Records, but the thin CLI still cannot query the current
Record set at a specific history point.

The next useful tool capability is a read-only list projection over the
versioned WorkState. It must preserve the Engine/Store boundary and avoid
turning Record listing into a direct SQL command in the CLI.

## Decision

1. Phase 3AQ adds `RecordListOptions` and `RecordListResult`.
2. `Engine::records_at` lists Records visible at a specific commit by replaying
   WorkState and then loading only entities whose `entity_kind` is `record`.
3. Record list results are sorted by `record_entity_id` for deterministic tool
   output.
4. The list projection supports an optional `RecordKind` filter.
5. The CLI exposes `record list STORE --commit <id> [--kind <kind>]` and renders
   stable key/value rows with IDs, current version IDs, state digests, kind, and
   status.
6. This slice does not implement text search, pagination, context ranking,
   Record detail rendering, `record show`, Attempt records, Handoff records, or
   promoted Decision entities.

## Consequences

- A local Agent can now inspect explicit semantic Records without reading raw
  history state or knowing entity IDs ahead of time.
- Record listing remains a history projection over a commit, not a mutable
  current-table scan.

## Implementation Findings

- No schema or Record state-shape changes were required.
