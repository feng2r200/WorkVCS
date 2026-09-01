# Phase 4MX Git Submodule Resource Policy Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MX advances the Resource registration, observation, applicability, and
drift gate by proving the V1-local Git worktree Resource submodule policy.

The slice is intentionally narrow:

- make the current parent-Git submodule behavior operator-visible in
  ResourceObservation summary JSON;
- prove tracked submodules are represented by parent Git gitlink mode `160000`
  and object id;
- prove submodule-internal dirty or untracked state is represented through
  parent Git status bytes rather than recursive parent untracked-file hashing;
- prove submodule pointer updates are represented through parent Git index and
  diff material;
- dogfood the behavior against a temporary clone of a pre-existing real local
  project plus a temporary local submodule;
- leave the original project unchanged.

Out of scope:

- `git-worktree-manifest-v1` fingerprint profile changes;
- Store schema, Core/History semantics, or command-shape changes;
- recursive submodule content hashing;
- WorkVCS submodule init/update/fetch behavior;
- semantic submodule relations;
- automatic scheduler, daemon, watcher, or implicit refresh;
- case-folding or sparse-checkout policy;
- remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Implementation Evidence

`crates/workvcs-cli/src/main.rs` now emits these fields in Git
ResourceObservation summary JSON:

```text
submodule_policy=parent_gitlink_status_diff
submodule_recursion=disabled
```

The fields are summary metadata only. The `git-worktree-manifest-v1`
fingerprint profile remains the same: HEAD object id, raw porcelain status,
raw index listing, raw staged diff, raw unstaged diff, and sorted untracked
regular-file fingerprints.

Current-behavior inspection passed in
`/tmp/workvcs-4mx-submodule-policy-inspection-20260901T151341Z/summary.txt`:

```text
parent_index_gitlink_mode_160000=yes
parent_status_after_inner_untracked= M deps/sub
parent_untracked_lists_inner_file=no
parent_status_marks_submodule_dirty=yes
parent_staged_diff_has_subproject_commit=yes
parent_staged_diff_mentions_old_new=yes
phase4mx_submodule_policy_inspection=pass
```

Focused regression coverage adds:

- `git_worktree_snapshot_reports_submodule_policy_from_parent_git_view`.

The test proves a tracked submodule appears in the parent Git index as mode
`160000`, proves the summary metadata exposes the bounded policy, proves
submodule-internal untracked files dirty the parent submodule status without
being listed as parent untracked files, proves the WorkVCS Git worktree
fingerprint changes for that parent-visible dirty state, and proves staged
submodule pointer updates appear in parent index and staged diff material.

The first focused validation attempt at
`/tmp/workvcs-4mx-focused-validation-initial-20260901T151537Z/status.tsv`
found a formatting-only failure:

```text
fmt=1
test-submodule-policy=0
```

After `cargo fmt --all`, corrected focused validation passed with status table
`/tmp/workvcs-4mx-focused-validation-20260901T151618Z/status.tsv`:

```text
diff-check=0
fmt-check=0
test-submodule-policy=0
test-symlink-policy=0
test-rename-policy=0
test-git-refresh=0
zero-test-check:test-submodule-policy.out=0
zero-test-check:test-symlink-policy.out=0
zero-test-check:test-rename-policy.out=0
zero-test-check:test-git-refresh.out=0
```

## Dogfood Evidence

Dogfood log:

```text
/tmp/workvcs-4mx-git-submodule-resource-dogfood-20260901T152048Z
```

The source project was the existing local Git repository:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Only the `/tmp` clone and temporary local submodule source were mutated:

```text
external_clone=/tmp/workvcs-4mx-git-submodule-resource-dogfood-20260901T152048Z/agent_soul-clone
external_submodule_source=/tmp/workvcs-4mx-git-submodule-resource-dogfood-20260901T152048Z/agent_soul-submodule-source
external_original_head_before=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_head_after=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_clean_after=yes
external_original_unchanged=yes
```

