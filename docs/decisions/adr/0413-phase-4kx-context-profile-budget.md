# ADR-0413: Phase 4KX Context Profile Budget

- **Status:** Accepted for implementation
- **Accepted by:** active Phase 4KX implementation Plan and the repository
  V1 readiness ledger.

## Context

The V1 readiness ledger identifies `context` profile and hard-budget behavior
as the highest-priority dogfood gap after Phase 4KW. Earlier context slices
made the current Session anchor, Branch head, runnable candidates, Records,
Knowledge, and relation summaries visible through a read-only overview. That
was useful, but it left `context` shaped like a broad dump instead of a
bounded Agent-facing packet.

The confirmed V1 architecture requires deterministic `brief`, `normal`, and
`full` profiles, the `P0` through `P9` priority order, and budget trimming by
whole item with omission summaries. It also excludes LLM inference, embedding
retrieval, transcript parsing, Agent orchestration, and unrelated runtime
semantics from V1 implementation.

## Decision

1. Add `ContextPacketOptions`, `ContextPacket`, `ContextPacketEnvelope`,
   `ContextProfile`, `ContextPriority`, `ContextItem`, typed item
   categories/subjects, and omission summary buckets to `workvcs-core`.
2. Build packets from the existing read-only `ContextOverview` surface. The
   packet implementation does not query through CLI-specific shortcuts and
   does not write Events, runtime rows, ChangeSets, WorkStateCommits, or
   Branch heads.
   `ContextPacket` exposes only a bounded envelope plus retained items and
   omission summaries; it must not expose the complete overview through the
   public packet API, because that would let callers bypass profile and budget
   semantics.
3. Implement profiles as deterministic category filters:
   - `brief`: Session/Branch anchor, current Task candidates, Task readiness,
     active Decisions, current non-invalidated Assumptions, and failed
     Attempts.
   - `normal`: `brief` plus direct causal-chain relation summaries, Findings,
     non-failed Attempts, scoped active Knowledge, Session continuity, and
     Handoff records.
   - `full`: `normal` plus remaining current cognition/provenance categories
     available through `ContextOverview`.
4. Assign implemented item categories to the confirmed V1 priority order:
   - `P0`: Session/Branch anchor and current Task candidates.
   - `P2`: Task readiness, including lifecycle, dependency, Claim, blocker,
     and unsatisfied dependency summaries.
   - `P3`: direct relation summaries already exposed by context overview.
   - `P4`: active Decision and Assumption records.
   - `P5`: Attempt records, with failed Attempts preserved as the brief-eligible
     category.
   - `P6`: Findings.
   - `P7`: scoped active Knowledge.
   - `P8`: Session continuity and Handoff records.
   - `P9`: older or otherwise full-only current provenance.
5. Interpret the CLI budget as `--budget-items`: a hard maximum number of
   context items after profile filtering. Zero is invalid. The budget removes
   complete lower-priority items and reports omitted counts by priority and
   category. Items are sorted by `P0` through `P9` priority while preserving
   each deterministic source order inside the same priority, including
   runnable candidate order. It does not truncate item summaries or strings.
6. Keep `workvcs context STORE --session <id>` on the existing overview output
   for compatibility. The packet renderer is opt-in through `--profile` and/or
   `--budget-items`.
7. Extend the CLI smoke workflow to exercise the packet path and zero-budget
   failure.

## Non-Goals

- No schema changes.
- No full AC/Goal/Plan path resolver.
- No path-sensitive Knowledge ranking beyond existing context overview data.
- No context packet persistence.
- No atomic `claim-next` packet rendering.
- No deterministic `verify` wrapper.
- No Claim takeover, transfer, or force provenance.
- No transcript parsing, LLM semantic extraction, embeddings, GUI/TUI, remote,
  deployment, or release behavior.

## Consequences

- `context` now has a reusable Agent-facing packet structure and bounded output
  behavior, while the older overview stays stable for scripts.
- Budget evidence becomes inspectable through deterministic omission summaries
  instead of process-level text truncation.
- The Context Resolver remains `Partial`: this slice closes the profile and
  item-budget mechanism, not every V1 context category.

## Implementation Findings

- The existing `ContextOverview` contained enough current Task, readiness,
  Record, Knowledge, relation, and Session-continuity data to implement the
  first packet mechanism without a schema change.
- The currently implemented overview does not yet expose complete V1 context
  families such as full Acceptance Criteria packets, Goal/Plan path packets,
  richer blocker records, Attempt execution details, deeper Handoff
  consumption, or persisted Agent protocol encoding. Those remain V1 follow-up
  work and must not be overclaimed as complete by this ADR.
