# ADR-0440: Phase 4LY Resource Scope File Observation

Status: Accepted
Date: 2026-09-01

## Context

Phase 4LX made explicit path scopes practical by adding path shorthands and
lexical path-scope matching. The next Resource-backed verification friction
was still concrete: operators had to pass a separate resource content string,
content file, or fingerprint even when `verify` already had an explicit
`--scope-path` naming the local file being verified.

Full Resource adapter-backed re-observation remains too broad for this step.
It needs adapter contracts, path/glob semantics, unavailable/error reporting,
and scheduling decisions. Phase 4LY therefore closes only the explicit local
file case.

## Decision

Add an explicit `workvcs verify` fingerprint source:

```text
--resource-content-from-scope-path
```

When this flag is present, `verify` reads bytes from the path supplied by
`--scope-path` and uses the existing Store content digest algorithm as the
ResourceObservation fingerprint. The invocation still records the same
Evidence, ResourceObservation, Verification resource basis, and applicability
cache as the existing wrapper path.

The flag is part of the existing mutually exclusive Resource fingerprint
source group, alongside:

```text
--resource-fingerprint
--resource-content
--resource-content-file
```

`--resource-content-from-scope-path` requires `--scope-path`. It does not infer
a file path from `--scope-path-prefix` or arbitrary `--scope-payload-json`.

## Non-Goals

- No Store schema change.
- No Resource adapter contract implementation.
- No automatic re-observation scheduler.
- No glob expansion, symlink canonicalization, case folding, rename tracking,
  or Git diff comparison.
- No implicit path extraction from Task text, transcript text, shell commands,
  or Agent messages.
- No change to `resource observe`, except for preserving the shared
  fingerprint-source validation path.

## Evidence

Focused regression coverage:

```text
cargo test -q -p workvcs-cli cli_verify_accepts_path_scope_shorthand_for_resource_observation
cargo test -q -p workvcs-cli cli_lazy_record_and_verify_commands_render_nested_help
cargo test -q -p workvcs-cli cli_verify_content_errors_use_verify_flag_labels
```

Read-only dogfood passed against the real local `agent_soul` project:

```text
phase4ly_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ly-resource-scope-file-observation/.work-governance/runtime/dogfood/phase-4ly-20260901T031203Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ly-resource-scope-file-observation/.work-governance/runtime/logs/phase-4ly/resource-scope-file-observation.20260901T031203Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
scope_path=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/../dianjin-research-skill-pack/SYSTEM.md
resource_fingerprint=5b07052923bb60b5a9be5dca7079ca90fd4ae78856fdb77349ce98ccef5b0a3b
expected_resource_fingerprint=5b07052923bb60b5a9be5dca7079ca90fd4ae78856fdb77349ce98ccef5b0a3b
target_status_unchanged=PASS
```

## Consequences

The common "verify this local file" path now needs one fewer manually supplied
value: the operator can provide `--scope-path` and explicitly opt in to using
that path as the ResourceObservation content source.

This advances Resource-backed verification dogfood without closing the broader
adapter-backed re-observation problem. The remaining Resource gap is now
explicitly about adapter contracts, glob semantics, unavailable/error handling,
and repeatable re-observation policy rather than simple local-file input
plumbing.
