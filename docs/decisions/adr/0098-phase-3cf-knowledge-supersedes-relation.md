# ADR-0098: Phase 3CF Knowledge Supersedes Relation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed Knowledge lifecycle and relationship vocabulary.

## Context

Phase 3CC added the explicit `active -> superseded` Knowledge lifecycle
transition. The confirmed relationship vocabulary also includes `supersedes`,
with canonical direction `replacement -> prior`. Without a machine-readable
Knowledge-to-Knowledge relation, tools can mark Knowledge obsolete but cannot
explain which active Knowledge replaced it.

## Decision

1. Phase 3CF adds `KnowledgeRelationCreateOptions::supersedes`.
2. The relation direction is replacement Knowledge source to prior Knowledge
   target.
3. Creation requires the replacement Knowledge to be `active` and the prior
   Knowledge to be `superseded` at the expected head commit.
4. The relation is stored as a normal relation with type `supersedes`, empty
   relation state, and `knowledge.relation.create` operation/event names.
5. `why` includes Knowledge supersession edges as `knowledge_supersedes`.
6. The CLI exposes creation as `knowledge link-supersedes`.
7. This slice does not implement Knowledge relation list/show/remove/restore,
   automatic replacement selection, or an atomic create-and-supersede command.

## Consequences

- Tools can explain obsolete Knowledge by following the replacement lineage.
- The lifecycle primitive and causal relation remain separate operations.
- Future slices can add read-side list/show/remove/restore commands without
  changing the storage shape introduced here.

## Implementation Findings

- Existing relation table writes were reusable, but the helper currently lives
  in the Record history module. This slice parameterized the operation/event
  names and kept the helper in place to avoid a broad relation-module refactor.
