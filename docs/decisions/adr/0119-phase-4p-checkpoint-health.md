# ADR-0119: Phase 4P Checkpoint Health Reporting

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal and the
  checkpoint sequence after ADR-0118.

## Context

Checkpoint creation, validation, listing, and latest-usable selection give tools
basic checkpoint operations. The existing `doctor` command already reports
integrity coverage for Branches and WorkStateCommits. Checkpoints need matching
health visibility without making invalid checkpoints equivalent to store
corruption.

## Decision

1. Phase 4P extends `IntegrityReport` with `checked_checkpoints` and
   `invalid_checkpoints`.
2. Integrity validation checks that each Checkpoint has a status row and that
   status text is stored safely.
3. A Checkpoint whose status is `invalid` is counted, not treated as an
   integrity failure.
4. CLI `doctor` reports checkpoint counts alongside existing Branch and Commit
   counts.
5. This slice does not revalidate checkpoints, mutate checkpoint status, select
   checkpoints, or change replay/projection behavior.

## Consequences

- Tooling can see checkpoint inventory health from the standard doctor path.
- A bad checkpoint remains operationally visible without making the Store
  unreadable.

## Implementation Findings

- No new contract ambiguity was found.
