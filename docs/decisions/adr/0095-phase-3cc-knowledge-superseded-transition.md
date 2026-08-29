# ADR-0095: Phase 3CC Knowledge Superseded Transition

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed Knowledge lifecycle.

## Context

The confirmed Knowledge lifecycle includes `active`, `superseded`, and
`invalidated`. Phase 3BQ implemented explicit invalidation and left
supersession for a later slice. The state parser and query filters already
recognize `superseded`; the missing piece is the explicit transition.

## Decision

1. Phase 3CC adds `KnowledgeTransitionOptions::supersede`.
2. The allowed lifecycle transition set now includes `active -> superseded`.
3. Supersession preserves the Knowledge statement, scope, and provenance while
   creating a new Knowledge entity version with status `superseded`.
4. The CLI exposes `knowledge supersede`.
5. Superseded Knowledge remains queryable by commit or `knowledge list --status
   superseded` and remains excluded from default active context.
6. This slice does not create a Knowledge-to-Knowledge `supersedes` relation and
   does not implement automatic replacement selection.

## Consequences

- Tools can mark obsolete Knowledge explicitly without deleting it.
- A later Knowledge relation slice can add machine-readable replacement lineage
  on top of this lifecycle primitive.

## Implementation Findings

- The existing Knowledge state shape already supported `superseded`; no schema
  or state-shape migration was needed.
