# V1 Readiness Ledger

Status: current implementation-readiness ledger
Last refreshed: 2026-09-01 by ADR-0439 / Phase 4LX

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
| Store bootstrap, open, manifest, lineage, and doctor | Yes | Yes | Yes | Partial | Phase 4LQ proves source/target integrity and target doctor in a bounded larger Store portability run. Phase 4LS adds an explicit narrow migration for pre-4LS Stores missing only context packet snapshot schema objects; broader long-lived Store maintenance remains to be proven. |
| Workspace, Branch, history, show-at, diff, and restore | Yes | Yes | Partial | Partial | Phase 4LO dogfoods post-Bundle `restore` and `show-at` against a target Store; still prove Branch/diff workflows in a real implementation slice. |
| Goal, Plan, Task, ordering, dependencies, and containment | Yes | Yes | Partial | Partial | Phase 4LV uses WorkVCS Goal, Plan, and Task entities to manage a bounded external-project review through closeout. Ordering, dependencies, and containment still need broader real-project repetition before release maturity claims. |
| Acceptance Criteria, Verification Requirements, Verification, and Evidence closure | Yes | Yes | Yes | Yes | Phase 4LE dogfoods AC/VR/Verification for an implementation closeout; repeat this in recovery and handoff-consumption scenarios. |
| Verification command wrapper | Yes | Yes | Yes | Yes | Phase 4LV proves the top-level `verify` wrapper in another-project dogfood with evidence content, Resource observation, Resource basis, and applicability cache output. Phase 4LX adds and dogfoods `verify --scope-path` / `--scope-path-prefix`, defaulting those shorthands to `scope_kind=path` and `scope_schema_version=1` while preserving explicit JSON input for advanced callers. Keep multi-target, shell execution, and LLM extraction outside V1 unless re-authorized. |
| Resource registration, observation, applicability, and drift | Yes | Partial | Yes | Partial | Phase 4LV uses Resource create/bind/associate and Resource-backed `verify` observation against a real external local project. Phase 4LX closes lexical explicit path-scope normalization for Resource-backed verification shorthands. Glob semantics and adapter-backed re-observation remain Open. |
| Session start/end/focus, Runnable projection, `claim next`, and `next` | Yes | Yes | Yes | Yes | Phase 4LG dogfoods a focused Handoff continuation that is initially blocked, then recovers and continues through the focused Task. |
| Claim modes and guard behavior | Yes | Yes | Yes | Partial | Phase 4LW dogfoods cooperative Claim transfer and stale-gated forced takeover in a realistic read-only external-project continuation loop. Shared-Claim collaboration remains smoke-proven but not yet real-project dogfooded. |
| Context resolver | Yes | Partial | Yes | Partial | Phase 4LR adds explicit packet scope and deterministic path-sensitive Knowledge filtering for `context --scope-json` and `claim next --context-scope-json`. Phase 4LS adds durable `context-packet save/show/list` snapshots for exact resolved packets. Phase 4LT projects recent non-empty ChangeSet rationale into bounded packet items so continuation Agents can see why recent state moved. Phase 4LX adds lexical path selector normalization and dogfoods `context --scope-path`, `claim next --context-scope-path`, and `context-packet save --scope-path-prefix` against a read-only external project. Continue with broader context and Resource resolver gaps before claiming release maturity. |
| Record, Decision, Knowledge, and `why` neighborhoods | Yes | Yes | Partial | Partial | Phase 4LH exposes recognized focused Handoff focus as read-only `why` scope links while preserving stored relation semantics; next prove broader causal/evolution and real-project explanation paths. |
| Handoff | Yes | Yes | Yes | Yes | Phase 4LG proves focused Handoff continuation and blocked recovery through stale-gated Claim takeover. Phase 4LV repeats focused Handoff creation/show after a read-only external-project closeout. Phase 4LW proves Claim transfer/takeover continuation around external-project Tasks; broader Handoff consumption across varied project and write-mode flows remains open. |
| Merge lifecycle | Yes | Yes | Yes | Yes | Phase 4LN dogfoods divergent Work Branch resolution, unresolved freeze guard, target/source moved-head continue rejection, abort/restart recovery, and completed two-parent merge commits. Repeat on another real project or larger Store before release maturity claims. |
| Checkpoint and Bundle portability | Yes | Yes | Yes | Yes | Phase 4LO dogfoods local copied-target export/validate/preflight/apply, imported Checkpoint validation, restore, and divergence refusal. Phase 4LP defines the V1-local directory profile and keeps external Store canonical DAG activation outside the current profile. Phase 4LQ proves the profile against a bounded larger Store workload and fixes a Verification basis import ordering blocker. |
| CLI discoverability and operator use | Partial | Partial | Partial | Partial | Phase 4LF reduces Handoff focus-copy friction with `handoff consume`. Phase 4LU reduces the Phase 4LG blocked-Claim recovery ID-capture friction by adding top-level `claim guard` stale-takeover hint fields. Phase 4LX reduces repeated scope JSON authoring for local-file Context and Resource-backed verification workflows with path shorthands. Continue reducing command friction only where dogfood shows repeated ID plumbing or workflow blockage. |
| Actionable errors and recovery | Yes | Partial | Partial | Partial | Phase 4LN documents merge unresolved and moved-head recovery; Phase 4LO documents Bundle divergence refusal and restore/checkpoint selector boundaries. Phase 4LQ records a concrete apply-ordering failure and recovery. Stable key-value `error_code` output remains Open. |
| Larger Store and performance evidence | Partial | Partial | No | Yes | Phase 4LQ runs the opt-in larger Store portability validation with 48 Tasks, 8 Verification records, 32 scheduling relation inputs, 267 payload files, 749 payload references, apply, restore, and integrity/doctor in 22 seconds. This is bounded local portability dogfood, not broad performance maturity. |

