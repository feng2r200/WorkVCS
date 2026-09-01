# ADR-0447: Phase 4MF Git Worktree Resource Refresh

Status: Accepted
Date: 2026-09-01

## Context

ADR-0001 requires Git-backed verification observations to represent the actual
verified working state, including relevant uncommitted changes, rather than HEAD
alone. After the exact path, path-prefix, and glob local-file contracts, the V1
readiness ledger still lists Git working-tree observation as Open.

## Decision

Add two explicit CLI flags:

```text
workvcs verify --resource-content-from-scope-git-worktree
workvcs verification cache-refresh --resource-content-from-scope-git-worktree
```

`verify` also accepts `--scope-git-worktree PATH` as a Resource scope shorthand.
The flags support only Resource basis entries with:

```text
adapter_kind=git
adapter_schema_version=1
scope_kind=git-worktree
scope_schema_version=1
scope_payload={"repo":"..."}
```

The Git worktree fingerprint is the content digest of a canonical manifest with
profile `git-worktree-manifest-v1`. The manifest includes:

```text
HEAD object id
raw `git status --porcelain=v1 -z --untracked-files=all` digest and size
raw `git ls-files -s -z` digest and size
raw staged diff digest and size
raw unstaged diff digest and size
sorted untracked regular-file paths with content fingerprints and sizes
```

The implementation invokes `git` with `GIT_OPTIONAL_LOCKS=0` and does not run
checkout, add, commit, write-tree, reset, or merge. The local repo path remains
in Resource scope and ResourceObservation summary; it is not part of the
observed content fingerprint.

When refreshing:

```text
unchanged Git worktree state -> new ResourceObservation + applicable/all_basis_applicable
changed tracked/untracked state -> stale/resource_drift
missing repo path -> unknown/resource_unavailable
non-Git/error state -> unknown/resource_error
```

## Non-Goals

- No Git mutation, checkout, add, commit, reset, merge, write-tree, or index
  refresh policy.
- No Git diff explanation or semantic merge.
- No automatic scheduler or daemon.
- No submodule, sparse checkout, rename, case-folding, or symlink policy beyond
  the narrow snapshot error behavior.
- No Core/History Git interpretation.

## Evidence

- Implementation: `crates/workvcs-cli/src/main.rs` adds the
  `--scope-git-worktree` scope shorthand and
  `--resource-content-from-scope-git-worktree` observation/refresh path.
- Focused validation:
  `/tmp/workvcs-4mf-focused-validation-20260901T060637Z`.
- Medium-finding fix validation:
  `/tmp/workvcs-4mf-medium-fix-validation-rerun3-20260901T063909Z`.
- Real read-only dogfood:
  `/tmp/workvcs-4mf-git-dogfood-20260901T060805Z`.
- Final matrix:
  `/tmp/workvcs-4mf-final-validation-rerun2-20260901T064009Z`.
- Independent review: Laplace found no blocker/high; four medium findings were
  fixed and the final re-review found no remaining blocker/high/medium.
- Local commit and cleanup:
  `af5fba35ae4877f7278e2eb3a70a24a328cb2aec`;
  `/tmp/workvcs-4mf-cleanup-20260901T064649Z`.

## Consequences

WorkVCS gains an executable Git worktree Resource observation contract that can
prove resource drift for real working states without reducing Git-backed
verification to HEAD-only checks.

Automatic re-observation policy and broader Git adapter semantics remain Open.
