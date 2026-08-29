# ADR-0081: Phase 3BO Record Statement Filter

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  implemented Record list projection.

## Context

Record list projection can filter by kind and status, and the CLI can query
records at a commit or branch head. A local Agent still needs a simple way to
find current Records by known statement text without adding a full indexing
subsystem.

## Decision

1. Extend `RecordListOptions` with an optional `statement_contains` filter.
2. The filter is an exact, case-sensitive substring match against the replayed
   Record statement.
3. Empty or whitespace-only filters are rejected.
4. The filter composes with existing kind and status filters.
5. The CLI exposes the filter as `record list --statement-contains <text>`.
6. This slice does not implement full-text search, case folding, stemming,
   ranking, pagination, indexes, or context-budget selection.

## Consequences

- Local Agent workflows can quickly narrow current Record facts by known text.
- The implementation remains a pure read projection over replayed WorkState and
  does not require schema changes.

## Implementation Findings

- Existing Record statement validation allows line breaks, so the filter is
  applied to the stored string value instead of parsing rendered CLI output.
