# ADR-0224: Phase 4DQ Evidence Source Session List CLI

Status: Accepted

Date: 2026-08-30

## Context

Evidence already stores optional `source_session_id` provenance and the CLI can
record it during `evidence create`. `evidence show` and `evidence list` render
the field, but operators could not ask which Evidence records came from a
specific Session without scanning all Store-level provenance output.

Evidence remains ObjectIdentity-backed immutable provenance and is not part of
WorkState.

## Decision

1. Extend `EvidenceListOptions` with an optional `source_session_id` filter.
2. Keep the existing optional `evidence_kind` filter and allow it to combine
   with the source Session filter.
3. Add `--source-session SESSION` to `workvcs evidence list`.
4. Parse `SESSION` as a typed `SessionId`.
5. Keep Evidence list rendering unchanged, because list entries already expose
   `source_session_id`.

## Non-Goals

- This slice does not change Evidence creation semantics.
- This slice does not infer a current Session implicitly.
- This slice does not attach Evidence to WorkState.
- This slice does not change Session lifecycle rules.

## Consequences

- CLI users can directly recover Evidence captured by one WorkVCS Session.
- Source-session provenance becomes queryable through the Engine facade instead
  of lower-level storage inspection.
- Verification and audit workflows can combine Evidence kind and source Session
  filters while preserving the existing provenance boundary.
