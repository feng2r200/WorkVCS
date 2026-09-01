# ADR-0460: Phase 4MS Handoff Consumption Write-Mode Dogfood

Status: Accepted
Date: 2026-09-01

## Context

After ADR-0459 closed the shared-Claim write/read-write dogfood gap, the V1
release gate matrix still marked Handoff consumption as blocking release
maturity. Existing evidence covered focused Handoff smoke, local Handoff
consumption, blocked Handoff recovery, and external-project read-only Handoff
creation/show or continuation-adjacent Claim work. The remaining Handoff gap
was write-mode continuation: a separate Session needed to consume a focused
Handoff and complete a real repository delivery slice.

## Decision

Record Phase 4MS as a dogfood-only Handoff consumption write-mode evidence
slice. The slice uses a real WorkVCS Store for this repository delivery, has a
source Session end with a SessionDiff and author a focused Handoff, then has a
separate active continuation Session consume that Handoff through
`handoff consume`. The continuation Session claims the focused Task, performs
the repository documentation write, records VR-backed post-write verification
evidence, transitions the Task to `done`, releases its Claim, and ends with a
SessionDiff.

The release gate for Handoff consumption can move from `Partial`/blocking to
`Pass`/non-blocking for the bounded V1-local scope. The overall release
decision remains false because other gate rows are still Partial or Blocked.

## Non-Goals

- No Rust code change.
- No schema or CLI behavior change.
- No release, release candidate, tag, Push, or deployment.
- No V2 scope expansion.
- No automatic Handoff consumption, takeover, or Agent orchestration.
- No remote/cloud Handoff, cross-Store synchronization, or distributed
  collaboration claim.
- No broad merge, Resource, recovery, performance, or candidate-release
  maturity claim.

## Evidence

- Inspection log directory:
  `/tmp/workvcs-4ms-inspection-20260901T122229Z`.
- Dogfood log directory:
  `/tmp/workvcs-4ms-handoff-consumption-write-mode-20260901T122825Z`.
- Dogfood Store:
  `/tmp/workvcs-4ms-handoff-consumption-write-mode-20260901T122825Z/store.workvcs`.
- Provenance document:
  `docs/provenance/phase-4ms-handoff-consumption-write-mode-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

WorkVCS now has current dogfood evidence that a focused Handoff can be consumed
by a separate continuation Session in a write-mode repository delivery slice.
This closes the specific Handoff-consumption write-mode gate named by the
release gate matrix while preserving the broader non-ready release decision.
