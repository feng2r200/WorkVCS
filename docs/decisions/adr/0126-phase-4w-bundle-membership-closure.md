# ADR-0126: Phase 4W Bundle Membership Closure

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4S-4V deterministic Bundle manifest/directory slices.

## Context

The Bundle manifest already exports the target commit closure, the current
WorkState mapping, current object-version refs, deterministic payload files, and
read-only import preflight. For historical self-containedness, a Bundle also
needs the entity and relation membership transitions inside the commit closure,
including the before/after object versions touched by those transitions.

Without those refs, a Bundle rooted at a later commit can describe the current
WorkState but omit historical object versions that are required to replay or
audit membership changes in the closure.

## Decision

1. Phase 4W adds entity and relation membership change refs to the Bundle
   manifest.
2. Entity-version closure is expanded from current WorkState refs to current
   WorkState refs plus all before/after entity versions referenced by entity
   membership changes in the commit closure.
3. Relation-version closure is expanded from current WorkState refs to current
   WorkState refs plus all before/after relation versions referenced by relation
   membership changes in the commit closure.
4. Membership change refs include changeset id, operation id, ordinal, subject
   id, before/after version ids, and canonical raw-byte digest/size for
   `field_delta_json`.
5. Payload directory export includes canonical `field_delta_json` payload refs
   under `entity_membership_field_delta` and
   `relation_membership_field_delta`.
6. CLI `bundle export` reports entity and relation membership change counts.
7. This slice does not ingest bundle data, write import attempts, define a final
   Bundle container, activate imported refs, or perform remote/federated
   transport.

## Consequences

- Bundle directories now carry the membership transition metadata needed for a
  later replay/audit-oriented import path.
- A Bundle rooted at a later entity transition includes both the current version
  and the previous version touched by the transition.
- The deterministic directory artifact remains the only exported Bundle shape
  until the final container/profile is explicitly accepted.

## Implementation Findings

- No new contract ambiguity was found. The slice only expands deterministic
  local Bundle manifest and payload closure semantics.
