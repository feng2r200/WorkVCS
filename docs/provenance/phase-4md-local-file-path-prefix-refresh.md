# Phase 4MD Local File Path Prefix Refresh Evidence

Date: 2026-09-01

## Scope

Phase 4MD adds explicit local-file path-prefix Resource observation and
re-observation modes:

```bash
workvcs verify "$STORE" \
  --scope-path-prefix "$PATH_PREFIX" \
  --resource-content-from-scope-path-prefix

workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-path-prefix
```

The flags support only:

```text
adapter_kind=local-file
scope_kind=path
scope_schema_version=1
scope_payload={"path_prefix":"..."}
```

The default `verification cache-refresh` path remains baseline-observation
based and unchanged. The Phase 4MC exact path mode remains unchanged.

## Implementation Evidence

The CLI now:

```text
computes a deterministic local-file path-prefix manifest for verify baselines
validates all Resource basis entries before recording refresh observations
recursively traverses regular files under the prefix
sorts relative paths before fingerprinting
records a new ResourceObservation when the prefix is readable
records unavailable/error applicability stamps when the prefix cannot be observed
records the resulting applicability cache through the existing Engine facade
```

The path-prefix manifest profile is:

```text
local-file-path-prefix-manifest-v1
```

Its entries contain:

```text
relative file path
file content fingerprint
file size in bytes
```

Core Store/History semantics remain unchanged.

## Focused Validation

```text
cargo_fmt_check=PASS
cargo_check=PASS
help=PASS
content_errors=PASS
focused_prefix=PASS
focused_path=PASS
log_dir=/tmp/workvcs-4md-focused-validation-rerun-20260901T045452Z
post_doc_validation=PASS
post_doc_validation_log_dir=/tmp/workvcs-4md-post-doc-validation-20260901T045941Z
final_validation=PASS
final_validation_log_dir=/tmp/workvcs-4md-final-validation-20260901T050307Z
```

The focused path-prefix test covers:

```text
verify --scope-path-prefix --resource-content-from-scope-path-prefix
unchanged prefix -> applicable, all_basis_applicable, new observation id
changed nested file -> stale, resource_drift
missing prefix -> unknown, resource_unavailable
file at prefix path -> unknown, resource_error
stored scope payload -> {"path_prefix":"..."}
```

## Dogfood Evidence

The read-only external path-prefix used as the real Resource baseline was:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/skills/use-financial-mcp-tools/references/financial-report/tools
```

The passing run:

```text
phase4md_dogfood_result=PASS
log_dir=/tmp/workvcs-4md-path-prefix-dogfood-20260901T045657Z
store=/tmp/workvcs-4md-path-prefix-dogfood-20260901T045657Z/store.sqlite
workspace_id=01a05b53-e36a-7063-b361-cc370386f2d5
branch_id=01a05b53-e36a-7063-b361-cc6a6e5430e4
head_commit_id=01a05b53-e468-7613-a0b8-0fb7f3920913
resource_id=01a05b53-e3b0-7ec0-9098-27d57e1f0ac0
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_prefix=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/skills/use-financial-mcp-tools/references/financial-report/tools
target_prefix_files=4
real_verification_id=01a05b53-e3c6-7611-bae7-325c7c4bec45
real_baseline_observation_id=01a05b53-e3c4-78b1-94e7-e62a8b1f1f4e
real_refresh_observation_id=01a05b53-e3ef-7c02-92f3-9e4cee7b53d3
real_baseline_fingerprint=592e0de5796714dd01a7d62babbe133f920f3b48b5b9d0f6b07848cbc738ca96
real_refresh_fingerprint=592e0de5796714dd01a7d62babbe133f920f3b48b5b9d0f6b07848cbc738ca96
real_refresh_applicability=applicable
real_refresh_reason_code=all_basis_applicable
real_ac_status=verified
copy_verification_id=01a05b53-e468-7613-a0b8-0f6e1bcd8ced
copy_baseline_fingerprint=592e0de5796714dd01a7d62babbe133f920f3b48b5b9d0f6b07848cbc738ca96
copy_stale_applicability=stale
copy_stale_reason_code=resource_drift
copy_stale_ac_status=stale
copy_unavailable_applicability=unknown
copy_unavailable_reason_code=resource_unavailable
copy_error_applicability=unknown
copy_error_reason_code=resource_error
resource_observations=4
target_status_unchanged=true
```

The real refresh observation id differs from the baseline observation id while
the fingerprint matches, proving a new successful observation of unchanged
path-prefix content. The controlled copy proves drift, unavailable, and error
projection without mutating the target project.

Independent review reported no blocker/high/medium findings for the 4MD diff,
including shared baseline/refresh fingerprinting, no partial observation writes
for unsupported mixed basis entries, default refresh preservation, exact path
preservation, and documentation boundaries.

## Boundary

This evidence supports the local-file path-prefix aggregation contract only. It
does not claim Git working-tree observation, glob matching, automatic
re-observation scheduling, broader symlink/case/rename semantics, or release
maturity.
