# ADR-0121: Phase 4R Bundle Manifest Validation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4Q Bundle export manifest foundation.

## Context

Phase 4Q added a deterministic local export manifest for a target
WorkStateCommit. That manifest is useful only if tooling can also emit its
canonical bytes and validate candidate bytes against the current Store state.
The full Bundle container, persisted bundle bytes, import staging, and remote
transport profile remain deferred.

## Decision

1. Phase 4R adds `BundleManifestValidationOptions` and
   `Engine::validate_bundle_manifest`.
2. Validation takes a target Commit id and candidate manifest bytes.
3. Validation rebuilds the expected Phase 4Q export manifest for the target
   commit, computes the expected manifest digest, computes the candidate raw
   byte digest, and compares fixed-point canonical bytes.
4. Invalid JSON, non-canonical JSON bytes, and semantic mismatch are reported as
   `valid=false` validation results rather than Store corruption.
5. CLI adds `workvcs bundle export-json` for canonical manifest bytes and
   `workvcs bundle validate-manifest --manifest-file` for read-only validation.
6. This slice does not write `import_attempt`, write `store_lineage`, persist
   bundle payload bytes, create a bundle container, perform import, or perform
   remote/federated transport.

## Consequences

- Export tooling now has a closed local loop: produce canonical manifest bytes
  and validate those bytes against current replay-backed Store state.
- Later import staging can reuse the validation result without changing Store
  identity or transport semantics.

## Implementation Findings

- No new contract ambiguity was found. The validation result intentionally
  names `actual_manifest_digest` for candidate bytes and preserves the 4Q
  distinction that this is not a final Bundle container digest.
