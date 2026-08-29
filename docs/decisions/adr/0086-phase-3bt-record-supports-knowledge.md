# ADR-0086: Phase 3BT Record Supports Knowledge

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed typed relationship vocabulary.

## Context

Knowledge is now a first-class Workspace entity and `why` can resolve it as a
typed subject. The confirmed relation vocabulary allows `supports` from a
finding or claim to target cognition, and Knowledge may be supported,
contradicted, validated, invalidated, derived from, or superseded.

The existing Record relation API is intentionally Record-to-Record. Directly
turning it into a generic endpoint API would widen the public contract and
disturb current CLI/list semantics.

## Decision

1. Phase 3BT adds a dedicated `RecordKnowledgeRelationCreateOptions::supports`
   semantic entrypoint.
2. The relation stores canonical relation type `supports` with source Record
   and target Knowledge object ids in the existing relation table.
3. Creation requires a Finding Record source, an active Knowledge target, both
   endpoints in the current Workspace, and branch-head compare-and-swap.
4. The existing Record-only relation projection skips non-Record endpoints.
5. `why` reads Record-to-Knowledge support relations and renders them as
   `RecordSupports` edges with `Record` source and `Knowledge` target.
6. The CLI exposes `record link-supports-knowledge`.
7. This slice does not implement `validates`/`invalidates` Knowledge,
   Knowledge contradiction, Knowledge supersession, Knowledge adoption,
   KnowledgeExposure, generic relation endpoints, or Record-to-Knowledge list
   commands.

## Consequences

- A Finding can now explicitly support reusable Knowledge and that support is
  visible from `why <knowledge>`.
- Record relation listing remains limited to Record-to-Record edges until a
  broader relation-query surface is confirmed and implemented.

## Implementation Findings

- The existing relation table, relation version, WorkState relation membership,
  replay, and branch CAS path were sufficient; no schema changes were needed.