## Dogfood-Biased Next Queue

Use this queue when selecting the next local implementation slice unless a
current user request supplies a narrower priority.

1. Close Resource glob semantics or adapter-backed re-observation only when
   the next continuation or verification workflow proves the need.
2. Dogfood broader `why` paths, including causal/evolution explanation gaps,
   before expanding explanation output again.
3. Reduce additional manual key-value capture in the operator CLI only where
   the next dogfood loop shows repeated workflow blockage.
4. Broaden larger Store and performance validation only when the next workload
   is meaningfully larger or more varied than Phase 4LQ; do not design indexes
   without evidence from that run.
5. Dogfood shared-Claim collaboration on a real project only when a realistic
   multi-operator read/write path needs it.

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
  Handoff focus link as a relation. Phase 4LG dogfoods a focused Handoff
  continuation blocked by another Session's active Claim, then recovers it with
  explicit stale marking and stale-gated forced takeover. Phase 4LH adds
  read-only `why` scope links for recognized focused Handoff focus, proving the
  Handoff side as outgoing and the focused Task side as incoming while keeping
  stored `relation_edges=0`. Phase 4LI adds explicit `claim next`
  `ContextPacket` output, proving a one-command claim/focus/packet loop with a
  brief profile and item budget. Phase 4LJ adds brief current-task AC/VR
  obligation items and dogfoods using the surfaced Verification Requirement id
  with the `verify` wrapper until `ac status` returns `verified`. Phase 4LK
  adds brief blocked dependency items and dogfoods using the surfaced blocking
  Task id as the next claim target. Phase 4LL adds brief Goal/Plan path items
  and dogfoods a CLI Goal -> Plan -> Task containment path through `claim next`
  packet output. Phase 4LM adds deterministic Attempt detail for failed brief
  items and normal/full Attempt items, and dogfoods running, succeeded, failed,
  and inconclusive status visibility through real CLI `context` packet output.
  Phase 4LN dogfoods the merge lifecycle in a durable local Store: divergent
  Branches produce `CONFLICT` and `AUTO` items, unresolved items block freeze,
  target/source moved heads block continue, abort/restart recovery succeeds, and
  the successful restarts create two-parent `merge.continue` commits. WorkVCS
  has still not been used for another real project. Phase 4LO dogfoods local
  copied-target Bundle portability with source Checkpoint creation, Bundle
  directory export/validation, target preflight/apply, imported Checkpoint
  validation, post-apply restore/show-at inspection, and explicit same-Store
  divergence refusal. Phase 4LP defines the V1-local Bundle directory profile
  as `manifest.json`, `payload-index.json`, and content-addressed JSON payload
  files under `payloads/`, with profile `workvcs-local-payload-index-v1`
  version `1`; external Store canonical DAG activation remains unsupported by
  this profile. Phase 4LQ adds an opt-in larger Store portability validator and
  proves the local profile on 48 Tasks, 8 Verification records, 32 scheduling
  relation inputs, 267 payload files, 749 payload references, apply, restore,
  and integrity/doctor in 22 seconds, after fixing a Verification basis import
  ordering blocker. Phase 4LR adds explicit context packet scope and dogfoods
  path-sensitive Knowledge filtering through real CLI `context --scope-json`
  and `claim next --context-scope-json` packet output. Phase 4LS persists
  scoped, budgeted ContextPacket snapshots through real CLI
  `context-packet save/show/list` and verifies the saved packet digest,
  canonical packet JSON, Session list entry, and Store doctor result. Phase
  4LT projects recent non-empty ChangeSet rationale into brief/normal/full
  ContextPacket output and dogfoods a Goal achievement rationale through
  `context` and persisted `context-packet show` JSON. Phase 4LU adds stable
  top-level `claim guard` stale-takeover hint fields so a blocked continuation
  can identify the blocking Claim and previous owning Session without scanning
  indexed active-claim rows. Phase 4LV runs WorkVCS against the real local
  `agent_soul` project in read-only mode: it creates a local Store, models an
  external-project Goal/Plan/Task with AC/VR, registers and observes the target
  Resource through the `verify` wrapper, persists scoped and transition
  ContextPackets, completes Task/Plan/Goal closeout, authors a focused Handoff,
  passes Store integrity, and proves target project status/diff snapshots are
  unchanged. Phase 4LW then dogfoods Claim transfer and stale-gated forced
  takeover against the same real external project in read-only mode: cooperative
  transfer hands a Claim from one active Session to another and closes the Task
  with Resource-backed verification, while the takeover path proves blocked
  guard hints, active-owner takeover rejection, explicit stale marking,
  forced takeover with rationale, recovered guard success, verified closeout,
  active-Session context boundaries, Store integrity, and unchanged target
  project status/diff snapshots. Phase 4LX adds lexical path-scope
  normalization and CLI path-scope shorthands for `knowledge`, `context`,
  `context-packet save`, `claim next`, and `verify`; the read-only
  `agent_soul` dogfood proves matching Knowledge remains visible across
  lexical path variants, unrelated path Knowledge is filtered, `verify
  --scope-path` records Resource-backed evidence, and target project status is
  unchanged.
