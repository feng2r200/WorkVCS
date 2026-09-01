# Phase 4ML Why Causal Anchor Traversal Evidence

Date: 2026-09-01

## Scope

Phase 4ML closes one concrete `why` dogfood gap: first-parent-reachable
ChangeSet causal anchors are now directly visible from `why` for the queried
Entity. The slice is limited to read-only projection and CLI rendering.

## Gap Dogfood

PASS: `/tmp/workvcs-4ml-why-gap-dogfood-20260901T092321Z`.

The pre-change run created a temporary Store, recorded a Decision, a Finding,
a replacement Decision, and a Decision supersede with `--because-record`.

Key observations:

```text
changeset_causal_anchors=1
filtered_anchor_count=1
why_relation_edges=1
why_deferred_relation_family_0=evolution
why_has_anchor_commit_or_changeset=no
```

This proved that the Store had the causal anchor and `why` could detect the
deferred evolution family, but `why` did not expose the anchoring commit or
ChangeSet.

## Implementation Evidence

Changed files:

```text
crates/workvcs-core/src/history/why.rs
crates/workvcs-core/src/history/mod.rs
crates/workvcs-core/src/lib.rs
crates/workvcs-cli/src/main.rs
```

Implemented behavior:

```text
causal_anchor_changesets=<N>
causal_anchor_changeset.<i>.commit_id=<COMMIT_ID>
causal_anchor_changeset.<i>.changeset_id=<CHANGESET_ID>
causal_anchor_changeset.<i>.operation_type=<OPERATION_TYPE>
causal_anchor_changeset.<i>.operation_schema_version=<VERSION>
causal_anchor_changeset.<i>.committed_at_us=<UTC_MICROS>
causal_anchor_changeset.<i>.changeset_created_at_us=<UTC_MICROS>
causal_anchor_changeset.<i>.anchor_object_id=<ENTITY_ID>
causal_anchor_changeset.<i>.anchor_object_kind=entity
```

The projection is computed from first-parent history and
`changeset_causal_anchor`. It does not add schema, stored Relation rows, or
automatic inference.

## Focused Validation

PASS: `/tmp/workvcs-4ml-focused-validation-rerun-20260901T093023Z`.

Covered checks:

```text
cargo fmt --all -- --check
cargo test -q -p workvcs-cli cli_why_reports_evolution_deferred_family_for_causal_anchor
cargo clippy -q -p workvcs-core -p workvcs-cli --tests -- -D warnings
```

## Post-Change Dogfood

PASS: `/tmp/workvcs-4ml-why-causal-anchor-dogfood-20260901T093115Z`.

Key observations:

```text
changeset_causal_anchors=1
filtered_anchor_count=1
why_relation_edges=1
why_causal_anchor_changesets=1
why_anchor_commit_matches=yes
why_anchor_changeset_matches=yes
why_anchor_operation_type=record.decision.supersede
why_anchor_object_matches=yes
why_anchor_object_kind=entity
why_deferred_relation_family_0=evolution
```

## Remaining Open

- Full evolution traversal remains Open.
- Epistemic traversal remains Open.
- Broader causal traversal remains Open.
- Causal-anchor filters and limits remain outside this slice.

## Independent Review

PASS: independent review found no blocker/high/medium issues for the scoped
implementation, CLI output, ADR, ledger, provenance, and evidence logs.

PASS: README addendum review found no blocker/high/medium issues for the 4ML
README index links and release-boundary wording.

## Final Validation

PASS: `/tmp/workvcs-4ml-final-validation-20260901T093751Z`.

Covered checks:

```text
git diff --cached --check
cargo fmt --all -- --check
scripts/validate-schema-v0.1.sh
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
scripts/smoke-v0.1-cli-workflow.sh
```

## Delivery Closeout

- Implementation commit:
  `eb55eeca1536428b0f66dd3d61f8c897221d3c16`.
- Fast-forward merged to `main`.
- Worktree cleanup proof:
  `/tmp/workvcs-4ml-cleanup-20260901T094429Z`.
- Cleanup proof fields:
  `clean=yes`, `attached=yes`, `unlocked=yes`, `covered_by_main=yes`,
  `removed=yes`, `branch_retained=yes`.
