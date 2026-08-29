# ADR-0073: Phase 3BG Record Derived From Relation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed canonical relationship vocabulary.

## Context

The confirmed relationship model defines `derived_from` as an Evolution
relation with canonical direction `new/result -> source`. It is used by
semantic explanations to preserve origin and causal traversal without parsing
natural language.

The current implemented semantic Record subset includes Findings, Assumptions,
Attempts, ordinary Decisions, Questions, Risks, and Handoffs. Broader generic
entity relation APIs are still outside this slice.

## Decision

1. Phase 3BG adds `RecordRelationType::DerivedFrom`.
2. `RecordRelationCreateOptions::derived_from` creates a Record-to-Record
   `derived_from` edge using canonical direction `result_record -> source_record`.
3. The operation requires distinct Record endpoints and non-empty rationale.
4. `record relation-list --type derived_from` and `why` expose the edge.
5. The CLI exposes `record link-derived-from ... --result-record ... --source-record ...`.
6. This slice does not implement generic non-Record `derived_from` endpoints,
   automatic causal anchors on every semantic operation, relation updates, or
   relation removal.

## Consequences

- A local Agent can explicitly preserve the source Record behind a later
  Record result.
- The relation can be traversed through `why` without promoting custom labels
  into core semantics.

## Implementation Findings

- The existing Record relation kernel was sufficient for this canonical
  relation; no schema or replay change was required.
