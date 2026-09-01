# ADR-0458: Phase 4MQ Branch Diff Dogfood

Status: Accepted
Date: 2026-09-01

## Context

After ADR-0457 added the V1 release gate matrix, the matrix still marked
Workspace, Branch, history, diff, show-at, and restore as blocking release
maturity. Phase 4LO had dogfooded copied-target `restore` and `show-at`, but
Branch fork and Branch-to-Branch diff still lacked evidence from a real
repository delivery slice.

Repository smoke exercises Branch behavior, mostly as setup for merge paths.
That is useful regression coverage, but it is not the same as using Branch and
diff commands to reason about an implementation Branch during real WorkVCS
delivery.

## Decision

Record Phase 4MQ as a dogfood-only Branch/diff evidence slice. The slice uses a
real WorkVCS Store for this repository delivery, forks a base Branch into an
implementation Branch, records implementation-branch work, compares the two
Branch heads in both directions, and proves the compared WorkStates with
`history`, `show-at`, and `store integrity`.

The release gate for Workspace, Branch, history, diff, show-at, and restore can
move from `Partial`/blocking to `Pass`/non-blocking for the bounded V1-local
scope. The overall release decision remains false because other gate rows are
still Partial or Blocked.

## Non-Goals

- No Rust code change.
- No schema or CLI behavior change.
- No release, release candidate, tag, Push, or deployment.
- No V2 scope expansion.
- No broad performance, merge, handoff, Resource, or multi-operator maturity
  claim.
- No hidden product fix for local dogfood harness assumptions.

## Evidence

- Dogfood log directory:
  `/tmp/workvcs-4mq-branch-diff-dogfood-20260901T113052Z`.
- Final dogfood summary:
  `/tmp/workvcs-4mq-branch-diff-dogfood-20260901T113052Z/final-summary.txt`.
- Final acceptance check:
  `/tmp/workvcs-4mq-branch-diff-dogfood-20260901T113052Z/final-evidence-check.txt`.
- Provenance document:
  `docs/provenance/phase-4mq-branch-diff-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

WorkVCS now has current, reproducible dogfood evidence for Branch fork and
Branch-to-Branch diff in a real repository delivery Store. This closes the
specific Branch/diff dogfood gap named by ADR-0457 while keeping the release
decision conservative: V1 is still not release-ready, V0.1 dogfood is still not
complete, and release-candidate work remains separately gated.
