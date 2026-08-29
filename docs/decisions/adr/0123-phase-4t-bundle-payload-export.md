# ADR-0123: Phase 4T Bundle Payload Export Directory

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4Q/4R/4S Bundle manifest foundation.

## Context

Phase 4Q introduced deterministic Bundle export manifests. Phase 4R added
canonical manifest validation. Phase 4S added current EntityVersion and
RelationVersion closure metadata, including raw canonical JSON digests.

The Bundle container/archive format remains Open, but ADR-0004 already requires
Bundle interchange to include manifest, canonical logical history, required
immutable objects, and integrity metadata. The next narrow step is to export the
canonical JSON payload bytes named by the current manifest and commit closure in
a deterministic local directory layout.

## Decision

1. Phase 4T introduces `BundlePayloadExportOptions` and
   `Engine::export_bundle_payloads`.
2. Core export returns exact canonical `manifest.json` bytes, a canonical
   `payload-index.json`, deduplicated content-addressed JSON payload files, and
   role-based payload references.
3. Payload files are written under `payloads/<raw-content-digest>.json`.
4. Exported payload roles are:
   - `changeset_operation_payload`
   - `changeset_rationale`
   - `change_operation_payload`
   - `entity_version_state`
   - `relation_version_metadata`
5. Every payload is reloaded from SQLite, validated as fixed-point WorkVCS
   canonical semantic JSON, and checked against its raw content digest before
   export.
6. CLI adds `workvcs bundle export-dir --commit --output-dir`. It writes files
   with create-new semantics so existing export files are not overwritten.
7. This slice does not define the final Bundle archive/container, persist bundle
   bytes, write `import_attempt`, write `store_lineage`, implement import, or
   perform remote/federated transport.

## Consequences

- A local export now contains the canonical JSON bytes needed by later Bundle
  packaging and import preflight checks.
- Duplicate JSON payloads are stored once by raw content digest while
  `payload-index.json` preserves every role-specific reference.
- The result is intentionally a deterministic local directory artifact, not the
  final portable Bundle container contract.

## Implementation Findings

- No new contract ambiguity was found. The final Bundle container/profile is
  still Open, so this slice exports a deterministic directory artifact rather
  than freezing archive encoding.
