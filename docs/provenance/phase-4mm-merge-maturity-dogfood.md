# Phase 4MM Merge Maturity Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MM advances the V1 readiness ledger's `Merge lifecycle` gap by repeating
merge dogfood on a larger and more varied temporary local Store than Phase 4LN.
It is a dogfood-only slice: no product code, schema, or CLI behavior changed.

This evidence does not claim semantic/LLM merge, custom semantic resolution,
remote collaboration, another real project write-mode maturity, GUI/TUI
support, or release-scale performance.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4mm-merge-maturity-dogfood
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The ignored local Store and log directory used for the dogfood run were:

```text
/tmp/workvcs-4mm-merge-maturity-dogfood-20260901T095120Z/store.sqlite
/tmp/workvcs-4mm-merge-maturity-dogfood-20260901T095120Z
```

## Workload

The run created a base target Branch and a source Branch, then applied mixed
target/source changes:

```text
workload_conflicts=12
workload_source_only_tasks=8
workload_target_only_tasks=6
workload_source_relations=4
```

The target Branch transitioned the shared Tasks to `in_progress`. The source
Branch transitioned the same shared Tasks to `blocked`, added source-only
Tasks, and added source-side `depends_on` scheduling Relations.

## Merge Result

`merge start`, item resolution, `merge freeze`, and `merge continue` completed.
All items were explicitly resolved with the source side for this dogfood run:

```text
merge_items=24
conflict_items=12
auto_items=12
relation_items=4
theirs_resolutions=24
runtime_state=completed
```

The final target head is a merge commit:

```text
commit_kind=merge
parents=2
```

The final WorkState matched the expected target/source combination:

```text
tasks_match_expected=true
relations_match_expected=true
matches_expected=true
```

## Integrity And Doctor

The same Store passed required-valid integrity and doctor checks after the
merge:

```text
checked_branches=2
checked_commits=56
checked_changesets=56
checked_change_operations=78
checked_changeset_causal_anchors=0
checked_events=58
checked_checkpoints=0
invalid_checkpoints=0
valid_required=true
```

`doctor --require-valid` reported the same clean integrity state.

## Script Finding

The first dogfood command did not write its final summary because the script
expected target Branch history to include source-side ordinary commits after
merge. Current `history` behavior is first-parent for the selected Branch. The
observed target first-parent count was:

```text
history_first_parent_commits=32
```

That count is consistent with the target line plus the merge commit. Source-side
ordinary commits are proven by the merge commit's secondary parent and by the
final WorkState containing the expected source-only Tasks and Relations. This
was a script-assumption issue, not a product blocker.

Future merge dogfood scripts should use merge parents, final WorkState, and
integrity checks as the primary source contribution evidence. Target
first-parent history counts should only be used for target-line assertions.

## Remaining Open

- Repeat merge in a real write-mode external-project workflow before using it
  as broad release-maturity evidence.
- Semantic/LLM merge remains outside V1.
- Custom semantic resolution maturity remains Open.
- Release-scale performance remains Open; this run is bounded local dogfood,
  not a benchmark.
- History traversal beyond target first-parent remains a separate product
  question, not a Phase 4MM change.

## Independent Review

PASS after fix. Independent review found one medium documentation-timing issue:
ADR-0454 said final validation and independent review were recorded while this
provenance file still showed them as pending. The ADR evidence wording now says
delivery validation and independent review closeout are tracked here, avoiding
a premature completion claim.

## Final Validation

PASS: `/tmp/workvcs-4mm-final-validation-20260901T100247Z`.

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
