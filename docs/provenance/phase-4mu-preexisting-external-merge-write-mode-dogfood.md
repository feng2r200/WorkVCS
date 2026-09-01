# Phase 4MU Preexisting External Merge Write-Mode Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MU dogfoods WorkVCS merge lifecycle against pre-existing real project
content. The external source project is:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

The original project was used only as a read-only clone source. All external
write-mode target/source Branch work occurred in a `/tmp` clone and Git
worktrees:

```text
/tmp/workvcs-4mu-preexisting-external-merge-write-mode-20260901T133226Z/agent_soul-clone
/tmp/workvcs-4mu-preexisting-external-merge-write-mode-20260901T133226Z/agent_soul-target
/tmp/workvcs-4mu-preexisting-external-merge-write-mode-20260901T133226Z/agent_soul-source
```

This is dogfood and documentation evidence only. It does not change Rust code,
schema, CLI behavior, release operations, Push state, tags, deployment, or V2
scope. It also does not claim remote, distributed, cross-Store, semantic/LLM,
or Agent-orchestrated merge behavior.

## Dogfood Setup

- WorkVCS source baseline: main commit
  `6dfccf55745d7556b76617f2013b62c49f062591`.
- External source project:
  `/Users/example/Projects/HeXun/Hernes/agent_soul`.
- External source project head:
  `e3004b29c8bf3791e87d877b021432dcd1158705`.
- Inspection log directory:
  `/tmp/workvcs-4mu-inspection-20260901T132814Z`.
- Dogfood log directory:
  `/tmp/workvcs-4mu-preexisting-external-merge-write-mode-20260901T133226Z`.
- Dogfood Store:
  `/tmp/workvcs-4mu-preexisting-external-merge-write-mode-20260901T133226Z/store.sqlite`.
- Scenario boundary:
  `pre-existing real local git project cloned to tmp sandbox; original remains read-only`.

T-001 first attempted inspection with an unsupported `cargo -C` command shape
and a nonexistent root `SYSTEM.md` file. That attempt stopped before dogfood
writes. The successful inspection used direct cargo commands from the WorkVCS
worktree and selected the tracked root `README.md` as the existing conflict
file.

## External Git Write-Mode Evidence

The sandbox clone started from the current `agent_soul` `main` commit:

```text
external_base_commit=e3004b29c8bf3791e87d877b021432dcd1158705
external_base_tree=53571256d9ae8871752e89ad2f070e2e29f8bb1c
external_base_readme_sha=62e573490ccf115fac2a01e621e6cbf206d21f7968b273185e909a94afe4f0d2
```

The target and source worktrees changed the first line of the existing
`README.md` differently. The source worktree also added a new note file:

```text
external_target_commit=59e8444bbfe92c7f0721dbfeafb5c1f36610225b
external_target_readme_sha=97e3fd0a340eae3d0011a5f86f41f4bd381c054f8c9e9930916872c06f2186b1
external_source_commit=2c239d4edccb70cab881f6fa6dc55c396aafcc21
external_source_readme_sha=59af67b567c464744a7c64bb34bb896b45e4cdc5659eb26dd63fe4fcf8f205d7
external_source_note_sha=472a39fbb2e9444ae0756c49fe41e12c6231ec845af17b03aeb0322b1c85c473
```

Merging the source branch into the target worktree produced an actual Git
conflict in the existing `README.md`:

```text
external_target_merge_conflict_status=1
external_conflict_seen=yes
```

The dogfood resolved the external Git merge to the source branch, committed the
merge, and verified the target sandbox was clean:

```text
external_merge_commit=5095f95a8c1d673f33c6992bdc38f63f84954275
external_merge_parents=2
external_git_merge_result_matches_source_resolution=yes
external_target_clean_after_merge=yes
```

The resulting external Git graph was:

```text
*   5095f95 (HEAD -> phase4mu-target) merge source README into target
|\
| * 2c239d4 (phase4mu-source) source adjusts README heading and adds note
* | 59e8444 target adjusts README heading
|/
* e3004b2 (origin/main, origin/HEAD, main) Refresh MCP help before Skill upload
* 7e8c663 Add DianJin Skill upload script
* 414efc7 Normalize DianJin MCP names
* e60ba64 feat: add AGENTS.md
```

The original external project status and head were unchanged before and after
the sandbox dogfood:

```text
external_original_unchanged=yes
```

## WorkVCS Merge Evidence

The WorkVCS Store represented the same external branch states as `external_file`
entities whose state JSON recorded the external project name, source project
head, Git commit, tree, path, content digest, and semantic state.

```text
workspace_id=01a05d2b-d65d-7a91-8d15-b91bf25e7d2d
target_branch_id=01a05d2b-d65d-7a91-8d15-b9429837f2ae
source_branch_id=01a05d2b-d690-76d0-9371-67f34d3bf0c1
base_commit_id=01a05d2b-d67a-78d1-8385-385dcd559c50
target_head_commit_id=01a05d2b-d6a7-7c33-a165-59e90f7e4a9a
source_head_commit_id=01a05d2b-d6d3-7802-ade0-a11b73593703
merge_id=01a05d2b-d6fa-7f42-8aad-a74c25037c5f
```

`merge show` reported two active items: one conflict for the existing README
entity and one auto item for the source-only note:

```text
items=2
item.0.merge_item_id=01a05d2b-d6fa-7f42-8aad-a72e2c395dc5
item.0.classification=CONFLICT
item.0.subject_kind=entity
item.0.subject_id=01a05d2b-d67a-78d1-8385-3881b56870c4
item.1.merge_item_id=01a05d2b-d6fa-7f42-8aad-a73e1ff8f29f
item.1.classification=AUTO
item.1.subject_kind=entity
item.1.subject_id=01a05d2b-d6d3-7802-ade0-a14a712b056a
```

