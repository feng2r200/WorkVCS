# ADR-0467: Phase 4MZ Resource Case-Folding Policy Dogfood

Status: Accepted
Date: 2026-09-02

## Context

ADR-0463 through ADR-0466 made bounded Git rename, symlink, submodule, and
sparse-checkout Resource adapter policies explicit, tested, and dogfooded. The
Resource registration, observation, applicability, and drift gate still listed
case-folding policy and background re-observation scheduling as open.

Phase 4MZ addresses case-folding only. Background re-observation scheduling
remains outside this decision.

Current inspection found:

- the host filesystem can resolve a wrong-case path for the same file, so
  WorkVCS must not claim filesystem-level case-sensitive exact path existence on
  this host;
- CLI lexical path normalization preserves segment case and contains no
  lowercase or casefold transform;
- local-file glob matching is configured as case-sensitive;
- Git worktree observation consumes parent Git path reporting for index, status,
  diff, and untracked paths.

## Decision

For current V1-local Resource adapters, WorkVCS performs no case folding.

- Local-file exact path observation uses filesystem-native path resolution. The
  summary reports that behavior as `filesystem_native_path_resolution`.
- Local-file path-prefix observation walks filesystem-native directory entries
  and preserves encountered relative entry names. The summary reports
  `filesystem_native_entry_names`.
- Local-file glob observation uses case-sensitive glob pattern matching. The
  summary reports `case_sensitive_glob_pattern` and `glob_case_sensitive=true`.
- Git worktree observation uses parent Git path reporting for index, status,
  diff, and untracked path material. The summary reports
  `parent_git_path_reporting`.

All supported observation summaries now expose:

```text
path_case_folding=disabled
```

Adapter-specific policy fields are:

```text
path_case_policy=filesystem_native_path_resolution
path_case_policy=filesystem_native_entry_names
path_case_policy=case_sensitive_glob_pattern
path_case_policy=parent_git_path_reporting
glob_case_sensitive=true
```

These fields make current behavior explicit to operators without changing
`local-file-path-prefix-manifest-v1`, `local-file-glob-manifest-v1`,
`git-worktree-manifest-v1`, Store schema, Resource basis shape, lexical path
normalization, or command-line contract.

## Non-Goals

- No filesystem canonicalization or case-normalized path lookup by WorkVCS.
- No cross-platform case-collision resolver.
- No change to canonical JSON string normalization.
- No manifest fingerprint profile change.
- No Store schema or Core/History semantic change.
- No new command or flag.
- No Git mutation by WorkVCS.
- No automatic scheduler, daemon, watcher, or implicit refresh.
- No remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Evidence

- Implementation and focused tests:
  `crates/workvcs-cli/src/main.rs`.
- Current-behavior inspection:
  `/tmp/workvcs-4mz-case-folding-policy-inspection-20260901T163650Z`.
- Focused validation:
  `/tmp/workvcs-4mz-focused-validation-20260901T164220Z/status.tsv`.
- Real pre-existing project clone dogfood:
  `/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T170209Z`.
- Candidate full pre-merge validation:
  `/tmp/workvcs-4mz-full-validation-candidate-20260901T171500Z/status.tsv`.
- Read-only independent review:
  `01a05ddd-782f-7252-9eea-a0d022928f98`.
- Final post-review validation:
  `/tmp/workvcs-4mz-final-validation-post-review-20260901T172330Z/summary.txt`.
- Provenance document:
  `docs/provenance/phase-4mz-resource-case-folding-policy-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

The Resource gate no longer needs to treat case-folding behavior as unknown for
the bounded V1-local local-file and Git worktree adapters. Case-sensitive or
case-insensitive behavior that comes from the host filesystem or parent Git is
reported as adapter-native behavior; WorkVCS itself does not fold path case.

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking overall. The remaining Resource work is background
re-observation scheduling when a real workflow proves it is needed.
