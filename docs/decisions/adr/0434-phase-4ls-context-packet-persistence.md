# ADR-0434: Phase 4LS Context Packet Persistence

Status: Accepted
Date: 2026-09-01

## Context

After Phase 4LR, the V1 readiness ledger still classified the Context Resolver
as Partial. Explicit packet scope and path-sensitive Knowledge filtering were
available, but a continuation Agent could only read a transient packet from
stdout. There was no durable way to save, list, and show the exact resolved
packet used for a handoff or continuation decision.

The remaining gap was not a new WorkState authority. It was immutable runtime
provenance: an inspectable record of the resolved packet, anchored to the
Session, Workspace, Branch, head Commit, and state digest observed at resolve
time.

## Decision

Add an immutable `context_packet_snapshot` table and expose it through core,
Engine, and CLI APIs:

```text
workvcs context-packet save STORE --session SESSION [--profile PROFILE] [--budget-items N] [--scope-json OBJECT]
workvcs context-packet show STORE --packet CONTEXT_PACKET_ID
workvcs context-packet list STORE --session SESSION [--limit N]
```

Saving a packet resolves the same `ContextPacket` used by transient
`workvcs context ... --profile/--budget-items/--scope-json`, then stores:

- `context_packet_id`;
- `session_id`, `workspace_id`, `branch_id`, `head_commit_id`, and
  `state_digest`;
- `profile`, optional `budget_items`, and optional canonical `scope_json`;
- `available_items`, `item_count`, and `omitted_items`;
- `packet_digest`; and
- canonical `packet_json`.

The packet digest is domain separated with `context-packet-snapshot-v1` and is
computed from canonical `packet_json`. Re-loading a snapshot verifies that the
stored digest still matches the stored packet JSON and that denormalized
columns still match the packet JSON envelope, profile, budget, scope,
available count, item count, and omitted count.

Context packet snapshots are append-only runtime provenance. They do not move a
Branch head, create a WorkStateCommit, create an Event, create or mutate a
Claim, or become an `object_identity` anchor for semantic relations.

Because this slice adds an additive physical table and indexes to
`schema-v0.1.sql` while preserving StoreManifest `schema_version=1`, ordinary
`Engine::open` remains exact and does not silently migrate pre-4LS Stores. A
Store whose schema object set is recognized as current schema minus only the
4LS snapshot table and indexes can be recovered explicitly with:

```text
workvcs store migrate-context-packet-snapshot STORE
```

That command applies only the missing context packet snapshot schema objects
and records a StoreMigrationAttempt outcome. Any other schema mismatch still
fails closed.

## Non-Goals

- No change to default `workvcs context STORE --session SESSION` overview
  behavior.
- No replacement of WorkState, Branch, Claim, Event, or Verification authority.
- No transition-rationale projection decision.
- No implicit path extraction, ranking, embeddings, vector search, or
  LLM-based retrieval.
- No Resource path normalization or adapter-backed re-observation.
- No packaged release or daemon behavior.

## Evidence

Core regression coverage:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_snapshots_persist_scope_json_and_stable_packet_digest --quiet
```

CLI regression coverage:

```text
cargo test -p workvcs-cli cli_saves_shows_and_lists_context_packet_snapshots --quiet
```

Schema validation:

```text
scripts/validate-schema-v0.1.sh
```

Compatibility and corruption regression coverage:

```text
cargo test -p workvcs-core --test store_phase2 explicit_context_packet_snapshot_schema_migration_recovers_pre_4ls_store --quiet
```

The context packet snapshot test also corrupts a denormalized count and proves
that snapshot load fails even though `packet_json` and `packet_digest` remain
self-consistent.

Final validation passed:

```text
git_diff_check=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/git_diff_check.log
cargo_fmt=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cargo_fmt.log
schema=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/schema.log
cargo_clippy=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cargo_clippy.log
cargo_test=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cargo_test.log
cli_smoke=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cli_smoke.log
```

Local dogfood passed through the real CLI:

```text
phase4ls_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/dogfood/phase-4ls-20260901T002106Z.sqlite
log_file=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/logs/phase-4ls/context-packet-persistence.20260901T002106Z/run.log
session_id=01a05a57-8c8d-79c2-a5fb-cd2fedcd302f
context_packet_id=01a05a57-8fdb-7223-8ddd-4e31e2e53f64
packet_digest=6d50bc202e1fa984d799de16bbf92c40bacac8fd4981fa74f88fe431f503b81e
head_commit_id=01a05a57-8952-7951-9364-5491f1ac8844
context_scope_json={"path":"crates/workvcs-core/src/runtime/context.rs"}
context_items=6
context_packets=1
```

Explicit migration dogfood passed through the real CLI:

```text
phase4ls_migration_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/dogfood/phase-4ls-migration-20260901T003605Z.sqlite
log_file=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/logs/phase-4ls/context-packet-migration.20260901T003605Z/run.log
migrated=true
added_schema_objects=3
migration_id=01a05a65-23f1-7df2-a926-e0a02eb0b9c8
schema_version=1
```

## Consequences

Agents can now persist the exact scoped, budgeted context packet used for a
continuation step and later prove what was visible at that point. This closes
the Context Resolver persistence gap without expanding into automatic
retrieval, Agent orchestration, or a second state authority.

Operators have an explicit local migration command for Stores created after the
current schema/migration tables existed but before Phase 4LS added context
packet snapshots. This is intentionally narrow and does not create a general
silent migration path.

The broader Context Resolver remains Partial until the transition-rationale
projection decision is closed. Resource path normalization and adapter-backed
re-observation remain separate V1 gaps.
