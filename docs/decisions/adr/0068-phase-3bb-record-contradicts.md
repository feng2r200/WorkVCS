# ADR-0068: Phase 3BB Record Contradicts Relation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed epistemic relation vocabulary.

## Context

The confirmed relationship vocabulary defines `contradicts` as an epistemic
edge from a claim to a target. The current Record implementation has Finding
Records and ordinary Decision Records, but not a separate Claim entity or full
Decision lifecycle. A small, tool-useful subset can still preserve explicit
negative evidence against an active Decision.

## Decision

1. Phase 3BB adds `RecordRelationType::Contradicts`.
2. `RecordRelationCreateOptions::contradicts` writes a canonical
   `contradicts` relation from a Finding Record to an active Decision Record.
3. The CLI exposes `record link-contradicts ...`.
4. `record relation-list` accepts `--type contradicts`.
5. Core `why` exposes `contradicts` as
   `WhyRelationKind::RecordContradicts`; the CLI renders it as
   `record_contradicts`.
6. This slice does not generalize `contradicts` to every cognition endpoint,
   does not add Claim endpoints, and does not implement Decision withdrawal,
   Decision supersession, Evidence endpoints, custom `related_to`, or relation
   removal.

## Consequences

- A local Agent can preserve a Finding that explicitly contradicts an active
  Decision Record without changing that Decision's lifecycle state.
- Broader conflict resolution remains separate from the relation edge itself.

## Implementation Findings

- The existing Record relation creation, listing, and why projection path
  supported this relation type without schema or replay changes.
