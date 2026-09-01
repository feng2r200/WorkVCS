# Phase 4MZ Resource Case-Folding Policy Dogfood Evidence

Date: 2026-09-02

## Scope

Phase 4MZ advances the Resource registration, observation, applicability, and
drift gate by proving the bounded V1-local Resource adapter case-folding policy.

The slice is intentionally narrow:

- make current no-WorkVCS-case-folding behavior operator-visible in
  ResourceObservation summary JSON;
- prove local-file exact path observation is filesystem-native and not a
  WorkVCS case-folded lookup;
- prove local-file path-prefix observation preserves filesystem entry names;
- prove local-file glob matching is case-sensitive;
- prove Git worktree observation preserves parent Git path reporting;
- dogfood the behavior against a temporary clone of a pre-existing real local
  project;
- leave the original project unchanged.

Out of scope:

- `local-file-path-prefix-manifest-v1`, `local-file-glob-manifest-v1`, or
  `git-worktree-manifest-v1` fingerprint profile changes;
- Store schema, Core/History semantics, or command-shape changes;
- filesystem canonicalization or case-normalized path lookup by WorkVCS;
- cross-platform case-collision resolution;
- automatic scheduler, daemon, watcher, or implicit refresh;
- remote, distributed, cross-Store, Agent-orchestrated, V2, release,
  release-candidate, tag, Push, or deployment behavior.

## Implementation Evidence

`crates/workvcs-cli/src/main.rs` now emits this common field in current
local-file and Git ResourceObservation summary JSON:

```text
path_case_folding=disabled
```

Adapter-specific fields are:

```text
path_case_policy=filesystem_native_path_resolution
path_case_policy=filesystem_native_entry_names
path_case_policy=case_sensitive_glob_pattern
path_case_policy=parent_git_path_reporting
glob_case_sensitive=true
```

The fields are summary metadata only. The existing manifest fingerprint
profiles remain unchanged.

Current-behavior inspection passed in
`/tmp/workvcs-4mz-case-folding-policy-inspection-20260901T163650Z/summary.txt`:

```text
host_wrong_case_path_exists=yes
normalize_preserves_case_source=0
normalize_no_ascii_lowercase_source=0
glob_match_options_case_sensitive=0
git_index_contains_ReadMe=0
git_index_contains_MixedCase=0
git_status_clean_bytes=0
git_status_dirty_contains_MixedCase=0
git_untracked_preserves_LooseCase=0
phase4mz_case_folding_policy_inspection=pass
```

The `host_wrong_case_path_exists=yes` result means this host can resolve the
same file through a wrong-case path. Phase 4MZ therefore records
filesystem-native local-file exact path resolution and does not claim
filesystem-level case-sensitive existence on this host.

Focused regression coverage adds:

- `local_file_resource_summaries_report_no_case_folding_policy`;
- `git_worktree_snapshot_reports_path_case_policy_from_parent_git_view`.

The tests prove lexical path normalization preserves case, local-file summaries
report no WorkVCS case folding, local-file glob matching is case-sensitive,
upper-case and lower-case glob patterns produce different fingerprints when
only one pattern matches `CaseName.TXT`, Git index/status/untracked path
reporting preserves mixed-case names, and Git Resource fingerprints still drift
when the parent-visible mixed-case tracked file changes.

Focused validation passed with status table
`/tmp/workvcs-4mz-focused-validation-20260901T164220Z/status.tsv`:

```text
diff-check=0
fmt-check=0
test-local-case=0
test-git-case=0
test-glob-refresh=0
test-git-refresh=0
test-sparse-policy=0
zero-test-check:test-local-case.out=0
zero-test-check:test-git-case.out=0
zero-test-check:test-glob-refresh.out=0
zero-test-check:test-git-refresh.out=0
zero-test-check:test-sparse-policy.out=0
anchors=0
```

## Dogfood Evidence

Dogfood log:

```text
/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T170209Z
```

The source project was the existing local Git repository:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

