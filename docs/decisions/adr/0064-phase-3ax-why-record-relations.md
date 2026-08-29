# ADR-0064: Phase 3AX Why Record Relations

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and
  ADR-0062/ADR-0063 Record relation scope.

## Context

The `why` projection already reports current structural, verification, and
evidence relation neighborhoods. After `invalidates` Record relations became
writable and listable, the semantic reason graph should expose those current
edges for Record subjects as well.

## Decision

1. Phase 3AX adds `WhyEntityKind::Record`.
2. Phase 3AX adds `WhyRelationKind::RecordInvalidates`.
3. `explain_why` reads current Record relations through
   `record_relations_at`, then adds matching `invalidates` edges for source or
   target Record subjects.
4. Finding Records see the edge as outgoing; invalidated Assumption Records see
   the same edge as incoming.
5. The existing deterministic edge ordering remains authoritative.
6. `WhyDeferredRelationFamily::Epistemic` remains present because only the
   `invalidates` Record relation family is implemented in this slice.
7. This slice does not add CLI `why`, additional Record relation families,
   relation updates/removal, or automatic relation creation.

## Consequences

- The core why query can now explain the causal link between a Finding and the
  Assumption it invalidates.
- Further why tool surfaces can reuse the existing relation-edge model without
  storage table access.

## Implementation Findings

- The `RecordRelationListOptions` surface from Phase 3AW was sufficient for why
  integration; no schema or replay changes were required.
