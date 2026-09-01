# ADR-0463: Phase 4MV Git Rename Resource Policy Dogfood

Status: Accepted
Date: 2026-09-01

## Context

ADR-0447 introduced Git worktree Resource observation and refresh. It kept
rename behavior outside the narrow Phase 4MF decision because the original
evidence proved tracked, staged, unstaged, untracked, unavailable, and error
states, but did not prove how a real Git rename should be interpreted for V1.

The current release gate matrix still lists Resource registration,
observation, applicability, and drift as `Partial` and blocking. The named
next evidence includes broader symlink/case/rename policy, Git adapter
semantics, and background re-observation scheduling. Phase 4MV narrows that
Resource gate by addressing the rename-policy part only.

## Decision

For the V1-local Git worktree Resource adapter, WorkVCS does not perform
semantic rename tracking. Git worktree Resource snapshots keep rename detection
disabled and treat a Git rename as deterministic delete/add drift in the raw
staged or unstaged diff material that contributes to the
`git-worktree-manifest-v1` fingerprint.

The CLI now exposes that policy in Git ResourceObservation summary JSON:

```text
rename_detection=disabled
rename_policy=delete_add
```

These summary fields make the existing behavior explicit to operators without
changing the `git-worktree-manifest-v1` observed-content fingerprint profile,
Store schema, Resource basis shape, or command-line contract.

## Non-Goals

- No `git-worktree-manifest-v1` fingerprint profile change.
- No Store schema or Core/History semantic change.
- No new command or flag.
- No semantic rename tracking or path lineage model.
- No Git mutation by WorkVCS.
- No submodule, sparse checkout, symlink, or case-folding policy.
- No automatic scheduler, daemon, watcher, or implicit refresh.
- No remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Evidence

- Implementation and focused tests:
  `crates/workvcs-cli/src/main.rs`.
- Focused validation:
  `/tmp/workvcs-4mv-focused-validation-20260901T140207Z`.
- Real pre-existing project clone dogfood:
  `/tmp/workvcs-4mv-git-rename-resource-dogfood-20260901T140544Z`.
- Provenance document:
  `docs/provenance/phase-4mv-git-rename-resource-policy-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

The Resource gate no longer needs to treat Git worktree rename behavior as
unknown for the bounded V1-local adapter. A staged or unstaged Git rename is
observable as drift and is represented under a delete/add policy rather than a
semantic rename relation.

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking overall. Remaining Resource work is limited to concrete
real-workflow needs such as symlink/case policy, submodule or sparse-checkout
Git semantics, and background re-observation scheduling.
