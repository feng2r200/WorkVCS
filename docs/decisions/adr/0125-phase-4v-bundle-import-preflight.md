# ADR-0125: Phase 4V Bundle Import Preflight

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4T/4U deterministic Bundle directory artifact.

## Context

Bundle export directories can now be produced and validated against the current
Store export semantics. Before implementing ingestion, import attempts, lineage,
or ref activation, WorkVCS needs a read-only preflight that can inspect a Bundle
directory artifact and classify whether the current Store already contains the
incoming target commit or whether import would be required later.

## Decision

1. Phase 4V introduces `BundleImportPreflightOptions` and
   `Engine::preflight_bundle_import`.
2. Preflight validates the directory artifact from its own canonical
   `manifest.json`, `payload-index.json`, and payload files, without requiring
   the target commit to already exist in the current Store.
3. Preflight compares the source Store manifest profile with the current Store
   manifest and reports `format_compatible`.
4. Preflight compares the source Store id with the current Store id and reports
   `same_store` or `external_store`.
5. Preflight checks whether the target commit id already exists locally, and if
   present verifies that the local state digest matches the incoming manifest.
6. CLI adds `workvcs bundle preflight-dir --input-dir`.
7. This slice does not ingest rows, write `import_attempt`, write
   `store_lineage`, activate refs, define the final Bundle container, or perform
   remote/federated transport.

## Consequences

- A Bundle directory can now be checked before any later import implementation.
- Current Store self-exports are classified as `already_present`.
- External Store artifacts are accepted as valid artifacts when self-consistent,
  but reported as `external_store_import_not_implemented` until ingestion is
  implemented.

## Implementation Findings

- No new contract ambiguity was found. The preflight action vocabulary is
  intentionally local to this implementation slice and does not freeze final
  import outcome terminology.
