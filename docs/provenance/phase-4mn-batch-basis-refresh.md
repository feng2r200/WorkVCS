# Phase 4MN Batch Basis Refresh Evidence

Date: 2026-09-01

## Scope

Phase 4MN adds an explicit operator batch mode for Resource-basis
re-observation:

```bash
workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --all-resource-backed \
  --resource-content-from-basis
```

The command refreshes current-head Verifications with non-empty Resource basis
entries. It skips non-resource-backed Verifications and does not create
WorkState commits, Evidence, semantic re-verifications, background jobs,
watchers, daemons, or implicit AC refreshes.

## Implementation Evidence

- `crates/workvcs-cli/src/main.rs` adds `--all-resource-backed` and
  `--expected-refreshed` to `verification cache-refresh`.
- Batch mode supports only `--resource-content-from-basis`.
- Batch mode rejects an explicit `--verification`, because the selected set is
  all current-head Resource-backed Verifications on the Branch.
- Batch mode rejects per-cache expectations such as `--expected-applicability`,
  `--expected-reason-code`, and `--expected-resource-stamps`; it accepts the
  batch-level `--expected-refreshed`.
- The implementation pre-validates every selected Resource basis before writing
  any ResourceObservation or applicability cache.
- The existing single-Verification `verification cache-refresh --verification`
  behavior remains compatible.

## Focused Validation

PASS: `/tmp/workvcs-4mn-focused-validation-rerun2-20260901T102321Z`.

Covered checks:

```text
cargo fmt --all
cargo check -q -p workvcs-cli
cargo test -q -p workvcs-cli cli_batch_refreshes_resource_backed_verifications_from_basis
cargo test -q -p workvcs-cli cli_batch_basis_refresh_rejects_unsupported_basis_before_observation_writes
```

The new focused tests cover:

```text
batch refresh of two Resource-backed Verifications
skip of one non-resource-backed Verification
Branch head unchanged after batch refresh
ResourceObservation count increasing only for refreshed Resource-backed entries
explicit-Verification/batch argument rejection
per-cache expectation rejection in batch mode
unsupported selected basis rejection before observation writes
existing single-Verification Resource-basis refresh compatibility
```

## Related Regression Validation

PASS: `/tmp/workvcs-4mn-related-validation-20260901T102430Z`.

Covered checks:

```text
cargo fmt --all -- --check
cargo test -q -p workvcs-cli cli_lazy_record_and_verify_commands_render_nested_help
cargo test -q -p workvcs-cli cli_refreshes_mixed_resource_basis_from_recorded_scopes
cargo test -q -p workvcs-cli local_file_scope_path_cache_refresh_rejects_mixed_basis_without_observation_side_effect
```

## Dogfood Evidence

PASS: `/tmp/workvcs-4mn-batch-basis-refresh-dogfood-20260901T102546Z`.

The process-level CLI run created a temporary Store with two local-file
Resource-backed Verifications, then refreshed both with one explicit batch
command.

Key observations:

```text
before_observations=2
batch_refreshed_caches=2
batch_refreshed_match=true
batch_evaluated_commit_matches=yes
after_observations=4
cache_list_caches=2
cache_list_match=true
branch_head_unchanged=yes
```

## Remaining Open

- Background Resource re-observation scheduling remains Open.
- Broader symlink/case/rename and Git adapter policies remain Open.
- Batch filtering beyond "all current-head Resource-backed Verifications" is
  outside this slice.
- Multi-Branch or multi-Store refresh remains outside this slice.

## Independent Review

PASS: independent review found no blocker/high/medium issues for the scoped CLI
batch refresh implementation, tests, documentation, and evidence summaries.

## Final Validation

PASS: `/tmp/workvcs-4mn-final-validation-20260901T103005Z`.

Covered checks:

```text
git diff --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```

## Delivery Closeout

Pending.
