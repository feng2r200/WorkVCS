# ADR-0083: Phase 3BQ Knowledge Invalidation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Knowledge lifecycle boundary.

## Context

Phase 3BP introduced Workspace-local Knowledge statements as versioned Work
State and defined `active`, `invalidated`, and `superseded` as readable
Knowledge statuses. The confirmed domain states that Knowledge may be active,
superseded, or invalidated, and that natural-language similarity alone does
not change Knowledge state.

## Decision

1. Phase 3BQ adds only explicit Knowledge invalidation.
2. The allowed transition is `active -> invalidated`.
3. The operation requires the current branch head, current Knowledge
   EntityVersion, and a non-empty rationale.
4. Invalidation creates a normal WorkStateCommit through the semantic
   Knowledge API and preserves the Knowledge Entity id.
5. Historical commits still show the prior active Knowledge version.
6. Current branch-head list/show projections expose the invalidated status.
7. Repeated invalidation, empty rationale, stale expected version, and other
   status changes are rejected.
8. The CLI exposes `knowledge invalidate`.
9. This slice does not implement Knowledge supersession, Knowledge relation
   APIs, KnowledgeExposure, adoption, cross-Workspace sharing, reopen, or
   merge review.

## Consequences

- Agents can explicitly retire stale learned statements without deleting their
  history.
- Later Knowledge supersession can add causal relation semantics without
  changing this invalidation behavior.

## Implementation Findings

- No new storage primitive was required. Existing Entity transition replay
  already preserves Knowledge history and branch-sensitive current state.
