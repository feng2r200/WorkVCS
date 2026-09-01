# ADR-0461: Phase 4MT External Merge Write-Mode Dogfood

Status: Accepted
Date: 2026-09-01

## Context

ADR-0454 and Phase 4MM left merge with bounded local Store maturity evidence:
larger conflict sets, auto items, explicit resolutions, freeze/continue, a
two-parent merge commit, and required-valid integrity/doctor. The V1 release
gate matrix still kept merge lifecycle and conflict recovery as blocking
release maturity because that evidence did not include a write-mode external
project workflow.

Phase 4MT targets that narrower gap by pairing an actual external local Git
merge with a WorkVCS semantic merge over the same external project state. The
external project is generated for this dogfood run. It is real write-mode Git
work, but it is not a pre-existing business repository or broad operator-owned
workflow.

## Decision

Record Phase 4MT as dogfood-only merge evidence. The run creates a generated
external local Git project, diverges target and source worktrees, observes an
actual Git conflict, resolves the external Git merge to the source branch, and
creates a two-parent external Git merge commit.

The same run models the external file states in a WorkVCS Store and completes
the WorkVCS merge lifecycle: `merge start`, active merge inspection, unresolved
freeze guard, explicit resolutions, `merge freeze`, `merge continue`,
completed merge inspection, final Branch head inspection, final `show-at`,
Branch diff, SessionDiff closeout, required-valid Store integrity, and doctor.

This evidence narrows the merge gate from "no write-mode external project
proof" to "generated external local Git project write-mode proof exists." It
does not move the V1 release gate row to `Pass`, because release maturity still
needs repetition against a pre-existing real external project or a broader
operator-owned workflow.

## Non-Goals

- No Rust code change.
- No schema or CLI behavior change.
- No release, release candidate, tag, Push, or deployment.
- No V2 scope expansion.
- No semantic or LLM merge.
- No custom semantic conflict resolver.
- No remote, distributed, cross-Store, or multi-operator merge claim.
- No claim that generated external-project proof equals pre-existing business
  repository maturity.
- No broader Resource, Handoff, recovery, performance, or candidate-release
  maturity claim.

## Evidence

- Inspection log directory:
  `/tmp/workvcs-4mt-inspection-20260901T125347Z`.
- Dogfood log directory:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z`.
- Dogfood Store:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z/store.sqlite`.
- Acceptance summary:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z/summary.acceptance.txt`.
- Provenance document:
  `docs/provenance/phase-4mt-external-merge-write-mode-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

WorkVCS now has current dogfood evidence that its merge lifecycle can track and
complete a generated external local Git write-mode merge workflow with one
conflict item, one source-only auto item, explicit source-side resolutions,
two-parent merge output, final WorkState proof, and required-valid
integrity/doctor.

The merge release gate remains `Partial` and blocking for broad V1 release
maturity. The next useful merge slice should repeat the workflow against a
pre-existing real external project or a broader operator-owned workflow before
any release-ready or release-candidate claim.
