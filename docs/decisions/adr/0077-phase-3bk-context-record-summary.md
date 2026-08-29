# ADR-0077: Phase 3BK Context Record Summary

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  implemented Context Resolver and Record slices.

## Context

Earlier Phase 3 slices added a read-only `context` overview for active Sessions
and later added semantic Records and Record relations. A local Agent can now
persist Findings, Assumptions, Decisions, Questions, Risks, Attempts, Handoffs,
and their causal Record edges, but the `context` entry point still reports only
the Session anchor and runnable Task candidates.

## Decision

1. Extend `ContextOverview` with the current branch-head `RecordListResult`.
2. The resolver reads Records through the existing semantic Record projection:
   `records_at(branch.head_commit_id)`.
3. The operation remains read-only and writes no WorkState or Runtime rows.
4. The CLI `context` output includes a `records` count and line-oriented
   `context_record.*` summaries containing Record id, version, digest, kind,
   status, and statement JSON.
5. The CLI `next` output adds only `context_records` as a summary count.
6. This slice does not implement context ranking, causal-neighborhood expansion,
   token budgeting, materialized context packets, Record relation summaries in
   context, or changes to `next` selection.

## Consequences

- A local Agent can inspect active task candidates and current semantic Records
  through the same context command before choosing its next action.
- `next` remains a task-selection workflow while still reporting whether Record
  context exists.

## Implementation Findings

- No schema change was required. Existing `RecordListOptions` and replayed
  WorkState already provide the current Record projection needed by context.
