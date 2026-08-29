# ADR-0090: Phase 3BX Record Knowledge Relation List

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  existing Record-to-Knowledge relation slices.

## Context

Phases 3BT through 3BW added explicit Finding Record-to-Knowledge relation
creation for `supports`, `invalidates`, `validates`, and `contradicts`. `why`
can explain those edges, but the CLI and public Engine API do not yet expose a
direct list operation for local tool workflows.

## Decision

1. Phase 3BX adds `RecordKnowledgeRelationListOptions` and
   `RecordKnowledgeRelationListResult`.
2. The list operation replays the requested commit, loads current WorkState
   relation memberships, and returns only Record-to-Knowledge relation snapshots.
3. Filtering supports relation type, source Record entity, and target Knowledge
   entity.
4. Results sort deterministically by relation type, source Record, target
   Knowledge, then relation id.
5. Store and Engine expose the operation without exposing SQLite handles.
6. The CLI exposes `record knowledge-relation-list`.

## Consequences

- Tooling can enumerate Record-to-Knowledge edges directly instead of relying on
  `why` output.
- Record-to-Record `relation-list` remains unchanged and does not mix Knowledge
  targets into its result shape.

## Implementation Findings

- The existing Record-to-Knowledge projection loader was already sufficient for
  this query path; this slice only formalizes options, result shape, and CLI
  access.
