# V1 Readiness Ledger

Status: current implementation-readiness ledger
Last refreshed: 2026-09-04 by Phase 4OK / ADR-0495

This ledger tracks the current WorkVCS V1 implementation state. It is an
evidence map, not a product specification. Confirmed product, architecture,
schema, and ADR documents remain the authority for what WorkVCS means.

The ledger exists to correct implementation focus: later slices should close
V1 readiness and dogfood gaps before adding more narrow query-detail or
expectation-only work.

Release-maturity judgment is tracked in
[`v1-release-gate-matrix.md`](v1-release-gate-matrix.md). The current matrix
records `V1_RELEASE_READY=false` and `V0_1_DOGFOOD_COMPLETE=false`.
`RELEASE_CANDIDATE_ALLOWED=false` remains recorded in the release gate matrix.
Phase 4OK adds no new release-state claim; it records the direct Plan-to-Task
Resource-backed `why` closure implementation and dogfood evidence while the
release gate matrix remains false.

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
| Store bootstrap, open, manifest, lineage, and doctor | Yes | Yes | Yes | Yes | Phase 4LQ proves source/target integrity and target doctor in a bounded larger Store portability run. Phase 4LS adds an explicit narrow migration for pre-4LS Stores missing only context packet snapshot schema objects. Phase 4ND proves one maintained source Store and the same copied target Store across three reopen, Checkpoint, Bundle export/apply, lineage-list, integrity, and doctor cycles. Keep as regression foundation; broader scale/performance evidence is tracked separately. |
| Workspace, Branch, history, show-at, diff, and restore | Yes | Yes | Partial | Yes | Phase 4LO dogfoods post-Bundle `restore` and `show-at` against a target Store. Phase 4MQ dogfoods Branch fork, bidirectional Branch diff, Branch history, Branch `show-at`, and two-Branch integrity in a real repository delivery Store. Keep as regression foundation; broaden only if a future real Branch/diff workflow exposes a concrete gap. |
| Goal, Plan, Task, ordering, dependencies, and containment | Yes | Yes | Partial | Yes | Phase 4LV uses WorkVCS Goal, Plan, and Task entities to manage a bounded external-project review through closeout. Phase 4NB repeats the workflow against a temporary clone of pre-existing `agent_soul` with one Goal, one Plan, three contained Tasks, two `depends_on` relations, two `ordered_before` relations, blocked dependency context, dependency readiness recovery, and Plan/Goal closeout. Keep as regression foundation; broaden only if a future real workflow exposes a concrete ordering or containment gap. |
| Acceptance Criteria, Verification Requirements, Verification, and Evidence closure | Yes | Yes | Yes | Yes | Phase 4LE dogfoods AC/VR/Verification for an implementation closeout. Phase 4MS repeats VR-backed evidence closure after focused Handoff consumption in a real write-mode repository delivery slice. Phase 4NB proves AC/VR closure in a recovery Handoff scenario: stale Resource-backed applicability blocks Task closeout, explicit refresh projects `resource_drift`, and recovery `verify` records new evidence before Task/Plan/Goal closeout. Keep as regression foundation; broaden only if a future recovery or handoff loop exposes a concrete evidence-closure gap. |
| Verification command wrapper | Yes | Yes | Yes | Yes | Phase 4LV proves the top-level `verify` wrapper in another-project dogfood with evidence content, Resource observation, Resource basis, and applicability cache output. Phase 4LX adds and dogfoods `verify --scope-path` / `--scope-path-prefix`, defaulting those shorthands to `scope_kind=path` and `scope_schema_version=1` while preserving explicit JSON input for advanced callers. Phase 4LY adds explicit `--resource-content-from-scope-path` so the wrapper can observe a real local file from `--scope-path`. Phase 4MD adds explicit `--resource-content-from-scope-path-prefix` so the wrapper can observe a deterministic local-file path-prefix manifest from `--scope-path-prefix`. Phase 4ME adds explicit `--scope-glob` / `--resource-content-from-scope-glob` for deterministic local-file glob manifests. Phase 4MF adds explicit `--scope-git-worktree` / `--resource-content-from-scope-git-worktree` for deterministic Git worktree manifests. Keep multi-target, shell execution, and LLM extraction outside V1 unless re-authorized. |
| Resource registration, observation, applicability, and drift | Yes | Yes | Yes | Yes | Phase 4LV uses Resource create/bind/associate and Resource-backed `verify` observation against a real external local project. Phase 4LX closes lexical explicit path-scope normalization for Resource-backed verification shorthands. Phase 4LY proves explicit local-file ResourceObservation from `--scope-path` with the observed fingerprint matching an independent content digest. Phase 4MB proves explicit unavailable/error applicability stamps against a real file baseline: both produce `applicability=unknown`, stable reason codes, empty observed data, and stale AC projection. Phase 4MC adds and dogfoods explicit local-file exact path re-observation for `verification cache-refresh --resource-content-from-scope-path`, creating a new ResourceObservation and preserving verified AC state when the file is unchanged. Phase 4MD adds and dogfoods deterministic local-file path-prefix aggregation for `verify --resource-content-from-scope-path-prefix` and `verification cache-refresh --resource-content-from-scope-path-prefix`, including unchanged, drift, unavailable, and error projections. Phase 4ME adds and dogfoods deterministic local-file glob aggregation for `verify --scope-glob --resource-content-from-scope-glob` and `verification cache-refresh --resource-content-from-scope-glob`, including unchanged, drift, missing-root unavailable, empty-match drift, and matched-directory error projections. Phase 4MF adds and dogfoods deterministic Git worktree aggregation for `verify --scope-git-worktree --resource-content-from-scope-git-worktree` and `verification cache-refresh --resource-content-from-scope-git-worktree`, including unchanged, tracked/untracked drift, missing-repo unavailable, and non-Git error projections. Phase 4MG adds explicit basis-aware refresh through `verification cache-refresh --resource-content-from-basis`, with focused proof for mixed supported local-file/Git basis entries, atomic unsupported-basis rejection, and read-only real-project dogfood. Phase 4MN adds explicit batch Resource-basis refresh for all current-head Resource-backed Verifications on a Branch, with prevalidation before writes and dogfood evidence that two Resource-backed Verifications refresh in one command without moving Branch head. Phase 4MV exposes, tests, and dogfoods the V1-local Git worktree rename policy as `rename_detection=disabled` and `rename_policy=delete_add` against a `/tmp` clone of pre-existing `agent_soul`, leaving the original repository unchanged. Phase 4MW exposes, tests, and dogfoods the V1-local Git worktree symlink policy as tracked symlinks represented through Git index/diff material and untracked non-regular entries, including symlinks, mapped to unsupported Resource error behavior against a `/tmp` clone of pre-existing `agent_soul`, leaving the original repository unchanged. Phase 4MX exposes, tests, and dogfoods the V1-local Git worktree submodule policy as parent Git gitlink/status/diff observation with recursion disabled against a `/tmp` clone of pre-existing `agent_soul` and a temporary local submodule, leaving the original repository unchanged. Phase 4MY exposes, tests, and dogfoods the V1-local Git worktree sparse-checkout policy as parent Git index/status/diff observation with expansion disabled against a `/tmp` sparse-checkout clone of pre-existing `agent_soul`, leaving the original repository unchanged. Phase 4MZ exposes, tests, and dogfoods bounded path case handling as `path_case_folding=disabled`: filesystem-native local-file exact path resolution, filesystem-native path-prefix entry names, case-sensitive local-file glob patterns, and parent-Git path reporting for Git worktree observations against a `/tmp` clone of pre-existing `agent_soul`, leaving the original repository unchanged. Phase 4NA makes explicit foreground operator-triggered batch refresh the V1-local re-observation scheduling policy and dogfoods stale/resource_drift plus applicable projections in a `/tmp` clone of pre-existing `agent_soul`, leaving the original repository unchanged. Keep as regression foundation; background daemons, watchers, automatic polling, implicit refresh, and Agent orchestration remain outside V1. |
| Session start/end/focus, Runnable projection, `claim next`, and `next` | Yes | Yes | Yes | Yes | Phase 4LG dogfoods a focused Handoff continuation that is initially blocked, then recovers and continues through the focused Task. Phase 4MR dogfoods `claim next --mode shared`, `runnable tasks`, and `context` in a real write-mode/read-write repository delivery slice. Phase 4OC dogfoods unsupported Session focus fail-fast: `session focus-set` now rejects a current `verification_requirement` entity with stable `session_invalid` before writing focus rows, leaves the Session unfocused, and preserves valid Task-focus context recovery hints. |
| Claim modes and guard behavior | Yes | Yes | Yes | Yes | Phase 4LW dogfoods cooperative Claim transfer and stale-gated forced takeover in a realistic read-only external-project continuation loop. Phase 4MI dogfoods shared-Claim collaboration against the same real external project in read-only mode. Phase 4MR dogfoods shared-Claim write/read-write collaboration in this repository: two active shared Claims block protected writer mutation with `reason=non_unique_shared_claim_set`, releasing the reader Claim restores `unique_shared_claimant`, and the remaining writer performs the documentation write and Store closeout. Keep as regression foundation; automatic ownership arbitration between shared claimants remains outside V1 unless explicitly authorized. |
| Context resolver | Yes | Partial | Yes | Partial | Phase 4LR adds explicit packet scope and deterministic path-sensitive Knowledge filtering for `context --scope-json` and `claim next --context-scope-json`. Phase 4LS adds durable `context-packet save/show/list` snapshots for exact resolved packets. Phase 4LT projects recent non-empty ChangeSet rationale into bounded packet items so continuation Agents can see why recent state moved. Phase 4LX adds lexical path selector normalization and dogfoods `context --scope-path`, `claim next --context-scope-path`, and `context-packet save --scope-path-prefix` against a read-only external project. Phase 4NN adds Resource basis recovery hints to brief `verification_requirement` packet items for current-task Verification Requirements with current-head Resource-backed Verifications, without changing packet schema. Phase 4NP adds Resource-backed Verification Requirement recovery hints to focused `blocked_dependency` packet items when the blocking dependency Task has current-head Resource-backed Verifications, without changing packet schema. Phase 4NU adds normal/full ContextPacket Resource-backed Verification Requirement recovery hints for runnable same-Plan peer Tasks when focus is a Task, without changing packet schema or public CLI flags. Phase 4NZ adds normal/full ContextPacket Resource-backed Verification Requirement recovery hints for runnable same-Goal cross-Plan peer Tasks when focus is a Task, without changing packet schema or public CLI flags. Continue with broader context and Resource resolver gaps outside these direct peer slices before claiming release maturity. |
| Record, Decision, Knowledge, and `why` neighborhoods | Yes | Yes | Partial | Partial | Phase 4LH exposes recognized focused Handoff focus as read-only `why` scope links while preserving stored relation semantics. Phase 4MA dogfoods a real external-project explanation across Task containment, Verification, Evidence, Record support, and Record-to-Knowledge support without adding display fields. Phase 4MH exposes `evolution` as a deferred relation family when the queried Entity anchors a first-parent-reachable ChangeSet, and dogfoods the behavior against a real external project. Phase 4ML makes that causal-anchor evolution evidence actionable by projecting the first-parent-reachable anchoring commit and ChangeSet from `why` for the queried Entity. Phase 4NC makes direct Record-to-Record and Record-to-Knowledge epistemic edges self-explanatory by projecting source and target statements from `why`. Phase 4NG projects direct ChangeOperation Entity/Relation subjects for those causal-anchor ChangeSets in `why`. Phase 4NH makes those direct operation subjects self-explanatory by projecting current Record/Knowledge statements and recognized Relation kind/source/target/state-digest details for operation subjects already reported by `why`. Phase 4NJ projects direct first-parent Entity-subject evolution operations for queried changed Entities that are not causal anchors. Phase 4NK makes those direct queried-Entity evolution operation details operation-local for Entity after-versions, so older direct operations no longer inherit the final target commit Entity version. Phase 4NL projects direct Record-to-Record relation remove/restore operations as endpoint evolution for queried Entity endpoints, using operation-local relation versions even when a removed relation is absent from current `relation_edges`. Phase 4NM projects direct Record-to-Knowledge and Knowledge-to-Knowledge relation remove/restore operations as endpoint evolution for queried Knowledge endpoints. Phase 4NO projects direct Record-to-Record, Record-to-Knowledge, and Knowledge-to-Knowledge relation create operations as endpoint evolution for queried Entity endpoints, so create/remove/restore chains are visible through existing evolution fields. Phase 4NQ projects current Task scheduling relation edges and direct `task.scheduling_relation.create` endpoint evolution for `depends_on` and `ordered_before` Task endpoints. Phase 4NR projects direct `primary_containment.create` endpoint evolution for queried Goal, Plan, and Task endpoints, so Goal->Plan and Plan->Task containment creation is explainable through existing `why` evolution fields. Phase 4NV projects direct `verification.record` defining `verifies` relation creation as endpoint evolution for queried Verification, Acceptance Criterion, and Verification Requirement endpoints. Phase 4NW projects direct `verification.record` `evidenced_by` relation creation as endpoint evolution for queried Verification endpoints backed by Evidence. Phase 4NX projects direct `verification.record` `evidenced_by` relation creation as endpoint evolution for queried Evidence endpoints reused by Verifications. Phase 4NY projects direct `knowledge_exposure_derived_from` relation creation as endpoint evolution for queried source Knowledge and target KnowledgeExposure endpoints. Phase 4OE projects current VR-backed Verification closure chains from Task and Acceptance Criterion `why`, listing the AC, VR, Verification, result, and Evidence ids for current-head closeout explanation. Phase 4OG extends that same Task/Acceptance Criterion closure projection for Resource-backed Verifications by listing each basis resource id, adapter, scope, canonical scope payload JSON, baseline observation id, and baseline fingerprint. Phase 4OK extends that closure projection to Plan `why` for directly contained Tasks, preserving the same AC, VR, Verification, Evidence, and Resource basis fields while keeping Goal ancestor rollup out of scope. Full relation-subject traversal beyond those direct endpoint and Task/Acceptance Criterion/direct-Plan closure slices, multi-hop/full evolution traversal, broader causal traversal, and broader context/Resource resolver maturity remain Open. |
| Handoff | Yes | Yes | Yes | Yes | Phase 4LG proves focused Handoff continuation and blocked recovery through stale-gated Claim takeover. Phase 4LV repeats focused Handoff creation/show after a read-only external-project closeout. Phase 4LW proves Claim transfer/takeover continuation around external-project Tasks. Phase 4MS proves focused Handoff consumption by a separate continuation Session in a real write-mode repository delivery slice, including Claim, VR-backed verification, Task closeout, and SessionDiff closeout. Keep as regression foundation; remote/cloud Handoff, cross-Store synchronization, automatic takeover, and Agent orchestration remain outside V1 unless explicitly authorized. |
| Merge lifecycle | Yes | Yes | Yes | Yes | Phase 4LN dogfoods divergent Work Branch resolution, unresolved freeze guard, target/source moved-head continue rejection, abort/restart recovery, and completed two-parent merge commits. Phase 4MM repeats merge dogfood on a larger and more varied local Store with 12 conflict items, 12 auto items including 8 source-only Tasks and 4 source-side scheduling Relations, explicit resolutions for all 24 items, freeze/continue, a two-parent merge commit, expected final WorkState, and required-valid integrity/doctor. Phase 4MT adds generated external local Git project write-mode merge proof. Phase 4MU repeats merge against pre-existing real `agent_soul` project content cloned into a write-mode sandbox: actual Git conflict on existing `README.md`, WorkVCS conflict and auto merge items, unresolved freeze guard, explicit source-side resolutions, freeze/continue, final WorkState proof, Branch diff, SessionDiff closeout, original project unchanged, and required-valid integrity/doctor. Keep as regression foundation; semantic/LLM, remote, distributed, cross-Store, Agent-orchestrated, and original-repository direct mutation merge flows remain outside the bounded V1-local release gate unless separately authorized. |
| Checkpoint and Bundle portability | Yes | Yes | Yes | Yes | Phase 4LO dogfoods local copied-target export/validate/preflight/apply, imported Checkpoint validation, restore, and divergence refusal. Phase 4LP defines the V1-local directory profile and keeps external Store canonical DAG activation outside the current profile. Phase 4LQ proves the profile against a bounded larger Store workload and fixes a Verification basis import ordering blocker. |
| CLI discoverability and operator use | Partial | Partial | Partial | Partial | Phase 4LF reduces Handoff focus-copy friction with `handoff consume`. Phase 4LU reduces the Phase 4LG blocked-Claim recovery ID-capture friction by adding top-level `claim guard` stale-takeover hint fields. Phase 4LX reduces repeated scope JSON authoring for local-file Context and Resource-backed verification workflows with path shorthands. Phase 4MJ reduces stale command-spelling recovery friction by normalizing top-level clap usage failures into stable key-value stderr. Phase 4MK adds a per-code recovery guide so operators do not have to infer recovery behavior from raw messages or source. Phase 4NF removes a maintained Store validator handoff ambiguity by appending the final stdout summary to the preserved `run.log` tail and proving stdout/log-tail equality in a real opt-in run. Phase 4NG adds `why` evolution operation key-value fields and `--expected-evolution-change-operations` for script assertions. Phase 4NH reduces post-4NG ID lookup by adding `why` evolution subject detail key-value fields for current recognized Entity and Relation operation subjects. Phase 4NJ reduces another `why` lookup gap by rendering a queried changed Entity's direct first-parent evolution operation even when `causal_anchor_changesets=0`. Phase 4NK reduces the next direct evolution lookup mismatch by making `subject_entity_version_id` operation-local for multiple direct queried-Entity changes. Phase 4NL reduces the next endpoint-neighborhood lookup gap by making `why` expose direct Record relation remove/restore operations for queried endpoints through existing evolution key-value fields. Phase 4NM extends that endpoint-neighborhood proof to Record-to-Knowledge and Knowledge-to-Knowledge remove/restore operations queried from Knowledge endpoints. Phase 4NO makes relation create explainable through the same `why` evolution fields and assertion flag for recognized Record-to-Record, Record-to-Knowledge, and Knowledge-to-Knowledge endpoints. Phase 4NN reduces Resource-backed VR recovery ID lookup by placing the basis-aware `verification cache-refresh --verification ... --resource-content-from-basis` hint directly in brief context. Phase 4NP extends that recovery hint to focused `blocked_dependency` items so an operator can recover the prerequisite VR without opening separate verification detail. Phase 4NU exposes the same recovery command in normal/full context for runnable same-Plan peer Tasks while keeping brief context focused. Phase 4NZ exposes the same recovery command in normal/full context for runnable same-Goal cross-Plan peer Tasks while keeping brief context focused. Phase 4NQ makes Task scheduling relation endpoints explainable through existing `why` relation and evolution key-value fields, with `task_depends_on` and `task_ordered_before` relation-kind filters. Phase 4NR makes primary containment creation explainable through existing `why` relation and evolution key-value fields for Goal, Plan, and Task endpoints, with `primary_containment` relation-kind filtering and expected-count assertions. Phase 4NV makes defining `verifies` relation creation explainable through existing `why` relation and evolution key-value fields for Verification, Acceptance Criterion, and Verification Requirement endpoints, with `verifies` relation-kind filtering and expected-count assertions. Phase 4NW makes Evidence-backed `evidenced_by` relation creation explainable through the same `why` relation and evolution key-value fields for queried Verification endpoints, with `evidenced_by` relation-kind filtering and expected-count assertions. Phase 4NX extends the same `evidenced_by` relation filtering and expected-count assertions to queried Evidence endpoints. Phase 4NY extends existing `knowledge_exposure_derived_from` relation-kind filtering and expected-count assertions to direct relation creation evolution from both Knowledge and KnowledgeExposure endpoints. Phase 4OE reduces Task closeout ID plumbing by rendering current AC -> VR -> Verification -> Evidence closure chains directly from Task and Acceptance Criterion `why`. Phase 4OG removes the follow-up `verification show` hop for Resource-backed closeout recovery by rendering Resource basis fields inside those same Task and Acceptance Criterion `why` closure chains. Phase 4OK removes the follow-up Task or Acceptance Criterion lookup for a Plan-level direct Task closeout by surfacing the same Resource-backed closure fields from Plan `why`; dogfood proves the Plan-discovered Verification id can drive `verification cache-refresh --resource-content-from-basis`. Continue reducing command friction only where dogfood shows repeated ID plumbing or workflow blockage. |
| Actionable errors and recovery | Yes | Yes | Yes | Yes | Phase 4LN documents merge unresolved and moved-head recovery; Phase 4LO documents Bundle divergence refusal and restore/checkpoint selector boundaries. Phase 4LQ records a concrete apply-ordering failure and recovery. Phase 4LZ adds stable process-level key-value fields for WorkVCS business errors: `error_code`, `error_category`, `retryable`, and escaped `message`. Phase 4MJ adds the same script-readable shape for top-level clap parse errors, with `error_code=cli_parse_error`, `error_category=usage`, `retryable=false`, `clap_error_kind`, escaped `message`, and preserved help display. Phase 4MK documents current per-code operator recovery actions for every stable WorkVCS business error code and `cli_parse_error`. Phase 4MO adds explicit `--error-format json` output for WorkVCS business errors and top-level clap parse errors, with process-level dogfood proving parseable JSON while preserving default key-value stderr. Phase 4NI adds `scripts/operator-recovery-maturity-v0.1.sh`, proving current guide coverage for all 41 core business error codes plus `cli_parse_error`, the retryability rule, parse-error JSON recovery, branch-head retry, Resource drift/unavailable/error recovery to applicable, stale-gated Claim takeover, merge unresolved recovery, and final Store integrity. Keep reducing command friction only where future dogfood exposes repeated blockage. |
| Larger Store and performance evidence | Partial | Partial | No | Yes | Phase 4LQ runs the opt-in larger Store portability validation with 48 Tasks, 8 Verification records, 32 scheduling relation inputs, 267 payload files, 749 payload references, apply, restore, and integrity/doctor in 22 seconds. Phase 4MM adds bounded merge-path evidence on a 56-commit, 78-operation local Store with required-valid integrity/doctor after merge. Phase 4NE adds a larger maintained Store run with 112 final Tasks, 20 Verifications, 80 script-counted scheduling relation versions, five same-target Bundle applies, 647 final payload files, 1,817 final payload references, and source/target required-valid integrity/doctor in 331 seconds. Treat these as bounded V1-local release-gate evidence, not broad performance maturity or index-tuning justification. |

