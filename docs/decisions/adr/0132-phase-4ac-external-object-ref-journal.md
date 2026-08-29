# ADR-0132: Phase 4AC External Object Reference Journal

- **Status:** Accepted for implementation
- **Date:** 2026-08-30

## Context

INV-077 and the federation/portability documents restrict ExternalObjectRef to
explicit provenance, lineage, and imported metadata. It must not become a
canonical Relation endpoint or a cross-Store Work Graph shortcut. The v0.1
schema already contains `external_object_ref` with object/version scope and
partial uniqueness.

## Decision

1. Add typed `ExternalRefId`, `ExternalObjectId`, and `ExternalVersionId`.
2. Add Engine-level APIs to record, show, and list ExternalObjectRef rows.
3. Recording rejects refs that point at the local Store ID.
4. Object-scope refs require `external_version_ref = NULL`; version-scope refs
   require a concrete external version ref.
5. Descriptor JSON is a WorkVCS fixed-point canonical JSON object and is
   revalidated on read.
6. Recording is idempotent for an existing matching external key and descriptor;
   a matching key with conflicting descriptor or kind is rejected.
7. CLI adds `workvcs store external-ref-record`,
   `workvcs store external-ref-show`, and `workvcs store external-ref-list`.
8. List supports filters by external Store, object kind, and reference scope.
9. This slice does not create KnowledgeExposure, adopt external Knowledge,
   rewrite canonical Relation endpoints, import canonical rows, resolve remote
   references, or perform network synchronization.

## Consequences

Portable unresolved provenance can now be registered and inspected through the
Engine facade and CLI while keeping ExternalObjectRef outside canonical
WorkState endpoints.

## Implementation Finding

No new contract ambiguity was found. The existing partial uniqueness rules map
cleanly to idempotent object/version scoped registration.
