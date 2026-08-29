# ADR-0067: Phase 3BA Record Supports Relation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  confirmed epistemic relation vocabulary.

## Context

The confirmed relationship vocabulary defines `supports` as an epistemic edge
from a finding/claim to target cognition. The current implementation has
Finding and ordinary Decision Records, while a separate Claim semantic entity
does not exist in this slice. Documentation also states that a Finding may
support a Decision.

## Decision

1. Phase 3BA adds `RecordRelationType::Supports`.
2. `RecordRelationCreateOptions::supports` writes a canonical `supports`
   relation from a Finding Record to an active Decision Record.
3. The CLI exposes `record link-supports ...`.
4. `record relation-list` accepts `--type supports`.
5. Core `why` exposes `supports` as `WhyRelationKind::RecordSupports`; the CLI
   renders it as `record_supports`.
6. This slice does not generalize `supports` to every cognition endpoint, does
   not add Claim endpoints, and does not implement `contradicts`, `derived_from`,
   `supersedes`, custom `related_to`, relation updates, or relation removal.

## Consequences

- A local Agent can preserve the causal Finding that supports an ordinary
  Decision Record.
- Broader epistemic support semantics remain separate small slices with their
  own endpoint rules.

## Implementation Findings

- The existing Record relation creation and projection path supported this
  relation type without schema or replay changes.
