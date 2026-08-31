# ADR-0428: Phase 4LM Context Attempt Detail Packet Items

Status: Accepted
Date: 2026-09-01

## Context

ADR-0413 introduced bounded `ContextPacket` profiles and budgets. ADR-0425
through ADR-0427 added current-task verification obligations, blocked
dependency context, and Goal/Plan path orientation.

The remaining Attempt gap was visible in dogfood continuation paths: an Agent
could see a generic `attempt` or `failed_attempt` item, but the summary did not
make execution state, terminality, current version identity, scope, or nearby
relation counts explicit. That made recovery after failed or inconclusive
work depend on separate `record show` and relation lookups.

## Decision

Enrich existing Record-derived ContextPacket items for `RecordKind::Attempt`
with deterministic Attempt detail.

The packet keeps the existing categories:

- `failed_attempt` for failed Attempts, priority `P5`, brief-eligible;
- `attempt` for running, succeeded, and inconclusive Attempts, priority `P5`,
  normal/full eligible.

Attempt summaries now include:

- current Attempt status;
- whether the status is terminal;
- Attempt record id;
- current record version id;
- current state digest;
- canonical scope JSON;
- outgoing and incoming Record relation counts;
- deterministic Record relation type buckets;
- Attempt statement.

The implementation uses only data already loaded by `ContextOverview`: current
Record snapshots and Record relation snapshots. It does not add a schema field,
query path, write operation, Claim behavior, runnable ordering rule, verify
wrapper behavior, or CLI command.

Transition rationale text remains a write-path rationale and is not currently
part of `RecordSnapshot`; this slice does not invent a projection for it.

## Non-Goals

- No schema change.
- No Record lifecycle change.
- No attempt to reopen terminal Attempts.
- No new Attempt relation semantics.
- No transition-rationale projection.
- No Record-to-Knowledge relation detail for Attempts; current V1 semantics
  only allow Finding records to source those relations.
- No path-sensitive Knowledge ranking.
- No context packet persistence or Agent protocol container.
- No Claim, runnable ordering, or verify wrapper change.

## Consequences

An Agent can now inspect bounded `context` or `claim next` packet output and
see enough Attempt execution state to distinguish running, succeeded, failed,
and inconclusive Attempts without issuing separate Record lookups. Failed
Attempts remain brief-eligible critical context; other Attempt statuses remain
normal/full context.

The broader Context Resolver remains Partial because path-sensitive Knowledge
policy, packet persistence, and the explicit decision on transition-rationale
projection are still open.
