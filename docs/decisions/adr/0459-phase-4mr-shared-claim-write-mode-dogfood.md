# ADR-0459: Phase 4MR Shared Claim Write-Mode Dogfood

Status: Accepted
Date: 2026-09-01

## Context

After ADR-0458 closed the Branch/diff dogfood gate, the V1 release gate matrix
still marked Session, Claim, Runnable, `claim next`, and `next` as blocking
release maturity. Phase 4LW had proven Claim transfer and stale-gated takeover
in read-only continuation loops, and Phase 4MI had proven shared-Claim
collaboration against a real external project in read-only mode.

The remaining shared-Claim maturity gap was write/read-write use: the system
needed evidence that shared Claims remain usable when a real repository write
is involved, while still preventing protected mutation until one writer is the
unique remaining shared claimant.

## Decision

Record Phase 4MR as a dogfood-only shared-Claim write-mode evidence slice. The
slice uses a real WorkVCS Store for this repository delivery, starts separate
reader and writer Sessions, has both Sessions hold shared Claims on one Task,
proves that protected writer mutation is blocked while the shared Claim set is
non-unique, then releases the reader Claim so the writer becomes the unique
shared claimant before writing the repository documentation.

The release gate for Session, Claim, Runnable, `claim next`, and `next` can
move from `Partial`/blocking to `Pass`/non-blocking for the bounded V1-local
scope. The overall release decision remains false because other gate rows are
still Partial or Blocked.

## Non-Goals

- No Rust code change.
- No schema or CLI behavior change.
- No release, release candidate, tag, Push, or deployment.
- No V2 scope expansion.
- No automatic ownership arbitration between shared claimants.
- No distributed, remote, cloud, or cross-Store collaboration claim.
- No broad Handoff, merge, Resource, or performance maturity claim.

## Evidence

- Pre-write dogfood log directory:
  `/tmp/workvcs-4mr-shared-claim-write-mode-20260901T115851Z`.
- Dogfood Store:
  `/tmp/workvcs-4mr-shared-claim-write-mode-20260901T115851Z/store.workvcs`.
- Final dogfood summary:
  `/tmp/workvcs-4mr-shared-claim-write-mode-20260901T115851Z/summary.txt`.
- Provenance document:
  `docs/provenance/phase-4mr-shared-claim-write-mode-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

WorkVCS now has current dogfood evidence that shared Claims can coordinate a
real write-mode repository delivery slice without permitting simultaneous
protected mutation. This closes the specific write/read-write shared-Claim gap
named by the release gate matrix while preserving the broader non-ready
release decision.
