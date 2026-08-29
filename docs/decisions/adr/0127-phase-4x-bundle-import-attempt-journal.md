# ADR-0127: Phase 4X Bundle Import Attempt Journal

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4T-4W deterministic Bundle directory/preflight sequence.

## Context

WorkVCS can now export deterministic Bundle directories, validate those
directories, preflight them against a target Store, and include membership
closure metadata. The schema already contains immutable `import_attempt` and
`import_attempt_outcome` tables, and INV-078 requires ImportAttempt provenance
to remain auditable.

Before implementing ingestion or ref activation, the tool needs a durable way
to record that a Bundle directory was inspected, what source Store it came from,
which deterministic artifact digest was checked, and which preflight outcome
was reached.

## Decision

1. Phase 4X introduces `ImportId`, `BundleImportAttemptOptions`, and
   `Engine::record_bundle_import_attempt`.
2. Recording an import attempt first reuses Bundle directory validation and
   preflight semantics from Phase 4V.
3. Valid Bundle directories write one immutable `import_attempt` row and one
   immutable `import_attempt_outcome` row in a single transaction.
4. The recorded Bundle digest is the raw content digest of
   `payload-index.json`, because Phase 4T/4U define the deterministic directory
   artifact by `manifest.json`, `payload-index.json`, and payload files rather
   than a final container byte stream.
5. The outcome is the preflight action, including `already_present`,
   `same_store_import_not_implemented`, and
   `external_store_import_not_implemented`.
6. Invalid Bundle directories return `recorded = false` and do not write an
   `import_attempt`, because the schema requires a known `source_store_id`.
7. CLI adds `workvcs bundle import-dir --input-dir`.
8. This slice does not ingest canonical rows, write Store lineage, activate
   refs, define a final Bundle container, or perform remote/federated transport.

## Consequences

- Bundle import preflight decisions are now durable local infrastructure
  provenance instead of transient CLI output.
- Future ingestion slices can link later behavior to a stable ImportAttempt
  without changing Bundle export format.
- Failed structural validation remains non-mutating until the final container
  and invalid-attempt policy are explicitly accepted.

## Implementation Findings

- No new contract ambiguity was found. The slice records the current
  deterministic directory artifact digest and does not freeze final Bundle
  container identity.
