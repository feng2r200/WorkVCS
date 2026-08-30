# ADR-0197: Phase 4CP Verification Applicability Cache Show CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The Engine already records and validates branch-sensitive Verification
Applicability Cache rows. CLI tooling could write cache records, but could not
read back the current cache row for a branch and Verification without relying on
Acceptance Criterion effective status as an indirect signal.

## Decision

1. Add `verification cache-show`.
2. Resolve only by branch id and Verification entity id, matching the existing
   Engine cache lookup API.
3. Return `cache_found=false` when no cache row exists.
4. When present, render the stored evaluated commit, applicability, reason code,
   detail JSON, and Resource Stamp fields.
5. Keep `verification cache-record` output unchanged.

## Non-Goals

- This slice does not add cache listing.
- This slice does not recompute applicability in the CLI.
- This slice does not change Resource Basis or cache validation semantics.
- This slice does not materialize a new projection table.

## Consequences

- CLI users can directly inspect the branch-sensitive cache used by effective
  Acceptance Criterion status.
- Resource-backed verification workflows can verify cache write/read behavior
  without querying SQLite directly.

## Implementation Findings

- None.
