# Phase 4MG Resource Basis Cache Refresh Evidence

Date: 2026-09-01

## Scope

Phase 4MG adds an explicit basis-aware Resource refresh mode:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-basis
```

The command inspects the selected Verification's persisted Resource basis entries
and dispatches each supported entry to the already implemented local-file or Git
worktree refresh path.

Supported contracts:

```text
local-file path
local-file path_prefix
local-file glob
git git-worktree
```

All supported contracts require `adapter_schema_version=1` and
`scope_schema_version=1`. Unsupported or malformed basis entries fail during
pre-validation before any ResourceObservation is written.

## Implementation Evidence

- `crates/workvcs-cli/src/main.rs` adds
  `--resource-content-from-basis` to `verification cache-refresh`.
- The implementation pre-validates every Resource basis entry before observing
  any Resource.
- Existing exact path, path-prefix, glob, and Git worktree flags remain
  supported and mutually exclusive with the new flag.

## Focused Validation

PASS: `/tmp/workvcs-4mg-focused-validation-rerun-20260901T065653Z`.

Covered checks:

```text
cargo fmt --all
cargo test -q -p workvcs-cli --no-run
cli_lazy_record_and_verify_commands_render_nested_help
cli_refreshes_mixed_resource_basis_from_recorded_scopes
local_file_scope_path_cache_refresh_rejects_mixed_basis_without_observation_side_effect
cli_refreshes_git_worktree_scope_applicability_cache
```

PASS: `/tmp/workvcs-4mg-medium-fix-validation-20260901T070956Z`.

Covered the independent-review medium fix:

```text
cargo fmt --all -- --check
cli_refreshes_local_file_scope_path_prefix_applicability_cache
cli_refreshes_local_file_scope_glob_applicability_cache
cli_refreshes_mixed_resource_basis_from_recorded_scopes
```

## Dogfood Evidence

PASS: `/tmp/workvcs-4mg-basis-refresh-dogfood-20260901T065738Z`.

Target project:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Key observations:

```text
target_status_unchanged=true
before_entries=39
after_entries=39
before_hash=25db244655994ac854f760c60e5d7adb943fdd2324a21df698eb6188a8521198
after_hash=25db244655994ac854f760c60e5d7adb943fdd2324a21df698eb6188a8521198
verification_id=01a05bc2-8b84-7f93-8da2-55958c913781
baseline_fingerprint=1cc1b5a0b8a0a3316b8c738cfe6d5654f988885eca569c2fec69b6f0062ea237
basis_refresh_applicability=applicable
reason_code=all_basis_applicable
cache_observed_fingerprint=1cc1b5a0b8a0a3316b8c738cfe6d5654f988885eca569c2fec69b6f0062ea237
```

This proves the new basis-aware flag against a real dirty external Git worktree
without mutating the target project.

## Final Validation

PASS: `/tmp/workvcs-4mg-final-validation-rerun2-20260901T071526Z`.

Covered checks:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```

Superseded invalid matrix attempt:
`/tmp/workvcs-4mg-final-validation-rerun-20260901T071228Z` used obsolete
schema and smoke command names, so it is not completion evidence.

## Independent Review

The independent review first found one medium issue: persisted path-prefix and
glob basis entries were supported by the implementation but lacked direct
`--resource-content-from-basis` success and drift coverage. The tests now cover
both persisted basis forms through the basis-aware flag, and re-review found no
remaining blocker/high/medium findings.

## Remaining Open

- Background Resource re-observation scheduling remains Open.
- Broader symlink/case/rename and Git adapter policies remain Open.
- Multi-Verification batch refresh remains Open.
