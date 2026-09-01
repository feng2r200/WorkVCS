# ADR-0464: Phase 4MW Git Symlink Resource Policy Dogfood

Status: Accepted
Date: 2026-09-01

## Context

ADR-0447 introduced Git worktree Resource observation and refresh. ADR-0463
closed the Git rename-policy part by making no-renames/delete-add behavior
explicit, tested, and dogfooded. The Resource registration, observation,
applicability, and drift gate still lists remaining adapter-policy gaps.

Phase 4MW narrows that Resource gate by addressing symlink behavior only.
Case-folding, submodule or sparse-checkout semantics, and background
re-observation scheduling remain outside this decision.

## Decision

For the V1-local Git worktree Resource adapter, tracked symlinks are represented
through Git's own index and diff material. WorkVCS does not dereference symlink
targets, hash target contents separately, or create semantic symlink-target
relations for `git-worktree-manifest-v1`.

Non-regular untracked entries, including untracked symlinks, are unsupported by
the current Git worktree manifest reader. When encountered during refresh, they
map to Resource error behavior rather than silently hashing linked target
content:

```text
applicability=unknown
reason_code=resource_error
observation_status=error
observation_id=none
```

The CLI now exposes these Git ResourceObservation summary policy fields:

```text
tracked_symlink_policy=git_index_and_diff
untracked_non_regular_policy=unsupported_resource_error
```

These fields are summary metadata only. They do not change the
`git-worktree-manifest-v1` fingerprint profile, Store schema, Resource basis
shape, or command-line contract.

## Non-Goals

- No `git-worktree-manifest-v1` fingerprint profile change.
- No Store schema or Core/History semantic change.
- No new command or flag.
- No symlink dereference, symlink target hashing, or symlink relation model.
- No Git mutation by WorkVCS.
- No case-folding policy.
- No submodule or sparse-checkout Git policy.
- No automatic scheduler, daemon, watcher, or implicit refresh.
- No remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Evidence

- Implementation and focused tests:
  `crates/workvcs-cli/src/main.rs`.
- Current-behavior inspection:
  `/tmp/workvcs-4mw-symlink-policy-inspection-20260901T144206Z`.
- Focused validation:
  `/tmp/workvcs-4mw-focused-validation-20260901T144544Z/status.tsv`.
- Real pre-existing project clone dogfood:
  `/tmp/workvcs-4mw-git-symlink-resource-dogfood-20260901T144729Z`.
- Provenance document:
  `docs/provenance/phase-4mw-git-symlink-resource-policy-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

The Resource gate no longer needs to treat Git worktree symlink behavior as
unknown for the bounded V1-local adapter. Tracked symlinks are visible through
the raw Git index and diff material that already contributes to deterministic
Git worktree Resource fingerprints. Untracked symlinks fail as unsupported
non-regular entries and project Resource error state instead of following or
hashing target content.

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking overall. Remaining Resource work is limited to concrete
real-workflow needs such as case-folding policy, submodule or sparse-checkout
Git semantics, and background re-observation scheduling.
