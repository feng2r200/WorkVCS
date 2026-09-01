# Phase 4MF Git Worktree Resource Refresh Evidence

Date: 2026-09-01

## Scope

Phase 4MF adds explicit Git worktree Resource observation and re-observation
modes:

```bash
workvcs verify "$STORE" \
  --scope-git-worktree "$REPO" \
  --resource-content-from-scope-git-worktree

workvcs verification cache-refresh "$STORE" \
  --branch "$BRANCH_ID" \
  --verification "$VERIFICATION_ID" \
  --resource-content-from-scope-git-worktree
```

The flags support only:

```text
adapter_kind=git
adapter_schema_version=1
scope_kind=git-worktree
scope_schema_version=1
scope_payload={"repo":"..."}
```

Core Store/History semantics remain unchanged. Git mutation, semantic merge,
automatic scheduling, and broader adapter policy remain outside this slice.

## Implementation Evidence

- `crates/workvcs-cli/src/main.rs` adds:
  - `verify --scope-git-worktree PATH`;
  - `verify --resource-content-from-scope-git-worktree`;
  - `verification cache-refresh --resource-content-from-scope-git-worktree`.
- Git worktree snapshots use `git-worktree-manifest-v1` over HEAD, raw
  porcelain status, raw index listing, raw staged diff, raw unstaged diff, and
  sorted untracked regular-file fingerprints.
- The implementation invokes `git` with `GIT_OPTIONAL_LOCKS=0` and does not
  run checkout, add, commit, reset, write-tree, or merge.

## Focused Validation

- PASS: `/tmp/workvcs-4mf-compile-after-compaction-20260901T060334Z`.
- PASS: `/tmp/workvcs-4mf-focused-validation-20260901T060637Z`.
- PASS: `/tmp/workvcs-4mf-medium-fix-validation-rerun3-20260901T063909Z`.
- Covered checks:
  - `cargo fmt --all`;
  - `cargo check -q -p workvcs-cli`;
  - `cli_lazy_record_and_verify_commands_render_nested_help`;
  - `cli_verify_content_errors_use_verify_flag_labels`;
  - `git_worktree_snapshot_tracks_uncommitted_states`;
  - `git_worktree_snapshot_is_stable_under_diff_config`;
  - `cli_git_worktree_scope_rejects_contract_mismatch`;
  - `git_worktree_refresh_rejects_extra_scope_payload_fields`;
  - `cli_refreshes_git_worktree_scope_applicability_cache`;
  - adjacent local-file glob and path-prefix refresh regressions.

Final matrix PASS:

```text
log_dir=/tmp/workvcs-4mf-final-validation-rerun2-20260901T064009Z
diff_check=PASS
fmt_check=PASS
schema=PASS
clippy=PASS
test=PASS
cli_smoke=PASS
```

## Dogfood Evidence

PASS: `/tmp/workvcs-4mf-git-dogfood-20260901T060805Z`.

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
verification_id=01a05b95-329a-7680-a1a6-a46a52028c00
baseline_fingerprint=b1adfc0b8c633288485c4f421f42d75a41f1b8a47de0e97085b931756e5b812e
refresh_applicability=applicable
reason_code=all_basis_applicable
cache_observed_fingerprint=b1adfc0b8c633288485c4f421f42d75a41f1b8a47de0e97085b931756e5b812e
```

The target project already had local status entries, so this proves unchanged
refresh over a real dirty worktree state, not only a clean HEAD-only repository.

## Independent Review

- Initial review by independent agent Laplace found no blocker/high and four
  medium issues: Git basis mismatch acceptance, extra Git scope payload fields,
  diff determinism under local Git config, and empty initial `verify` summary.
- The implementation now rejects Git adapter/scope/schema mismatches, rejects
  extra Git scope payload fields, fixes Git diff command/config inputs, records
  scope-derived summary on initial `verify`, and covers each case in focused
  tests.
- Final re-review confirmed all four medium findings closed and found no
  remaining blocker/high/medium.

## Remaining Open

- Automatic Resource re-observation scheduling.
- Broader Git adapter semantics such as submodules, sparse checkouts, rename,
  case, and symlink policy.
