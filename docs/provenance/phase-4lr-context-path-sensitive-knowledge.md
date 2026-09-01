# Phase 4LR Context Path-Sensitive Knowledge Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LR closes one V1 Context Resolver gap by making Knowledge packet
selection path-sensitive when the caller supplies an explicit context scope.

This evidence does not claim full Context Resolver completion, context packet
persistence, transition-rationale projection, Resource path normalization,
glob matching, implicit path extraction, ranking, embeddings, LLM retrieval,
another-project dogfood, or release readiness.

## Behavior

The new CLI contract is:

```text
workvcs context STORE --session SESSION --scope-json <object>
workvcs claim next STORE --session SESSION --context-scope-json <object>
```

The scope object is echoed in packet output as:

```text
context_scope_json=...
claim_next_context_scope_json=...
```

The resolver recognizes these selectors in both packet scope and Knowledge
scope:

```text
path
paths
path_prefix
path_prefixes
scope_payload.path
scope_payload.paths
scope_payload.path_prefix
scope_payload.path_prefixes
```

Empty Knowledge scope, non-path Knowledge scope, absent packet scope, and packet
scope without recognized path selectors preserve the previous inclusive
behavior.

## Dogfood Run

Commands were run from:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lr-context-path-sensitive-knowledge
```

The local dogfood Store and log were:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lr-context-path-sensitive-knowledge/.work-governance/runtime/dogfood/phase-4lr-20260831T234745Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lr-context-path-sensitive-knowledge/.work-governance/runtime/logs/phase-4lr/context-path-scope.VpmFhc/run.log
```

The dogfood Store contained:

- one pending Task;
- one matching path-scoped Knowledge item;
- one global Knowledge item; and
- one unrelated path-scoped Knowledge item.

The operator requested both a scoped context packet and a scoped claim-next
packet. Both packets included the matching and global Knowledge items and
excluded the unrelated path-scoped Knowledge item.

Summary:

```text
phase4lr_dogfood_result=PASS
session_id=01a05a38-d0ee-7443-a971-384f19815642
task_id=01a05a38-d09f-7b20-8ca1-4747dd30b8ae
context_scope_json={"path":"crates/workvcs-core/src/runtime/context.rs"}
context_items=7
claim_next_context_scope_json={"path":"crates/workvcs-core/src/runtime/context.rs"}
claim_next_context_items=7
```

## Validation

Targeted validation passed:

```text
cargo fmt --all
cargo test -p workvcs-core --test context_profile_budget_phase4kx
cargo test -p workvcs-cli cli_context_packet_filters_path_scoped_knowledge_by_scope_json
cargo test -p workvcs-cli cli_claim_next_can_return_budgeted_context_packet
cargo build -p workvcs-cli --quiet
```

The core context test includes a scoped packet with an item budget, proving that
the unrelated path-scoped Knowledge is outside `available_items` and is not
counted as a budget omission.

Validation logs:

```text
/tmp/workvcs-4lr-cargo-fmt.log
/tmp/workvcs-4lr-core-context-tests.log
/tmp/workvcs-4lr-cli-context-scope-test.log
/tmp/workvcs-4lr-cli-claim-next-scope-test.log
/tmp/workvcs-4lr-cli-build.log
```

## Findings

- The default packet path remains backward compatible because absent packet
  scope includes all active Knowledge.
- Packet scope affects `scoped_knowledge` items only.
- Path-sensitive filtering happens before item-budget trimming and is not
  counted as budget omission.
- The policy intentionally ignores arbitrary JSON fields to avoid accidental
  path inference.
- The remaining Context Resolver gaps are context packet persistence and the
  transition-rationale projection decision.
