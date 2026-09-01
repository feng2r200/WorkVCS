# ADR-0445: Phase 4MD Local File Path Prefix Refresh

Status: Accepted
Date: 2026-09-01

## Context

ADR-0444 added the first executable local-file Resource re-observation contract
for exact path Resource basis entries. The V1 readiness ledger still listed
path-prefix aggregation as Open. Existing `verify --scope-path-prefix` could
store a path-prefix selector, but there was no CLI path that produced or
refreshed a deterministic path-prefix ResourceObservation fingerprint.

## Decision

Add two explicit CLI flags:

```text
workvcs verify --resource-content-from-scope-path-prefix
workvcs verification cache-refresh --resource-content-from-scope-path-prefix
```

Both flags support only Resource basis entries with:

```text
adapter_kind=local-file
scope_kind=path
scope_schema_version=1
scope_payload={"path_prefix":"..."}
```

The path-prefix fingerprint is the content digest of a canonical manifest with
profile `local-file-path-prefix-manifest-v1`. The manifest contains sorted
relative file paths, each file's content fingerprint, and each file's byte size.
The prefix path itself is not part of the fingerprint; it remains in the
Resource basis scope and ResourceObservation summary.

When refreshing:

```text
unchanged files     -> new ResourceObservation + applicable/all_basis_applicable
changed files       -> stale/resource_drift
missing prefix      -> unknown/resource_unavailable
non-directory/error -> unknown/resource_error
```

The command validates every Resource basis entry against the selected local-file
scope contract before recording any new ResourceObservation.

## Non-Goals

- No Git working-tree adapter.
- No glob matching.
- No automatic scheduler or daemon.
- No broad symlink, case-folding, rename, or deletion semantics. Symlinks and
  special files encountered by this narrow contract are unsupported observation
  errors.
- No Core/History filesystem interpretation.

## Evidence

Focused validation:

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

Real external-project dogfood:

```text
phase4md_dogfood_result=PASS
log_dir=/tmp/workvcs-4md-path-prefix-dogfood-20260901T045657Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_prefix=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/skills/use-financial-mcp-tools/references/financial-report/tools
target_prefix_files=4
real_baseline_fingerprint=592e0de5796714dd01a7d62babbe133f920f3b48b5b9d0f6b07848cbc738ca96
real_refresh_fingerprint=592e0de5796714dd01a7d62babbe133f920f3b48b5b9d0f6b07848cbc738ca96
real_refresh_applicability=applicable
real_refresh_reason_code=all_basis_applicable
real_ac_status=verified
copy_stale_reason_code=resource_drift
copy_unavailable_reason_code=resource_unavailable
copy_error_reason_code=resource_error
resource_observations=4
target_status_unchanged=true
```

Independent review reported no blocker/high/medium findings for the 4MD diff,
including shared baseline/refresh fingerprinting, no partial observation writes
for unsupported mixed basis entries, default refresh preservation, exact path
preservation, and documentation boundaries.

## Consequences

WorkVCS now has two executable local-file Resource re-observation contracts:
exact path and path-prefix aggregation. This narrows the Resource readiness gap
while keeping Store/History authority unchanged.

Git working-tree observation, glob matching, automatic re-observation policy,
and broader filesystem normalization semantics remain Open.
