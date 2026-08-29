# ADR-0085: Phase 3BS Why Knowledge Subject

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed Knowledge/context/why boundaries.

## Context

Phase 3BP through 3BR introduced Knowledge creation, invalidation, query, and
context summary projection. The `why` query already resolves entity subjects
through replayed `WorkState`, but its public entity-kind vocabulary and CLI
rendering did not identify Knowledge.

## Decision

1. Phase 3BS extends `WhyEntityKind` with `Knowledge`.
2. Resolved entity subjects now include the entity kind loaded from
   `entity.entity_kind` and validated against the current workspace.
3. `why` recognizes current Knowledge entities as supported semantic subjects.
4. The CLI `why` output includes `subject_entity_kind=knowledge` for Knowledge
   subjects.
5. This slice does not add Knowledge relation traversal, Knowledge ranking,
   Knowledge adoption state, profile-budget behavior, or path-sensitive
   explanation rules.

## Consequences

- Agents can ask `why` about a Knowledge entity and receive a typed subject
  result instead of only an entity version id.
- Relation graph expansion remains unchanged; Knowledge edges require later
  explicit relation semantics.

## Implementation Findings

- The replayed `WorkState` is sufficient to identify the current entity version,
  but not the entity kind. The kind must be loaded from the authoritative
  `entity` table and checked against the resolved workspace.
