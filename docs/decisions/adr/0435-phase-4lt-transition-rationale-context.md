# ADR-0435: Phase 4LT Transition Rationale Context Projection

Status: Accepted
Date: 2026-09-01

## Context

After Phase 4LS, the V1 readiness ledger still classified the Context Resolver
as Partial because Agents could see current state and persisted packets, but
not the rationale behind recent important transitions. That kept continuation
work dependent on separate history and ChangeSet lookups.

The confirmed architecture already treats major transitions as rationale- or
causal-anchor-bearing operations. The existing `changeset` table and
`ChangeSetSnapshot` query surface already store canonical `rationale_json`,
digests, operation metadata, event counts, causal anchor counts, and commit
anchors. The missing work is projection into the Agent-facing context packet,
not a new semantic entity or storage authority.

## Decision

Add a deterministic `transition_rationale` ContextPacket item category.

The resolver reads the active Branch head's first-parent history, bounded to
the 8 most recent commits. For each ChangeSet in that window, it projects an
item only when `rationale_json` is not the canonical empty object. Empty
rationale objects remain invisible context because they do not explain a
transition.

Each projected item is:

- priority `P3`;
- category `transition_rationale`;
- subject `changeset:<CHANGESET_ID>@<COMMIT_ID>`; and
- a summary containing commit id, ChangeSet id, commit kind, operation type,
  schema version, commit/ChangeSet timestamps, rationale digest, rationale
  size, change operation count, causal anchor count, event count, origin
  Session, and the canonical `rationale_json`.

The category is available in `brief`, `normal`, and `full` profiles. Packet
budgets still trim whole items and report omitted `transition_rationale` items
through the existing omission summary.

The projection uses existing history provenance only. It does not create
ChangeSets, move Branch heads, mutate Sessions or Claims, write Events, add a
schema table, or change context packet snapshot storage. Saved context packets
automatically persist the projected items because snapshots store canonical
`packet_json`.

## Non-Goals

- No schema change or migration.
- No new rationale write path.
- No inferred rationale from Task descriptions, transcripts, command output,
  shell history, or Agent messages.
- No LLM extraction, ranking, embeddings, vector search, or semantic
  retrieval.
- No new relation semantics or `why` relation edge.
- No expansion beyond the bounded first-parent history window.
- No claim that Context Resolver is release mature or fully dogfood complete.

## Evidence

Targeted validation passed:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_includes_recent_transition_rationales --quiet
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_profiles_filter_categories_deterministically --quiet
cargo test -p workvcs-core --test context_profile_budget_phase4kx --quiet
cargo test -p workvcs-cli cli_context_packet_projects_transition_rationale --quiet
```

Local dogfood passed through the real CLI:

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

## Consequences

Agents can now use bounded packet output to understand why recent state moved,
without issuing a separate history/changeset lookup for the common
continuation case. This closes the specific transition-rationale projection
decision while preserving the V1/V2 boundary and keeping context snapshots as
immutable packet provenance.

The next implementation focus should move back to real dogfood friction:
manual key-value capture, another-project context/claim/handoff repetition,
and Resource path normalization or adapter-backed re-observation only when a
concrete continuation path needs them.
