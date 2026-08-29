# ADR-0066: Phase 3AZ Record Validates Relation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed epistemic relation vocabulary.

## Context

The Record semantic kernel already supports Assumption validation and explicit
`invalidates` causal relations from Findings to invalidated Assumptions. The
confirmed vocabulary also defines `validates` in the same canonical direction:
finding/evidence -> assumption/knowledge. Implementing the Record/Finding
subset gives the CLI a symmetric way to preserve positive causal support for a
validated Assumption.

## Decision

1. Phase 3AZ adds `RecordRelationType::Validates`.
2. `RecordRelationCreateOptions::validates` writes a canonical `validates`
   relation from a Finding Record to an Assumption Record.
3. The target Assumption must already be `validated` at the expected head
   commit, so relation state and Record lifecycle state do not diverge.
4. Relation creation reuses the existing Record relation commit path,
   WorkState relation membership, event, and branch-head compare-and-swap.
5. `record relation-list` accepts `--type validates`.
6. Core `why` exposes `validates` as `WhyRelationKind::RecordValidates`; the
   CLI renders it as `record_validates`.
7. The CLI exposes `record link-validates ...`.
8. This slice does not implement Evidence endpoints, `supports`,
   `contradicts`, `derived_from`, `supersedes`, custom `related_to`, relation
   removal, or automatic relation creation during Assumption validation.

## Consequences

- A local Agent can preserve why a Finding validated an Assumption.
- Positive and negative Assumption lifecycle outcomes now have symmetric
  explicit Record relation commands.

## Implementation Findings

- The Phase 3AV relation creation path was generic enough for `validates`; no
  schema or replay changes were required.
