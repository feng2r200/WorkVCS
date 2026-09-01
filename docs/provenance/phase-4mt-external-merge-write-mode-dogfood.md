# Phase 4MT External Merge Write-Mode Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MT dogfoods the merge lifecycle in a write-mode external-project
workflow. The run creates a generated external local Git project, performs real
target/source writes in external Git worktrees, observes and resolves an actual
Git merge conflict, and then mirrors that external branch divergence through
WorkVCS merge commands.

This is dogfood and documentation evidence only. It does not change Rust code,
schema, CLI behavior, release operations, Push state, tags, deployment, or V2
scope. It also does not claim that WorkVCS V1 is release-ready, that V0.1
dogfood is complete, or that a generated external local project proves
pre-existing business repository maturity.

## Dogfood Setup

- Source repository baseline: main commit
  `37562537fe42af3e30310499d9c9d38e898dd8b3`.
- Inspection log directory:
  `/tmp/workvcs-4mt-inspection-20260901T125347Z`.
- Dogfood log directory:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z`.
- Dogfood Store:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z/store.sqlite`.
- External project:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z/external-project`.
- External target worktree:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z/external-target`.
- External source worktree:
  `/tmp/workvcs-4mt-external-merge-write-mode-20260901T125804Z/external-source`.
- Scenario boundary:
  `generated external local git project; not pre-existing business repository maturity`.

## External Git Write-Mode Evidence

The external project started from one base commit, then target and source
worktrees changed the same tracked file differently:

```text
external_base_commit=bedfa453140a68ab6423991105a962e924fbdea1
external_target_commit=bc225f5af109c3d40c18f856687f674135888a12
external_source_commit=af0059079fa1bca97f82e0b3400ca29bbcc8770f
```

Merging the source branch into the target worktree produced a real Git
conflict and exited non-zero before manual resolution:

```text
external_target_merge_conflict_status=1
external_conflict_seen=yes
```

The dogfood then resolved the Git conflict to the source branch wording,
committed the merge, and verified the target worktree was clean:

```text
external_merge_commit=d2eeea03f2250de359d822e94262f960dec6fb5a
external_merge_parents=2
external_git_merge_result_matches_source_resolution=yes
```

The resulting external Git graph was:

```text
*   d2eeea0 (HEAD -> phase4mt-target) merge source wording into target
|\
| * af00590 (phase4mt-source) source writes release wording
* | bc225f5 target writes operator wording
|/
* bedfa45 (main) base external project
```

## WorkVCS Merge Evidence

The WorkVCS Store represented the same external branch states as WorkVCS
entities whose state JSON recorded external Git commit, tree, path, and content
digest anchors.

```text
workspace_id=01a05d0c-5dc4-7141-a85d-c204bb92bcd2
target_branch_id=01a05d0c-5dc4-7141-a85d-c234c1dfac34
source_branch_id=01a05d0c-5df7-7751-8aed-4305165a8f26
base_commit_id=01a05d0c-5de0-7753-80c8-4322234e10b4
target_head_commit_id=01a05d0c-5e0e-7ff1-bb43-19a32b2596db
source_head_commit_id=01a05d0c-5e3a-7593-9ae2-1633d22b3a64
merge_id=01a05d0c-5e5f-73f3-915c-3d5726c248b4
```

`merge show` reported two merge items: one conflicting shared file entity and
one source-only auto entity:

```text
items=2
item.0.merge_item_id=01a05d0c-5e5f-73f3-915c-3d361742b1f8
item.0.classification=CONFLICT
item.0.subject_kind=entity
item.0.subject_id=01a05d0c-5de0-7753-80c8-43544b375eee
item.1.merge_item_id=01a05d0c-5e5f-73f3-915c-3d4e2b21e821
item.1.classification=AUTO
item.1.subject_kind=entity
item.1.subject_id=01a05d0c-5e3a-7593-9ae2-16677b4b636b
```

The unresolved freeze guard rejected freezing while both items were still
unresolved:

```text
merge_freeze_before_resolution_status=1
error_code=workspace_invalid
message=workspace invalid: merge 01a05d0c-5e5f-73f3-915c-3d5726c248b4 has 2 unresolved item(s)
```

The dogfood resolved both items to `theirs`, froze the merge, and continued it:

```text
merge_resolve_conflict_status=0
merge_resolve_auto_status=0
merge_freeze_status=0
merge_continue_status=0
```

Completed merge inspection confirmed both source-side resolutions and the
result commit:

```text
runtime_state=completed
outcome=completed
item.0.resolution=theirs
item.1.resolution=theirs
outcome.result_commit_id=01a05d0c-5eea-7622-856b-41dfa461c894
```

The resulting WorkVCS commit is a merge commit with the target head as primary
parent and source head as secondary parent:

```text
commit_id=01a05d0c-5eea-7622-856b-41dfa461c894
commit_kind=merge
operation_type=merge.continue
state_digest=0c33751dc88b38222a1e0e37b0490c1aabcaba9def424a0f1d5a6c7dacdbb79e
parents=2
parent[0].commit_id=01a05d0c-5e0e-7ff1-bb43-19a32b2596db
parent[1].commit_id=01a05d0c-5e3a-7593-9ae2-1633d22b3a64
matches_expected=true
```

