# Phase 4LS Context Packet Persistence Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LS closes the V1 Context Resolver packet-persistence gap by adding a
durable, inspectable snapshot for resolved context packets.

This evidence does not claim full Context Resolver completion,
transition-rationale projection, Resource path normalization, adapter-backed
re-observation, implicit path extraction, ranking, embeddings, LLM retrieval,
another-project dogfood, or release readiness.

## Behavior

The new CLI contract is:

```text
workvcs context-packet save STORE --session SESSION [--profile PROFILE] [--budget-items N] [--scope-json OBJECT]
workvcs context-packet show STORE --packet CONTEXT_PACKET_ID
workvcs context-packet list STORE --session SESSION [--limit N]
```

`save` resolves the same packet shape used by transient `workvcs context`
packet output and stores immutable metadata plus canonical `packet_json`.
`show` returns the snapshot metadata and packet JSON. `list` returns snapshots
for one Session in newest-first order.

Snapshot rows record the Session, Workspace, Branch, head Commit, state digest,
profile, optional item budget, optional scope, item counts, omitted count,
packet digest, canonical packet JSON, and creation time.

Loading a snapshot verifies both:

- `packet_digest == digest(canonical packet_json)` under the
  `context-packet-snapshot-v1` domain; and
- denormalized table columns match the canonical packet JSON envelope, profile,
  budget, scope, available count, item count, and omitted count.

The snapshot is provenance, not WorkState authority. It does not move Branch
heads, create WorkState commits, create Events, create Claims, or change
Runnable selection.

For Stores whose object set is recognized as pre-4LS only because it is
missing `context_packet_snapshot` and its two indexes, the explicit recovery
command is:

```text
workvcs store migrate-context-packet-snapshot STORE
```

The command is not part of ordinary `Engine::open`; unrelated schema drift
continues to fail closed.

## Dogfood Run

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence
```

The local dogfood Store and log were:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/dogfood/phase-4ls-20260901T002106Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/logs/phase-4ls/context-packet-persistence.20260901T002106Z/run.log
```

The dogfood Store contained:

- one pending Task;
- one matching path-scoped Knowledge item; and
- one unrelated path-scoped Knowledge item.

The operator saved a normal, budgeted, path-scoped packet, showed it by packet
id, listed it by Session, and ran `doctor --require-valid`. The shown packet
digest matched the saved digest, the list entry matched the same snapshot, the
packet JSON included the matching Knowledge item, and the unrelated path
Knowledge was absent.

Summary:

```text
phase4ls_dogfood_result=PASS
session_id=01a05a57-8c8d-79c2-a5fb-cd2fedcd302f
context_packet_id=01a05a57-8fdb-7223-8ddd-4e31e2e53f64
packet_digest=6d50bc202e1fa984d799de16bbf92c40bacac8fd4981fa74f88fe431f503b81e
head_commit_id=01a05a57-8952-7951-9364-5491f1ac8844
context_scope_json={"path":"crates/workvcs-core/src/runtime/context.rs"}
context_items=6
context_packets=1
```

An explicit migration dogfood run simulated a pre-4LS Store by removing only
the context packet snapshot table and indexes, confirmed ordinary Store inspect
failed, ran `workvcs store migrate-context-packet-snapshot`, and then confirmed
`store info` succeeded:

```text
phase4ls_migration_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/dogfood/phase-4ls-migration-20260901T003605Z.sqlite
log_file=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ls-context-packet-persistence/.work-governance/runtime/logs/phase-4ls/context-packet-migration.20260901T003605Z/run.log
migrated=true
added_schema_objects=3
migration_id=01a05a65-23f1-7df2-a926-e0a02eb0b9c8
schema_version=1
```

## Validation

Targeted validation passed:

```text
cargo fmt --all
cargo test -p workvcs-core --test context_profile_budget_phase4kx context_packet_snapshots_persist_scope_json_and_stable_packet_digest --quiet
cargo test -p workvcs-core --test store_phase2 explicit_context_packet_snapshot_schema_migration_recovers_pre_4ls_store --quiet
cargo test -p workvcs-cli cli_saves_shows_and_lists_context_packet_snapshots --quiet
cargo test -p workvcs-cli cli_context_packet_snapshot_schema_migration_reports_noop_for_current_store --quiet
scripts/validate-schema-v0.1.sh
```

Final validation passed:

```text
git_diff_check=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/git_diff_check.log
cargo_fmt=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cargo_fmt.log
schema=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/schema.log
cargo_clippy=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cargo_clippy.log
cargo_test=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cargo_test.log
cli_smoke=PASS log=/tmp/workvcs-4ls-final-validation-20260901T003650Z-14550/cli_smoke.log
```

The core test saves the same resolved packet twice and proves that snapshot IDs
differ while the packet digest and packet JSON remain stable. It also verifies
scope persistence, stored counts, load-by-id behavior, list-by-session
behavior, zero-limit rejection, and fail-closed behavior when a denormalized
count no longer matches `packet_json`.

The CLI test proves the user-facing save/show/list path and verifies that a
shown packet preserves the saved digest and scoped Knowledge filtering.

The store test simulates a pre-4LS Store by removing only the context packet
snapshot table and indexes, proves ordinary open fails, then proves the
explicit migration command path restores exact schema validation and records a
StoreMigrationAttempt outcome.

Independent review initially found one High compatibility issue and one Medium
consistency issue. Both were fixed; the same reviewer then confirmed no
blocker/high/medium residual findings.

## Findings

- Packet persistence is append-only provenance.
- Digest verification is tied to canonical `packet_json`, and loader
  validation rejects table-column/packet-JSON disagreement.
- The default transient overview path remains backward compatible.
- Existing pre-4LS Stores need explicit local migration before ordinary open.
- Path-sensitive filtering remains a resolver concern; filtered-out Knowledge
  is not a budget omission.
- The remaining Context Resolver gap is the transition-rationale projection
  decision.
