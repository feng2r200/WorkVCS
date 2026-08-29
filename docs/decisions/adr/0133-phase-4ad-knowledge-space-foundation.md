# ADR-0133: Phase 4AD Knowledge Space Foundation

- **Status:** Accepted for implementation
- **Date:** 2026-08-30

## Context

Knowledge federation is Store-local in V1. The confirmed model treats
KnowledgeSpace and KnowledgeExposure as ObjectIdentity-backed federation
objects outside Workspace Work State, with no independent Knowledge Space DAG
and no live cross-Store service. The v0.1 schema already contains
`knowledge_space` with exact case-sensitive unique names.

## Decision

1. Add typed `KnowledgeSpaceId`.
2. Add Engine-level APIs to create, show, and list KnowledgeSpace records.
3. Creating a KnowledgeSpace writes `object_identity` and `knowledge_space` in
   one transaction with object kind `knowledge_space`.
4. KnowledgeSpace names are exact, case-sensitive UTF-8 identity; they must be
   non-empty, trimmed, and free of NUL/ASCII control characters.
5. Duplicate names are rejected before insert and remain protected by the schema
   unique constraint.
6. CLI adds `workvcs store knowledge-space-create`,
   `workvcs store knowledge-space-show`, and
   `workvcs store knowledge-space-list`.
7. This slice does not create KnowledgeExposure, publish Knowledge, adopt
   external Knowledge, implement source-status projection, add a KnowledgeSpace
   DAG, or perform cross-Store network federation.

## Consequences

The Store-local federation container is now available through the Engine facade
and thin CLI. Later slices can attach immutable Exposure bindings and transition
history to this foundation without changing Workspace WorkState semantics.

## Implementation Finding

No new contract ambiguity was found. The existing schema and confirmed
documents align on KnowledgeSpace as a Store-local, ObjectIdentity-backed
federation object outside Workspace branch state.
