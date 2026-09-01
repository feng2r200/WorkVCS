# ADR-0450: Phase 4MI Shared Claim Dogfood

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger still marked shared-Claim collaboration as
smoke-proven but not real-project dogfooded. Previous dogfood slices proved
focused Handoff, blocked Claim recovery, cooperative Claim transfer, stale-gated
takeover, and Resource-backed verification against a real local project, but
the shared Claim path had not been exercised outside repository smoke.

## Decision

Accept the current V1 shared-Claim evidence boundary:

- two active Sessions can claim the same real external-project Task in
  `shared` mode;
- `context` projects the second Session focus, runnable candidate, and shared
  Claim set;
- `claim guard --action structural-task` reports `allowed=false`,
  `reason=non_unique_shared_claim_set`, and two active shared Claims when more
  than one shared claimant is active;
- protected terminal Task transition is also blocked while the shared set is
  non-unique;
- after one shared Claim is released, the remaining claimant is a
  `unique_shared_claimant` and can close the Task;
- Resource-backed verification can read the real external file through
  `verify --scope-path --resource-content-from-scope-path`; and
- the target file status, hash, and stat remain unchanged.

This preserves the existing guard semantics: non-unique shared Claims are a
coordination state, not permission for simultaneous protected Task mutation.

## Non-Goals

- No change to Claim guard allow/deny semantics.
- No write-mode external-project mutation.
- No distributed collaboration or remote synchronization.
- No automatic ownership arbitration between shared claimants.
- No CLI spelling redesign in this slice.
- No change to version-scoped AC staleness after Task closeout.

## Evidence

- Focused shared-Claim dogfood:
  `/tmp/workvcs-4mi-shared-claim-dogfood-rerun9-20260901T082157Z`.
- Supplemental current doctor output:
  `doctor-current.out` in the same log directory.
- Target-file unchanged checks:
  `target-status-before.txt`, `target-status-after.txt`,
  `target-hash-before.txt`, `target-hash-after.txt`,
  `target-stat-before.txt`, and `target-stat-after.txt` in the same log
  directory.
- Documentation validation:
  `/tmp/workvcs-4mi-doc-validation-clean-20260901T082728Z` and
  `/tmp/workvcs-4mi-fmt-validation-clean-20260901T082728Z`.
- Independent review found no blocker/high/medium issues.

## Consequences

The Claim row can move from shared smoke-only evidence to real read-only
external-project dogfood evidence. Release maturity still needs broader
write-mode or read/write multi-operator proof, plus guidance for the current
version-scoped AC behavior: an AC can be `verified` at the verification commit
and become `stale` after the Task closeout commit advances the Task version.
