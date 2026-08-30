# ADR-0202: Phase 4CU Evidence List CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 4CS exposed metadata-only Evidence creation/readback through the CLI.
Phase 4CT added CLI input for Evidence content metadata. Users can now create
and inspect individual Evidence objects, but the tool surface cannot enumerate
captured Evidence without using lower-level storage inspection.

Evidence is ObjectIdentity-backed provenance and remains outside WorkState.
Listing Evidence is therefore a Store-level provenance query, not a semantic
state projection at a commit.

## Decision

1. Add `EvidenceListOptions` and `EvidenceListResult` to the core Engine
   facade.
2. Add `Engine::evidences` as a read-only API backed by the Store boundary.
3. List Evidence through `object_identity` + `evidence`, requiring the stored
   object kind to be `evidence`.
4. Return full `EvidenceSnapshot` values so callers get the same canonical
   metadata and content metadata shape as `evidence show`.
5. Sort results by `captured_at_us` and `evidence_id` for stable output.
6. Add an optional `evidence_kind` filter.
7. Add `workvcs evidence list STORE [--kind KIND]` to the CLI.
8. Render list output in the existing script-readable `key=value` format with
   compact per-Evidence fields and content counts. Detailed content fields
   remain available through `evidence show`.

## Non-Goals

- This slice does not add raw Evidence blob storage.
- This slice does not add Evidence retention policy or storage locations.
- This slice does not make Evidence part of WorkState.
- This slice does not add commit-scoped Evidence projections.

## Consequences

- CLI users can complete the basic Evidence create/show/list workflow.
- Verification workflows can discover reusable Evidence without bypassing the
  Engine facade.
- The Evidence list remains provenance-scoped and does not imply semantic
  membership in any commit state.
