# ADR-0084: Phase 3BR Context Knowledge Summary

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed V1 context and Knowledge boundary.

## Context

Phase 3BP and 3BQ introduced Workspace-local Knowledge creation, query, and
explicit invalidation. The confirmed V1 context boundary says context is a
deterministic derived projection and includes scoped Knowledge. The current
context overview already exposes runnable tasks, Records, and Record
relations, but not Knowledge.

## Decision

1. Phase 3BR adds a Knowledge summary to `ContextOverview`.
2. The resolver reads Knowledge at the active Session's current Branch head.
3. Only `active` Knowledge enters this projection.
4. Invalidated Knowledge remains queryable through `knowledge show/list` and
   history, but is excluded from current context by default.
5. Context resolution remains read-only and validates that the Knowledge
   projection uses the same workspace and commit anchor as the active Branch
   head.
6. The `context` CLI output includes Knowledge count and
   `context_knowledge.*` summary rows.
7. The `next` CLI output includes only `context_knowledge` count.
8. This slice does not implement profile budgets, omission summaries,
   path-sensitive Knowledge ranking, KnowledgeExposure, adoption,
   Knowledge relation traversal, or causal exceptions for inactive Knowledge.

## Consequences

- Agents restoring a session can see current active reusable Knowledge without
  issuing a separate Knowledge list command.
- The implementation stays a deterministic read projection over replayed Work
  State.

## Implementation Findings

- The existing `KnowledgeListOptions` status filter is sufficient for the
  default active-only context rule; no new storage or replay mechanism was
  required.
