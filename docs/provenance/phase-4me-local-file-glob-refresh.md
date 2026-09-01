# Phase 4ME Local File Glob Refresh Evidence

Date: 2026-09-01

## Scope

Phase 4ME adds explicit local-file glob Resource observation and re-observation
modes:

```bash
workvcs verify "$STORE" \
  --scope-glob "$GLOB" \
  --resource-content-from-scope-glob

workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-glob
```

The flags support only:

```text
adapter_kind=local-file
scope_kind=path
scope_schema_version=1
scope_payload={"glob":"..."}
```

Core Store/History semantics remain unchanged. Context, Knowledge, Git
working-tree observation, automatic scheduling, and broad filesystem
normalization remain outside this slice.

## Implementation Evidence

The CLI now:

```text
normalizes `--scope-glob` as a Resource scope shorthand
requires a fixed non-wildcard root before the first wildcard segment
rejects unbounded wildcard roots and parent-directory root traversal
rejects absolute filesystem-root scans
computes a deterministic local-file glob manifest for verify baselines
validates all Resource basis entries before recording refresh observations
matches sorted regular files under the fixed root
records unavailable/error applicability stamps when the glob cannot be observed
records the resulting applicability cache through the existing Engine facade
```

The glob manifest profile is:

```text
local-file-glob-manifest-v1
```

Its entries contain:

```text
relative file path
file content fingerprint
file size in bytes
```

## Focused Validation

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

The focused glob test covers:

```text
verify --scope-glob --resource-content-from-scope-glob
unchanged glob -> applicable, all_basis_applicable, new observation id
changed matched file -> stale, resource_drift
missing fixed root -> unknown, resource_unavailable
existing empty root -> stale, resource_drift
matched directory -> unknown, resource_error
stored scope payload -> {"glob":"..."}
```

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

## Dogfood Evidence

The read-only external glob used as the real Resource baseline was:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/skills/use-financial-mcp-tools/references/financial-report/tools/*.md
```

The passing run:

```text
phase4me_dogfood_result=PASS
log_dir=/tmp/workvcs-4me-glob-dogfood-after-scope-fix-20260901T054406Z
store=/tmp/workvcs-4me-glob-dogfood-after-scope-fix-20260901T054406Z/store.sqlite
branch_id=01a05b7f-125b-7161-a980-3b703544355a
head_commit_id=01a05b7f-135d-7133-80db-5f393a92bd8d
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_glob=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/skills/use-financial-mcp-tools/references/financial-report/tools/*.md
target_files=4
target_symlinks=0
real_verification_id=01a05b7f-12b7-7a33-b611-649fb132ba4e
real_baseline_observation_id=01a05b7f-12b6-7aa1-876c-e057b3a29b0e
real_refresh_observation_id=01a05b7f-12e2-74a3-b0ff-ff3a1695f682
real_baseline_fingerprint=f03090b38d551f4d97479c79eb8186732fad89d191ef8727a2c466019cf86df4
real_refresh_fingerprint=f03090b38d551f4d97479c79eb8186732fad89d191ef8727a2c466019cf86df4
real_refresh_applicability=applicable
real_refresh_reason_code=all_basis_applicable
copy_verification_id=01a05b7f-135d-7133-80db-5ee876713e26
copy_baseline_fingerprint=f03090b38d551f4d97479c79eb8186732fad89d191ef8727a2c466019cf86df4
copy_stale_reason_code=resource_drift
copy_unavailable_reason_code=resource_unavailable
copy_empty_reason_code=resource_drift
copy_error_reason_code=resource_error
target_status_unchanged=true
```

## Independent Review

Independent review found that unbounded glob validation had to apply to both
Resource observation and the manual fingerprint/content `--scope-glob`
shorthand path. The implementation now rejects glob selectors without a fixed
non-wildcard root and rejects fixed roots that retain parent-directory
traversal after normalization from both paths. The focused
`cli_scope_glob_requires_fixed_non_parent_root` regression covers `**/*.rs`,
`*.md`, `/**/*.rs`, `../project/**/*.rs`, and an allowed `src/**/*.rs` selector.
Independent re-review confirmed the high is closed and reported no new
blocker/high/medium findings.

## Remaining Open

- Git working-tree observation.
- Automatic Resource re-observation scheduling.
- Broader symlink, case, rename, and deletion policy.
