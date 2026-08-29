# ADR-0062: Phase 3AV Record Invalidates Relation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed typed relation vocabulary.

## Context

Confirmed Record semantics say Records participate in the typed causal graph.
In particular, a Finding may invalidate an Assumption. The canonical relation
vocabulary stores this as `invalidates` in canonical direction
finding/evidence -> assumption/knowledge.

After Record creation/list/show and Attempt/Handoff support, the next useful
tool step is a small explicit relation command. Implementing every canonical
epistemic and evolution edge at once would widen the slice.

## Decision

1. Phase 3AV adds `RecordRelationType::Invalidates`.
2. `RecordRelationCreateOptions::invalidates` writes a canonical
   `invalidates` relation from a Finding Record to an Assumption Record.
3. The target Assumption must already be `invalidated` at the expected head
   commit, so relation state and Record lifecycle state do not diverge.
4. Relation creation is a normal WorkState commit with relation membership
   change, event, and branch-head compare-and-swap.
5. Replay accepts `record.relation.create` operation schema version 1.
6. The CLI exposes `record link-invalidates ...`.
7. This slice does not implement `supports`, `contradicts`, `validates`,
   `derived_from`, `supersedes`, custom `related_to`, relation listing,
   automatic relation creation during Assumption invalidation, or Handoff
   supplemental relations.

## Consequences

- A local Agent can preserve the explicit causal reason that a Finding
  invalidated an Assumption.
- Other canonical Record relation types remain separate tool-development
  slices.

## Implementation Findings

- No schema changes were required; the existing relation table and WorkState
  relation membership model were sufficient.
