# Phase 4LO Bundle Portability Dogfood Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LO closes the V1 readiness ledger's local Checkpoint and Bundle
portability dogfood gap by using the implemented CLI against source and target
SQLite Stores. The run proves a local copied-target portability loop:
Checkpoint creation, Bundle directory export, validation, target preflight,
target apply, imported Checkpoint validation, post-apply Work State restore,
and same-Store divergence refusal.

This evidence does not claim a packaged Bundle container contract, cross-Store
federation, cloud sync, Agent protocol encoding, release readiness, or larger
Store performance maturity.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lo-bundle-portability-dogfood
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The ignored local Stores and log used for the dogfood run were:

```text
.work-governance/runtime/dogfood/phase-4lo-20260831T223519Z/
.work-governance/runtime/logs/phase-4lo-bundle-portability-dogfood-phase-4lo-20260831T223519Z.log
```

## Fast-Forward Portability

The source Store created a Workspace and one baseline Task, then the Store file
was copied to a target Store. The source Store advanced with a second Task,
created a Checkpoint for that head, and exported a Bundle directory for the
source head.

Captured IDs:

```text
workspace_id=01a059f6-7c50-7b50-8d4d-f5d645ba53aa
branch_id=01a059f6-7c50-7b50-8d4d-f60f46c7394e
first_task_id=01a059f6-7c67-7060-bba9-841728204103
bundle_task_id=01a059f6-7c7d-7810-a552-12cf58f164a7
bundle_head_id=01a059f6-7c7d-7810-a552-129876c1c783
checkpoint_id=01a059f6-7ca1-7390-93ed-87514ca543c4
bundle_import_id=01a059f6-7d32-7b42-b7af-44472b8f9bac
```

The exported directory used the current local payload-index profile:

```text
bundle_container_files=9
bundle_index_profile=workvcs-local-payload-index-v1
bundle_index_version=1
bundle_manifest_path=manifest.json
bundle_payload_count=7
bundle_reference_count=17
```

Target preflight accepted the copied-target fast-forward:

```text
can_apply=true
action=same_store_fast_forward_ready
source_store_relation=same_store
incoming_commit_present=false
import_required=true
```

Target apply imported exactly the missing objects and moved the target Branch:

```text
applied=true
outcome=same_store_fast_forward_applied
imported_commits=1
imported_entity_versions=1
imported_checkpoints=1
imported_checkpoint_statuses=1
updated_branch_heads=1
```

After apply, the target Store proved the imported Work State and Checkpoint:

```text
head_commit_id=01a059f6-7c7d-7810-a552-129876c1c783
checkpoint_found=true
checkpoint_id=01a059f6-7ca1-7390-93ed-87514ca543c4
valid=true
state_matches_expected=true
content_matches_expected=true
```

The apply also persisted an import record that `bundle import-show` and
`bundle import-list` could find by `bundle_digest` and
`same_store_fast_forward_applied`.

## Restore After Import

The target Store then created one target-local Task after the imported Bundle
head and restored the Branch back to the imported head.

Captured IDs:

```text
target_local_task_id=01a059f6-7dd0-7883-85e6-b72898794094
target_local_head_id=01a059f6-7dd0-7883-85e6-b6f55ef38f96
restore_commit_id=01a059f6-7de3-78a0-80fc-55f268054417
```

Restore produced a normal Work State restore commit:

```text
previous_head_commit_id=01a059f6-7dd0-7883-85e6-b6f55ef38f96
target_commit_id=01a059f6-7c7d-7810-a552-129876c1c783
head_operation_type=workstate.restore
entities=2
relations=0
tasks=2
```

The restored state matched the imported Bundle head. The imported Checkpoint
remained valid and discoverable for the imported head. `checkpoint latest` is a
commit-anchored selector, so querying it at the later restore commit returned:

```text
query invalid: no usable checkpoint found for commit 01a059f6-7de3-78a0-80fc-55f268054417
```

Operators should treat this as a selector boundary, not as Checkpoint
corruption: use `checkpoint show` or `checkpoint validate` for the candidate,
and use `checkpoint latest` against the commit that owns the candidate.

## Divergence Recovery

A second source/target Store pair proved the same-Store divergence path. Both
Stores shared the same baseline commit, then source and target advanced the
same Branch independently.

Captured IDs:

```text
divergence_branch_id=01a059f6-7ea2-7e91-92b7-88cb87923113
divergence_source_task_id=01a059f6-7ec9-74d3-b5cf-415260f0901c
divergence_source_head_id=01a059f6-7ec9-74d3-b5cf-412280c3e89a
divergence_target_task_id=01a059f6-7edd-7a10-a12b-254189545621
divergence_target_head_id=01a059f6-7edd-7a10-a12b-251cae839f14
divergence_import_id=01a059f6-7f55-78a2-ae56-17b39f2f94f3
```

Preflight refused direct apply and reported the merge base:

```text
can_apply=false
action=same_store_divergence_detected
branch_head_detail.0.status=diverged
branch_head_detail.0.source_head_commit_id=01a059f6-7ec9-74d3-b5cf-412280c3e89a
branch_head_detail.0.target_head_commit_id=01a059f6-7edd-7a10-a12b-251cae839f14
branch_head_detail.0.merge_base_commit_id=01a059f6-7eb5-7132-98e9-8ec84300b322
```

`bundle import-dir` recorded the refused import attempt with
`same_store_divergence_detected`. `bundle apply-dir` without `--require-applied`
reported no applied changes:

```text
applied=false
import_id=none
outcome=same_store_divergence_detected
imported_commits=0
updated_branch_heads=0
```

`bundle preflight-dir --require-can-apply` and
`bundle apply-dir --require-applied` both failed as expected. The target Branch
head stayed on the target-local commit, and the target-local Task was still
visible after refusal.

## Findings

- The current local CLI is sufficient for a copied-target Bundle portability
  loop: export, validate, preflight, apply, inspect imported records, validate
  imported Checkpoints, and restore Work State after import.
- The current Bundle directory profile is observable as
  `workvcs-local-payload-index-v1` with `payload-index.json` version `1`, but
  the packaged container/profile contract remains Open for V1.
- Same-Store divergence recovery is explicit and non-destructive: preflight
  names the divergence, import can record it, apply refuses it, and the target
  Branch head is unchanged.
- `checkpoint latest` selects usable Checkpoints for the commit that owns the
  candidate. It should not be used as state-digest lookup after a later restore
  commit.

## Remaining Gaps

- Formalize the V1 Bundle container/profile contract beyond the current local
  directory layout.
- Prove or explicitly defer external Store import/apply semantics; this run
  only proves the same-Store copied-target relation.
- Run a larger Store portability workload before making scale or release
  maturity claims.
