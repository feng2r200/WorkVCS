# ADR-0439: Phase 4LX Resource Scope Path Normalization

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LV and Phase 4LW moved WorkVCS back toward real dogfood loops, but
operators still had to hand-author scope JSON for common local-file workflows.
That friction made Context and Resource-backed `verify` usable in principle but
too easy to avoid in practice.

The V1 readiness ledger also kept Resource path normalization and Context
Resolver friction open. Closing the whole Resource adapter problem would be too
large for this slice, so Phase 4LX narrows the decision to deterministic
lexical path scope handling and CLI shorthands.

## Decision

Normalize recognized path selectors when Context matches path-scoped Knowledge.
The normalized selectors are used only for comparison; stored and rendered
canonical scope JSON remains the caller-supplied canonical object.

Recognized selectors remain the Phase 4LR set:

```text
path
paths
path_prefix
path_prefixes
scope_payload.path
scope_payload.paths
scope_payload.path_prefix
scope_payload.path_prefixes
```

Normalization is pure lexical handling of repeated separators, `.`, `..`, and
trailing separators. It does not read the filesystem, resolve symlinks, inspect
Git, apply case folding, expand globs, or infer path scope from prose.

Expose local-file shorthands that build the same canonical scope objects:

```text
workvcs knowledge create ... --scope-path PATH
workvcs knowledge list ... --scope-path PATH
workvcs context STORE --session SESSION --scope-path PATH
workvcs context-packet save STORE --session SESSION --scope-path-prefix PATH
workvcs claim next STORE --session SESSION --context-scope-path PATH
workvcs verify STORE ... --scope-path PATH
```

For `verify`, `--scope-path` and `--scope-path-prefix` default to
`scope_kind=path` and `scope_schema_version=1`. Existing
`--scope-payload-json` callers keep the previous advanced contract and must
still provide explicit scope kind and schema version.

Relative path shorthands remain relative after lexical normalization. Absolute
scope is available only when the caller passes an absolute path. This preserves
interoperation with existing relative `--scope-json` data.

## Non-Goals

- No Store schema change.
- No Resource adapter implementation or adapter-backed re-observation.
- No glob engine, symlink resolution, case-normalization, rename tracking, or
  filesystem existence check.
- No automatic transcript, Task, shell, or LLM path extraction.
- No change to Knowledge storage, Knowledge list semantics beyond the new CLI
  input shorthand, or Context item ranking.

## Evidence

Focused regression coverage:

```text
cargo test -q -p workvcs-core --test context_profile_budget_phase4kx context_packet_path_scope_matching_normalizes_lexical_variants
cargo test -q -p workvcs-cli path_scope_shorthand
cargo test -q -p workvcs-cli relative_scope_json
```

Read-only dogfood passed against the real local `agent_soul` project:

```text
phase4lx_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lx-resource-scope-normalization/.work-governance/runtime/dogfood/phase-4lx-20260901T024704Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lx-resource-scope-normalization/.work-governance/runtime/logs/phase-4lx/resource-scope-normalization.20260901T024704Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
context_scope_json={"path":"/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md"}
claim_next_context_scope_json={"path":"/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md"}
packet_scope_json={"path_prefix":"/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack"}
target_status_unchanged=PASS
```

## Consequences

Common file-scoped Context and Resource-backed Verification flows no longer
require operators to hand-author JSON. This directly advances the Context
Resolver, Resource scope, and CLI discoverability rows without expanding into
V2 retrieval or Resource adapter automation.

Remaining V1 Resource gaps are now narrower: glob semantics and adapter-backed
re-observation remain open, while lexical path normalization for explicit path
scopes is implemented and dogfood-proven.
