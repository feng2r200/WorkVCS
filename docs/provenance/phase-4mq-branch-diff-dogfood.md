# Phase 4MQ Branch Diff Dogfood Evidence

Date: 2026-09-01

## Scope

Phase 4MQ dogfoods WorkVCS Branch and diff workflows in a real repository
delivery slice for WorkVCS itself.

The slice creates a real WorkVCS Store, forks a base Branch into an
implementation Branch, records implementation-branch work, compares the two
Branch heads with bidirectional `diff`, and proves the compared WorkStates with
`history`, `show-at`, and `store integrity`.

This is dogfood and documentation evidence only. It does not change Rust code,
schema, CLI behavior, release operations, Push state, tags, deployment, or V2
scope. It also does not claim that WorkVCS V1 is release-ready or that V0.1
dogfood is complete.

## Dogfood Setup

- Source repository baseline: main commit
  `24e184d6ba4393b9e4494b6ddca1859432c4ff2c`.
- Dogfood log directory:
  `/tmp/workvcs-4mq-branch-diff-dogfood-20260901T113052Z`.
- Dogfood Store:
  `/tmp/workvcs-4mq-branch-diff-dogfood-20260901T113052Z/store.workvcs`.
- Workspace id: `01a05cbd-834d-76b0-aedf-29b55bcd6a6e`.
- Base Branch id: `01a05cbd-834d-76b0-aedf-29ed88d3d3c3`.
- Implementation Branch id: `01a05cbd-83b7-7400-bd9c-8d9de61319d3`.
- Base head commit id: `01a05cbd-8373-71d1-a908-eac02ba7a22a`.
- Implementation head commit id:
  `01a05cbd-83f2-7792-9301-ffeba8074948`.
- Base WorkState digest:
  `ffd8a6dc2de28fbd2fe2c0f5cc4effb2db04ecd7d14f78175980d14f58cd13b6`.
- Implementation WorkState digest:
  `0fa1eaa47cb8a634d44a05e7daa144d72fee61062ba3eea46bf2167053d9fbca`.

## Command Coverage

The dogfood run first built the current CLI with:

```text
cargo build -q -p workvcs-cli
```

It then exercised these process-boundary commands against the dogfood Store:

```text
init
workspace create
task create --branch main
branch head --branch main
branch fork --from-branch main --name phase-4mq-implementation
branch list --workspace
task create --branch phase-4mq-implementation --head
branch head --branch phase-4mq-implementation
diff --from-branch main --to-branch phase-4mq-implementation
diff --from-branch phase-4mq-implementation --to-branch main
history --branch main
history --branch phase-4mq-implementation
show-at --branch main
show-at --branch phase-4mq-implementation
store integrity --require-valid --expected-checked-branches 2
```

Every captured command exited with status 0 and every captured stderr file was
empty.

## Key Observations

```text
final_dogfood=pass
forward_diff_entity_changes=1
forward_diff_change_kind=added
reverse_diff_entity_changes=1
reverse_diff_change_kind=removed
base_history_entries=2
implementation_history_entries=3
show_at_base_matches_expected=true
show_at_implementation_matches_expected=true
integrity_valid_required=true
```

The final evidence check also proves:

- `branch fork` preserved the source workspace, source Branch id, and base head
  commit id.
- `branch list` returned two Branches and matched the expected count.
- Forward diff compared Branch head to Branch head, from the base head/state to
  the implementation head/state, with one added entity and no relation changes.
- Reverse diff compared the same two Branch heads in the opposite direction,
  with one removed entity and no relation changes.
- `history` reported the expected first-parent entry counts for both Branches.
- `show-at` reported the expected WorkState digest for both Branch heads.
- `store integrity --require-valid` checked both Branches and returned
  `valid_required=true`.

## Harness Findings

The first local dogfood harness attempt failed because it assumed `init` output
would start with `store_id=...`; the current CLI prints the same field in a
whitespace key-value line:

```text
initialized store_id=... schema_version=1 ...
```

The successful run also corrected two assertion assumptions in the local
harness:

- `show-at` reports the expectation result as `matches_expected=true`.
- `store integrity --require-valid` proves the validity requirement through
  exit status 0 and `valid_required=true`, not a separate `valid=true` field.

These are dogfood harness findings. They did not require a WorkVCS product,
schema, or CLI behavior change in this slice.

## Readiness Impact

Phase 4MQ advances the ledger row for Workspace, Branch, history, show-at,
diff, and restore. Phase 4LO already dogfooded copied-target restore and
`show-at`; Phase 4MQ adds real Branch fork, bidirectional Branch diff, Branch
history, Branch `show-at`, and two-Branch integrity evidence in a repository
delivery Store.

This moves the release gate for Branch and state navigation from missing
Branch/diff dogfood to bounded pass evidence for the V1-local scope. It does
not change the remaining blocking gates or authorize a release-candidate
operation.

## Final Validation

PASS: `/tmp/workvcs-4mq-final-validation-20260901T114447Z`.

Covered checks:

```text
git diff --check
git diff --check with new files staged intent-to-add
cargo fmt --all -- --check
file existence checks for this provenance page, ADR-0458, ledger, matrix, and README
literal README link checks
literal ledger and matrix status checks
dogfood final-summary checks
dogfood final-evidence-check checks
```

Independent review found no blocker/high/medium issues. The review confirmed
that the Phase 4MQ docs and evidence support moving the Workspace, Branch,
history, diff, show-at, and restore release gate from `Partial`/blocking to
`Pass`/non-blocking within the bounded V1-local scope, while preserving the
overall `V1_RELEASE_READY=false`, `V0_1_DOGFOOD_COMPLETE=false`, and
`RELEASE_CANDIDATE_ALLOWED=false` decisions.

Residual risk: the independent review did not rerun the full dogfood flow; it
reviewed the current diff, final evidence summaries, validation summary, and
spot checks of raw outputs. The evidence remains bounded to one local
repository delivery Store.
