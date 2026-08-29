# ADR-0078: Phase 3BL Context Record Relations

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  implemented Context Resolver and Record relation slices.

## Context

Phase 3BK made current semantic Records visible through `context`. Record
relations are now equally important to local Agent work because Findings,
Assumptions, Decisions, Attempts, Risks, Questions, and Handoffs derive much of
their meaning from current causal or semantic edges.

## Decision

1. Extend `ContextOverview` with the current branch-head
   `RecordRelationListResult`.
2. The resolver reads relations through the existing semantic relation
   projection: `record_relations_at(branch.head_commit_id)`.
3. The operation remains read-only and writes no WorkState or Runtime rows.
4. The CLI `context` output includes a `record_relations` count and
   line-oriented `context_record_relation.*` summaries containing relation id,
   version, type, optional label, endpoints, and digest.
5. The CLI `next` output adds only `context_record_relations` as a summary
   count.
6. This slice does not implement relation ranking, causal-neighborhood
   expansion, graph path scoring, budget trimming, materialized context
   packets, or changes to `next` selection.

## Consequences

- A local Agent can inspect the current Record facts and the current Record
  relation graph from the same context command.
- Record relation removal automatically affects context because context reads
  from the replayed current relation projection.

## Implementation Findings

- No schema change was required. The existing `record_relations_at` projection
  already enforces commit-relative current membership and endpoint validation.
