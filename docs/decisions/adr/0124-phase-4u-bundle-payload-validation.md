# ADR-0124: Phase 4U Bundle Payload Directory Validation

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4Q/4R/4S/4T Bundle export foundation.

## Context

Phase 4T added deterministic local Bundle payload export directories with
`manifest.json`, `payload-index.json`, and content-addressed canonical JSON
payload files. Before import or final container work, the exported directory
needs a read-only validation path that proves the local artifact still matches
the current Store export semantics for a target commit.

## Decision

1. Phase 4U introduces `BundlePayloadValidationOptions` and
   `Engine::validate_bundle_payloads`.
2. Core validation rebuilds the expected payload export for the target commit
   and compares:
   - `manifest.json` fixed-point canonical bytes and manifest digest.
   - `payload-index.json` fixed-point canonical bytes and expected index bytes.
   - payload file path set, raw content digests, sizes, and exact bytes.
3. Payload inputs must use `payloads/<lowercase-hex-digest>.json` relative
   paths.
4. CLI adds `workvcs bundle validate-dir --commit --input-dir`.
5. Invalid manifests, noncanonical indexes, missing payloads, duplicate paths,
   unexpected payload paths, and payload byte drift return `valid=false` with a
   problem string.
6. This slice does not write `import_attempt`, write `store_lineage`, persist
   bundle bytes, implement import, define the final Bundle container, or perform
   remote/federated transport.

## Consequences

- A local payload export directory can now be checked before any later import
  preflight or packaging step.
- Validation remains centralized in `workvcs-core`; the CLI only reads files and
  renders results.
- The directory artifact remains an intermediate deterministic export format,
  not the final Bundle container contract.

## Implementation Findings

- No new contract ambiguity was found. The final Bundle container/profile remains
  Open, so validation targets the deterministic directory artifact introduced by
  Phase 4T.
