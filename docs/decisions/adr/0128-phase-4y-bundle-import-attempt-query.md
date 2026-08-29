# ADR-0128: Phase 4Y Bundle Import Attempt Query

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  Phase 4X import attempt journal.

## Context

Phase 4X records valid Bundle directory preflight outcomes into immutable
`import_attempt` and `import_attempt_outcome` rows. Without an Engine-level
read path, that infrastructure provenance is only visible through direct SQL or
the immediate CLI response that created it.

WorkVCS needs local tool support to inspect and list recorded Bundle import
attempts before any later ingestion, Store lineage, or ref activation work.

## Decision

1. Phase 4Y introduces `BundleImportAttemptSnapshot`,
   `BundleImportAttemptOutcomeSnapshot`, `BundleImportAttemptListOptions`, and
   `Engine::bundle_import_attempt` / `Engine::bundle_import_attempts`.
2. Import attempt show reads one immutable `import_attempt` row and its optional
   `import_attempt_outcome`.
3. Import attempt list returns recent attempts ordered by `started_at_us`
   descending, then import id descending, with a positive configurable limit.
4. Outcome `detail_json` is parsed and fixed-point validated as canonical
   semantic JSON before being returned.
5. CLI adds `workvcs bundle import-show --import` and
   `workvcs bundle import-list [--limit]`.
6. This slice does not ingest canonical rows, write Store lineage, activate
   refs, define a final Bundle container, or perform remote/federated
   transport.

## Consequences

- Recorded Bundle import attempts are inspectable through the same Engine facade
  used by the rest of the tool.
- Later import implementation can build on a visible immutable journal instead
  of adding direct SQL inspection paths.
- Invalid directories remain non-recorded by Phase 4X policy and therefore do
  not appear in this query surface.

## Implementation Findings

- No new contract ambiguity was found. Querying the existing immutable import
  journal does not change import activation semantics.
