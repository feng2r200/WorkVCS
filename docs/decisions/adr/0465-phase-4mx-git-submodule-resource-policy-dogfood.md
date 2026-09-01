# ADR-0465: Phase 4MX Git Submodule Resource Policy Dogfood

Status: Accepted
Date: 2026-09-01

## Context

ADR-0447 introduced Git worktree Resource observation and refresh. ADR-0463
closed the Git rename-policy part by making no-renames/delete-add behavior
explicit, tested, and dogfooded. ADR-0464 closed the Git symlink-policy part by
making tracked-symlink and untracked-non-regular behavior explicit, tested,
and dogfooded.

The Resource registration, observation, applicability, and drift gate still
lists remaining adapter-policy gaps. Phase 4MX narrows that Resource gate by
addressing submodule behavior only. Case-folding policy, sparse-checkout Git
semantics, and background re-observation scheduling remain outside this
decision.

## Decision

For the V1-local Git worktree Resource adapter, WorkVCS observes submodules
through the parent Git worktree view only.

- Tracked submodule entries are represented by parent `git ls-files -s`
  gitlink mode `160000` and object id.
- Submodule dirty state is represented by parent
  `git status --porcelain=v1 -z --untracked-files=all` bytes.
- Submodule pointer updates are represented by parent index and staged or
  unstaged diff material, including `Subproject commit` lines when Git emits
  them.
- WorkVCS does not recurse into submodule worktrees and does not hash
  submodule-internal untracked files as parent untracked file content.

The CLI now exposes these Git ResourceObservation summary policy fields:

```text
submodule_policy=parent_gitlink_status_diff
submodule_recursion=disabled
```

These summary fields make the existing behavior explicit to operators without
changing the `git-worktree-manifest-v1` observed-content fingerprint profile,
Store schema, Resource basis shape, or command-line contract.

## Non-Goals

- No `git-worktree-manifest-v1` fingerprint profile change.
- No Store schema or Core/History semantic change.
- No new command or flag.
- No recursive submodule content hashing.
- No WorkVCS `git submodule init`, `update`, or `fetch` behavior.
- No semantic submodule relation model.
- No Git mutation by WorkVCS.
- No case-folding policy.
- No sparse-checkout Git policy.
- No automatic scheduler, daemon, watcher, or implicit refresh.
- No remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Evidence

- Implementation and focused tests:
  `crates/workvcs-cli/src/main.rs`.
- Current-behavior inspection:
  `/tmp/workvcs-4mx-submodule-policy-inspection-20260901T151341Z`.
- Focused validation:
  `/tmp/workvcs-4mx-focused-validation-20260901T151618Z/status.tsv`.
- Real pre-existing project clone dogfood:
  `/tmp/workvcs-4mx-git-submodule-resource-dogfood-20260901T152048Z`.
- Provenance document:
  `docs/provenance/phase-4mx-git-submodule-resource-policy-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

The Resource gate no longer needs to treat Git worktree submodule behavior as
unknown for the bounded V1-local adapter. A tracked submodule is visible
through parent Git gitlink, status, and diff material, and submodule-internal
untracked files are not promoted to parent untracked file content.

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking overall. Remaining Resource work is limited to concrete
real-workflow needs such as case-folding policy, sparse-checkout Git semantics,
and background re-observation scheduling.
