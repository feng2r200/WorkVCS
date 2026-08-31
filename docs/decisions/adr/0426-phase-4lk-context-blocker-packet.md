# ADR-0426: Phase 4LK Context Blocker Packet Items

Status: Accepted
Date: 2026-09-01

## Context

ADR-0413 introduced bounded `ContextPacket` profiles and budgets. ADR-0425
added current-task Acceptance Criterion and Verification Requirement items so
an Agent can see what must be verified after claiming or focusing work.

The remaining blocker-context gap was visible in the same packet surface:
`task_readiness` reported `dependency_ready=false` and listed unsatisfied
dependency Task ids, but it did not tell the Agent what those dependency Tasks
were. That forced an extra Task lookup before the Agent could continue with the
blocking work.

## Decision

Add a `blocked_dependency` ContextPacket item for each current Task candidate
and unsatisfied dependency Task id.

The item is built from existing read-only Task snapshots at the active Branch
head. It does not change schema, semantic write paths, runnable ordering, Claim
behavior, or the `verify` wrapper.

The item uses priority `P2` and is included in the `brief` profile with
`task_readiness`. Its subject binds both sides of the blocker:

```text
blocked_dependency:<blocked_task_entity_id>:<dependency_task_entity_id>
```

The summary includes:

- blocked Task id
- dependency Task id
- dependency Task status
- dependency Task priority
- dependency Task description

If the dependency Task snapshot is absent from the current Work State, packet
resolution fails closed instead of rendering a misleading blocker item.

## Non-Goals

- No automatic dependency resolution or Task transition.
- No new runnable selection semantics.
- No Claim takeover, transfer, or stale-session policy change.
- No full Goal or Plan path packet resolver.
- No Attempt execution-detail packet.
- No path-sensitive Knowledge ranking.
- No context packet persistence or Agent protocol container.

## Consequences

An Agent can now inspect a bounded packet and understand which Task blocks a
blocked candidate without issuing a separate Task lookup. This closes one
richer blocker-context gap while the broader Context Resolver remains Partial.