The parent repository recorded the submodule as a Git gitlink:

```text
parent_index_gitlink=160000 e3004b29c8bf3791e87d877b021432dcd1158705 0	vendor/agent-soul-submodule
parent_index_gitlink_mode_160000=yes
```

Baseline WorkVCS verification recorded a Git ResourceObservation with the new
summary policy fields:

```text
verification_id=01a05d8f-0c20-70d2-9aa0-7fa90ab5e977
baseline_observation_id=01a05d8f-0c1e-7dc2-aa8c-96545f78861d
baseline_fingerprint=e8442cbc279ad05cb1f6adb99451d417d3cca341f01d048fa9c68ef033088c63
baseline_summary_has_submodule_policy=yes
baseline_summary_has_submodule_recursion=yes
```

After adding an untracked file inside the submodule, parent Git reported the
submodule dirty while parent untracked-file listing still did not include the
inner file:

```text
parent_status_after_inner_untracked= M vendor/agent-soul-submodule
parent_status_marks_submodule_dirty=yes
parent_untracked_after_inner_untracked_lines=0
parent_untracked_lists_inner_file=no
```

WorkVCS cache refresh projected the parent-visible submodule dirty state as
Resource drift and wrote a new ResourceObservation:

```text
refresh_applicability=stale
refresh_reason_code=resource_drift
refresh_observation_id=01a05d8f-0cf9-7fe2-9888-370197a5d68c
refresh_fingerprint=40e4cea44f37574cd777f933c3afd7ce05e3ef7a4ca170ed27834b95b2171103
dirty_refresh_fingerprint_changed=yes
cache_applicability_after_dirty=stale
cache_reason_after_dirty=resource_drift
cache_observation_status_after_dirty=observed
dirty_summary_has_submodule_policy=yes
dirty_summary_has_submodule_recursion=yes
```

Store checks passed:

```text
resource_observations=2
integrity_valid_required=true
doctor_valid_required=true
phase4mx_git_submodule_resource_policy=pass
```

The first dogfood attempt reached the expected dirty-refresh behavior but the
harness tried to read `resource_stamp.*` fields from `verification
cache-refresh` output where those fields are not emitted. The corrected run
read those fields from `verification cache-show`; this was a harness
evidence-capture correction, not a WorkVCS behavior correction.

## Readiness Impact

Phase 4MX closes the Git submodule-policy subgap for V1-local Git worktree
Resource observations. The bounded policy is:

```text
submodule_policy=parent_gitlink_status_diff
submodule_recursion=disabled
```

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking because case-folding policy, sparse-checkout Git
semantics, and background re-observation scheduling remain unproven.

The overall release decision remains false.

## Final Validation

Full pre-merge validation passed in
`/tmp/workvcs-4mx-final-validation-post-review-corrected/summary.txt`:

```text
diff_check=0
fmt_check=0
schema_v0_1=0
clippy=0
cargo_test_workspace=0
smoke_v0_1=0
anchors=0
phase4mx_final_validation=pass
```

Independent read-only review checked the current diff, the Plan, this
provenance record, the readiness ledger, the release gate matrix, the focused
validation directory, the dogfood summary, and the final validation summary. It
found one medium issue: this Final Validation section initially cited the fixed
final-validation path before that evidence file existed. The issue is resolved
by the fresh full validation result above. No blocker, high, or medium findings
remain unresolved after that correction.

The review also noted that the formal WorkVCS `plan independent-review record`
path was not used because this lightweight Plan has no
`independent_validation` object or trusted attestation structure. The review is
therefore treated as read-only subagent evidence, not as a formal attested
review record.

The final validation and review did not authorize a release candidate, tag,
Push, deployment, V2 feature, recursive submodule hashing, submodule
init/update/fetch behavior, case-folding policy, sparse-checkout policy,
automatic scheduling, remote/distributed behavior, cross-Store synchronization,
or direct mutation of the original `agent_soul` repository.
