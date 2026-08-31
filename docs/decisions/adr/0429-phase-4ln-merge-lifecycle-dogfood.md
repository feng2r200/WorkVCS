# ADR-0429: Phase 4LN Merge Lifecycle Dogfood

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger marked Merge lifecycle as design-confirmed,
implemented, and smoke-proven, but not dogfood-proven. The known risk was that
continued Phase 4K/4L work could over-focus on narrow CLI smoke expectations
instead of proving real operator workflows.

The confirmed V1 merge design is persistent and optimistic: `merge start`
captures base, target head, and source head; `merge continue` may only apply
after every item is resolved/frozen and both captured heads are unchanged. If a
head moves, V1 rejects continuation and requires restart, not automatic
recomputation.

## Decision

Accept Phase 4LN as the local dogfood proof for the current merge lifecycle.

The dogfood run used the real CLI and a durable local Store to prove:

- divergent Work Branch classification with one `CONFLICT` item and one `AUTO`
  source-only item;
- unresolved-item guard before freeze;
- explicit resolution and freeze for all items;
- target-head moved rejection before continue;
- source-head moved rejection before continue;
- aborting stale merge attempts without changing target Work State;
- restarting from current heads and completing a two-parent `merge.continue`
  commit;
- closed merge listing for both aborted and completed attempts;
- final `store integrity --require-valid` and `doctor --require-valid`.

No core or CLI behavior change is required by this dogfood slice.

## Non-Goals

- No schema change.
- No merge command spelling change.
- No automatic merge recomputation after moved heads.
- No semantic, natural-language, or LLM-assisted merge.
- No custom resolution application beyond the already deferred behavior.
- No GUI/TUI or Agent protocol container.
- No Bundle, Claim, Handoff, Verification, or Context Resolver change.
- No another-project or larger-Store claim.

## Consequences

The V1 readiness ledger can mark Merge lifecycle dogfood-proven for local
operator use. Remaining merge maturity work is narrower: repeat on another real
project or larger Store before release claims, and consider structured CLI
failure fields under the broader actionable-error backlog.

Operator recovery documentation now names the concrete V1 recovery loop:
inspect active merge, resolve every item, freeze, continue; if either captured
head moved, abort the stale attempt and restart from current heads.
