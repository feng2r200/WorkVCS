# ADR-0433: Phase 4LR Context Path-Sensitive Knowledge

Status: Accepted
Date: 2026-09-01

## Context

After Phase 4LQ, the V1 readiness ledger still identified the Context Resolver
as Partial. The most user-visible gap was path-sensitive Knowledge handling:
normal context packets included every active Knowledge item, even when a caller
was working on a narrow file or resource path.

The current runtime has explicit `scope` objects for Knowledge and Records, but
Session focus only stores entity paths. It does not store a file path, resource
path, or active textual focus that can be safely interpreted as a path. Parsing
Task descriptions would make context behavior implicit and brittle.

## Decision

Add explicit packet scope to `ContextPacketOptions` and expose it through:

```text
workvcs context STORE --session SESSION --scope-json <object>
workvcs claim next STORE --session SESSION --context-scope-json <object>
```

The packet scope is optional and must be a canonical object. If omitted, context
packet behavior remains compatible: all active Knowledge items remain eligible.

When packet scope is provided, only `scoped_knowledge` packet items are filtered.
Overview output, Knowledge list/show behavior, Record summaries, Runnable
projection, and packet budget accounting remain otherwise unchanged.

The path policy is deterministic and conservative:

- empty Knowledge scope remains global and visible;
- Knowledge scope with no recognized path selector remains visible;
- absent packet scope, or packet scope with no recognized path selector,
  preserves existing all-active-Knowledge behavior;
- recognized selectors are `path`, `paths`, `path_prefix`, and
  `path_prefixes`;
- `scope_payload` is inspected for the same selectors to support existing
  resource-style scopes;
- exact paths match exact paths;
- a path matches a prefix when it equals the prefix or is under that slash
  boundary; and
- prefix-prefix overlap matches when either prefix contains the other at a
  slash boundary.

Filtered-out Knowledge is not reported as budget omission. It is outside the
resolved packet scope, not a lower-priority item trimmed by the item budget.

## Non-Goals

- No schema change.
- No context packet persistence.
- No path normalization, glob handling, symlink resolution, or resource adapter
  lookup.
- No implicit path extraction from Task descriptions, transcript text, shell
  commands, or Agent messages.
- No ranking, embeddings, vector search, or LLM-based retrieval.
- No filtering of overview Knowledge counts or `knowledge list/show`.
- No transition-rationale projection decision.
- No release-readiness claim for the full Context Resolver.

## Evidence

Core regression coverage:

```text
cargo test -p workvcs-core --test context_profile_budget_phase4kx
```

The core test `context_packet_filters_path_scoped_knowledge_by_explicit_scope`
proves that an unscoped packet still includes all active Knowledge, while a
path-scoped packet includes global Knowledge, matching exact path Knowledge,
matching prefix Knowledge, resource `scope_payload.path` Knowledge, and
non-path-scoped Knowledge, while excluding unrelated path-scoped Knowledge.

CLI regression coverage:

```text
cargo test -p workvcs-cli cli_context_packet_filters_path_scoped_knowledge_by_scope_json
cargo test -p workvcs-cli cli_claim_next_can_return_budgeted_context_packet
```

Local dogfood passed through the real CLI:

```text
phase4lr_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lr-context-path-sensitive-knowledge/.work-governance/runtime/dogfood/phase-4lr-20260831T234745Z.sqlite
log_file=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lr-context-path-sensitive-knowledge/.work-governance/runtime/logs/phase-4lr/context-path-scope.VpmFhc/run.log
session_id=01a05a38-d0ee-7443-a971-384f19815642
task_id=01a05a38-d09f-7b20-8ca1-4747dd30b8ae
context_scope_json={"path":"crates/workvcs-core/src/runtime/context.rs"}
context_items=7
claim_next_context_scope_json={"path":"crates/workvcs-core/src/runtime/context.rs"}
claim_next_context_items=7
```

## Consequences

The Context Resolver now has a usable, deterministic, path-sensitive Knowledge
slice for real CLI dogfood. Agents can pass an explicit current path scope
without needing a separate Knowledge query and without relying on implicit text
parsing.

The broader Context Resolver remains Partial until packet persistence and the
transition-rationale projection decision are closed. Resource path
normalization and adapter-backed re-observation also remain outside this slice.