Only the `/tmp` clone was mutated:

```text
external_original_head_before=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_head_after=e3004b29c8bf3791e87d877b021432dcd1158705
external_original_status_before_sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
external_original_status_after_sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
external_original_unchanged=yes
external_clone=/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T170209Z/work/agent_soul-case-clone
tracked_mixed_case_path=README.md
```

Local-file exact path observation recorded the no-case-folding summary:

```text
local_exact_observation_id=01a05dd8-fbd3-7ff2-8a07-37a19ff03880
local_exact_summary_has_case_folding=yes
local_exact_summary_policy=filesystem_native_path_resolution
```

Glob dogfood proved case-sensitive matching for the real `README.md` path:

```text
upper_glob=/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T170209Z/work/agent_soul-case-clone/README.*
upper_glob_summary_has_case_sensitive=yes
upper_glob_summary_files=1
lower_glob=/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T170209Z/work/agent_soul-case-clone/readme.*
lower_glob_summary_has_case_sensitive=yes
lower_glob_summary_files=0
glob_case_sensitive_fingerprints_differ=yes
```

Git worktree dogfood proved parent Git path reporting and drift:

```text
git_index_contains_mixed_case_path=yes
git_baseline_observation_id=01a05dd8-fd42-7293-a885-97cc27d3c6ff
git_summary_has_path_case_folding=yes
git_summary_policy=parent_git_path_reporting
git_baseline_fingerprint=ce4959eb3aaea1ea432a1b06e91c8f0b1c4b982cb4f97747e2809ec09b12889c
git_status_dirty_contains_mixed_case_path=yes
git_refresh_applicability=stale
git_refresh_reason_code=resource_drift
git_refresh_fingerprint=18ad70978cbc2075191b4aaf2c2b08f147e6780f4e7eaf9e103a3de925fd7e1a
git_refresh_fingerprint_changed=yes
```

Store checks passed:

```text
local_resource_observations=3
git_resource_observations=2
integrity_valid_required=yes
doctor_valid_required=yes
phase4mz_resource_case_folding_policy=pass
```

Three dogfood harness attempts were corrected before the successful run:

- `/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T165114Z`
  failed before exercising WorkVCS behavior because `pipefail` surfaced a
  SIGPIPE from first-line path selection.
- `/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T165401Z`
  used an outdated Task version after creating an AC; WorkVCS correctly
  rejected the stale version.
- `/tmp/workvcs-4mz-resource-case-folding-policy-dogfood-20260901T165804Z`
  completed through Git cache refresh, but the harness looked for the observed
  fingerprint in cache-refresh output instead of cache-show/list output.

## Readiness Impact

Phase 4MZ closes the case-folding policy subgap for bounded V1-local local-file
and Git worktree Resource observations. The bounded policy is:

```text
path_case_folding=disabled
```

The Resource registration, observation, applicability, and drift gate remains
`Partial` and blocking because background re-observation scheduling remains
unproven.

The overall release decision remains false.

## Final Validation

Candidate full pre-merge validation passed before independent review in
`/tmp/workvcs-4mz-full-validation-candidate-20260901T171500Z/status.tsv`:

```text
diff-check=0
fmt-check=0
schema-v0-1=0
clippy=0
cargo-test-workspace=0
smoke-v0-1=0
anchors=0
```

Read-only independent review agent
`01a05ddd-782f-7252-9eea-a0d022928f98` found no blocker and one high issue:
the already-passing full pre-merge validation evidence was not yet referenced
from the Phase 4MZ provenance/ADR evidence chain. This Final Validation section
and ADR-0467 close that evidence-chain gap before local git delivery.

Final post-review validation is recorded in
`/tmp/workvcs-4mz-final-validation-post-review-20260901T172330Z/summary.txt`
and covers the final Phase 4MZ diff:

```text
diff-check=0
fmt-check=0
schema-v0-1=0
clippy=0
cargo-test-workspace=0
smoke-v0-1=0
anchors=0
phase4mz_final_validation=pass
```
