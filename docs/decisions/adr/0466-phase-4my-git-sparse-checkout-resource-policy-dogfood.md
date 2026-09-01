# ADR-0466: Phase 4MY Git Sparse Checkout Resource Policy Dogfood

Status: Accepted
Date: 2026-09-02

## Context

ADR-0447 introduced Git worktree Resource observation and refresh. ADR-0463,
ADR-0464, and ADR-0465 closed the bounded Git rename, symlink, and submodule
policy parts by making each behavior explicit, tested, and dogfooded.

The Resource registration, observation, applicability, and drift gate still
lists remaining adapter-policy gaps. Phase 4MY narrows that Resource gate by
addressing sparse-checkout behavior only. Case-folding policy and background
re-observation scheduling remain outside this decision.

## Decision

For the V1-local Git worktree Resource adapter, WorkVCS observes sparse
checkouts through the parent Git worktree view only.

- Sparse-excluded tracked files remain represented by parent `git ls-files -s`
  index entries.
- Sparse-checkout skip state is parent-Git metadata; WorkVCS does not expand
  the sparse definition, materialize sparse-excluded paths, or hash
  sparse-excluded tracked file contents from the working tree.
- Parent-visible status, staged diff, unstaged diff, and untracked regular
  files continue to contribute to the existing `git-worktree-manifest-v1`
  fingerprint.
- Parent-visible untracked regular files under sparse-excluded directories use
  the same sorted untracked regular-file fingerprint policy as other parent
  untracked files.

The CLI now exposes these Git ResourceObservation summary policy fields:

```text
sparse_checkout_policy=parent_index_status_diff
sparse_checkout_expansion=disabled
```

These summary fields make the existing behavior explicit to operators without
changing the `git-worktree-manifest-v1` observed-content fingerprint profile,
Store schema, Resource basis shape, or command-line contract.

## Non-Goals

- No `git-worktree-manifest-v1` fingerprint profile change.
- No Store schema or Core/History semantic change.
- No new command or flag.
- No sparse-checkout expansion or materialization by WorkVCS.
- No recursive hashing of sparse-excluded tracked file contents from the
  working tree.
- No semantic sparse-checkout relation model.
- No Git mutation by WorkVCS.
- No case-folding policy.
- No automatic scheduler, daemon, watcher, or implicit refresh.
- No remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Evidence

- Implementation and focused tests:
  `crates/workvcs-cli/src/main.rs`.
- Current-behavior inspection:
  `/tmp/workvcs-4my-sparse-checkout-policy-inspection-20260901T155400Z`.
- Focused validation:
  `/tmp/workvcs-4my-focused-validation-20260901T155706Z/status.tsv`.
- Real pre-existing project clone dogfood:
  `/tmp/workvcs-4my-git-sparse-checkout-resource-dogfood-20260901T160432Z`.
- Independent validation:
  subagent `01a05dbd-94ce-7871-9732-caccb9163e95`.
- Final validation:
  `/tmp/workvcs-4my-final-validation-post-review-20260901T162357Z/summary.txt`.
- Provenance document:
  `docs/provenance/phase-4my-git-sparse-checkout-resource-policy-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

The Resource gate no longer needs to treat Git worktree sparse-checkout
behavior as unknown for the bounded V1-local adapter. Sparse-excluded tracked
files remain visible through parent Git index material, while WorkVCS avoids
expanding or materializing sparse-excluded working-tree paths.

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking overall. Remaining Resource work is limited to concrete
real-workflow needs such as case-folding policy and background re-observation
scheduling.
