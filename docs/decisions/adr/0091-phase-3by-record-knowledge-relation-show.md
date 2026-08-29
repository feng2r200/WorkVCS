# ADR-0091: Phase 3BY Record Knowledge Relation Show

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and ADR-0090.

## Context

ADR-0090 exposed Record-to-Knowledge relation listing for local tool workflows.
Once a tool has a relation id from a list or `why` output, it also needs a
stable command to inspect that exact edge without parsing a broader relation
neighborhood.

## Decision

1. Phase 3BY adds `record_knowledge_relation_at` to the history, Store, and
   Engine boundaries.
2. The lookup resolves a single current Record-to-Knowledge relation at the
   requested commit by reusing the Record-to-Knowledge list projection.
3. The CLI exposes `record knowledge-relation-show`.
4. The output mirrors Record relation show, with `target_knowledge_entity_id`
   instead of `target_record_entity_id`.
5. This slice does not add Record-to-Knowledge relation removal, restore, or
   mutation behavior.

## Consequences

- Tools can round-trip from list or `why` relation ids to a precise edge view.
- The public API continues to expose behavior only through Engine facade
  methods.

## Implementation Findings

- Reusing the list projection keeps show semantics aligned with WorkState
  membership and avoids a separate SQL-only shortcut.
