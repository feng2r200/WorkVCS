# Phase 4MV Git Rename Resource Policy Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MV advances the Resource registration, observation, applicability, and
drift gate by proving the V1-local Git worktree Resource rename policy.

The slice is intentionally narrow:

- make the existing Git snapshot no-rename behavior operator-visible in
  ResourceObservation summary JSON;
- prove a staged Git rename is treated as deterministic delete/add drift;
- dogfood the behavior against a temporary clone of a pre-existing real local
  project;
- leave the original project unchanged.

Out of scope:

- `git-worktree-manifest-v1` fingerprint profile changes;
- Store schema, Core/History semantics, or command-shape changes;
- semantic rename tracking, path lineage, or custom rename resolution;
- automatic scheduler, daemon, watcher, or implicit refresh;
- submodule, sparse checkout, symlink, or case-folding policy;
- remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Implementation Evidence

`crates/workvcs-cli/src/main.rs` now emits these fields in Git
ResourceObservation summary JSON:

```text
rename_detection=disabled
rename_policy=delete_add
```

The fields are summary metadata only. The `git-worktree-manifest-v1`
fingerprint profile remains the same: HEAD object id, raw porcelain status,
raw index listing, raw staged diff, raw unstaged diff, and sorted untracked
regular-file fingerprints.

Focused regression coverage adds
`git_worktree_snapshot_reports_rename_policy_as_delete_add`. The test:

- creates a Git repo with one tracked file;
- enables `diff.renames=true` in repo config;
- stages `git mv original.txt renamed.txt`;
- confirms `git diff --cached --find-renames` reports rename from/to;
- confirms WorkVCS' no-renames diff shape reports delete/add markers;
- confirms the Git worktree Resource fingerprint changes;
- confirms the summary JSON exposes the rename policy.

Focused validation passed with status table
`/tmp/workvcs-4mv-focused-validation-20260901T140207Z/status.tsv`:

```text
diff-check=0
fmt-check=0
test-rename-policy=0
test-diff-config=0
test-git-refresh=0
code-anchors=0
```

## Dogfood Evidence

Dogfood log:

```text
/tmp/workvcs-4mv-git-rename-resource-dogfood-20260901T140544Z
```

The source project was the existing local Git repository:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Only the `/tmp` clone was mutated:

```text
external_clone=/tmp/workvcs-4mv-git-rename-resource-dogfood-20260901T140544Z/agent_soul-clone
external_original_head_before=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_head_after=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_unchanged=yes
```

The clone staged a real Git rename:

```text
git mv README.md README.phase4mv.md
```

Independent Git output proved the contrast between rename detection and the
WorkVCS no-renames policy:

```text
git_find_renames_reports_rename=yes
git_no_renames_reports_delete_add=yes
```

Baseline WorkVCS verification recorded a Git ResourceObservation with the new
summary policy fields:

```text
baseline_observation_id=01a05d4a-4b6a-7d52-9c41-896f35cfc6e6
baseline_fingerprint=ce4959eb3aaea1ea432a1b06e91c8f0b1c4b982cb4f97747e2809ec09b12889c
baseline_summary_has_rename_detection=yes
baseline_summary_has_rename_policy=yes
```

After the staged rename, WorkVCS cache refresh observed drift:

```text
refresh_applicability=stale
refresh_reason_code=resource_drift
refresh_observation_status=observed
refresh_observation_id=01a05d4a-4c38-7ed2-acd4-2acbe4b6915c
refresh_fingerprint=81d19c115d3acb28f9cb3eb0eac7875cd2d200eb0a8062320723776c0a4b5ac7
rename_refresh_fingerprint_changed=yes
refresh_summary_has_rename_detection=yes
refresh_summary_has_rename_policy=yes
refresh_summary_staged_diff_size=3209
cache_applicability_after_rename=stale
cache_reason_after_rename=resource_drift
ac_status_after_rename=stale
```

Store checks passed:

```text
resource_observations=2
integrity_valid_required=true
doctor_valid_required=true
phase4mv_git_rename_resource_policy=pass
```

## Readiness Impact

Phase 4MV closes the Git rename-policy subgap for V1-local Git worktree
Resource observations. Rename behavior is now explicit, tested, and dogfooded
against a clone of a pre-existing real project. The bounded policy is:

```text
rename_detection=disabled
rename_policy=delete_add
```

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking because symlink/case policy, submodule or sparse-checkout
Git semantics, and background re-observation scheduling remain unproven.

The overall release decision remains false.

## Final Validation

Full pre-merge validation passed in
`/tmp/workvcs-4mv-final-validation-post-review/summary.txt`:

```text
diff_check=0
fmt_check=0
schema_v0_1=0
clippy=0
cargo_test_workspace=0
smoke_v0_1=0
anchors=0
phase4mv_final_validation=pass
```

Independent read-only review checked the current diff, the Plan, this
provenance record, the readiness ledger, the release gate matrix, the focused
validation directory, the dogfood summary, and the full validation summary. It
found no blocker or high findings.

The review found two medium issues:

- the focused validation evidence was incorrectly identified to the reviewer as
  `summary.txt`, but the actual evidence file is `status.tsv`;
- this Final Validation section was still pending after full validation passed.

Both issues are resolved here by pointing focused validation to the existing
`status.tsv` evidence file and replacing the pending Final Validation text with
the full validation and review result. The review also noted that the formal
WorkVCS `plan independent-review record` path was not used because this
lightweight Plan has no `independent_validation` object or review-attestation
structure. The review is therefore treated as read-only subagent evidence, not
as a formal attested review record.

The final validation and review did not authorize a release candidate, tag,
Push, deployment, V2 feature, semantic rename tracking, automatic scheduling,
remote/distributed behavior, cross-Store synchronization, or direct mutation of
the original `agent_soul` repository.
