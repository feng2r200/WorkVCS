# Phase 4MY Git Sparse Checkout Resource Policy Dogfood Evidence

Date: 2026-09-02

## Scope

Phase 4MY advances the Resource registration, observation, applicability, and
drift gate by proving the V1-local Git worktree Resource sparse-checkout
policy.

The slice is intentionally narrow:

- make the current parent-Git sparse-checkout behavior operator-visible in
  ResourceObservation summary JSON;
- prove sparse-excluded tracked files remain represented through parent Git
  index entries;
- prove WorkVCS does not expand, materialize, or hash sparse-excluded tracked
  files from the working tree;
- prove parent-visible status and diff material still project Resource drift;
- dogfood the behavior against a temporary sparse-checkout clone of a
  pre-existing real local project;
- leave the original project unchanged.

Out of scope:

- `git-worktree-manifest-v1` fingerprint profile changes;
- Store schema, Core/History semantics, or command-shape changes;
- sparse-checkout expansion or materialization by WorkVCS;
- recursive hashing of sparse-excluded tracked file contents from the working
  tree;
- semantic sparse-checkout relations;
- automatic scheduler, daemon, watcher, or implicit refresh;
- case-folding policy;
- remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Implementation Evidence

`crates/workvcs-cli/src/main.rs` now emits these fields in Git
ResourceObservation summary JSON:

```text
sparse_checkout_policy=parent_index_status_diff
sparse_checkout_expansion=disabled
```

The fields are summary metadata only. The `git-worktree-manifest-v1`
fingerprint profile remains the same: HEAD object id, raw porcelain status,
raw index listing, raw staged diff, raw unstaged diff, and sorted untracked
regular-file fingerprints.

Current-behavior inspection passed in
`/tmp/workvcs-4my-sparse-checkout-policy-inspection-20260901T155400Z/summary.txt`:

```text
head_unchanged_by_sparse_checkout=yes
core_sparse_checkout=true
sparse_checkout_list=keep
omitted_exists_after_sparse=no
clean_status_bytes=0
clean_untracked_bytes=0
index_contains_included=yes
index_contains_excluded=yes
ls_files_t_marks_excluded_sparse=yes
status_after_kept_update= M keep/included.txt|
status_after_excluded_untracked=?? omit/new-untracked.txt|
untracked_after_excluded_untracked=omit/new-untracked.txt|
phase4my_sparse_checkout_policy_inspection=pass
```

Focused regression coverage adds:

- `git_worktree_snapshot_reports_sparse_checkout_policy_from_parent_git_view`.

The test proves a clean sparse checkout has no parent status or untracked bytes,
proves a sparse-excluded tracked file is not materialized but remains present
in parent Git index material, proves `git ls-files -t` marks that path with
`S`, proves WorkVCS summary metadata exposes the bounded policy, proves an
included-file modification changes the WorkVCS Git worktree fingerprint, and
proves a parent-visible untracked regular file under a sparse-excluded
directory follows the existing untracked-file policy.

The first focused validation attempt at
`/tmp/workvcs-4my-focused-validation-initial-20260901T155624Z/status.tsv`
found a formatting-only failure:

```text
fmt-check=1
test-sparse-policy=0
test-submodule-policy=0
test-symlink-policy=0
test-rename-policy=0
test-git-refresh=0
```

After `cargo fmt --all`, corrected focused validation passed with status table
`/tmp/workvcs-4my-focused-validation-20260901T155706Z/status.tsv`:

```text
diff-check=0
fmt-check=0
test-sparse-policy=0
test-submodule-policy=0
test-symlink-policy=0
test-rename-policy=0
test-git-refresh=0
zero-test-check:test-sparse-policy.out=0
zero-test-check:test-submodule-policy.out=0
zero-test-check:test-symlink-policy.out=0
zero-test-check:test-rename-policy.out=0
zero-test-check:test-git-refresh.out=0
code-anchors=0
```

## Dogfood Evidence

Dogfood log:

```text
/tmp/workvcs-4my-git-sparse-checkout-resource-dogfood-20260901T160432Z
```

The source project was the existing local Git repository:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Only the `/tmp` sparse clone was mutated:

