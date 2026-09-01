# Phase 4LT Transition Rationale Context Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LT closes the V1 Context Resolver transition-rationale projection gap by
surfacing existing non-empty ChangeSet rationale provenance as bounded
ContextPacket items.

This evidence does not claim full Context Resolver maturity, another-project
dogfood, Resource path normalization, adapter-backed re-observation, inferred
rationale, semantic ranking, LLM retrieval, new `why` relation semantics, or
release readiness.

## Behavior

The resolver now projects recent non-empty ChangeSet rationale into packet
items:

```text
category=transition_rationale
priority=P3
subject=changeset:<CHANGESET_ID>@<COMMIT_ID>
```

The projection reads only the active Branch head's first-parent history and is
bounded to the 8 most recent commits. It skips canonical empty rationale
objects, because empty provenance does not explain a transition.

Each item summary includes the commit id, ChangeSet id, commit kind, operation
type, schema version, commit/ChangeSet timestamps, rationale digest, rationale
size, change operation count, causal anchor count, event count, origin Session,
and canonical `rationale_json`.

The item category is included in `brief`, `normal`, and `full` profiles and is
trimmed by the existing whole-item budget logic. Saved context packet snapshots
persist the item through the existing canonical `packet_json`; no schema or
snapshot contract changed.

## Dogfood Run

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lt-transition-rationale-context
```

The local dogfood Store and log were:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lt-transition-rationale-context/.work-governance/runtime/dogfood/phase-4lt-20260901T005846Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lt-transition-rationale-context/.work-governance/runtime/logs/phase-4lt/transition-rationale-context.20260901T005846Z/run.log
```

The dogfood Store contained one Goal creation and one Goal achievement with a
non-empty rationale. The operator then started a Session, resolved a brief
budgeted packet, saved that packet, and showed the saved packet JSON.

The run proved:

- `context` included exactly one `transition_rationale` item;
- the item subject matched the Goal achievement ChangeSet and Commit;
- the item summary included `operation=entity.transition`;
- the item summary included the exact transition rationale text;
- `context-packet save` persisted the same packet shape; and
- `context-packet show` returned packet JSON containing the
  `transition_rationale` item and rationale text.

Summary:

```text
phase4lt_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lt-transition-rationale-context/.work-governance/runtime/dogfood/phase-4lt-20260901T005846Z.sqlite
log_file=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lt-transition-rationale-context/.work-governance/runtime/logs/phase-4lt/transition-rationale-context.20260901T005846Z/run.log
session_id=01a05a79-dff8-7f00-8aa9-f5c187da2b99
goal_entity_id=01a05a79-dfc5-79d0-aec0-d7f2d0baa147
transition_changeset_id=01a05a79-dfe3-74d0-9724-65d0c231b395
transition_commit_id=01a05a79-dfe3-74d0-9724-65e4cd7d33c9
context_items=3
context_packet_id=01a05a79-e030-76e1-b65d-26e5006934d5
packet_digest=10b33eb0f6a17aecdfa4c29925c367da07494cc1cbe310512e6be4a8acb93d7d
```

## Validation

Targeted validation passed:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_includes_recent_transition_rationales --quiet
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_profiles_filter_categories_deterministically --quiet
cargo test -p workvcs-core --test context_profile_budget_phase4kx --quiet
cargo test -p workvcs-cli cli_context_packet_projects_transition_rationale --quiet
```

The core test proves that:

- a later empty-rationale write does not hide a recent non-empty rationale;
- the projected item uses `transition_rationale`, priority `P3`, and
  `changeset:<CHANGESET_ID>@<COMMIT_ID>`;
- tight budget trimming omits the whole transition-rationale item and records
  the omission category; and
- saving a context packet persists the item through canonical packet JSON.

The CLI test proves the user-facing `context` and `context-packet save/show`
path against a real Goal achievement rationale.

## Findings

- Transition rationale projection is a read-only view of existing ChangeSet
  provenance.
- Empty rationale objects remain unprojected and do not consume packet budget.
- The projection is bounded by recent first-parent history and does not add a
  new query authority.
- Context Resolver is improved for continuation dogfood, but release maturity
  still depends on broader real-project repetition and remaining Resource
  resolver work.
