# ADR-0063: Phase 3AW Record Relation List

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and
  ADR-0062 `invalidates` Record relation scope.

## Context

Phase 3AV made `invalidates` Record relations writable, but the only inspection
surface was generic WorkState relation membership. For the tool to be useful, a
local Agent needs to list the current semantic Record relations at a commit and
filter them by endpoint.

## Decision

1. Phase 3AW adds `RecordRelationSnapshot`, `RecordRelationListOptions`, and
   `RecordRelationListResult`.
2. `record_relations_at` replays the target commit first and lists only
   relation versions that are current in that WorkState.
3. The loader recognizes only the confirmed `invalidates` Record relation
   family in this slice.
4. Returned relations are sorted by type, source Record id, target Record id,
   and relation id for deterministic CLI output.
5. The CLI exposes `record relation-list STORE --commit ...` with optional
   `--type invalidates`, `--source-record`, and `--target-record` filters.
6. This slice does not implement new relation creation paths, relation removal,
   relation updates, `why` projection changes, or additional relation families.

## Consequences

- Record causal edges are now inspectable through the semantic API and CLI.
- Later `why` or context projection slices can reuse the snapshot surface
  without reading storage tables directly.

## Implementation Findings

- No schema changes were required; replayed WorkState relation membership is
  sufficient to determine the current Record relation set.
