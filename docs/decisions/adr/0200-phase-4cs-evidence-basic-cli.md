# ADR-0200: Phase 4CS Evidence Basic CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

Evidence is an immutable ObjectIdentity-backed provenance object. Core Engine
APIs already support metadata-only Evidence creation/readback and Verification
creation with a deterministic Evidence set, but the CLI did not expose Evidence
objects or allow Verification records to attach Evidence.

## Decision

1. Add `evidence create` for metadata-only Evidence.
2. Add `evidence show`.
3. Add repeatable `--evidence` to `verification record`, delegating duplicate
   detection and closure validation to existing core Verification creation.
4. Render Evidence metadata, optional source session id, and content metadata
   fields.
5. Keep Evidence out of WorkState and preserve the existing immutable
   `evidenced_by` relation semantics.

## Non-Goals

- This slice does not add Evidence content input flags.
- This slice does not add Evidence listing.
- This slice does not alter Verification target or applicability semantics.
- This slice does not add Evidence retention or storage-location behavior.

## Consequences

- CLI users can create and inspect metadata-only Evidence.
- CLI Verification records can now include defining Evidence and read it back
  through `verification show/list`.

## Implementation Findings

- None.