```text
external_clone=/tmp/workvcs-4my-git-sparse-checkout-resource-dogfood-20260901T160432Z/agent_soul-sparse-clone
external_original_head_before=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_head_after=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_clean_after=yes
external_original_unchanged=yes
```

The clone enabled sparse checkout for `src` without moving HEAD. A tracked
file in `src` remained materialized, while a tracked file outside the sparse
checkout was absent from the working tree but still present in the parent Git
index and marked sparse by Git:

```text
external_clone_head_unchanged_by_sparse_checkout=yes
clone_core_sparse_checkout=true
clone_sparse_checkout_list=src
tracked_included=src/agent_soul/__init__.py
tracked_excluded=docs/design/skill-contract-v0.3.md
included_exists_after_sparse=yes
excluded_exists_after_sparse=no
parent_index_contains_excluded=yes
parent_flags_mark_excluded_sparse=yes
parent_status_clean_lines=0
parent_untracked_clean_lines=0
```

Baseline WorkVCS verification recorded a Git ResourceObservation with the new
summary policy fields:

```text
verification_id=01a05db7-1650-7272-97fc-117fd10371eb
baseline_observation_id=01a05db7-164f-76b2-bdff-4aded8de834f
baseline_fingerprint=ce4959eb3aaea1ea432a1b06e91c8f0b1c4b982cb4f97747e2809ec09b12889c
baseline_summary_has_sparse_policy=yes
baseline_summary_has_sparse_expansion=yes
baseline_summary_untracked_files=0
```

After modifying the materialized included file, parent Git reported the
working-tree change and WorkVCS cache refresh projected Resource drift:

```text
parent_status_marks_included_dirty=yes
parent_unstaged_diff_after_included_update_bytes=339
refresh_applicability=stale
refresh_reason_code=resource_drift
refresh_observation_id=01a05db7-16e4-7210-8612-e9ea12de5fb7
refresh_fingerprint=db53b28d32a0494221d82aa5a689bb574ca7b270d0a911e220a2f89ebcd5887f
dirty_refresh_fingerprint_changed=yes
cache_applicability_after_dirty=stale
cache_reason_after_dirty=resource_drift
cache_observation_status_after_dirty=observed
dirty_summary_has_sparse_policy=yes
dirty_summary_has_sparse_expansion=yes
dirty_summary_status_size_bytes=30
dirty_summary_unstaged_diff_size_bytes=339
```

Store checks passed:

```text
resource_observations=2
integrity_valid_required=yes
doctor_valid_required=yes
phase4my_git_sparse_checkout_resource_policy=pass
```

The first dogfood attempt at
`/tmp/workvcs-4my-git-sparse-checkout-resource-dogfood-20260901T160241Z`
proved the Git sparse-checkout filesystem/index facts but omitted the required
`--adapter-kind git` argument on the WorkVCS `verify` command. The corrected
run added `--adapter-kind git --adapter-schema-version 1` and stopped on CLI
errors. This was a dogfood harness correction, not a WorkVCS behavior
correction.

## Readiness Impact

Phase 4MY closes the Git sparse-checkout policy subgap for V1-local Git
worktree Resource observations. The bounded policy is:

```text
sparse_checkout_policy=parent_index_status_diff
sparse_checkout_expansion=disabled
```

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking because case-folding policy and background
re-observation scheduling remain unproven.

The overall release decision remains false.

## Final Validation

Independent review agent `01a05dbd-94ce-7871-9732-caccb9163e95` found no
unresolved blocker/high issues. Its only medium finding was that the final
post-review full validation matrix and this Final Validation section were not
yet complete.

Candidate full validation before this final provenance closeout passed in
`/tmp/workvcs-4my-full-validation-candidate-20260901T161529Z/status.tsv`:

```text
diff-check=0
fmt-check=0
schema-v0-1=0
clippy=0
cargo-test-workspace=0
smoke-v0-1=0
anchors=0
```

Final post-review validation is recorded in
`/tmp/workvcs-4my-final-validation-post-review-20260901T162357Z/summary.txt`
and covers the final Phase 4MY diff after this provenance section was updated:

```text
diff-check=0
fmt-check=0
schema-v0-1=0
clippy=0
cargo-test-workspace=0
smoke-v0-1=0
anchors=0
phase4my_final_validation=pass
```
