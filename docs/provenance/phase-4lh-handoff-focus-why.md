# Phase 4LH Handoff Focus Why Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LH closes the Phase 4LF reviewability gap where focused Handoff
continuation was dogfood-proven, but `why` did not expose the Handoff focus
link. The implementation adds read-only `scope_links` for recognized focused
Handoff scope and leaves stored `relation_edges` unchanged.

This evidence does not claim typed Handoff relations, full causal/evolution
`why`, another-project dogfood, or release readiness.

## Dogfood Setup

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lh-handoff-focus-why
```

The CLI was built locally with:

```text
cargo build -q -p workvcs-cli
```

The ignored local Store and log used for the dogfood run were:

```text
.work-governance/runtime/dogfood/phase-4lh-20260831193400.sqlite
.work-governance/runtime/logs/phase-4lh-dogfood-20260831193400.log
```

Captured IDs:

```text
task_entity_id=01a05950-7db8-7c41-b0e1-821da10414ef
handoff_record_id=01a05950-7e08-7e62-8c28-9fdc38d06536
handoff_commit_id=01a05950-7e08-7e62-8c28-9fa7a3186777
```

## Dogfood Run

The Handoff Record side was queried with `why` at the Handoff commit and
asserted no stored relation edge plus one scope link:

```text
relation_edges=0
scope_links=1
scope_link.0.link_kind=handoff_focus
scope_link.0.direction=outgoing
```

The focused Task side was queried at the same commit and reported the same
scope link as incoming:

```text
relation_edges=0
scope_links=1
scope_link.0.link_kind=handoff_focus
scope_link.0.direction=incoming
```

Full summary:

```text
dogfood_result=passed
store=.work-governance/runtime/dogfood/phase-4lh-20260831193400.sqlite
log=.work-governance/runtime/logs/phase-4lh-dogfood-20260831193400.log
```

## Validation

Targeted validation passed before documentation closeout:

```text
cargo test -q -p workvcs-core --test why_handoff_focus_scope_phase4lh
cargo test -q -p workvcs-cli cli_runs_focused_handoff_workflow
```

Independent review found one Medium issue: malformed or future-version Handoff
scope could block unrelated `why` queries. The fix treats unrecognized scope as
absent from auxiliary `scope_links`; the core regression test now covers
malformed v1 and future-version scope.

Final pre-commit validation passed:

```text
cargo fmt --all -- --check
cargo clippy --quiet --all-targets --all-features -- -D warnings
cargo test --workspace --quiet
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
git diff --check
```

Validation log:

```text
.work-governance/runtime/logs/phase-4lh-final-validation-20260831194005.log
```

## Findings

- `why` can now review a focused Handoff from either endpoint without creating
  stored Relation rows.
- The CLI exposes `--expected-scope-links`, allowing future dogfood scripts to
  assert this reviewability behavior directly. Existing relation filters still
  apply only to stored `relation_edges`.
- Generic Handoff scope is not traversed; only recognized focused Handoff scope
  participates in `scope_links`.
- Malformed v1 and future-version Handoff scope is skipped by this auxiliary
  `why` path, so it does not block unrelated Entity explanation.
- Remaining gaps are broader `why` maturity, larger Store evidence, and use on
  another real project.
