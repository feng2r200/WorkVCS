# ADR-0147: Phase 4AR Bundle KnowledgeExposure Closure

- **Status:** Accepted
- **Date:** 2026-08-30

## Context

Phase 4Q through Phase 4Z made deterministic Bundle manifest/payload export,
validation, preflight, and import-attempt journaling available for canonical
WorkState history. Phase 4AD through Phase 4AQ then added Store-local
KnowledgeSpace, KnowledgeExposure, explicit adoption, provenance relations, and
context exposure provenance.

ADR-0004 requires portable Bundles not to omit data needed to validate
canonical history and Knowledge provenance. A Bundle containing adopted
Knowledge and its `derived_from` relation therefore also needs the referenced
KnowledgeSpace/KnowledgeExposure closure and the exact source KnowledgeVersion
payload named by the Exposure.

## Decision

1. Phase 4AR extends `BundleExportManifest` with KnowledgeSpace,
   KnowledgeExposure, KnowledgeExposure local-source, transition, and
   source-status closure metadata.
2. The closure is derived from KnowledgeExposure object ids referenced by
   relation endpoints in the exported RelationVersion closure.
3. KnowledgeExposure local-source metadata records the exact source Workspace,
   Knowledge entity, Knowledge entity version, domain-separated source
   Knowledge state digest, raw source `state_json` digest, and byte size.
4. KnowledgeExposure transition and source-status detail JSON are recorded by
   raw content digest and byte size after fixed-point canonical JSON
   validation.
5. Payload export adds references for:
   `knowledge_exposure_source_knowledge_state`,
   `knowledge_exposure_transition_detail`, and
   `knowledge_exposure_source_status_detail`.
6. CLI `bundle export` reports the new closure counts.
7. This slice does not ingest Bundle rows, activate imported refs, implement
   external Exposure sources, define final archive/container encoding, or add
   live cross-Store federation.

## Consequences

- Bundle export now carries the Store-local Exposure provenance needed to audit
  adopted Knowledge that points back to a KnowledgeExposure.
- Existing manifest and payload validation continue to rebuild expected export
  state from the local Store before comparing bytes.
- Import remains preflight/journal-only until a later slice defines activation
  semantics.

## Implementation Findings

- None.
