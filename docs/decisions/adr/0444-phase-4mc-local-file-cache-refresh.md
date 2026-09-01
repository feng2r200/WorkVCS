# ADR-0444: Phase 4MC Local File Cache Refresh

Status: Accepted
Date: 2026-09-01

## Context

Before this slice, `verification cache-refresh` refreshed applicability by
reconstructing observed stamps from the Verification's baseline Resource
observations. That preserved branch-head recovery after semantic WorkState
changes, but it did not actually re-observe the external Resource. The V1
readiness ledger therefore still had a Resource adapter-backed re-observation
gap.

Phase 4MB proved explicit unavailable/error applicability stamps. The next
small vertical slice was to make one real adapter contract executable without
changing Store/History authority or defining broad filesystem, Git, glob, or
scheduler behavior.

## Decision

Add an explicit CLI flag:

```text
workvcs verification cache-refresh --resource-content-from-scope-path
```

When present, the CLI refresh path loads the current branch head, reads the
Verification's Resource basis, and supports only this contract:

```text
adapter_kind=local-file
scope_kind=path
scope_schema_version=1
scope_payload={"path":"..."}
```

The command validates every Resource basis entry against that contract before
recording any new ResourceObservation.

For each supported basis entry:

```text
readable file     -> new ResourceObservation + observed stamp
changed content   -> applicability=stale, reason_code=resource_drift
missing file      -> unavailable stamp, applicability=unknown, reason_code=resource_unavailable
other read error  -> error stamp, applicability=unknown, reason_code=resource_error
```

The default `verification cache-refresh` behavior is unchanged when the flag is
absent.

## Non-Goals

- No glob or path-prefix aggregation.
- No Git working-tree adapter.
- No symlink, case-folding, rename, or deletion policy beyond normal local file
  read success versus failure.
- No automatic scheduler or daemon.
- No JSON error-output mode.
- No Core/History direct filesystem interpretation.

## Evidence

Focused validation:

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

The focused CLI test proves:

```text
unchanged file -> applicable, all_basis_applicable, new observation id
changed file -> stale, resource_drift
missing file -> unknown, resource_unavailable
directory read -> unknown, resource_error
mixed supported/unsupported basis -> task_invalid, no partial observation write
```

Independent review initially found a medium partial-observation side effect for
mixed Resource basis entries. A follow-up review after the fix reported no
blocker/high/medium findings.

Real external-project dogfood:

```text
phase4mc_dogfood_result=PASS
log_dir=/tmp/workvcs-4mc-local-file-cache-refresh-dogfood-20260901T042506Z
store=/tmp/workvcs-4mc-local-file-cache-refresh-dogfood-20260901T042506Z/store.sqlite
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
target_file=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md
verification_id=01a05b36-e2eb-70c1-b95f-b17852cbd957
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

The refresh observation id differs from the baseline observation id, proving
that this path performed a new observation rather than replaying the baseline
observation.

## Consequences

WorkVCS now has one executable Resource adapter-backed re-observation contract:
exact local-file scope-path refresh through an explicit CLI flag. This narrows
the Resource readiness gap and makes `verification cache-refresh` usable for a
real local-file dogfood loop.

Broader adapter contracts remain Open. Future Resource slices should address
glob/path-prefix behavior, Git working-tree observation, or automatic
re-observation only when a concrete dogfood workflow needs them.