## Dogfood-Biased Next Queue

Use this queue when selecting the next local implementation slice unless a
current user request supplies a narrower priority.

1. Expand `why` only when a dogfood continuation exposes a concrete causal,
   evolution, or broader explanation gap; do not add more explanation fields
   speculatively.
2. Reduce additional manual key-value capture in the operator CLI only where
   the next dogfood loop shows repeated workflow blockage.
3. Refresh the V1 release gate matrix after any blocking gate changes status
   and before any release-ready or release-candidate claim.

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
  unchanged. Phase 4LY then lets `verify` explicitly read ResourceObservation
  content from `--scope-path` with `--resource-content-from-scope-path`; the
  read-only `agent_soul` dogfood proves the observed fingerprint equals an
  independent `canonical content-digest --content-file` result and that target
  project status is unchanged. Phase 4MB dogfoods the existing unavailable and
  error Resource applicability stamp states against the same read-only real
  file baseline: both project to `applicability=unknown`, expose stable
  `resource_unavailable` or `resource_error` reason codes, keep the AC status
  `stale`, reject observed data on unavailable stamps with `task_invalid`, and
  leave the target project status unchanged. Phase 4MC adds explicit
  `verification cache-refresh --resource-content-from-scope-path` support for
  exact `local-file` path Resource basis entries. The focused test proves
  unchanged files create a new observation and remain applicable, changed files
  become `resource_drift`, missing files become `resource_unavailable`, and
  directory reads become `resource_error`. A follow-up focused regression proves
  mixed supported/unsupported basis entries fail before any partial observation
  write; real read-only `agent_soul` dogfood proves an unchanged external file
  refresh creates a new observation, keeps the AC `verified`, and leaves target
  project status unchanged. Phase 4MD adds
  `--resource-content-from-scope-path-prefix` for `verify` and
  `verification cache-refresh`. Its focused test proves deterministic
  path-prefix baseline recording, unchanged refresh, nested-file drift, missing
  prefix, and non-directory error projection; real read-only `agent_soul`
  dogfood proves unchanged refresh over a four-file reference-tools prefix, and
  a controlled copy proves `resource_drift`, `resource_unavailable`, and
  `resource_error` without mutating the target project. Phase 4ME adds
  `--scope-glob` and `--resource-content-from-scope-glob` for `verify`, plus
  `verification cache-refresh --resource-content-from-scope-glob`. Its focused
  test proves deterministic glob baseline recording, unchanged refresh,
  matched-file drift, missing fixed-root unavailable, empty-match drift, and
  matched-directory error projection. Real read-only `agent_soul` dogfood
  proves unchanged refresh over a four-file reference-tools glob, and a
  controlled copy proves `resource_drift`, `resource_unavailable`,
  empty-match `resource_drift`, and `resource_error` without mutating the
  target project. Phase 4MF adds `--scope-git-worktree` and
  `--resource-content-from-scope-git-worktree` for `verify`, plus
  `verification cache-refresh --resource-content-from-scope-git-worktree`. Its
  focused tests prove deterministic Git worktree baseline recording, unchanged
  refresh, unstaged/staged/untracked drift, missing-repo unavailable, and
  non-Git error projection. Real read-only `agent_soul` dogfood proves unchanged
  refresh over a dirty external project and confirms the target Git status is
  unchanged. Phase 4MG adds `verification cache-refresh
  --resource-content-from-basis`, allowing the CLI to choose the already
  implemented exact path, path-prefix, glob, or Git worktree refresh adapter from
  persisted Resource basis entries. Its focused tests prove path-prefix and glob
  persisted-basis dispatch, mixed local-file/Git basis refresh, stale projection
  after local drift, mutual exclusion with specific flags, and unsupported-basis
  failure without partial observation writes. Real read-only `agent_soul`
  dogfood proves unchanged basis-aware Git refresh and unchanged target Git
  status. Phase 4MN adds explicit batch Resource-basis refresh through
  `verification cache-refresh --all-resource-backed
  --resource-content-from-basis`. It selects all current-head Verifications with
  non-empty Resource basis entries, skips non-resource-backed Verifications,
  prevalidates every selected basis before writing observations, and dogfoods
  refreshing two Resource-backed Verifications in one command without moving the
  Branch head. Phase 4MH then adds a narrow `why` deferred-family disclosure for
  anchored evolution: a causal Finding used to anchor a
  first-parent-reachable ChangeSet reports
  `deferred_relation_family.0=evolution`, while the superseded prior Decision
  in the same workflow reports no deferred family. The read-only `agent_soul`
  dogfood proves the target project status is unchanged. Phase 4MI dogfoods
  shared-Claim collaboration against the same real external project in read-only
  mode: two active Sessions claim one Task in `shared` mode, `context` shows the
  second Session focus and shared Claim runnable candidate, `claim guard
  --action structural-task` reports `allowed=false` with
  `reason=non_unique_shared_claim_set` and two active shared Claims, protected
  Task closeout is blocked until one shared Claim is released, the remaining
  `unique_shared_claimant` closes the Task after Resource-backed verification,
  and the target file status, hash, and stat remain unchanged. The dogfood also
  records a release-guidance issue: AC status is `verified` at the verification
  commit and becomes `stale` after Task closeout advances the Task version.
  Phase 4MJ normalizes top-level clap usage failures observed during 4MI:
  stale arguments and invalid subcommands now exit 2 with
  `error_code=cli_parse_error`, `error_category=usage`, `retryable=false`,
  stable `clap_error_kind`, and escaped `message`, while `workvcs --help`
  remains exit 0 with usage on stdout, empty stderr, and no `error_code`.
  Phase 4MK adds the current per-code operator recovery guide for every stable
  WorkVCS business error code and the CLI-local `cli_parse_error`; it preserves
  the current retryability contract that only `branch_head_conflict` is
  directly retryable and keeps JSON error output Open.
  Phase 4MO closes the JSON error encoding gap with explicit
  `--error-format json` for WorkVCS business errors and top-level clap parse
  errors. The process-level dogfood parses both JSON stderr forms with `jq` and
  proves the default failure path remains key-value.
  Phase 4MP adds `docs/provenance/v1-release-gate-matrix.md`, an explicit
  release-maturity gate view that records `V1_RELEASE_READY=false`,
  `V0_1_DOGFOOD_COMPLETE=false`, and `RELEASE_CANDIDATE_ALLOWED=false` until
  blocking gates receive fresh pass evidence from a candidate commit.
  Phase 4MQ closes the Branch/diff dogfood gap named by that matrix with a real
  repository delivery Store: it forks a base Branch into an implementation
  Branch, records implementation-branch work, proves one added entity in the
  forward Branch diff, one removed entity in the reverse Branch diff, expected
  first-parent histories, expected `show-at` WorkState digests, and two-Branch
  required-valid integrity. The run records `final_dogfood=pass` while keeping
  `V1_RELEASE_READY=false`, `V0_1_DOGFOOD_COMPLETE=false`, and
  `RELEASE_CANDIDATE_ALLOWED=false`.
  Phase 4MR extends shared-Claim evidence from read-only collaboration into a
  real write-mode/read-write repository delivery slice. A reader and writer
  Session both hold shared Claims on the same Task; `runnable tasks` and
  `context` expose the shared coordination state; `claim guard
  --action structural-task` and an actual `task transition --session` reject
  protected writer mutation while the shared Claim set is non-unique; after the
  reader releases its Claim, the writer receives `unique_shared_claimant`,
  performs the documentation write, records post-write verification evidence,
  and closes the Task. The run keeps automatic arbitration, distributed
  collaboration, and remote coordination outside V1.
  Phase 4MS closes the focused Handoff consumption write-mode gap: a source
  Session ended with a SessionDiff and authored a focused Handoff; a separate
  continuation Session started with no focus, consumed that Handoff through
  `handoff consume`, selected and claimed the focused Task through `next`,
  passed the Claim guard, performed the repository documentation write,
  recorded VR-backed post-write evidence, transitioned the Task to `done`,
  released the Claim, and ended with a SessionDiff. The run keeps remote/cloud
  Handoff, cross-Store synchronization, automatic takeover, and Agent
  orchestration outside V1.
  Phase 4MT narrows the merge release gate with generated external local Git
  write-mode proof: the external target/source branches produced an actual Git
  conflict and two-parent merge commit, and WorkVCS completed the mirrored
  merge through conflict and auto item inspection, unresolved freeze guard,
  explicit source-side resolutions, freeze/continue, final WorkState version
  anchors plus read-only entity-version state proof, Branch diff, SessionDiff
  closeout, and required-valid integrity/doctor. Because the external project
  was generated for this dogfood, this does not prove pre-existing business
  repository maturity or broad release maturity.
  Phase 4MU closes the remaining pre-existing external-project write-mode merge
  evidence gap for the bounded V1-local release gate: it cloned the real local
  `agent_soul` repository into `/tmp`, changed existing `README.md` content on
  target/source branches, added one source-only file, observed a real Git
  conflict, resolved the Git merge to the source branch, and completed the
  mirrored WorkVCS merge through active conflict/auto inspection, unresolved
  freeze guard, explicit source-side resolutions, freeze/continue, final
  WorkState version anchors plus read-only entity-version state proof, Branch
  diff, SessionDiff closeout, original-project unchanged proof, and
  required-valid integrity/doctor. This supports bounded V1-local merge release
  gate pass evidence while leaving semantic/LLM, remote, distributed,
  cross-Store, and Agent-orchestrated merge outside V1.
  Phase 4MV, Phase 4MW, Phase 4MX, Phase 4MY, Phase 4MZ, and Phase 4NA narrow
  Git worktree and local-file Resource policy gaps. Phase 4MV exposes
  no-renames/delete-add behavior for Git renames, Phase 4MW exposes tracked
  symlink handling through Git index/diff material and untracked symlink failure
  as unsupported non-regular Resource error behavior, Phase 4MX exposes
  parent-Git submodule gitlink/status/diff observation with recursion disabled,
  Phase 4MY exposes parent-Git sparse-checkout index/status/diff observation
  with expansion disabled, and Phase 4MZ exposes no WorkVCS case folding for
  local-file exact path, path-prefix, glob, and Git worktree observations.
  Phase 4NA makes explicit foreground operator-triggered batch refresh the
  V1-local re-observation scheduling policy, exposes
  `reobservation_policy=explicit_operator_batch_refresh`,
  `background_reobservation=disabled`, `reobservation_trigger=operator_explicit`,
  `reobservation_execution=foreground_command`,
  `selection_policy=current_head_resource_backed_verifications`, and
  `branch_head_mutation=disabled`, then dogfoods two Resource-backed refreshes
  plus one non-resource-backed skip. These policy runs dogfood against `/tmp`
  clones of pre-existing `agent_soul` and leave the original repository
  unchanged; Resource is now dogfood-proven for the bounded V1-local release
  gate while background daemons, watchers, automatic polling, implicit refresh,
  and Agent orchestration remain outside V1.
  Phase 4NB closes the Goal/Plan/Task and AC/VR release-maturity repetition
  gaps with a real-project clone recovery loop: one Goal, one Plan, three
  contained Tasks, `depends_on` and `ordered_before` scheduling, blocked
  dependency context, pre-verification Task closeout refusal, focused Handoff
  consumption by a separate Session, Resource-backed stale `resource_drift`
  projection after sandbox-only README mutation, recovery `verify`, Task B and
  Task C closeout, Plan completion, Goal achievement, and required-valid
  integrity/doctor. The original `agent_soul` repository remained unchanged.
  Phase 4ML closes a concrete `why` dogfood gap: Decision supersede ChangeSets
  already had causal anchors visible through `changeset anchors`, but `why`
  could not report which first-parent-reachable commit/ChangeSet the queried
  causal Entity anchored. `why` now renders `causal_anchor_changesets` and
  `causal_anchor_changeset.<i>.*` fields while keeping full evolution traversal
  and epistemic traversal Open.
  Phase 4NC closes the direct epistemic statement lookup gap for current
  Record-to-Record and Record-to-Knowledge epistemic edges. `why` now renders
  `epistemic_explanations` and `epistemic_explanation.<i>.*` fields with
  source/target statement JSON for direct `supports`, `contradicts`,
  `validates`, and `invalidates` relations, while keeping full evolution
  traversal, broader causal traversal, and broader context/Resource resolver
  maturity Open.
  Phase 4ND closes the maintained Store portability gap for the bounded
  V1-local same-Store copied-target profile. The opt-in dogfood initializes one
  source Store, copies one target Store after a seed baseline, then runs three
  maintenance cycles with source reopen, semantic state creation, Checkpoint,
  Bundle export/validate/preflight/apply, target Branch head and WorkState
  digest convergence, imported Checkpoint validation, same-Store lineage-list
  checks with zero cross-Store lineage records, source and target integrity,
  source and target doctor, and target-local restore proof on a forked target
  Branch without corrupting later main-Branch applies.
  Phase 4NE broadens workload evidence with a larger maintained Store run:
  five maintenance cycles, 112 final Tasks, 20 Verification records, 80
  script-counted scheduling relation versions, five same-target Bundle applies,
  647 final payload files, 1,817 final payload references, source/target head
  and state digest
  convergence, and source/target required-valid integrity/doctor in 331
  seconds. This is bounded V1-local evidence, not a general benchmark or
  index-tuning basis.
  Phase 4NF improves operator handoff evidence for the maintained Store
  validator by appending the final success summary to the preserved `run.log`.
  A two-cycle opt-in run proved the stdout summary and final `run.log` tail are
  byte-identical while preserving per-command log output and final head/digest
  convergence. This is operator discoverability evidence, not a new workload or
  release-readiness claim.
  Phase 4NG narrows the causal-anchor `why` evolution gap: a Decision
  supersede workflow now renders `evolution_change_operations=3` for the causal
  Finding, with direct operation subjects for the prior Decision entity, the
  supersedes relation, and the derived_from causal relation. The same dogfood
  run proves the superseded prior Decision remains non-anchor with
  `evolution_change_operations=0`, and relation filters/limits do not hide the
  evolution operation projection. The release state remains false.
  Phase 4NH narrows the next `why` evolution lookup gap: the same old-main
  Store queried through the previous main binary showed
  `old_evolution_change_operations=3` but no `subject_statement_json` or
  `subject_relation_kind`; queried through the Phase 4NH binary it kept
  `new_evolution_change_operations=3`, proved `new_evolution_match=true`, and
  rendered the prior Record statement plus `record_supersedes` and
  `record_derived_from` relation source/target details. The release state
  remains false.
  Phase 4NI closes the broader operator recovery maturity gap for the bounded
  V1-local release scope. The new
  `scripts/operator-recovery-maturity-v0.1.sh` harness proves current guide
  coverage for 41 core business error codes plus `cli_parse_error`, verifies
  that only `branch_head_conflict` is retryable, and exercises parse-error JSON
  recovery, branch-head refresh retry, Resource drift/unavailable/error
  recovery to applicable, stale-gated Claim takeover, merge unresolved
  resolution/freeze/continue recovery, and final Store integrity. The release
  state remains false because context/why and candidate-release gates remain
  open.
  Phase 4NJ narrows a concrete changed-Entity `why` evolution gap. In the same
  Decision supersede workflow, the old main binary reported
  `old_prior_evolution_change_operations=0` for the superseded prior Decision.
  The Phase 4NJ binary reports `new_prior_evolution_change_operations=1`,
  `new_prior_operation_type=record.decision.supersede`,
  `new_prior_subject_statement_json="Use optimistic writes"`, and
  `new_prior_deferred_relation_family_0=evolution`, while preserving the
  causal Finding behavior at `new_finding_causal_anchor_changesets=1` and
  `new_finding_evolution_change_operations=3`. Relation-subject traversal,
  multi-hop/full evolution traversal, broader causal traversal, and broader
  context/Resource resolver maturity remain open, so the release state remains
  false.
  Phase 4NK narrows the next direct queried-Entity `why` detail gap. In one
  Assumption status workflow with two direct Entity transitions, the old main
  binary reported two operations but rendered the older validated operation
  with the final invalidated Entity version:
  `old_op1_actual_matches_final_invalidated=true` and
  `old_op1_actual_matches_operation_validated=false`. The Phase 4NK binary
  kept `new_evolution_change_operations=2` and rendered the older operation's
  own validated version with
  `new_op1_actual_matches_operation_validated=true`. Relation-subject
  traversal, multi-hop/full evolution traversal, broader causal traversal, and
  broader context/Resource resolver maturity remain open, so the release state
  remains false.
  Phase 4NL narrows the direct relation-subject endpoint evolution gap. In one
  Record supports relation removal workflow, the old main binary reported
  `old_relation_edges=0`, `old_causal_anchor_changesets=0`,
  `old_evolution_change_operations=0`, and failed
  `--expected-evolution-change-operations 1`. The Phase 4NL binary reports
  `new_evolution_change_operations=1`, `new_subject_family=relation`,
  `new_subject_relation_kind=record_supports`, and matching operation-local
  relation version plus source Finding and target Decision endpoint ids, while
  preserving `relation_edges=0` after removal. Full relation-subject traversal
  beyond direct Record-to-Record remove/restore endpoint projection,
  multi-hop/full evolution traversal, broader causal traversal, and broader
  context/Resource resolver maturity remain open, so the release state remains
  false.
  Phase 4NM narrows the next direct relation-subject endpoint evolution gap.
  Temporary Store probes showed Record-to-Knowledge and Knowledge-to-Knowledge
  relation removals queried from Knowledge endpoints both produced
  `relation_edges=0`, `causal_anchor_changesets=0`,
  `evolution_change_operations=0`, and failed
  `--expected-evolution-change-operations 1`. The Phase 4NM binary reports
  `rk_remove_evolution_change_operations=1` with
  `rk_remove_subject_relation_kind=record_supports`,
  `kk_remove_evolution_change_operations=1` with
  `kk_remove_subject_relation_kind=knowledge_supersedes`, and restore queries
  for both paths report `evolution_change_operations=2` with restore followed
  by remove. Full relation-subject traversal beyond direct remove/restore
  endpoint projection, multi-hop/full evolution traversal, broader causal
  traversal, and broader context/Resource resolver maturity remain open, so the
  release state remains false.
  Phase 4NP narrows a focused blocked-dependency Resource recovery gap.
  Pre-change public CLI context output for a Session focused on a dependent
  Task showed `blocked_dependency_items=1` and
  `verification_requirement_items=0`, but
  `context_has_any_refresh_hint=false`,
  `blocked_dependency_has_resource_basis=false`, and
  `blocked_dependency_has_refresh_hint=false` even though the blocking
  prerequisite Task had a Resource-backed Verification Requirement. The Phase
  4NP binary keeps the packet schema unchanged and appends summary text to the
  existing `blocked_dependency` item with
  `dependency_resource_requirements=1`, `dependency_acceptance_criterion`,
  `dependency_verification_requirement`, `dependency_vr_local_key`,
  `resource_basis=1`, `verification_id`, `resource_id`,
  `adapter=local-file@1`, `scope=path@1`, `baseline_observation_id`, and a
  basis-aware `refresh_hint`. Broader context/Resource resolver maturity, full
  relation-subject traversal, multi-hop/full evolution traversal, and broader
  causal traversal remain open, so the release state remains false.
  Phase 4NO narrows the direct relation-create endpoint evolution gap.
  Pre-change public CLI output for a newly created Record supports relation
  showed `relation_edges=1` but `evolution_change_operations=0` and failed
  `--expected-evolution-change-operations 1`. The Phase 4NO binary keeps the
  current relation edge and reports `evolution_change_operations=1` with
  `record.relation.create` and `record_supports` detail for Record-to-Record
  and Record-to-Knowledge endpoints, plus `knowledge.relation.create` and
  `knowledge_supersedes` detail for Knowledge-to-Knowledge endpoints. Full
  relation-subject traversal beyond direct create/remove/restore endpoint
  projection, multi-hop/full evolution traversal, broader causal traversal,
  and broader context/Resource resolver maturity remain open, so the release
  state remains false.
  Phase 4NQ narrows the Task scheduling relation `why` endpoint gap.
  Pre-change public CLI output for a newly created `depends_on` relation
  showed `dependent_relation_edges=0`,
  `dependent_evolution_change_operations=0`, `prereq_relation_edges=0`, and
  `prereq_evolution_change_operations=0` even though
  `task scheduling-list` exposed the relation. The Phase 4NQ binary reports
  `task_depends_on` relation edges and direct
  `task.scheduling_relation.create` evolution for dependent and prerequisite
  Task endpoints, plus `task_ordered_before` relation edges and direct create
  evolution for earlier and later Task endpoints. Full relation-subject
  traversal beyond direct endpoint slices, multi-hop/full evolution traversal,
  broader causal traversal, and broader context/Resource resolver maturity
  remain open, so the release state remains false.
  Phase 4NR narrows the primary containment `why` endpoint evolution gap.
  Pre-change public CLI output for Goal -> Plan and Plan -> Task containment
  showed `relation_edges=1`,
  `relation.0.relation_kind=primary_containment`, and
  `relation.0.direction=incoming` for the contained Task, but
  `evolution_change_operations=0` and an expected-count failure for the missing
  direct create operation. The Phase 4NR binary reports direct
  `primary_containment.create` endpoint evolution for queried Goal, Plan, and
  Task endpoints, with relation subject detail for relation kind, relation
  version, source endpoint, target endpoint, and state digest. Full
  relation-subject traversal beyond direct endpoint slices, multi-hop/full
  evolution traversal, broader causal traversal, and broader context/Resource
  resolver maturity remain open, so the release state remains false.
  Phase 4NX narrows the Evidence endpoint side of the `evidenced_by` `why`
  relation evolution gap. Pre-change public CLI output for one Evidence reused
  by two Verifications showed two incoming `evidenced_by` relation edges but
  `evolution_change_operations=0`, and
  `--expected-evolution-change-operations 2` failed with
  `error_code=query_invalid`. The Phase 4NX binary reports the two direct
  `verification.record` `evidenced_by` relation operations from
  `workvcs why --evidence <Evidence> --relation-kind evidenced_by`, preserving
  existing relation/evolution fields and adding no schema or CLI surface.
  Full relation-subject traversal beyond direct endpoint slices,
  multi-hop/full evolution traversal, broader causal traversal, and broader
  context/Resource resolver maturity remain open, so the release state remains
  false.
  Phase 4NY narrows the KnowledgeExposure derived-from direct endpoint `why`
  relation evolution gap. Pre-change public CLI output for an explicit
  `knowledge-exposure-derived-from-link` showed one outgoing
  `knowledge_exposure_derived_from` edge from the source Knowledge endpoint and
  one incoming edge from the target KnowledgeExposure endpoint, but
  `evolution_change_operations=0` for both endpoint queries and
  `--expected-evolution-change-operations 1` failed with
  `error_code=query_invalid`. The Phase 4NY binary reports direct
  `knowledge.relation.create` endpoint evolution for both queries, preserving
  existing relation/evolution fields and adding no schema or CLI surface. Full
  relation-subject traversal beyond direct endpoint slices, multi-hop/full
  evolution traversal, broader causal traversal, and broader context/Resource
  resolver maturity remain open, so the release state remains false.
  Phase 4NU narrows one focused same-Plan peer Resource context gap. The
  pre-change public CLI probe created two direct sibling Tasks under one Plan;
  the sibling was runnable and had a Resource-backed Verification Requirement.
  With no focus, full context showed the sibling Resource basis and
  basis-aware refresh hint, but when focused on the current Task, brief,
  normal, and full context all omitted the sibling Resource-backed VR while
  full context reported zero omitted items. The Phase 4NU binary keeps brief
  context focused and adds normal/full `verification_requirement` summary text
  for runnable same-Plan peer Tasks with `same_plan_peer_task`, `parent_plan`,
  `peer_runnable=true`, `criterion`, `resource_basis=1`, `verification_id`,
  `resource_id`, `adapter=local-file@1`, `scope=path@1`,
  `baseline_observation_id`, and the basis-aware `refresh_hint`. Broader
  context/Resource resolver maturity outside this direct same-Plan peer slice,
  full relation-subject traversal, multi-hop/full evolution traversal, and
  broader causal traversal remain open, so the release state remains false.
  Phase 4NZ narrows one focused same-Goal cross-Plan peer Resource context gap.
  The pre-change public CLI probe created one Goal with two direct child Plans,
  one runnable Task under each Plan, and a Resource-backed Verification
  Requirement on the cross-Plan peer Task. With no focus and with focus on the
  peer Task, context showed the peer Resource basis and basis-aware refresh
  hint, but when focused on the current Task, normal/full context and focused
  `claim next --context-profile full` omitted the peer Resource-backed VR while
  full context reported zero omitted items. The Phase 4NZ binary keeps brief
  context focused and adds normal/full `verification_requirement` summary text
  for runnable same-Goal cross-Plan peer Tasks with `same_goal_peer_task`,
  `parent_goal`, `focused_plan`, `peer_plan`, `peer_runnable=true`,
  `criterion`, `resource_basis=1`, `verification_id`, `resource_id`,
  `adapter=local-file@1`, `scope=path@1`, `baseline_observation_id`, and the
  basis-aware `refresh_hint`. Broader context/Resource resolver maturity
  outside these direct peer slices, full relation-subject traversal,
  multi-hop/full evolution traversal, and broader causal traversal remain open,
  so the release state remains false.
  Phase 4OC narrows one Session focus contract mismatch. The pre-change public
  CLI probe created a Resource-backed Verification Requirement and showed that
  `session focus-set --focus <verification_requirement>` succeeded even though
  downstream `context` and `runnable tasks` returned `session_invalid` because
  focus resolution only supports current Goal, Plan, and Task entities. The
  Phase 4OC binary rejects that unsupported focus kind before writing focus
  rows, preserves `focus_entity_id=none`, and keeps valid Task-focus context
  showing the Resource-backed VR `resource_basis=1` and basis-aware
  `verification cache-refresh --resource-content-from-basis` hint. This does
  not add new focus kinds or broaden context/Resource resolver behavior, so
  the release state remains false.
  Phase 4OE narrows one Task closeout `why` explanation gap. The corrected
  pre-change public CLI probe showed a done Task and verified Acceptance
  Criterion backed by VR -> Verification -> Evidence, while Task and AC `why`
  could not identify the AC, VR, Verification, or Evidence ids from the
  closeout query. The Phase 4OE binary adds read-only
  `verification_closure_chains` key-value output for Task and Acceptance
  Criterion `why`, with current AC, VR, Verification, result, and Evidence
  ids. Direct VR/Verification/Evidence endpoint controls remain passing. Full
  relation-subject traversal beyond direct endpoint slices, multi-hop/full
  evolution traversal, broader causal traversal, and broader context/Resource
  resolver maturity remain open, so the release state remains false.
  Phase 4OG narrows the Resource-backed version of that closeout `why`
  explanation gap. The Phase 4OF pre-change public CLI probe showed that
  `verification show` exposed a Resource-backed Verification's basis while the
  Task and Acceptance Criterion `why` closure chains omitted it. The Phase 4OG
  binary keeps the existing closure-chain projection and adds read-only
  Resource basis fields inside each chain: `resource_id`, adapter kind/version,
  scope kind/version, canonical `scope_payload_json`,
  `baseline_observation_id`, and `baseline_fingerprint`. The public CLI
  dogfood proves the Task and Acceptance Criterion `why` output carry the same
  Resource basis after closeout. Full relation-subject traversal beyond direct
  endpoint slices, multi-hop/full evolution traversal, broader causal
  traversal, and broader context/Resource resolver maturity remain open, so
  the release state remains false.
  Phase 4OH is a read-only follow-up probe for that same Resource-backed
  closeout path. The probe at
  `/tmp/workvcs-4oh-recovery-from-why-probe-20260904T011704Z` proves
  `probe_execution_status=PASS`, `probe_result=RECOVERY_SUPPORTED`,
  `commands_exit_failures=0`, `failed_assertions=0`,
  `task_why_verification_id_present=true`,
  `ac_why_verification_id_present=true`, `task_why_resource_basis=true`,
  `ac_why_resource_basis=true`, `task_why_basis_matches_verification=true`,
  `ac_why_basis_matches_verification=true`,
  `cache_refresh_from_why_id=true`, and `post_refresh_ac_status=verified`.
  The probe found no new implementation gap and did not change code, schema,
  CLI flags, tests, ADR authority, or release state.
  Phase 4OK narrows the direct Plan version of that closeout `why`
  explanation gap. The Phase 4OJ read-only probe showed that Plan `why` could
  already see direct primary containment edges to the Task, but still reported
  `verification_closure_chains=0` after Task/Plan/Goal closeout. The Phase 4OK
  binary reuses the existing Task closure-chain projection for Tasks directly
  contained by the queried Plan, including Resource basis fields. The public
  CLI dogfood proves `plan_why_closure_chains=true`,
  `plan_why_resource_basis=true`,
  `cache_refresh_from_plan_why_id_applicable=true`, and
  `goal_why_closure_chains=true` for the expected zero-chain Goal control.
  Goal ancestor rollup, full relation-subject traversal, multi-hop/full
  evolution traversal, broader causal traversal, and broader context/Resource
  resolver maturity remain open, so the release state remains false.
  Phase 4NN narrows a concrete brief ContextPacket Resource recovery gap.
  Pre-change public CLI context output showed the current
  `verification_requirement` item for a Resource-backed VR but omitted the
  Resource basis details and basis-aware cache-refresh command. The Phase 4NN
  binary keeps the packet schema unchanged and appends summary text with
  `resource_basis=1`, `verification_id`, `resource_id`, `adapter=local-file@1`,
  `scope=path@1`, `baseline_observation_id`, and a `refresh_hint` for
  `verification cache-refresh --verification <id> --resource-content-from-basis`.
  Multiple matching Resource-backed Verifications count all matching Resource
  basis entries while preferring a basis-refreshable Verification for the
  displayed command.
  Broader context/Resource resolver maturity, full relation-subject traversal,
  multi-hop/full evolution traversal, and broader causal traversal remain open,
  so the release state remains false.
  Phase 4MM repeats merge lifecycle dogfood on a larger and more varied local
  Store: 12 shared Task conflicts, 8 source-only Tasks, 6 target-only Tasks, 4
  source-side scheduling Relations, all 24 merge items explicitly resolved,
  freeze/continue, a two-parent merge commit, expected final WorkState, and
  required-valid integrity/doctor. The run also corrects a dogfood script
  assumption: target Branch `history` is first-parent, so source-side ordinary
  commits are proven through the secondary parent and final WorkState rather
  than counted inline in target first-parent history.
