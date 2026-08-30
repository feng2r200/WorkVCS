# ADR-0218: Phase 4DK Acceptance Criterion Status Commit CLI

Status: Accepted

Date: 2026-08-30

## Context

The Engine already exposes two Acceptance Criterion effective-status queries:
one at a concrete Commit and one at a Branch head. The CLI exposed only the
Branch-head path through `workvcs ac status --branch`, while `ac show` and
`ac list` already accepted either `--branch` or `--commit`.

## Decision

1. Extend `workvcs ac status` with the same mutually exclusive `--branch` /
   `--commit` target selector.
2. Keep the existing Branch path on
   `Engine::acceptance_criterion_effective_status_for_branch`.
3. Route the new Commit path to
   `Engine::acceptance_criterion_effective_status`.
4. Keep the output unchanged as `status=...`.

## Non-Goals

- This slice does not change effective-status semantics.
- This slice does not alter Verification or applicability projection logic.
- This slice does not add JSON output mode.
- This slice does not change `ac show` or `ac list`.

## Consequences

- CLI users can inspect historical Acceptance Criterion status at an exact
  Commit.
- Branch-aware status remains available without behavior changes.
- The command surface is now consistent across AC show, list, and status.
