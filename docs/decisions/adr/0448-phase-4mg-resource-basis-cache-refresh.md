# ADR-0448: Phase 4MG Resource Basis Cache Refresh

Status: Accepted
Date: 2026-09-01

## Context

Phases 4MC through 4MF added executable Resource re-observation modes for exact
local files, local-file path-prefix manifests, local-file glob manifests, and Git
worktree manifests. Operators still need to choose the exact refresh flag from
the persisted Verification Resource basis, which keeps routine dogfood recovery
more manual than necessary.

The V1 readiness ledger keeps automatic re-observation policy Open. This slice
closes the explicit CLI policy subset only: an operator can request basis-aware
re-observation for a single Verification. It does not create a daemon, scheduler,
background watcher, or implicit AC refresh.

## Decision

Add:

```text
workvcs verification cache-refresh --resource-content-from-basis
```

The flag tells the CLI to inspect every persisted Resource basis entry on the
selected Verification and choose the existing adapter-backed observation path.

Supported basis contracts are exactly:

```text
adapter_kind=local-file
adapter_schema_version=1
scope_kind=path
scope_schema_version=1
scope_payload={"path":"..."}

adapter_kind=local-file
adapter_schema_version=1
scope_kind=path
scope_schema_version=1
scope_payload={"path_prefix":"..."}

adapter_kind=local-file
adapter_schema_version=1
scope_kind=path
scope_schema_version=1
scope_payload={"glob":"..."}

adapter_kind=git
adapter_schema_version=1
scope_kind=git-worktree
scope_schema_version=1
scope_payload={"repo":"..."}
```

The implementation pre-validates every Resource basis before observing any
Resource. If any basis entry is unsupported or malformed, the command fails
without writing partial ResourceObservations or a partial applicability cache.

Existing specific flags remain supported:

```text
--resource-content-from-scope-path
--resource-content-from-scope-path-prefix
--resource-content-from-scope-glob
--resource-content-from-scope-git-worktree
```

The new basis-aware flag is mutually exclusive with those specific flags.

## Non-Goals

- No daemon, scheduler, watcher, automatic polling, or implicit refresh during
  `ac status`.
- No unknown adapter inference.
- No new Resource adapter semantics beyond dispatching to already implemented
  local-file and Git worktree contracts.
- No multi-Verification refresh batch.

## Evidence

- Focused validation:
  `/tmp/workvcs-4mg-focused-validation-rerun-20260901T065653Z`.
- Medium-fix focused validation:
  `/tmp/workvcs-4mg-medium-fix-validation-20260901T070956Z`.
- Real read-only dogfood:
  `/tmp/workvcs-4mg-basis-refresh-dogfood-20260901T065738Z`.
- Final validation matrix:
  `/tmp/workvcs-4mg-final-validation-rerun2-20260901T071526Z`.
- Independent review:
  one medium coverage gap was fixed; re-review found no remaining
  blocker/high/medium findings.

## Consequences

Routine recovery can re-observe a Verification's supported Resource basis entries
without manual adapter flag selection. Unsupported basis entries remain explicit
failures rather than best-effort partial refreshes.
