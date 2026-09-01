# ADR-0446: Phase 4ME Local File Glob Refresh

Status: Accepted
Date: 2026-09-01

## Context

ADR-0444 and ADR-0445 added executable local-file Resource observation contracts
for exact paths and path-prefix directories. The V1 readiness ledger still
listed glob semantics as Open. Operators can already provide arbitrary
`scope_payload_json`, but there is no explicit CLI contract that records or
refreshes a deterministic ResourceObservation for a local-file glob selector.

## Decision

Add two explicit CLI flags:

```text
workvcs verify --resource-content-from-scope-glob
workvcs verification cache-refresh --resource-content-from-scope-glob
```

`verify` also accepts `--scope-glob GLOB` as a Resource scope shorthand. The
flags support only Resource basis entries with:

```text
adapter_kind=local-file
scope_kind=path
scope_schema_version=1
scope_payload={"glob":"..."}
```

The glob must have a fixed non-wildcard root before the first wildcard segment.
Unbounded patterns such as `*.md`, `**/*.rs`, and `/**/*.rs` are rejected.
Normalized fixed roots that still contain parent-directory traversal are also
rejected. The root is used only to bound traversal and derive relative manifest
paths. The glob fingerprint is the content digest of a canonical manifest with
profile `local-file-glob-manifest-v1`. The manifest contains sorted relative
file paths, each file's content fingerprint, and each file's byte size. The
root and glob string remain in the Resource basis scope and ResourceObservation
summary; they are not part of the observed content fingerprint.

When refreshing:

```text
unchanged matched files -> new ResourceObservation + applicable/all_basis_applicable
changed matched files   -> stale/resource_drift
missing fixed root      -> unknown/resource_unavailable
matched unsupported/read error -> unknown/resource_error
```

An existing root with no matching files is an observed empty manifest, not an
unavailable Resource.

## Non-Goals

- No Context, Knowledge, or Claim glob filtering.
- No Git working-tree adapter.
- No automatic scheduler or daemon.
- No broad symlink, case-folding, rename, or deletion semantics. Symlinks,
  directories matched as files, and special files are unsupported observation
  errors.
- No Core/History filesystem interpretation.

## Evidence

Focused validation:

```text
cargo_fmt=PASS
cargo_check=PASS
help=PASS
content_errors=PASS
glob_root=PASS
focused_glob=PASS
focused_prefix=PASS
focused_path=PASS
log_dir=/tmp/workvcs-4me-focused-validation-after-scope-fix-rerun-20260901T054309Z
```

Real external-project dogfood:

```text
phase4me_dogfood_result=PASS
log_dir=/tmp/workvcs-4me-glob-dogfood-after-scope-fix-20260901T054406Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_glob=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/skills/use-financial-mcp-tools/references/financial-report/tools/*.md
target_files=4
target_symlinks=0
real_baseline_fingerprint=f03090b38d551f4d97479c79eb8186732fad89d191ef8727a2c466019cf86df4
real_refresh_fingerprint=f03090b38d551f4d97479c79eb8186732fad89d191ef8727a2c466019cf86df4
real_refresh_applicability=applicable
real_refresh_reason_code=all_basis_applicable
copy_stale_reason_code=resource_drift
copy_unavailable_reason_code=resource_unavailable
copy_empty_reason_code=resource_drift
copy_error_reason_code=resource_error
target_status_unchanged=true
```

Independent review found that unbounded glob validation had to apply to both
Resource observation and the manual fingerprint/content `--scope-glob`
shorthand path. The fix rejects globs without a fixed non-wildcard root and
fixed roots containing parent-directory traversal from both paths; focused
validation now covers `**/*.rs`, `*.md`, `/**/*.rs`, `../project/**/*.rs`, and a
valid `src/**/*.rs` scope shorthand. Independent re-review confirmed the high
is closed and reported no new blocker/high/medium findings.

Final validation:

```text
diff_check=PASS
fmt_check=PASS
schema=PASS
clippy=PASS
test=PASS
cli_smoke=PASS
log_dir=/tmp/workvcs-4me-final-validation-after-scope-fix-20260901T054651Z
```

## Consequences

WorkVCS gains a third executable local-file Resource re-observation contract:
exact path, path-prefix aggregation, and glob aggregation. This narrows the
Resource readiness gap while keeping Store/History authority unchanged.

Git working-tree observation, automatic re-observation policy, and broader
filesystem normalization semantics remain Open.