## Final WorkState Proof

`show-at` for the result commit reported the expected final WorkState digest
and two entity-version anchors:

```text
commit_id=01a05d0c-5eea-7622-856b-41dfa461c894
state_digest=0c33751dc88b38222a1e0e37b0490c1aabcaba9def424a0f1d5a6c7dacdbb79e
entities=2
entity=01a05d0c-5de0-7753-80c8-43544b375eee version=01a05d0c-5e24-70c3-8764-14966da17194
entity=01a05d0c-5e3a-7593-9ae2-16677b4b636b version=01a05d0c-5e3a-7593-9ae2-165211a02fac
relations=0
matches_expected=true
```

The current CLI does not render entity state JSON in `show-at`. Final source
state proof therefore uses those version anchors plus a read-only
`entity_version` query against the dogfood Store:

```text
final_show_at_contract_version_matches_source=yes
final_show_at_note_version_matches_source=yes
final_store_contract_state_matches_external_source=yes
final_store_note_state_matches_external_source=yes
```

The queried final entity-version states both point at the external source Git
commit:

```text
git_commit=af0059079fa1bca97f82e0b3400ca29bbcc8770f
path=docs/merge-contract.md
semantic_state=source
content_sha256=bacdbb9e999c480d474acf41e896ff65d922946a11eadf8d1f3aaf69d21736d1
```

```text
git_commit=af0059079fa1bca97f82e0b3400ca29bbcc8770f
path=docs/source-only-release-note.md
semantic_state=source_only
content_sha256=1d15c6bedfa8782316f6ba485953c9a6d2f31d996e2cf0d7342cc6ec11404a46
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
session_diff_id=01a05d0e-b264-7430-a157-702a1a96be2b
session_lifecycle=ended
```

Whole-Store integrity and doctor checks passed with required-valid mode:

```text
checked_branches=2
checked_commits=6
checked_changesets=6
checked_change_operations=6
checked_events=10
valid_required=true
```

```text
ok store_id=01a05d0c-5d9f-7f03-830d-c8ef5ecb9528 schema_version=1 canonical_json_profile=workvcs-jcs-v1 checked_branches=2 checked_commits=6 checked_changesets=6 checked_change_operations=6 checked_events=10
valid_required=true
```

## Dogfood Findings

- The first dogfood run expected a stale output field name. Current Branch head
  and commit inspection output uses `matches_expected=true`, not
  `state_digest_matches_expected`.
- The second core run initially treated `show-at` as if it rendered entity
  state JSON. Current `show-at` exposes final WorkState entity/version anchors;
  the run continued by pairing those anchors with a read-only Store query for
  the exact final entity-version state.
- The external project was generated under `/tmp` for this dogfood. That keeps
  the run bounded and reproducible, but it does not prove pre-existing business
  repository maturity or broad operator-owned workflow maturity.

These are dogfood harness and release-evidence-boundary findings. They did not
require a WorkVCS product, schema, or CLI behavior change in this slice.

## Readiness Impact

Phase 4LN and Phase 4MM already proved local merge mechanics in durable Stores,
including conflict classification, explicit resolutions, freeze/continue, and
two-parent merge commits. Phase 4MT extends that evidence into generated
external local Git write-mode work: an actual external Git conflict and merge
commit are mirrored by a WorkVCS conflict item, source-only auto item, explicit
source-side resolutions, final WorkState proof, and required-valid
integrity/doctor.

This narrows the merge release gate, but it does not close it. The release gate
matrix remains `Partial` and blocking for merge lifecycle and conflict recovery
until a later slice repeats this path against a pre-existing real external
project or a broader operator-owned workflow.

## Final Validation

PASS:
`/tmp/workvcs-4mt-final-validation-20260901T130932Z`.

T-004 final validation passed `git diff --check`,
`cargo fmt --all -- --check`, `cargo test --workspace`, required-document
existence and link anchors, release false anchors, merge gate `Partial` /
blocking anchors, dogfood raw-output anchors, and negative checks for release
or merge-gate overclaim.

Independent read-only review checked the current diff and status,
PLAN-20260901-046 runtime and Plan file, the Phase 4MT dogfood raw logs, the
final validation log, ADR-0461, this provenance document, the readiness ledger,
and the release gate matrix. It found no blocker or high findings. Its only
medium finding was sequencing evidence: T-004 could not be closed before the
independent review result itself was recorded. That finding is resolved by
recording the review in the final validation log and T-004 evidence before
closing T-004.

The reviewer confirmed that the evidence supports generated external local Git
write-mode merge dogfood, the documents preserve
`V1_RELEASE_READY=false`, `V0_1_DOGFOOD_COMPLETE=false`, and
`RELEASE_CANDIDATE_ALLOWED=false`, and the release gate matrix correctly keeps
merge lifecycle and conflict recovery as `Partial` and blocking until a
pre-existing real external project or broader operator-owned write-mode
workflow repeats the path.

The formal WorkVCS `plan independent-review record` path was not used because
this lightweight Plan has no `independent_validation` object and no revision
value for the recorder. The review is therefore treated as read-only subagent
evidence, not as a formal attested review record.
