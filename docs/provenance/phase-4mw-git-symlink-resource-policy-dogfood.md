# Phase 4MW Git Symlink Resource Policy Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MW advances the Resource registration, observation, applicability, and
drift gate by proving the V1-local Git worktree Resource symlink policy.

The slice is intentionally narrow:

- make the current tracked-symlink and untracked-non-regular behavior
  operator-visible in ResourceObservation summary JSON;
- prove tracked symlinks are represented by raw Git index and diff material;
- prove untracked symlinks map to Resource error behavior as unsupported
  non-regular entries;
- dogfood the behavior against a temporary clone of a pre-existing real local
  project;
- leave the original project unchanged.

Out of scope:

- `git-worktree-manifest-v1` fingerprint profile changes;
- Store schema, Core/History semantics, or command-shape changes;
- symlink dereference, symlink target hashing, or semantic symlink relations;
- automatic scheduler, daemon, watcher, or implicit refresh;
- case-folding, submodule, or sparse-checkout policy;
- remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Implementation Evidence

`crates/workvcs-cli/src/main.rs` now emits these fields in Git
ResourceObservation summary JSON:

```text
tracked_symlink_policy=git_index_and_diff
untracked_non_regular_policy=unsupported_resource_error
```

The fields are summary metadata only. The `git-worktree-manifest-v1`
fingerprint profile remains the same: HEAD object id, raw porcelain status,
raw index listing, raw staged diff, raw unstaged diff, and sorted untracked
regular-file fingerprints.

Current-behavior inspection passed in
`/tmp/workvcs-4mw-symlink-policy-inspection-20260901T144206Z/summary.txt`:

```text
tracked_symlink_git_index_diff=mode_120000
untracked_symlink_listed_by_git=yes
untracked_symlink_regular_file=no
filesystem_case_sensitive=no
phase4mw_symlink_policy_inspection=pass
```

Focused regression coverage adds:

- `git_worktree_snapshot_reports_symlink_policy_and_tracks_staged_symlink`;
- `git_worktree_snapshot_rejects_untracked_symlink_as_non_regular`;
- `git_worktree_applicability_stamp_maps_untracked_symlink_to_error`.

The tests prove a staged tracked symlink appears in Git's raw index and staged
diff as mode `120000`, proves WorkVCS summary metadata exposes the bounded
policy, and proves an untracked symlink returns unsupported-non-regular
Resource error behavior.

Focused validation passed with status table
`/tmp/workvcs-4mw-focused-validation-20260901T144544Z/status.tsv`:

```text
diff-check=0
fmt-check=0
test-symlink-policy=0
test-untracked-symlink=0
test-untracked-symlink-stamp=0
test-rename-policy=0
test-git-refresh=0
code-anchors=0
```

## Dogfood Evidence

Dogfood log:

```text
/tmp/workvcs-4mw-git-symlink-resource-dogfood-20260901T144729Z
```

The source project was the existing local Git repository:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Only the `/tmp` clone was mutated:

```text
external_clone=/tmp/workvcs-4mw-git-symlink-resource-dogfood-20260901T144729Z/agent_soul-clone
external_original_head_before=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_head_after=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_unchanged=yes
```

The clone staged a tracked symlink and Git represented it as mode `120000`:

```text
tracked_symlink_git_index_diff=mode_120000
```

Baseline WorkVCS verification recorded a Git ResourceObservation with the new
summary policy fields:

```text
verification_id=01a05d70-aae9-7e70-8985-375b4e59a5ee
baseline_observation_id=01a05d70-aae8-7e73-9c82-e7149844167f
baseline_fingerprint=7200068ec83cee5a58b8f48d64026d87958a4c43347b0d0e68acb858ba6e92fc
baseline_summary_has_tracked_symlink_policy=yes
baseline_summary_has_untracked_non_regular_policy=yes
```

After adding an untracked symlink, WorkVCS cache refresh projected Resource
error behavior and wrote no new ResourceObservation:

```text
untracked_symlink_listed_by_git=yes
refresh_applicability=unknown
refresh_reason_code=resource_error
cache_applicability_after_untracked_symlink=unknown
cache_reason_after_untracked_symlink=resource_error
cache_observation_status_after_untracked_symlink=error
cache_observation_id_after_untracked_symlink=none
```

Store checks passed:

```text
resource_observations=1
integrity_valid_required=true
doctor_valid_required=true
phase4mw_git_symlink_resource_policy=pass
```

## Readiness Impact

Phase 4MW closes the Git symlink-policy subgap for V1-local Git worktree
Resource observations. The bounded policy is:

```text
tracked_symlink_policy=git_index_and_diff
untracked_non_regular_policy=unsupported_resource_error
```

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking because case-folding policy, submodule or sparse-checkout
Git semantics, and background re-observation scheduling remain unproven.

The overall release decision remains false.

## Final Validation

Full pre-merge validation passed in
`/tmp/workvcs-4mw-final-validation-post-review/summary.txt`:

```text
diff_check=0
fmt_check=0
schema_v0_1=0
clippy=0
cargo_test_workspace=0
smoke_v0_1=0
anchors=0
phase4mw_final_validation=pass
```

Corrected focused document/code validation passed in
`/tmp/workvcs-4mw-doc-code-validation-corrected-20260901T151500Z/status.tsv`.
That corrected run replaced an earlier validation attempt whose
`test-git-refresh` filter matched zero tests. The corrected run uses
`cli_refreshes_git_worktree_scope_applicability_cache` and verifies that every
focused Rust test output executed at least one test:

```text
diff-check=0
fmt-check=0
test-symlink-policy=0
test-untracked-symlink=0
test-untracked-symlink-stamp=0
test-rename-policy=0
test-git-refresh=0
anchors=0
zero-test-check:test-symlink-policy.out=0
zero-test-check:test-untracked-symlink.out=0
zero-test-check:test-untracked-symlink-stamp.out=0
zero-test-check:test-rename-policy.out=0
zero-test-check:test-git-refresh.out=0
```

Independent read-only review checked the current diff, the Plan, this
provenance record, the readiness ledger, the release gate matrix, the focused
validation directory, the dogfood summary, and the document/code validation
summary. It found no blocker or high findings.

The review found two medium issues:

- the first document/code validation attempt recorded a `test-git-refresh`
  command that exited successfully but executed zero tests because its filter
  did not match a real test name;
- this Final Validation section was still pending after validation activity had
  begun.

Both issues are resolved here by replacing the zero-test validation evidence
with the corrected validation run above and by replacing the pending Final
Validation text with the final validation and review result. The review also
noted that the formal WorkVCS `plan independent-review record` path was not
used because this lightweight Plan has no `independent_validation` object or
trusted attestation structure. The review is therefore treated as read-only
subagent evidence, not as a formal attested review record.

The final validation and review did not authorize a release candidate, tag,
Push, deployment, V2 feature, symlink dereference, symlink target hashing,
case-folding policy, submodule or sparse-checkout policy, automatic scheduling,
remote/distributed behavior, cross-Store synchronization, or direct mutation of
the original `agent_soul` repository.
