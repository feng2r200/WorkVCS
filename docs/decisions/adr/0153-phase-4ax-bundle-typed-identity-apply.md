# ADR-0153: Phase 4AX Bundle Typed Identity Apply Foundation

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0152 made same-Store Bundle apply work for task entity-only Bundles and
recorded that generic EntityVersion rows were not enough to reconstruct typed
identity tables. The next narrow capability is to carry the typed identity rows
needed by the Phase 3C/3D semantic kernel.

## Decision

1. Bundle manifests now include:
   - `acceptance_criterion_identities`;
   - `verification_requirement_identities`.
2. Each identity ref carries only the existing schema identity fields:
   `entity_id`, `owner_entity_id`, and `local_key`.
3. Same-Store apply support is widened from task-only to:
   - `task`;
   - `acceptance_criterion`;
   - `verification_requirement`.
4. A Bundle is apply-supported only when every exported AcceptanceCriterion and
   VerificationRequirement EntityVersion has exactly one matching identity ref.
5. Apply imports missing EntityVersion rows first, then restores
   AcceptanceCriterion and VerificationRequirement identity rows, then imports
   Commit closure rows and advances Branch heads through the existing CAS path.
6. Existing identical typed identity rows are no-ops. Conflicting entity ids,
   owner ids, local keys, or owner/local-key uniqueness collisions reject the
   import as immutable import invalid.
7. Missing identity fields in older task-only manifests are treated as empty
   arrays for import parsing compatibility.

## Consequences

- Same-Store Bundle apply can now fast-forward a stale Store through AC/VR
  semantic commits and keep typed queries such as `acceptance_criterion_at` and
  `verification_requirement_at` usable after import.
- The Bundle apply path still does not reconstruct VerificationResult,
  Evidence, Resource, Relation, Knowledge, Checkpoint, cross-Store, or missing
  Branch refs.

## Implementation Findings

- The typed identity rows needed for AcceptanceCriterion and
  VerificationRequirement are schema-local and do not require a schema change.
  Carrying them in manifest arrays is sufficient for deterministic same-Store
  import of Phase 3C/3D commits.
