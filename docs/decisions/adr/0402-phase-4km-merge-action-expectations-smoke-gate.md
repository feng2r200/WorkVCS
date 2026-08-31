# ADR-0402: Phase 4KM Merge Action Expectations Smoke Gate

Status: Accepted
Date: 2026-08-31

## Context

ADR-0397 introduced the repository-level CLI smoke workflow, and ADR-0398
through ADR-0401 extended it through verified Task completion, runtime closeout,
Store integrity, Task scheduling, and explicit `claim next`.

That smoke workflow still runs on a single Work Branch. WorkVCS already has
accepted and implemented Branch fork plus merge start, item classification,
runtime resolution, resolution freeze, merge continue, two-parent replay, merge
show, and merge list behavior. The current smoke therefore does not yet prove
the local CLI can demonstrate a minimal branch divergence and merge lifecycle.

Automation also has direct expectation gates for `merge show` and `merge list`,
but not for the action commands that start, resolve, freeze, and continue a
merge. Scripts can parse those outputs manually, but the rest of the CLI has
been moving toward first-class expectation flags for process-boundary checks.

## Decision

Phase 4KM adds CLI-only expectation checks to merge action commands and extends
`scripts/smoke-v0.1-cli-workflow.sh` with a branch fork and merge lifecycle
gate.

The CLI adds optional post-result expectations:

- `merge start`: expected runtime state, merge base, target head, source head,
  and origin session;
- `merge resolve`: expected merge id, resolution kind, and resolved-by session;
- `merge freeze`: expected merge id and frozen item count;
- `merge continue`: expected runtime state, target branch, source branch,
  and continued-by session.

Each expectation compares only the result that the Engine already returned. A
mismatch returns the same query-validation style error used by the existing CLI
expectation surfaces. These flags do not change merge selection, conflict
classification, runtime mutation, frozen resolution semantics, replay, commit
shape, Branch head movement, or Store integrity.

The smoke workflow now creates a source Work Branch from the target Branch,
adds a target-side merge marker plus source-side work, starts a merge, verifies
the AUTO item, resolves it as `theirs`, freezes the resolution set, continues
the merge, reads the generated result commit from the command output, verifies
the target Branch head moved to that merge commit, verifies the merge commit has
the expected primary and secondary parents, verifies closed merge visibility,
and updates final integrity expectations from the actual workflow shape.

## Consequences

The repository smoke workflow now demonstrates the minimal local branch
divergence and merge lifecycle in the same process-level CLI artifact as the
Task, Verification, Runtime, and integrity loop.

The new expectation flags remain a CLI scripting contract only. Future merge
semantic extensions, custom materialization, richer replay vocabulary,
cross-Store merge, remote/federated behavior, Branch deletion, and priority
ordering remain separate decisions.
