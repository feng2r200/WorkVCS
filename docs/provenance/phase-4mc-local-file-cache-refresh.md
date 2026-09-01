# Phase 4MC Local File Cache Refresh Evidence

Date: 2026-09-01

## Scope

Phase 4MC adds an explicit local-file exact path re-observation mode to
`verification cache-refresh`:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-path
```

The flag supports only Resource basis entries with:

```text
adapter_kind=local-file
scope_kind=path
scope_schema_version=1
scope_payload={"path":"..."}
```

The default `verification cache-refresh` path remains baseline-observation
based and unchanged.

## Implementation Evidence

The CLI now:

```text
loads the branch head
checks --expected-evaluated-commit when supplied
loads the Verification at that head
validates all Resource basis entries before recording observations
reads each local-file exact path Resource basis
records a new ResourceObservation when the file is readable
records unavailable/error applicability stamps when the file cannot be observed
records the resulting applicability cache through the existing Engine facade
```

Core Store/History semantics remain unchanged.

## Focused Validation

```text
cargo_fmt_check=PASS
focused_cli_test=PASS
log_dir=/tmp/workvcs-4mc-focused-validation-rerun-20260901T042332Z
default_refresh_regression=PASS
default_refresh_log_dir=/tmp/workvcs-4mc-default-refresh-regression-20260901T042915Z
atomicity_fix_focused_validation=PASS
atomicity_fix_log_dir=/tmp/workvcs-4mc-atomicity-fix-focused-20260901T043617Z
clippy_fix_focused_validation=PASS
clippy_fix_log_dir=/tmp/workvcs-4mc-clippy-fix-focused-20260901T043915Z
final_validation=PASS
final_validation_log_dir=/tmp/workvcs-4mc-final-validation-rerun-20260901T043936Z
```

The focused test covers:

```text
unchanged file -> applicable, all_basis_applicable, new observation id
changed file -> stale, resource_drift
missing file -> unknown, resource_unavailable
directory read -> unknown, resource_error
mixed supported/unsupported basis -> task_invalid, no partial observation write
```

Independent review initially found a medium partial-observation side effect for
mixed Resource basis entries. The implementation now validates all basis
entries before any observation write, and the follow-up independent review
reported no blocker/high/medium findings.

## Dogfood Evidence

The read-only external file used as the Resource baseline was:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
```

The passing run:

```text
phase4mc_dogfood_result=PASS
log_dir=/tmp/workvcs-4mc-local-file-cache-refresh-dogfood-20260901T042506Z
store=/tmp/workvcs-4mc-local-file-cache-refresh-dogfood-20260901T042506Z/store.sqlite
store_id=01a05b36-e20e-76f3-825c-eeaa5dadbba1
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_file=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
workspace_id=01a05b36-e248-74a3-92d6-3c9dc30c935e
branch_id=01a05b36-e248-74a3-92d6-3cc5d630fa84
head_commit_id=01a05b36-e2eb-70c1-b95f-b1cba289e5af
goal_id=01a05b36-e25d-7e13-84be-2c20a55e75dc
plan_id=01a05b36-e271-7fe0-b025-d5b25ed0e95f
task_id=01a05b36-e285-71a3-99b4-f549295a5ff7
criterion_id=01a05b36-e2ae-70e3-9ce6-f1eeb76e3a5a
verification_id=01a05b36-e2eb-70c1-b95f-b17852cbd957
session_id=01a05b36-e2c2-7f13-9189-550e2d53de6c
resource_id=01a05b36-e2d4-7bb3-8402-e0b91c5d8c5d
baseline_observation_id=01a05b36-e2e9-7a43-aa3f-2de1da23389a
refresh_observation_id=01a05b36-e316-7dc1-bcb0-7579dda71114
baseline_fingerprint=a0ae4eccafb9eca86e82a5b6d9e6d23ba8891cdc6b0db3c01f33a8a9ccd3c4e1
refresh_fingerprint=a0ae4eccafb9eca86e82a5b6d9e6d23ba8891cdc6b0db3c01f33a8a9ccd3c4e1
refresh_applicability=applicable
refresh_reason_code=all_basis_applicable
resource_observations=2
ac_status=verified
target_status_unchanged=true
```

The refresh observation id differs from the baseline observation id while the
fingerprint matches, proving a new successful observation of unchanged content.

## Boundary

This evidence supports the exact local-file scope-path adapter contract only.
It does not claim glob semantics, path-prefix aggregation, Git working-tree
observation, symlink/case/rename policy, automatic scheduling, or release
maturity.
