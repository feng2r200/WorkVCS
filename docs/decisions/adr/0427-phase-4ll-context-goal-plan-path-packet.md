# ADR-0427: Phase 4LL Context Goal/Plan Path Packet Items

Status: Accepted
Date: 2026-09-01

## Context

ADR-0413 introduced bounded `ContextPacket` profiles and budgets. ADR-0425
added current-task Acceptance Criterion and Verification Requirement items, and
ADR-0426 added blocked dependency items.

The remaining Goal/Plan path gap was still visible in the same packet surface:
an Agent could see current Task candidates but could not see the Goal and Plan
hierarchy that explained where a Task sat in the work structure. That forced a
separate Goal, Plan, or containment lookup before continuing work with enough
orientation.

## Decision

Add a `goal_plan_path` ContextPacket item for each current Task candidate whose
current primary containment ancestors include a Goal or Plan.

The item is built from existing read-only Goal snapshots, Plan snapshots, and
primary containment relation snapshots at the active Branch head. It does not
change schema, semantic write paths, runnable ordering, Claim behavior, the
`verify` wrapper, or the default non-packet context overview.

The item uses priority `P1` and is included in the `brief` profile. Its subject
binds the current Task:

```text
goal_plan_path:<task_entity_id>
```

The summary includes:

- current Task id
- root-to-leaf Goal/Plan path segments
- ancestor Goal and Plan ids
- ancestor lifecycle status
- ancestor descriptions
- primary containment relation ids along the path

If the current containment graph is internally inconsistent, or a referenced
Goal or Plan snapshot is absent from the current Work State, packet resolution
fails closed instead of rendering a misleading path item.

## Non-Goals

- No schema change.
- No new containment write behavior.
- No Claim or runnable ordering change.
- No use of empty `claim next` focus paths as authoritative hierarchy.
- No automatic path disambiguation beyond current primary containment.
- No Attempt execution-detail packet.
- No path-sensitive Knowledge ranking.
- No context packet persistence or Agent protocol container.

## Consequences

An Agent can now inspect bounded `claim next` or `context` packet output and
understand the containing Goal/Plan path for current Task candidates without
issuing a separate lookup. This closes the Goal/Plan path packet gap while the
broader Context Resolver remains Partial.