The unresolved freeze guard rejected freezing before explicit resolutions:

```text
merge_freeze_before_resolution_status=1
unresolved_freeze_guard_seen=yes
```

The dogfood then resolved both items to `theirs`, froze the merge, and
continued it:

```text
merge-resolve-conflict_status=0
merge-resolve-auto_status=0
merge-freeze_status=0
merge-continue_status=0
```

Completed merge inspection confirmed both source-side resolutions:

```text
runtime_state=completed
outcome=completed
item.0.resolution=theirs
item.1.resolution=theirs
outcome.result_commit_id=01a05d2b-d783-7410-93c7-59ae71f31e0d
```

The resulting WorkVCS commit is a two-parent merge commit:

```text
commit_id=01a05d2b-d783-7410-93c7-59ae71f31e0d
commit_kind=merge
operation_type=merge.continue
state_digest=5986d0b327ce311f465ad99548edf41830dd2450b2098564498c822bf73889dc
parents=2
parent[0].commit_id=01a05d2b-d6a7-7c33-a165-59e90f7e4a9a
parent[1].commit_id=01a05d2b-d6d3-7802-ade0-a11b73593703
matches_expected=true
```

## Final WorkState Proof

The final target Branch head and `show-at` both matched the result WorkState
digest. `show-at` exposed the source-side README entity version and the
source-only note entity version:

```text
commit_id=01a05d2b-d783-7410-93c7-59ae71f31e0d
state_digest=5986d0b327ce311f465ad99548edf41830dd2450b2098564498c822bf73889dc
entities=2
entity=01a05d2b-d67a-78d1-8385-3881b56870c4 version=01a05d2b-d6bd-7e43-bd3a-9b447ba2ae90
entity=01a05d2b-d6d3-7802-ade0-a14a712b056a version=01a05d2b-d6d3-7802-ade0-a1357fab2646
relations=0
matches_expected=true
```

As in Phase 4MT, `show-at` provides entity-version anchors rather than
rendering entity state JSON. A read-only `entity_version` query against the
dogfood Store proved both final entity versions point at the external source
Git commit and source-side content digests:

```text
git_commit=2c239d4edccb70cab881f6fa6dc55c396aafcc21
path=README.md
semantic_state=source
content_sha256=59af67b567c464744a7c64bb34bb896b45e4cdc5659eb26dd63fe4fcf8f205d7
```

```text
git_commit=2c239d4edccb70cab881f6fa6dc55c396aafcc21
path=docs/phase4mu-source-note.md
semantic_state=source_only
content_sha256=472a39fbb2e9444ae0756c49fe41e12c6231ec845af17b03aeb0322b1c85c473
```

`diff` from the base WorkVCS commit to the result commit matched the expected
external source-side result:

```text
entity_changes=2
entity[0].change_kind=updated
entity[1].change_kind=added
relation_changes=0
entity_changes_match_expected=true
relation_changes_match_expected=true
```

## Store Closeout

The merge Session ended with a SessionDiff:

```text
session_diff_id=01a05d2b-d833-7812-bee0-a081c381442b
session_lifecycle=ended
```

Whole-Store integrity and doctor checks passed with required-valid mode:

```text
integrity_valid_required=true
doctor_valid_required=true
phase4mu_preexisting_external_project_write_mode=pass
phase4mu_workvcs_merge_lifecycle=pass
dogfood_complete=pass
```

## Dogfood Findings

- The first T-001 inspection attempt used `cargo -C`, which is unstable on the
  current Rust toolchain. The successful inspection ran cargo from the worktree
  directory.
- The first T-001 inspection also assumed a root `SYSTEM.md`. Current
  `agent_soul` has `dianjin-research-skill-pack/SYSTEM.md` and a tracked root
  `README.md`; the dogfood correctly used the existing root `README.md` as the
  conflict file.
- The external writes happened only in the sandbox clone. The original
  `agent_soul` project remained unchanged.
- The evidence proves local WorkVCS merge lifecycle against pre-existing real
  project content cloned into a write-mode sandbox. It does not prove remote,
  cross-Store, distributed, semantic/LLM, or Agent-orchestrated merge behavior.

## Readiness Impact

Phase 4MT left the merge release gate narrowed but still blocking because its
external project was generated for the dogfood. Phase 4MU repeats the same
merge lifecycle against pre-existing real `agent_soul` project content cloned
into a local write-mode sandbox, with an actual Git conflict on an existing
tracked file and a source-only added file.

This closes the merge lifecycle and conflict recovery evidence gap for the
bounded V1-local release gate. The release gate matrix can move that row to
`Pass` and non-blocking while preserving the overall non-ready release decision
because other gate rows remain `Partial` or `Blocked`.

## Final Validation

Final validation passed in
`/tmp/workvcs-4mu-final-validation-20260901T134940Z`:

- `git diff --check`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- README, ADR, ledger, matrix, release-state, next-work, dogfood-summary, final
  SQLite-state, and original-project unchanged anchors

A read-only independent review in subagent
`01a05d38-806d-7f50-b231-327677a73734` found no blocker or high findings. It
reported one medium finding: this final validation section still used stale
pending-validation wording. That closeout wording is corrected here.

The independent review remains degraded evidence because this controller has no
trusted attestation mechanism for a formal verified review record. It supports
the local documentation closeout, but it does not authorize a release,
release-candidate, tag, Push, deployment, remote operation, V2 scope change, or
any mutation of the original external project.
