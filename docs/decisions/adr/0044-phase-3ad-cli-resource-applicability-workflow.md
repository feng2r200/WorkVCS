# ADR-0044: Phase 3AD CLI Resource Applicability Workflow

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  repository-confirmed Resource / Verification applicability boundary.

## Context

Phase 3AC made the basic Task, Acceptance Criterion, Verification, and Task
transition workflow available from the CLI. Phase 3AA and Phase 3AB added Core
support for Resource applicability cache stamps and branch-aware effective
status, but Resource-backed Verification still required direct Engine calls.

## Decision

1. Phase 3AD adds thin CLI commands for Resource creation and Resource
   Observation recording.
2. `verification record` can attach one explicit Resource Basis using typed
   IDs, adapter/scope versions, a canonical object scope payload, baseline
   fingerprint, and optional baseline observation ID.
3. `verification cache-record` records one Resource Stamp for a Verification
   and lets Core compute the final applicability and reason code.
4. CLI fingerprint input accepts either an explicit lowercase BLAKE3 hex digest
   or raw CLI text content that Core hashes as a ContentObject digest.
5. JSON inputs remain canonical object values parsed by the core canonical JSON
   boundary.
6. This is still a practical V0.1 Agent-facing shell, not a final protocol
   commitment. Multi-resource Basis/Stamp batches, Evidence attachment,
   Resource binding, adapter execution, cache refresh automation, runtime
   session/claim commands, merge/restore/federation, and remote operations stay
   outside this slice.

## Consequences

- A user or Agent can now run a Resource-backed Verification workflow through
  the CLI and make branch-aware AC status become `verified` through an
  applicability cache record.
- CLI remains a thin layer over Engine APIs and does not compute semantic
  applicability itself.

## Implementation Findings

- The existing one-row Engine API can support the first CLI workflow without a
  schema change. Batch Resource Basis/Stamp UX should be a later usability
  slice, not part of this foundation.
