# ADR-0462: Phase 4MU Preexisting External Merge Write-Mode Dogfood

Status: Accepted
Date: 2026-09-01

## Context

ADR-0461 and Phase 4MT narrowed the merge lifecycle release gate by proving a
generated external local Git project write-mode merge. The release gate still
kept merge lifecycle and conflict recovery as `Partial` and blocking because a
generated project did not prove behavior against pre-existing real project
content.

The next named merge evidence was a repeated write-mode merge against a
pre-existing real external project or a broader operator-owned workflow.

## Decision

Record Phase 4MU as dogfood-only merge evidence against a pre-existing real
local Git project. The run uses `/Users/example/Projects/HeXun/Hernes/agent_soul`
as the read-only source project and clones it into `/tmp` for all write-mode
target/source branch work. The original external project remains unchanged.

The sandbox modifies existing `agent_soul` `README.md` content on both target
and source branches, adds one source-only file, observes a real Git conflict,
resolves the merge to the source branch, and creates a two-parent Git merge
commit. The WorkVCS Store mirrors the same divergence and completes
`merge start`, active inspection, unresolved freeze guard, explicit source-side
resolutions, `merge freeze`, `merge continue`, completed inspection, final
Branch head and commit inspection, final `show-at`, Branch diff, SessionDiff
closeout, required-valid Store integrity, and doctor.

This closes the specific pre-existing external-project write-mode merge
evidence gap for the bounded V1-local release gate. The merge lifecycle and
conflict recovery row can move to `Pass`/non-blocking. The overall release
decision remains false because other release gates remain `Partial` or
`Blocked`.

## Non-Goals

- No Rust code change.
- No schema or CLI behavior change.
- No release, release candidate, tag, Push, or deployment.
- No V2 scope expansion.
- No semantic or LLM merge.
- No custom semantic conflict resolver.
- No GUI/TUI flow.
- No mutation of the original `agent_soul` repository.
- No remote, distributed, cross-Store, or multi-operator merge claim.
- No broader Resource, Handoff, recovery, performance, or candidate-release
  maturity claim.

## Evidence

- Inspection log directory:
  `/tmp/workvcs-4mu-inspection-20260901T132814Z`.
- Dogfood log directory:
  `/tmp/workvcs-4mu-preexisting-external-merge-write-mode-20260901T133226Z`.
- Dogfood Store:
  `/tmp/workvcs-4mu-preexisting-external-merge-write-mode-20260901T133226Z/store.sqlite`.
- External source project:
  `/Users/example/Projects/HeXun/Hernes/agent_soul`.
- External source project head:
  `e3004b29c8bf3791e87d877b021432dcd1158705`.
- Provenance document:
  `docs/provenance/phase-4mu-preexisting-external-merge-write-mode-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

WorkVCS now has merge lifecycle evidence across local durable Stores, a larger
merge Store, a generated external Git write-mode project, and a pre-existing
real external Git project cloned into a write-mode sandbox. For the bounded
V1-local release gate, merge lifecycle and conflict recovery are no longer a
blocking row.

The next local release-oriented slice should move to another blocking gate,
with Resource adapter/re-observation policy as the current highest-value
candidate unless a newer real dogfood blocker appears.
