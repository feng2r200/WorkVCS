# V1 Readiness Ledger

Status: current implementation-readiness ledger
Last refreshed: 2026-09-01 by ADR-0421 / Phase 4LF

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
| Store bootstrap, open, manifest, lineage, and doctor | Yes | Yes | Yes | Partial | Phase 4LE creates a durable local dogfood Store for a full implementation closeout; next prove the path with a larger Store validation run. |
| Workspace, Branch, history, show-at, diff, and restore | Yes | Yes | Partial | No | Create a real dogfood walkthrough that branches and restores Work State for an implementation slice. |
| Goal, Plan, Task, ordering, dependencies, and containment | Yes | Yes | Partial | Partial | Phase 4LE uses a real WorkVCS Task through implementation closeout; still use WorkVCS itself to manage a nontrivial Goal/Plan. |
| Acceptance Criteria, Verification Requirements, Verification, and Evidence closure | Yes | Yes | Yes | Yes | Phase 4LE dogfoods AC/VR/Verification for an implementation closeout; repeat this in recovery and handoff-consumption scenarios. |
| Verification command wrapper | Yes | Yes | Yes | Partial | Extend beyond caller-supplied observation data only after Resource adapter/path normalization is confirmed; keep multi-target, shell execution, and LLM extraction outside V1 unless re-authorized. |
| Resource registration, observation, applicability, and drift | Yes | Partial | Yes | Partial | Resource-backed stale-cache recovery from baseline observations is implemented; Resource path/glob normalization and adapter-backed re-observation remain Open. |
| Session start/end/focus, Runnable projection, `claim next`, and `next` | Yes | Yes | Yes | Partial | Phase 4LF consumes a focused Handoff into a continuation Session and uses `next` to claim the continuation Task; still dogfood blocked recovery continuation. |
| Claim modes and guard behavior | Yes | Yes | Yes | Partial | Transfer, stale marking, and stale-gated forced takeover are implemented and smoke-covered; dogfood the full takeover recovery path in a real handoff. |
| Context resolver | Yes | Partial | Yes | Partial | Phase 4LF proves current Context is sufficient for focused Handoff continuation; complete remaining V1 categories: AC packets, Goal/Plan path packets, richer blocker context, Attempt details, path-sensitive Knowledge policy, packet persistence, and claim-next packet rendering. |
| Record, Decision, Knowledge, and `why` neighborhoods | Yes | Yes | Partial | Partial | Phase 4LF inspects `why` during Handoff continuation and finds zero relation edges for the Handoff focus link; decide that relation semantics before adding `why` output. |
| Handoff | Yes | Yes | Yes | Yes | Phase 4LF adds and dogfoods `handoff consume` as a focused continuation entrypoint; next prove blocked Handoff recovery with stale-gated Claim takeover. |
| Merge lifecycle | Yes | Yes | Yes | No | Dogfood divergent Work Branch resolution and document recovery behavior for moved heads or unresolved items. |
| Checkpoint and Bundle portability | Yes | Yes | Yes | No | Validate a real export/import/restore path and record Bundle container/profile details still Open for V1. |
| CLI discoverability and operator use | Partial | Partial | Partial | Partial | Phase 4LF reduces Handoff focus-copy friction with `handoff consume`; continue reducing manual key-value capture only where dogfood shows repeated friction. |
| Actionable errors and recovery | Yes | Partial | Partial | No | Audit common failures and document safe next actions; add behavior only where current errors block dogfood. |
| Larger Store and performance evidence | Partial | No | No | No | Run a representative larger Store workload before adding indexes or claiming scale readiness. |

## Dogfood-Biased Next Queue

Use this queue when selecting the next local implementation slice unless a
current user request supplies a narrower priority.

1. Dogfood Claim transfer, stale marking, and stale-gated takeover in a durable
   implementation recovery path and record missing ergonomics.
2. Decide whether Handoff focus belongs in `why` as a relation, using the Phase
   4LF zero-edge evidence before changing explanation semantics.
3. Complete the remaining V1 Context Resolver categories when the verify and
   handoff surfaces can supply real packet content.
4. Reduce manual key-value capture in the operator CLI only where the dogfood
   evidence shows repeated friction.
5. Run larger Store validation before designing performance indexes.

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
  head. Phase 4KZ adds explicit `verification cache-refresh` recovery for the
  baseline-observation case and replaces the manual `cache-record` recovery step
  in repository smoke. Phase 4LA adds focused `handoff create/show` commands
  over existing SessionDiff and `Record(kind=handoff)` semantics, and smoke now
  proves an end-session, handoff-author, handoff-consume loop. Phase 4LB adds
  explicit Claim transfer and forced takeover replacement operations over
  existing Claim occurrence/runtime/Event tables; smoke now proves a blocked
  guard, transfer recovery, and forced takeover recovery loop. Phase 4LC adds a
  local operator quickstart and recovery guide tied to the current CLI surface.
  Phase 4LD adds explicit `session mark-stale` support and smoke-proves
  `potentially_stale` show/list behavior. Phase 4LE makes forced Claim takeover
  stale-gated, fixes the previous Session lifecycle evidence, and dogfoods a
  real implementation Task/Session/Claim/AC/VR/Verification/SessionDiff/Handoff
  closeout. Phase 4LF dogfoods a focused Handoff as the continuation entrypoint,
  adds `handoff consume`, and records that `why` still does not expose the
  Handoff focus link as a relation. WorkVCS has still not been used to recover a
  real blocked handoff with stale-gated takeover or for another real project.
