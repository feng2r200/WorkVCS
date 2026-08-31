# ADR-0424: Phase 4LI Claim Next Context Packet

Status: Accepted
Date: 2026-09-01

## Context

ADR-0048 made `claim next` the atomic operation that selects the first
runnable unclaimed Task, writes the Claim, and updates Session focus. ADR-0051
added `next`, which returns post-claim context overview. ADR-0413 later added
bounded `ContextPacket` profile and item-budget behavior, but explicitly left
atomic claim-next packet rendering as a follow-up.

The V1 readiness ledger now prioritizes Context Resolver gaps that improve real
Agent dogfood over display-only CLI growth. Returning bounded context from
`claim next` removes one repeated Agent workflow step while keeping the
selection and Claim semantics unchanged.

## Decision

Add an explicit opt-in packet output path to `workvcs claim next`:

```text
workvcs claim next STORE \
  --session SESSION \
  --context-profile brief \
  --context-budget-items 20
```

When either context option is provided, the CLI resolves a `ContextPacket` for
the same Session after `claim_next_task` returns, then appends it to the normal
claim-next output with `claim_next_` prefixes:

```text
claim_next_context_packet=true
claim_next_context_profile=brief
claim_next_context_budget_items=20
claim_next_context_items=...
```

The context options reuse the existing `ContextPacketOptions`,
`ContextProfile`, and whole-item budget validation. They are validated before
the mutating `claim_next_task` call so an invalid packet option cannot write a
Claim first and fail afterward.

Default `claim next` output is unchanged when neither context option is
provided.

## Non-Goals

- No change to runnable ordering, selection, Claim creation, Claim mode, or
  Session focus semantics.
- No schema migration or context packet persistence.
- No new Engine transaction shape for this CLI convenience path.
- No automatic TaskStart, execution, verification, takeover, or retry.
- No new context item categories beyond the existing packet resolver.

## Consequences

Agents can now use one command to claim the next Task and receive a bounded
post-claim packet for continuation. Scripts that rely on the older claim-next
output remain compatible unless they opt in to context packet rendering.

The Context Resolver remains Partial. This closes claim-next packet rendering,
but AC packets, Goal/Plan path packets, richer blocker context, Attempt
execution detail, path-sensitive Knowledge policy, and packet persistence
remain V1 follow-up work.
