# ADR-0425: Phase 4LJ Context AC and VR Packet Items

Status: Accepted
Date: 2026-09-01

## Context

ADR-0413 introduced bounded `ContextPacket` profiles and budgets, but left full
Acceptance Criteria packets as a V1 follow-up. ADR-0414 added the single-target
`verify` wrapper, and ADR-0424 let `claim next` opt in to packet output after a
successful claim/focus update.

That made the next dogfood gap clearer: after claiming work, an Agent could see
the current Task but still had to issue separate `ac` and `vr` commands to learn
the concrete verification obligations needed by `workvcs verify`.

## Decision

Add current-task verification obligation items to `ContextPacket`:

- `acceptance_criterion`
- `verification_requirement`

The resolver builds these items from existing read-only snapshots at the active
Branch head:

- `acceptance_criteria_at`
- `verification_requirements_at`
- branch-scoped Acceptance Criterion effective status

No new schema, write transaction, Claim behavior, runnable ordering, or default
overview output is introduced.

The new items use priority `P1` and are included in the `brief` profile. This
keeps Session/Branch/Task anchors at `P0`, keeps Task readiness at `P2`, and
lets tight budgets retain the actionable verification targets before lower
priority readiness detail.

For each current runnable or focused Task candidate, packet items follow the
Task's Acceptance Criterion reference order, then each Criterion's Verification
Requirement reference order. The item subjects expose typed target identifiers:

```text
acceptance_criterion:<entity_id>
verification_requirement:<entity_id>
```

When surfaced through `claim next --context-*`, these appear with the existing
`claim_next_` output prefix.

If a current Task references an Acceptance Criterion whose stored task id or
local key does not match the Task reference, packet resolution fails closed. The
same guard is applied for Verification Requirement owner and local-key mismatch.

## Non-Goals

- No full Goal or Plan path packet resolver.
- No richer blocker explanation packet.
- No Attempt execution-detail packet.
- No path-sensitive Knowledge ranking.
- No context packet persistence or Agent protocol container.
- No automatic verification execution, shell adapter, filesystem adapter, or
  multi-target verify wrapper.

## Consequences

An Agent can now claim work and immediately see the concrete Acceptance
Criterion and Verification Requirement identifiers needed for the existing
`verify` wrapper. This closes the first AC/VR obligation packet gap while the
broader Context Resolver remains Partial.
