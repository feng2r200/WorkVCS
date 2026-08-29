# ADR-0154: Phase 4AY Bundle Relation Apply Foundation

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

ADR-0153 restored the typed identity rows needed for AcceptanceCriterion and
VerificationRequirement during same-Store Bundle apply. The next closure gap is
Relation state: exported Bundle manifests already contain RelationVersion refs,
Relation membership changes, and their payloads, but same-Store apply still
rejected all relation-bearing Bundles.

## Decision

1. Same-Store Bundle apply now supports RelationVersion refs and
   RelationMembershipChange refs contained in the Bundle manifest.
2. Apply reconstructs relation object identity, relation identity fields, and
   relation_version rows before importing Commit closure rows.
3. Relation metadata payloads are validated through the existing
   `relation_version_metadata` payload-index role and checked against both raw
   content digest/size and RelationVersion semantic digest.
4. Relation membership ChangeOperation rows use `subject_family = "relation"`
   and reuse the existing `change_operation_payload` and
   `relation_membership_field_delta` payload-index roles.
5. Existing identical relation and relation_version rows are no-ops. Conflicting
   relation identity fields, relation_version content, or unique relation
   identity collisions reject the import as immutable import invalid.
6. The support gate remains closed for Knowledge federation rows, Checkpoint
   candidates, external Store import, missing Branch creation, and typed
   VerificationResult/Evidence/Resource identity reconstruction.

## Consequences

- Same-Store Bundle apply can now fast-forward a stale Store through task
  scheduling relation commits while preserving replayable WorkState relation
  mappings.
- Verification/Evidence/Resource Bundle apply still requires later typed
  identity and object-family import slices.

## Implementation Findings

- Existing Bundle payload exports already carried the relation metadata and
  relation membership field-delta payloads needed by apply; no manifest version
  bump or schema change was required for this relation-only import slice.
