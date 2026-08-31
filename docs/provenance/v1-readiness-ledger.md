# V1 Readiness Ledger

Status: current implementation-readiness ledger
Last refreshed: 2026-08-31 by ADR-0414 / Phase 4KY

This ledger tracks the current WorkVCS V1 implementation state. It is an
evidence map, not a product specification. Confirmed product, architecture,
schema, and ADR documents remain the authority for what WorkVCS means.

The ledger exists to correct implementation focus: later slices should close
V1 readiness and dogfood gaps before adding more narrow query-detail or
expectation-only work.

## Classification

- **Design confirmed:** repository product, architecture, schema, or accepted
  ADRs confirm the capability semantics.
- **Implemented:** the current local Rust core or CLI exposes the capability.
- **Smoke proven:** repository automation exercises the capability at the
  process boundary, usually through `scripts/smoke-v0.1-cli-workflow.sh`.
- **Dogfood proven:** the capability has been used in a real WorkVCS-style
  continuation or handoff loop outside a throwaway smoke Store.
- **Open / next action:** the most concrete remaining V1 gap.

`Partial` means some sub-capabilities are covered while the named V1 area is
not complete.

## Current Ledger

| V1 area | Design confirmed | Implemented | Smoke proven | Dogfood proven | Open / next action |
| --- | --- | --- | --- | --- | --- |
| Canonical IDs, digests, and WorkState hashing | Yes | Yes | Partial | No | Keep as regression foundation; no further work unless another V1 slice exposes a concrete compatibility gap. |
| Store bootstrap, open, manifest, lineage, and doctor | Yes | Yes | Yes | No | Add install/use documentation and a larger Store validation run so the operator path is not only a temporary smoke Store. |
| Workspace, Branch, history, show-at, diff, and restore | Yes | Yes | Partial | No | Create a real dogfood walkthrough that branches and restores Work State for an implementation slice. |
| Goal, Plan, Task, ordering, dependencies, and containment | Yes | Yes | Partial | No | Use WorkVCS itself to manage a nontrivial implementation Plan, then record the missing ergonomics. |
| Acceptance Criteria, Verification Requirements, Verification, and Evidence closure | Yes | Yes | Yes | Partial | Use the `verify` wrapper in more durable implementation handoffs and close the post-verification cache refresh/recovery path exposed by Phase 4KY dogfood. |
| Verification command wrapper | Yes | Yes | Yes | Partial | Extend beyond caller-supplied observation data only after Resource adapter/path normalization is confirmed; keep multi-target, shell execution, and LLM extraction outside V1 unless re-authorized. |
| Resource registration, observation, applicability, and drift | Yes | Partial | Yes | Partial | Close Resource path/glob normalization and add a clearer recovery flow for Resource-backed Verification cache staleness after later branch-head commits. |
| Session start/end/focus, Runnable projection, `claim next`, and `next` | Yes | Yes | Yes | No | Dogfood the select-and-continue loop with real implementation work instead of only script-generated Tasks. |
| Claim modes and guard behavior | Yes | Partial | Partial | No | Shared claims exist; stale takeover, transfer, and force provenance remain V1 gaps. |
| Context resolver | Yes | Partial | Yes | Partial | Complete the remaining V1 context categories and dogfood flow: AC packets, Goal/Plan path packets, richer blocker context, Attempt details, focused Handoff reading, path-sensitive Knowledge policy, packet persistence, and claim-next packet rendering. |
| Record, Decision, Knowledge, and `why` neighborhoods | Yes | Yes | Partial | No | Use real Findings/Decisions/Handoffs during implementation and inspect `why` output for continuation quality. |
| Handoff | Yes | Partial | No | No | Move beyond explicit `Record(kind=handoff)` creation toward Session-end diff plus focused handoff authoring/reading. |
| Merge lifecycle | Yes | Yes | Yes | No | Dogfood divergent Work Branch resolution and document recovery behavior for moved heads or unresolved items. |
| Checkpoint and Bundle portability | Yes | Yes | Yes | No | Validate a real export/import/restore path and record Bundle container/profile details still Open for V1. |
| CLI discoverability and operator use | Partial | Partial | Partial | No | Add install, quickstart, and common recovery documentation tied to the current runnable command surface. |
| Actionable errors and recovery | Yes | Partial | Partial | No | Audit common failures and document safe next actions; add behavior only where current errors block dogfood. |
| Larger Store and performance evidence | Partial | No | No | No | Run a representative larger Store workload before adding indexes or claiming scale readiness. |

## Dogfood-Biased Next Queue

Use this queue when selecting the next local implementation slice unless a
current user request supplies a narrower priority.

1. Close the cache refresh/recovery flow exposed by Phase 4KY dogfood: after a
   later WorkState commit advances a branch head, a Resource-backed Verification
   needs an explicit, ergonomic applicability refresh path.
2. Close Handoff beyond `Record(kind=handoff)`: Session-end diff, focused
   handoff creation, and handoff consumption.
3. Add stale Claim takeover, transfer, and force provenance, then dogfood a
   blocked-claim recovery path.
4. Add install, quickstart, and recovery documentation for a local operator.
5. Complete the remaining V1 Context Resolver categories when the verify and
   handoff surfaces can supply real packet content.
6. Run larger Store validation before designing performance indexes.

Narrow smoke expectation, list/detail, count, and display-only slices are still
valid when they are required for one of the gaps above. They should name the
ledger row they advance.

## V2 Exclusions

The following remain beyond V1 even if they would make dogfood easier:

- transcript parsing or automatic extraction of Findings and Decisions;
- LLM-generated semantic records without explicit Agent confirmation;
- embeddings, vector search, or semantic retrieval;
- LLM-based semantic merge or natural-language conflict detection;
- automatic knowledge distillation;
- hooks that infer and prompt for possible semantic records;
- Agent launching, scheduling, orchestration, or automatic execution;
- cloud synchronization, distributed collaboration, and live cross-Store
  federation;
- destructive compaction of core Decision, Finding, Knowledge, ChangeSet, or
  WorkStateCommit history; and
- required TUI, GUI, or human-first storage.

## Evidence Snapshot

- `workvcs --help` currently exposes the broad V0.1 command surface, including
  Store, history, Workspace, Branch, Goal, Plan, Task, AC/VR, Evidence,
  Resource, Record, Session, Claim, Context, Next, Runnable, the high-level
  `verify` wrapper, Verification, Projection, Bundle, Checkpoint, and Merge
  command families.
- `scripts/smoke-v0.1-cli-workflow.sh` is the repository process-level smoke
  entrypoint and has been extended through ADR-0414 to cover integrity,
  scheduling/claim-next, context profile/budget packets, Merge, Checkpoint,
  Bundle, divergence, and VR-backed AC closure through the `verify` wrapper.
- The core test inventory currently contains focused tests for the major
  implemented V1 areas.
- Current smoke Stores are temporary and scripted. Phase 4KX adds a local
  dogfood Store for context packet budget behavior. Phase 4KY adds a local
  dogfood Store for the single-target `verify` wrapper and records one concrete
  recovery gap: after a Task completion advances the branch head, Resource-backed
  Verification applicability is stale until the cache is refreshed for the new
  head. WorkVCS has still not been used as the durable state system for a
  complete implementation slice or for another real project.
