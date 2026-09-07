# V1 Release Gate Matrix

Status: current release-maturity gate matrix
Last refreshed: 2026-09-07 by Phase 4PA V1 bounded Context/Resource/why scope decision and candidate validation prep

This matrix is an evidence map for deciding whether the local Rust V0.1
implementation can support a V1 release-maturity claim. It is not a product
specification and does not replace the confirmed product, architecture, schema,
or accepted ADR authorities.

The source-state basis at the start of Phase 4PA was main commit
`60170f067dcdfcc7a65e13897332fcda8a3d71cb` and the V1 readiness ledger last
refreshed by Phase 4OY structural-reference two-level nested Plan target
Context Resource recovery. Historical
governance Plans and logs are treated only as provenance unless their
conclusions are reflected in current project documents or current validation
evidence.

## Current Decision

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false` until a later matrix refresh records fresh
  candidate validation from an exact commit and the remaining candidate gate no
  longer blocks.

Phase 4PA records a current user-authorized release-scope decision: V1 local release maturity is bounded to deterministic local workflows already evidenced in current docs through Phase 4OY. Deeper nested Plan traversal beyond the currently evidenced two-level Plan target boundary, multi-hop structural references, broader Context/Resource/why traversal, full relation-subject traversal, multi-hop/full evolution traversal, and broader causal traversal are deferred post-V1 unless a later explicit scope decision reopens them.

This changes the Context resolver, packets, and `why` explanations gate from `Partial` / `Blocks V1 release=Yes` to `Pass` / `Blocks V1 release=No` for the bounded V1-local release scope. It does not change code, schema, packet schema, public CLI flags, ADR semantics, release state, or candidate validation status.

Candidate release operation remains `Blocked` until complete local candidate validation runs from an exact commit and a later matrix refresh records the result.

## Gate States

- `Pass`: current project authority and validation evidence satisfy the named
  gate for the V1-local scope.
- `Partial`: current evidence is real, but it is too narrow, too scripted, or
  missing named scenarios required for release maturity.
- `Blocked`: the gate cannot support a release claim until the named evidence
  exists.

## Gate Matrix

| Gate | Required for V1 release maturity | Current evidence | Status | Blocks V1 release | Required next evidence |
| --- | --- | --- | --- | --- | --- |
| Product authority and V2 boundary | Confirmed product/domain/architecture/schema/ADR authorities define the release scope, and V2 exclusions remain explicit. | `docs/README.md`, product and architecture docs, schema v0.1, accepted ADRs, and the V2 exclusions in `v1-readiness-ledger.md`. | Pass | No | Keep future release claims bound to current authority and continue excluding transcript parsing, LLM semantics, orchestration, cloud sync, federation, and destructive compaction. |
| Core Store, lineage, integrity, and local portability | Store bootstrap/open/manifest/lineage/doctor and local Bundle/Checkpoint portability work across ordinary and maintained Stores. | Smoke coverage plus Phase 4LO, Phase 4LP, Phase 4LQ, and Phase 4ND provenance, including bounded copied-target portability, larger Store portability, and repeated maintained Store reopen/checkpoint/apply/integrity/doctor cycles. | Pass | No | Keep as regression foundation; broaden only if a future ordinary or maintained Store workflow exposes a concrete Store, lineage, integrity, or portability gap. |
| Workspace, Branch, history, diff, show-at, and restore | Branch and state navigation workflows are proven in real implementation work, not only narrow smoke or post-Bundle inspection. | Current ledger marks this area implemented with partial smoke and dogfood evidence. Phase 4LO dogfoods `restore` and `show-at` against a target Store. Phase 4MQ dogfoods Branch fork, bidirectional Branch diff, Branch history, Branch `show-at`, and two-Branch integrity in a real repository delivery Store. | Pass | No | Keep as regression foundation; broaden only if a future real Branch/diff workflow exposes a concrete gap. |
| Goal, Plan, Task, ordering, dependencies, and containment | Work graph planning and dependency semantics are repeatedly used in real project workflows through closeout. | Phase 4LV dogfoods Goal/Plan/Task for a bounded external-project review. Phase 4NB repeats the workflow against a temporary clone of pre-existing `agent_soul` with one Goal, one Plan, three contained Tasks, two `depends_on` relations, two `ordered_before` relations, blocked dependency context, dependency readiness recovery, and Plan/Goal closeout. | Pass | No | Keep as regression foundation; broaden only if a future real workflow exposes a concrete ordering or containment gap. |
| AC, VR, Verification, Evidence, and verification wrapper | Acceptance and verification records can close obligations through the CLI and remain understandable in recovery and handoff scenarios. | Ledger marks AC/VR/Verification/Evidence and the top-level `verify` wrapper as dogfood-proven for current covered scenarios. Phase 4NB proves AC/VR closure in a recovery Handoff scenario: stale Resource-backed applicability blocks Task closeout, explicit refresh projects `resource_drift`, and recovery `verify` records new evidence before Task/Plan/Goal closeout. | Pass | No | Keep as regression foundation; broaden only if a future recovery or handoff loop exposes a concrete evidence-closure gap. |
| Resource registration, observation, applicability, and drift | Resource-backed verification covers explicit basis refresh, unavailable/error states, drift projection, adapter boundaries, and re-observation policy. | Phases 4LV, 4LX through 4MG, and 4MN cover exact path, path-prefix, glob, Git worktree, persisted-basis, and batch basis refresh scenarios; Phase 4MV covers explicit no-renames/delete-add Git rename-policy scenarios; Phase 4MW covers explicit tracked-symlink Git index/diff and untracked-non-regular Resource error-policy scenarios; Phase 4MX covers explicit parent-Git submodule gitlink/status/diff and disabled-recursion policy scenarios; Phase 4MY covers explicit parent-Git sparse-checkout index/status/diff and disabled-expansion policy scenarios; Phase 4MZ covers explicit no-WorkVCS-case-folding scenarios for local-file exact path, path-prefix, glob, and Git worktree Resource observations; Phase 4NA covers explicit foreground operator-triggered re-observation scheduling through current-head Resource-backed batch refresh, with background re-observation disabled. | Pass | No | Keep as regression foundation; background daemons, watchers, automatic polling, implicit refresh, and Agent orchestration remain outside V1. |
| Session, Claim, Runnable, `claim next`, and `next` | Continuation, focus, and Claim guard flows remain usable across stale recovery and multi-operator scenarios. | Ledger marks the current Session/Runnable/claim-next surface dogfood-proven. Phase 4LW and Phase 4MI cover Claim transfer, stale takeover, and shared read-only collaboration. Phase 4MR covers shared-Claim write/read-write coordination in this repository: non-unique shared Claims block protected writer mutation, then reader release restores unique-writer closeout. Phase 4OC covers unsupported focus-kind fail-fast: `session focus-set` rejects a current Verification Requirement focus with stable `session_invalid`, leaves the Session unfocused, and preserves valid Task-focus context recovery hints. | Pass | No | Keep as regression foundation; automatic ownership arbitration, distributed collaboration, and remote multi-operator coordination remain outside V1 unless explicitly authorized. |
| Context resolver, packets, and `why` explanations | Context packets and `why` output expose enough focused, causal, and explanatory state for continuation Agents without speculative LLM extraction. | Phases 4LR through 4LT and 4LX cover scoped packets and rationale projection; Phases 4MA, 4MH, and 4ML cover selected `why` relationships and causal anchors. Phase 4NC covers direct Record-to-Record and Record-to-Knowledge epistemic statement projections in `why`. Phase 4NG covers direct ChangeOperation Entity/Relation subject projection for causal-anchor ChangeSets in `why`. Phase 4NH covers current recognized subject detail for those direct operation subjects, including Record/Knowledge statements and Relation kind/source/target detail. Phase 4NJ covers direct first-parent Entity-subject evolution operations for queried changed Entities that are not causal anchors. Phase 4NK covers operation-local Entity detail for multiple direct queried-Entity evolution operations. Phase 4NL covers direct Record-to-Record relation remove/restore operations as endpoint evolution for queried Entity endpoints, including operation-local relation detail when a removed relation is absent from current `relation_edges`. Phase 4NM covers direct Record-to-Knowledge and Knowledge-to-Knowledge relation remove/restore operations as endpoint evolution for queried Knowledge endpoints. Phase 4NO covers direct Record-to-Record, Record-to-Knowledge, and Knowledge-to-Knowledge relation create operations as endpoint evolution for queried Entity endpoints. Phase 4NN covers brief ContextPacket Resource basis recovery hints for current-task Verification Requirements with current-head Resource-backed Verifications, without changing packet schema. Phase 4NP covers brief ContextPacket Resource basis recovery hints on focused `blocked_dependency` items when the blocking dependency Task has current-head Resource-backed Verifications, without changing packet schema. Phase 4NU covers normal/full ContextPacket Resource basis recovery hints for runnable same-Plan peer Tasks when focus is a Task, without changing packet schema or public CLI flags. Phase 4NZ covers normal/full ContextPacket Resource basis recovery hints for runnable same-Goal cross-Plan peer Tasks when focus is a Task, without changing packet schema or public CLI flags. Phase 4OP proves Plan-focused ContextPacket normal/full output already surfaces a directly contained runnable Task's Resource-backed Verification Requirement, Resource basis, and basis-aware refresh hint, and that focused `claim next --context-profile full` preserves the same recovery path. Phase 4OQ proves Goal-focused ContextPacket normal/full output already surfaces the same recovery path for a direct Goal-to-Plan-to-Task shape, and that focused `claim next --context-profile full` preserves it. Phase 4OR proves Goal-focused ContextPacket normal/full output already surfaces two direct Goal child Plans' runnable Task Resource-backed Verification Requirement recovery paths with Resource basis, baseline observation, and basis-aware refresh hints, and that focused `claim next --context-profile full` preserves the selected target recovery path. Phase 4OS proves parent Plan-focused and Goal-focused ContextPacket normal/full output already surfaces a one-level nested SubPlan Task Resource-backed Verification Requirement recovery path with Resource basis, baseline observation, and basis-aware refresh hint, and that focused `claim next --context-profile full` preserves the selected nested Task recovery path. Phase 4OT proves a focused blocked Task already surfaces a different-Goal prerequisite Task's Resource-backed Verification Requirement recovery path in the existing `blocked_dependency` item, and an unfocused `claim next --context-profile full` selects the runnable prerequisite with the same recovery path. Phase 4OU covers normal/full ContextPacket Resource basis recovery hints for direct Plan/Goal structural references to Resource-backed Tasks, without changing packet schema or public CLI flags. Phase 4OV covers normal/full ContextPacket Resource basis recovery hints for direct Plan/Goal structural references to Plans with direct child Resource-backed Tasks, without changing packet schema or public CLI flags. Phase 4OW covers normal/full ContextPacket Resource basis recovery hints for direct Plan/Goal structural references to Plans whose one-level child Plans directly contain Resource-backed Tasks, without changing packet schema or public CLI flags. Phase 4OY covers normal/full ContextPacket Resource basis recovery hints for direct Plan/Goal structural references to Plans whose child Plans contain child Plans that directly contain Resource-backed Tasks, without changing packet schema or public CLI flags. Phase 4NQ covers current Task scheduling relation edges and direct `task.scheduling_relation.create` endpoint evolution for `depends_on` and `ordered_before` Task endpoints. Phase 4NR covers direct `primary_containment.create` endpoint evolution for queried Goal, Plan, and Task endpoints, reusing existing `primary_containment` relation edges and relation subject detail. Phase 4NV covers direct `verification.record` defining `verifies` relation creation as endpoint evolution for queried Verification, Acceptance Criterion, and Verification Requirement endpoints. Phase 4NW covers direct `verification.record` `evidenced_by` relation creation as endpoint evolution for queried Verification endpoints backed by Evidence. Phase 4NX covers direct `verification.record` `evidenced_by` relation creation as endpoint evolution for queried Evidence endpoints reused by Verifications. Phase 4NY covers direct `knowledge_exposure_derived_from` relation create operation as endpoint evolution for queried source Knowledge and target KnowledgeExposure endpoints. Phase 4OE covers current VR-backed Verification closure chains from Task and Acceptance Criterion `why`, exposing the AC, VR, Verification, result, and Evidence ids after Task closeout. Phase 4OG covers Resource basis fields inside those same Task/Acceptance Criterion closure chains for Resource-backed Verifications, exposing resource id, adapter, scope, canonical scope payload JSON, baseline observation id, and baseline fingerprint. Phase 4OK covers Plan `why` closure chains for directly contained Tasks, exposing the same AC, VR, Verification, Evidence, and Resource basis fields while leaving Goal ancestor closure unsupported. Phase 4OM proves the current Goal-start recovery route: Goal `why` exposes the direct Plan, Plan `why` exposes the direct Task closure and Resource basis, and basis-aware refresh from the Plan-discovered Verification id restores the AC to verified; it finds no concrete implementation gap requiring Goal ancestor rollup. Phase 4PA records current user authorization that deeper nested, multi-hop, and broader Context/Resource/why traversal are deferred post-V1 and that the bounded deterministic local workflow evidence satisfies this gate. | Pass | No | Run candidate validation prep from the exact local commit. Keep deeper nested Plan traversal beyond the currently evidenced two-level Plan target boundary, multi-hop structural references, broader Context/Resource/why traversal, full relation-subject traversal, multi-hop/full evolution traversal, broader causal traversal, and broader epistemic traversal deferred post-V1 unless a later explicit scope decision reopens them. |
| Handoff consumption | Handoff creation, display, focus consumption, blocked recovery, and continuation work across varied project and write-mode workflows. | Current evidence covers focused Handoff smoke, Handoff consumption, blocked recovery, read-only external-project Handoff creation/show, external-project continuation-adjacent Claim work, and Phase 4MS write-mode Handoff consumption with continuation Claim, VR-backed verification, Task closeout, and SessionDiff closeout. | Pass | No | Keep as regression foundation; remote/cloud Handoff, cross-Store synchronization, automatic takeover, and Agent orchestration remain outside V1 unless explicitly authorized. |
| Merge lifecycle and conflict recovery | Divergent Work Branch resolution, freeze/continue/abort/restart recovery, and final WorkState proof hold in realistic write-mode external-project work. | Phase 4LN and Phase 4MM prove merge behavior in durable local Stores, including larger conflict sets and two-parent merge commits. Phase 4MT adds generated external local Git project write-mode merge proof. Phase 4MU repeats merge against pre-existing real `agent_soul` project content cloned into a write-mode sandbox with an actual Git conflict on existing `README.md`, WorkVCS conflict and auto merge items, unresolved freeze guard, explicit source-side resolutions, freeze/continue, a two-parent WorkVCS merge commit, final WorkState proof, Branch diff, SessionDiff closeout, original-project unchanged proof, and required-valid integrity/doctor. | Pass | No | Keep as regression foundation; semantic/LLM merge, remote or distributed merge, cross-Store synchronization, Agent orchestration, and direct mutation of an original external repository remain outside the bounded V1-local release gate unless separately authorized. |
| Operator discoverability and actionable recovery | Operators and scripts can identify failures, choose recovery, and parse error output without source inspection. | Phase 4LZ, Phase 4MJ, Phase 4MK, and Phase 4MO cover stable key-value and JSON error output plus per-code recovery guidance for current error codes. Phase 4NF makes the maintained Store portability validator's successful preserved `run.log` self-contained by appending the final stdout summary to the log tail, proving stdout/log-tail equality in a real opt-in run. Phase 4NI adds a local recovery maturity matrix proving guide coverage for all 41 core business error codes plus `cli_parse_error`, the retryability rule, parse-error JSON recovery, branch-head retry, Resource drift/unavailable/error recovery to applicable, stale-gated Claim takeover, merge unresolved recovery, and final Store integrity. Phase 4NN exposes Resource-backed VR basis-aware cache-refresh hints directly in brief current-task context. Phase 4NP exposes the same recovery path in focused `blocked_dependency` context for blocking prerequisite Tasks. Phase 4NU exposes the same recovery command in normal/full context for runnable same-Plan peer Tasks while keeping brief context focused. Phase 4NZ exposes the same recovery command in normal/full context for runnable same-Goal cross-Plan peer Tasks while keeping brief context focused. Phase 4OP proves Plan-focused context and focused `claim next --context-profile full` already expose the same basis-aware recovery command for a directly contained runnable Task. Phase 4OQ proves Goal-focused context and focused `claim next --context-profile full` already expose the same command for a direct Goal-to-Plan-to-Task Resource-backed VR path. Phase 4OR proves the same command remains visible for two direct Goal child Plans with two runnable Resource-backed Tasks and for the selected target after focused `claim next --context-profile full`. Phase 4OS proves the same command remains visible through one nested SubPlan from parent Plan-focused and Goal-focused context and for the selected nested Task after focused `claim next --context-profile full`. Phase 4OT proves the same command remains visible for a different-Goal prerequisite Task through the focused dependent Task's `blocked_dependency` item and through an unfocused `claim next` on the runnable prerequisite. Phase 4OU exposes the same basis-aware recovery command in normal/full context for direct Plan/Goal structural references to Resource-backed Tasks. Phase 4OV exposes the same basis-aware recovery command in normal/full context for direct Plan/Goal structural references to Plans with direct child Resource-backed Tasks. Phase 4OW exposes the same basis-aware recovery command in normal/full context for direct Plan/Goal structural references to Plans whose one-level child Plans directly contain Resource-backed Tasks. Phase 4NQ exposes Task scheduling relation endpoint IDs and direct create operation detail through existing `why` key-value fields and expected-count flags. Phase 4NR exposes primary containment direct create operation detail through existing `why` relation and evolution key-value fields for Goal, Plan, and Task endpoints. Phase 4NV exposes defining `verifies` relation endpoint IDs and direct `verification.record` operation detail through existing `why` key-value fields and expected-count flags for Verification, Acceptance Criterion, and Verification Requirement endpoints. Phase 4NW exposes Evidence-backed `evidenced_by` relation endpoint IDs and direct `verification.record` operation detail through existing `why` key-value fields and expected-count flags for queried Verification endpoints. Phase 4NX exposes incoming Evidence-backed `evidenced_by` relation endpoint IDs and direct `verification.record` operation detail through existing `why` key-value fields and expected-count flags for queried Evidence endpoints. Phase 4NY exposes `knowledge_exposure_derived_from` relation endpoint IDs and direct `knowledge.relation.create` operation detail through existing `why` key-value fields and expected-count flags for queried Knowledge and KnowledgeExposure endpoints. Phase 4OE exposes Task closeout AC -> VR -> Verification -> Evidence closure ids directly in Task and Acceptance Criterion `why`, reducing repeated manual endpoint queries for that closeout path. Phase 4OG exposes Resource basis fields inside those same closure chains, reducing the Resource-backed closeout recovery hop to `verification show`. Phase 4OK exposes the same Resource-backed closure fields from Plan `why` for directly contained Tasks, reducing a Plan-level closeout recovery hop to Task, Acceptance Criterion, or Verification detail; dogfood proves cache refresh works from the Plan-discovered Verification id. | Pass | No | Keep as regression foundation; reduce command friction only where future dogfood exposes repeated workflow blockage. |
| Larger Store and performance evidence | Candidate release behavior is bounded by workload evidence that is larger and more varied than smoke, with integrity/doctor proof. | Phase 4LQ validates portability on a bounded larger Store; Phase 4MM validates a larger merge-path Store; Phase 4NE validates a larger maintained Store with five cycles, 112 final Tasks, 20 Verifications, 80 script-counted scheduling relation versions, five same-target applies, 647 final payload files, 1,817 final payload references, and source/target required-valid integrity/doctor in 331 seconds. | Pass | No | Keep as bounded regression evidence; do not design indexes or claim general performance maturity without future workload-specific profiling. |
| Candidate release operation | A named candidate commit has a fresh full validation matrix, clean git state, current governance status, refreshed release gate matrix, and explicit release authorization for any release/tag/push/deploy action. | No candidate validation has passed yet for the current exact commit. Phase 4PA makes candidate validation prep the remaining local release-maturity gate, while release, tag, push, deploy, remote, production, and credential actions remain outside current authority. | Blocked | Yes | Run the complete local candidate validation matrix from the exact candidate commit, including schema, core tests, smoke, key dogfood/probe coverage, docs gate consistency, clean Git, and `workctl work status`; only after PASS may a later matrix refresh consider positive values for `V0_1_DOGFOOD_COMPLETE` and `RELEASE_CANDIDATE_ALLOWED`. |

## Next Highest-Value Work

Use the blocking rows above as the release-oriented queue. The next local slice
should name the exact gate it advances and should prefer real dogfood evidence
over speculative broadening.

Priority candidates:

1. Expand context/Resource resolver or `why` behavior only when a dogfood
   continuation exposes a concrete causal, evolution, or broader explanation
   gap.
2. Reduce command friction only where future dogfood exposes repeated workflow
   blockage.
3. After all functional/dogfood gates pass, run release-candidate validation
   from the exact candidate commit and obtain explicit release authorization.
4. Refresh this matrix after each blocking gate changes status and before any
   release-ready or release-candidate claim.

## Phase 4PA Update

Phase 4PA refreshes this matrix after the current user-authorized scope
decision: deeper nested Plan traversal beyond the currently evidenced two-level
Plan target boundary, multi-hop structural references, broader
Context/Resource/why traversal, full relation-subject traversal, multi-hop/full
evolution traversal, and broader causal traversal are deferred post-V1 unless a
later explicit scope decision reopens them.

The evidence basis is the current main commit
`60170f067dcdfcc7a65e13897332fcda8a3d71cb`, the Phase 4OY ledger and matrix
state, accepted ADR-0412 and ADR-0457 authority for the ledger/matrix surfaces,
the V1/V2 boundary, and the Phase 4PA short-status log at
`/tmp/workvcs-4pa-v1-bounded-context-scope-refresh-20260907T062741Z`. Phase 4PA
does not change code, Store schema, ContextPacket schema, public CLI flags, ADR
semantics, tests, runtime behavior, release state, or remote state.

The Context resolver, packets, and `why` explanations gate is now `Pass` for
the bounded V1-local release scope. The remaining blocking local gate is
Candidate release operation: a complete validation matrix must run from the
exact candidate commit before `V0_1_DOGFOOD_COMPLETE` or
`RELEASE_CANDIDATE_ALLOWED` can be reconsidered.

The release decision remains:

```text
V1_RELEASE_READY=false
V0_1_DOGFOOD_COMPLETE=false
RELEASE_CANDIDATE_ALLOWED=false
```

## Phase 4OY Update

Phase 4OY refreshes this matrix with a bounded direct structural-reference
two-level nested Plan target Context/Resource recovery implementation and
validation. It does not change Store schema, packet schema, public CLI flags,
`why`, runnable, `claim next`, structural reference semantics, ADR authority,
or release state.

The pre-change public CLI probe at
`/tmp/workvcs-4ox-structural-reference-two-level-nested-plan-target-context-resource-probe-20260907T055440Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_TWO_LEVEL_NESTED_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_UNSUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_plan_target=false`,
`plan_context_has_first_nested_plan=false`,
`plan_context_has_second_nested_plan=false`,
`plan_context_has_target_task=false`, `plan_context_has_vr=false`,
`plan_context_has_resource_basis=false`,
`plan_context_has_refresh_hint=false`,
`goal_context_has_plan_target=false`,
`goal_context_has_first_nested_plan=false`,
`goal_context_has_second_nested_plan=false`,
`goal_context_has_target_task=false`, `goal_context_has_vr=false`,
`goal_context_has_resource_basis=false`,
`goal_context_has_refresh_hint=false`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, `repo_dirty=0`, and
`release_positive_hits=0`.

The Phase 4OY implementation adds normal/full ContextPacket Resource-backed
Verification Requirement recovery hints for direct structural references from
a focused Plan or Goal to a Plan target's child Plans whose child Plans
directly contain Tasks. The summary includes the referenced Plan, first nested
Plan, second nested Plan, direct child Task, referrer id, referrer kind,
relation id, all three containment relation ids, direct-reference marker,
Resource basis, baseline observation, and basis-aware cache-refresh hint.
Brief context, direct Task structural-reference context, direct Plan target
context, one-level nested Plan target context, same-Plan peer context,
same-Goal peer context, and blocked-dependency context behavior remain covered
by targeted tests.

The post-change public CLI probe at
`/tmp/workvcs-4ox-structural-reference-two-level-nested-plan-target-context-resource-probe-20260907T055815Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_TWO_LEVEL_NESTED_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_plan_target=true`,
`plan_context_has_first_nested_plan=true`,
`plan_context_has_second_nested_plan=true`,
`plan_context_has_target_task=true`, `plan_context_has_vr=true`,
`plan_context_has_resource_basis=true`,
`plan_context_has_refresh_hint=true`,
`goal_context_has_plan_target=true`,
`goal_context_has_first_nested_plan=true`,
`goal_context_has_second_nested_plan=true`,
`goal_context_has_target_task=true`, `goal_context_has_vr=true`,
`goal_context_has_resource_basis=true`,
`goal_context_has_refresh_hint=true`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, and `release_positive_hits=0`.

Targeted core tests cover the new two-level nested referenced Plan recovery
path and regressions for one-level nested Plan target, direct Plan target, and
direct Task structural-reference Resource-backed recovery hints.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OY closes only direct Plan/Goal structural references to Plan targets
whose child Plans contain child Plans that directly contain Resource-backed
Tasks. Deeper nested Plan traversal, multi-hop structural references, broad
Resource resolver maturity, full relation-subject traversal, multi-hop/full
evolution traversal, broader causal traversal, and candidate release
validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OY records that the same basis-aware recovery command is visible in
normal/full context for direct Plan/Goal structural references to two-level
nested Plan target Tasks.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OW Update

Phase 4OW refreshes this matrix with a bounded direct structural-reference
nested Plan target Context/Resource recovery implementation and validation. It
does not change Store schema, packet schema, public CLI flags, `why`,
runnable, `claim next`, structural reference semantics, ADR authority, or
release state.

One local drift attempt after Phase 4OV reused the Phase 4OV probe shape while
setting up Phase 4OW. It produced only `/tmp` output, did not modify
repository state, and is not used as Phase 4OW behavioral evidence.

The pre-change public CLI probe at
`/tmp/workvcs-4ow-structural-reference-nested-plan-target-context-resource-probe-20260907T053659Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_NESTED_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_UNSUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_plan_target=false`,
`plan_context_has_nested_plan=false`,
`plan_context_has_target_task=false`, `plan_context_has_vr=false`,
`plan_context_has_resource_basis=false`,
`plan_context_has_refresh_hint=false`,
`goal_context_has_plan_target=false`,
`goal_context_has_nested_plan=false`,
`goal_context_has_target_task=false`, `goal_context_has_vr=false`,
`goal_context_has_resource_basis=false`,
`goal_context_has_refresh_hint=false`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, `repo_dirty=0`, and
`release_positive_hits=0`.

The Phase 4OW implementation adds normal/full ContextPacket Resource-backed
Verification Requirement recovery hints for direct structural references from
a focused Plan or Goal to a Plan target's one-level child Plans' directly
contained Tasks. The summary includes the referenced Plan, nested Plan, direct
child Task, referrer id, referrer kind, relation id, both containment relation
ids, direct-reference marker, Resource basis, baseline observation, and
basis-aware cache-refresh hint. Brief context, direct Task
structural-reference context, direct Plan target context, same-Plan peer
context, same-Goal peer context, and blocked-dependency context behavior remain
covered by targeted tests.

The post-change public CLI probe at
`/tmp/workvcs-4ow-structural-reference-nested-plan-target-context-resource-probe-20260907T053903Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_NESTED_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_plan_target=true`,
`plan_context_has_nested_plan=true`,
`plan_context_has_target_task=true`, `plan_context_has_vr=true`,
`plan_context_has_resource_basis=true`,
`plan_context_has_refresh_hint=true`,
`goal_context_has_plan_target=true`,
`goal_context_has_nested_plan=true`,
`goal_context_has_target_task=true`, `goal_context_has_vr=true`,
`goal_context_has_resource_basis=true`,
`goal_context_has_refresh_hint=true`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, and `release_positive_hits=0`.

Targeted core tests cover the new one-level nested referenced Plan recovery
path and regressions for direct Plan target, direct Task structural references,
same-Plan peer, same-Goal peer, and blocked-dependency Resource-backed
recovery hints.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OW closes only direct Plan/Goal structural references to Plan targets
whose one-level child Plans directly contain Resource-backed Tasks. Deeper
nested Plan traversal, multi-hop structural references, broad Resource
resolver maturity, full relation-subject traversal, multi-hop/full evolution
traversal, broader causal traversal, and candidate release validation remain
open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OW records that the same basis-aware recovery command is visible in
normal/full context for direct Plan/Goal structural references to Plan targets
whose one-level child Plans directly contain Resource-backed Tasks.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OV Update

Phase 4OV refreshes this matrix with a bounded direct structural-reference Plan
target Context/Resource recovery implementation and validation. It does not
change Store schema, packet schema, public CLI flags, `why`, runnable,
`claim next`, structural reference semantics, ADR authority, or release state.

The pre-change public CLI probe at
`/tmp/workvcs-4ov-structural-reference-plan-target-context-resource-probe-20260907T052344Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_UNSUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_plan_target=false`,
`plan_context_has_target_task=false`, `plan_context_has_vr=false`,
`plan_context_has_resource_basis=false`,
`plan_context_has_refresh_hint=false`,
`goal_context_has_plan_target=false`,
`goal_context_has_target_task=false`, `goal_context_has_vr=false`,
`goal_context_has_resource_basis=false`,
`goal_context_has_refresh_hint=false`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, `repo_dirty=0`, and
`release_positive_hits=0`.

The Phase 4OV implementation adds normal/full ContextPacket Resource-backed
Verification Requirement recovery hints for direct structural references from
a focused Plan or Goal to a Plan target's directly contained Tasks. The summary
includes the referenced Plan, direct child Task, referrer id, referrer kind,
relation id, containment relation id, direct-reference marker, Resource basis,
baseline observation, and basis-aware cache-refresh hint. Brief context,
direct Task structural-reference context, same-Plan peer context, same-Goal
peer context, and blocked-dependency context behavior remain covered by
targeted tests.

The post-change public CLI probe at
`/tmp/workvcs-4ov-structural-reference-plan-target-context-resource-probe-20260907T052625Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_PLAN_TARGET_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_plan_target=true`,
`plan_context_has_target_task=true`, `plan_context_has_vr=true`,
`plan_context_has_resource_basis=true`,
`plan_context_has_refresh_hint=true`,
`goal_context_has_plan_target=true`,
`goal_context_has_target_task=true`, `goal_context_has_vr=true`,
`goal_context_has_resource_basis=true`,
`goal_context_has_refresh_hint=true`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, and `release_positive_hits=0`.

Targeted core tests cover the new direct structural-reference Plan target
recovery path and regressions for direct Task structural references, same-Plan
peer, same-Goal peer, and blocked-dependency Resource-backed recovery hints.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OV closes only direct Plan/Goal structural references to Plan targets
with direct child Tasks. Nested referenced Plan traversal, multi-hop
structural references, broad Resource resolver maturity, full
relation-subject traversal, multi-hop/full evolution traversal, broader causal
traversal, and candidate release validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OV records that the same basis-aware recovery command is visible in
normal/full context for direct Plan/Goal structural references to Plan targets
with direct child Tasks.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OU Update

Phase 4OU refreshes this matrix with a bounded direct structural-reference
Context/Resource recovery implementation and validation. It does not change
Store schema, packet schema, public CLI flags, `why`, runnable, `claim next`,
structural reference semantics, ADR authority, or release state.

The pre-change public CLI probe at
`/tmp/workvcs-4ou-structural-reference-context-resource-probe-20260907T050012Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_CONTEXT_RESOURCE_RECOVERY_UNSUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_target=false`, `plan_context_has_vr=false`,
`plan_context_has_resource_basis=false`,
`plan_context_has_refresh_hint=false`, `goal_context_has_target=false`,
`goal_context_has_vr=false`, `goal_context_has_resource_basis=false`,
`goal_context_has_refresh_hint=false`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, `repo_dirty=0`, and
`release_positive_hits=0`.

The Phase 4OU implementation adds normal/full ContextPacket Resource-backed
Verification Requirement recovery hints for direct structural references from
a focused Plan or Goal to a Task. The summary includes the referenced Task,
referrer id, referrer kind, relation id, direct-reference marker, Resource
basis, baseline observation, and basis-aware cache-refresh hint. Brief context,
same-Plan peer context, same-Goal peer context, and blocked-dependency context
behavior remain covered by existing tests.

The post-change public CLI probe at
`/tmp/workvcs-4ou-structural-reference-context-resource-probe-20260907T050627Z`
proves `probe_execution_status=PASS`,
`probe_result=STRUCTURAL_REFERENCE_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`commands_exit_failures=0`, `why_reference_visible=true`,
`plan_context_has_target=true`, `plan_context_has_vr=true`,
`plan_context_has_resource_basis=true`,
`plan_context_has_refresh_hint=true`, `goal_context_has_target=true`,
`goal_context_has_vr=true`, `goal_context_has_resource_basis=true`,
`goal_context_has_refresh_hint=true`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`, and `release_positive_hits=0`. The probe
run log records `status=0` for `doctor --require-valid`.

Targeted core tests cover the new direct structural-reference recovery path
and regressions for same-Plan peer, same-Goal peer, and blocked-dependency
Resource-backed recovery hints.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OU closes only direct Plan/Goal structural references to Tasks.
Structural-reference Plan targets, multi-hop structural references, broad
Resource resolver maturity, full relation-subject traversal, multi-hop/full
evolution traversal, broader causal traversal, and candidate release
validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OU records that the same basis-aware recovery command is visible in
normal/full context for direct Plan/Goal structural references to Tasks.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OT Update

Phase 4OT refreshes this matrix with read-only current-main cross-Goal
Task-dependency Context/Resource recovery evidence. It does not change WorkVCS
behavior, schema, CLI flags, tests, ADR authority, or release state.

The effective public CLI probe run at
`/tmp/workvcs-4ot-cross-goal-dependency-context-resource-probe-20260907T044427Z`
proves `probe_execution_status=PASS`,
`probe_result=CROSS_GOAL_DEPENDENCY_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`gap_kind=none`, `commands_exit_failures=0`,
`focus_dependent_task_supported=true`,
`focused_runnable_candidates_match_expected=true`,
`focused_candidate_dependent=true`, `focused_candidate_blocked=true`,
`focused_brief_blocked_dependency_items=1`,
`focused_full_blocked_dependency_items=1`,
`focused_brief_has_prereq_task=true`,
`focused_full_has_prereq_task=true`,
`focused_brief_has_dependency_vr_key=true`,
`focused_full_has_dependency_vr_key=true`,
`focused_brief_has_resource_basis=true`,
`focused_brief_has_refresh_hint=true`,
`focused_brief_has_baseline=true`,
`claim_runnable_candidates_match_expected=true`,
`claim_candidate_0_prereq=true`, `claim_candidate_0_runnable=true`,
`claim_candidate_1_dependent=true`, `claim_candidate_1_blocked=true`,
`claim_next_selected=true`, `claim_next_task_matches=true`,
`claim_next_has_vr_key=true`, `claim_next_has_resource_basis=true`,
`claim_next_has_refresh_hint=true`, `claim_next_has_baseline=true`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`,
`doctor_required_valid_exit=0`, `repo_git_dirty_lines_after=0`, and
`release_flag_positive_hits=0`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OT adds current-main evidence that a focused blocked Task can see a
different-Goal prerequisite Task's Resource-backed Verification Requirement,
Resource basis, baseline observation, and basis-aware refresh hint in the
existing `blocked_dependency` item, and that an unfocused
`claim next --context-profile full` selects the runnable prerequisite with the
same recovery path. Cross-Goal traversal outside explicit Task dependencies,
non-Task dependency Resource traversal, broad Resource resolver maturity, full
relation-subject traversal, multi-hop/full evolution traversal, broader causal
traversal, and release-candidate validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OT records that the cross-Goal prerequisite recovery command is visible
without a separate prerequisite verification lookup.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OS Update

Phase 4OS refreshes this matrix with read-only current-main one-level nested
Plan Context/Resource recovery evidence. It does not change WorkVCS behavior,
schema, CLI flags, tests, ADR authority, or release state.

The effective public CLI probe run at
`/tmp/workvcs-4os-nested-plan-context-resource-probe-20260907T043535Z`
proves `probe_execution_status=PASS`,
`probe_result=NESTED_PLAN_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`gap_kind=none`, `commands_exit_failures=0`,
`runnable_candidates_match_expected=true`, `runnable_candidate_0_task=true`,
`runnable_candidate_0_runnable=true`,
`focus_parent_plan_supported=true`,
`parent_plan_focus_normal_has_vr_key=true`,
`parent_plan_focus_full_has_vr_key=true`,
`parent_plan_focus_full_has_resource_basis=true`,
`parent_plan_focus_full_has_refresh_hint=true`,
`parent_plan_focus_full_has_baseline=true`,
`parent_plan_focus_full_has_child_plan_path=true`,
`focus_goal_supported=true`, `goal_focus_normal_has_vr_key=true`,
`goal_focus_full_has_vr_key=true`,
`goal_focus_full_has_resource_basis=true`,
`goal_focus_full_has_refresh_hint=true`,
`goal_focus_full_has_baseline=true`,
`goal_focus_full_has_child_plan_path=true`,
`goal_focus_claim_next_selected=true`,
`goal_focus_claim_next_task_matches=true`,
`goal_focus_claim_next_has_vr_key=true`,
`goal_focus_claim_next_has_resource_basis=true`,
`goal_focus_claim_next_has_refresh_hint=true`,
`goal_focus_claim_next_has_baseline=true`,
`cache_refresh_applicability=applicable`,
`cache_refresh_resource_stamps=1`,
`doctor_required_valid_exit=0`, `repo_git_dirty_lines_after=0`, and
`release_flag_positive_hits=0`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OS adds current-main evidence that a parent Plan-focused operator and
a Goal-focused operator can see a one-level nested SubPlan Task's
Resource-backed Verification Requirement, Resource basis fields, baseline
observation, and basis-aware refresh hint in normal and full context, and that
focused `claim next --context-profile full` preserves the selected nested Task
recovery path. Cross-Goal traversal, non-Task dependency Resource traversal,
broad Resource resolver maturity, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, and
release-candidate validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OS records that Plan-focused and Goal-focused context continue to expose
the basis-aware recovery command across one nested SubPlan boundary.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OR Update

Phase 4OR refreshes this matrix with read-only current-main Goal-start
multi-Plan Context/Resource recovery evidence. It does not change WorkVCS
behavior, schema, CLI flags, tests, ADR authority, or release state.

The effective public CLI probe run at
`/tmp/workvcs-4or-goal-start-multiplan-context-resource-probe-20260907T042246Z`
proves `probe_execution_status=PASS`,
`probe_result=GOAL_START_MULTIPLAN_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`gap_kind=none`, `commands_exit_failures=0`,
`runnable_candidates_match_expected=true`,
`runnable_candidate_0_target=true`, `runnable_candidate_1_peer=true`,
`runnable_candidate_0_runnable=true`, `runnable_candidate_1_runnable=true`,
`focus_goal_supported=true`,
`goal_focus_normal_has_target_vr_key=true`,
`goal_focus_normal_has_peer_vr_key=true`,
`goal_focus_full_has_target_vr_key=true`,
`goal_focus_full_has_peer_vr_key=true`,
`goal_focus_full_target_has_resource_basis=true`,
`goal_focus_full_peer_has_resource_basis=true`,
`goal_focus_full_target_has_refresh_hint=true`,
`goal_focus_full_peer_has_refresh_hint=true`,
`goal_focus_full_target_has_baseline=true`,
`goal_focus_full_peer_has_baseline=true`,
`goal_focus_claim_next_selected=true`,
`goal_focus_claim_next_task_matches=true`,
`goal_focus_claim_next_has_target_vr_key=true`,
`goal_focus_claim_next_has_resource_basis=true`,
`goal_focus_claim_next_has_refresh_hint=true`,
`goal_focus_claim_next_has_baseline=true`,
`cache_refresh_target_applicability=applicable`,
`cache_refresh_target_resource_stamps=1`,
`cache_refresh_peer_applicability=applicable`,
`cache_refresh_peer_resource_stamps=1`,
`doctor_required_valid_exit=0`, `repo_git_dirty_lines_after=0`, and
`release_flag_positive_hits=0`.

An earlier attempt at
`/tmp/workvcs-4or-goal-start-multiplan-context-resource-probe-20260907T042033Z`
is retained only as a shell harness failure from reading `commit_id` instead
of `genesis_commit_id` in `workspace create` output and is not used as
behavioral evidence.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OR adds current-main evidence that a Goal-focused operator can see two
direct child Plans' runnable Task Resource-backed Verification Requirements,
Resource basis fields, baseline observations, and basis-aware refresh hints in
normal and full context, and that focused `claim next --context-profile full`
preserves the selected target recovery path. Nested Plan traversal, cross-Goal
traversal, non-Task dependency Resource traversal, broad Resource resolver
maturity, full relation-subject traversal, multi-hop/full evolution traversal,
broader causal traversal, and release-candidate validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OR records that Goal-focused context continues to expose the
basis-aware recovery command when more than one direct child Plan has a
runnable Resource-backed Task.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OQ Update

Phase 4OQ refreshes this matrix with read-only current-main Goal-start
Context/Resource recovery evidence. It does not change WorkVCS behavior,
schema, CLI flags, tests, ADR authority, or release state.

The effective public CLI probe run at
`/tmp/workvcs-4oq-goal-start-context-resource-probe-20260907T040620Z` proves
`probe_execution_status=PASS`,
`probe_result=GOAL_START_DIRECT_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`gap_kind=none`, `commands_exit_failures=0`,
`focus_goal_supported=true`, `task_focus_brief_has_resource_basis=true`,
`task_focus_brief_has_refresh_hint=true`,
`plan_focus_full_has_resource_basis=true`,
`plan_focus_full_has_refresh_hint=true`,
`goal_focus_normal_has_resource_basis=true`,
`goal_focus_full_has_resource_basis=true`,
`goal_focus_normal_has_refresh_hint=true`,
`goal_focus_full_has_refresh_hint=true`,
`goal_start_claim_next_recovery_supported=true`,
`cache_refresh_applicability=applicable`,
`doctor_required_valid_exit=0`, `repo_git_dirty_lines_after=0`, and
`release_flag_positive_hits=0`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OQ adds current-main evidence that a Goal-focused operator can see a
descendant runnable Task's Resource-backed Verification Requirement, Resource
basis, baseline observation, and basis-aware refresh hint in normal and full
context for a direct Goal -> Plan -> Task shape, and that focused
`claim next --context-profile full` preserves the same recovery path. Nested
Plan traversal, cross-Goal traversal, multiple-Plan ranking, non-Task
dependency Resource traversal, broad Resource resolver maturity, full
relation-subject traversal, multi-hop/full evolution traversal, broader
causal traversal, and release-candidate validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OQ records that Goal-focused context already removes the direct Plan or
Task lookup hop for this Resource-backed VR recovery path.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OP Update

Phase 4OP refreshes this matrix with read-only current-main Plan-start
Context/Resource recovery evidence. It does not change WorkVCS behavior,
schema, CLI flags, tests, ADR authority, or release state.

The effective public CLI probe run at
`/tmp/workvcs-4op-plan-start-context-resource-probe-20260907T035327Z` proves
`probe_execution_status=PASS`,
`probe_result=PLAN_START_DIRECT_CONTEXT_RESOURCE_RECOVERY_SUPPORTED`,
`gap_kind=none`, `commands_exit_failures=0`,
`focus_plan_supported=true`, `task_focus_brief_has_resource_basis=true`,
`task_focus_brief_has_refresh_hint=true`,
`plan_focus_normal_has_resource_basis=true`,
`plan_focus_full_has_resource_basis=true`,
`plan_focus_normal_has_refresh_hint=true`,
`plan_focus_full_has_refresh_hint=true`,
`plan_start_claim_next_recovery_supported=true`,
`cache_refresh_applicability=applicable`,
`doctor_required_valid_exit=0`, `repo_git_dirty_lines_after=0`, and
`release_flag_positive_hits=0`.

An earlier attempt at
`/tmp/workvcs-4op-plan-start-context-resource-probe-20260907T035159Z` is
retained only as a shell harness failure and is not used as behavioral
evidence.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OP adds current-main evidence that a Plan-focused operator can see a
directly contained runnable Task's Resource-backed Verification Requirement,
Resource basis, baseline observation, and basis-aware refresh hint in normal
and full context, and that focused `claim next --context-profile full`
preserves the same recovery path. Goal-start Context/Resource recovery,
nested Plan traversal, cross-Goal traversal, non-Task dependency Resource
traversal, broad Resource resolver maturity, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, and
release-candidate validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OP records that Plan-focused context already removes the direct Task
lookup hop for this Resource-backed VR recovery path.

The overall release decision remains false because the context/why gate is
still `Partial` and the candidate release operation gate is still `Blocked`.

## Phase 4OK Update

Phase 4OK refreshes this matrix after making current VR-backed Verification
closure chains visible from Plan `why` for directly contained Tasks. The
change was driven by the Phase 4OJ read-only probe where a temporary Store had
one Goal, one Plan, one directly contained Task, one Acceptance Criterion, one
Verification Requirement, one Resource, and one Resource-backed Verification.
After Task, Plan, and Goal closeout, Task and Acceptance Criterion `why`
exposed the Resource-backed closure chain, while Plan `why` did not, even
though Plan `why` already exposed direct primary containment relation edges to
the Task.

The Phase 4OK binary keeps Store schema, CLI flags, ContextPacket behavior,
Verification semantics, Evidence semantics, Resource observation,
applicability, cache-refresh, direct relation endpoint evolution behavior, and
Task/Plan/Goal closeout semantics unchanged. It adds only read-only Plan
projection of existing `verification_closure_chains` for directly contained
Tasks.

The public CLI dogfood run at
`/tmp/workvcs-4ok-plan-direct-task-closure-20260904T030424Z` proves
`phase4ok_dogfood=PASS`, `commands_exit_failures=0`,
`failed_assertions=0`, `critical_failure=none`, `task_show_done=true`,
`plan_show_completed=true`, `goal_show_achieved=true`,
`ac_status_before_closeout_verified=true`,
`ac_status_after_closeout_stale=true`, `task_why_closure_chains=true`,
`ac_why_closure_chains=true`, `plan_why_closure_chains=true`,
`goal_why_closure_chains=true`, `plan_why_verification_id_present=true`,
`plan_why_verification_id_matches=true`, `plan_why_evidence_id_matches=true`,
`plan_why_resource_basis=true`, `plan_why_resource_id_matches=true`,
`plan_why_observation_id_matches=true`, `plan_why_fingerprint_matches=true`,
`cache_refresh_from_plan_why_id_applicable=true`,
`post_refresh_ac_status_verified=true`, `doctor_required_valid=true`, and
`release_flag_positive_hits=0`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OK closes this concrete Plan-to-direct-Task closeout explanation gap,
but Goal ancestor rollup, broader context/Resource resolver maturity, full
relation-subject traversal beyond direct endpoint slices and the current
Task/Acceptance Criterion/direct-Plan closure projection, multi-hop/full
evolution traversal, broader causal traversal, and release-candidate
validation remain open.

The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OK removes one Plan-level recovery lookup hop by proving the operator
can use the Verification id surfaced from Plan `why` with the existing
`verification cache-refresh --resource-content-from-basis` path.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4OM Update

Phase 4OM refreshes this matrix with read-only current-main Goal-start
recovery evidence. It does not change WorkVCS behavior, schema, CLI flags,
tests, ADR authority, or release state.

The effective public CLI probe run at
`/tmp/workvcs-4om-goal-plan-recovery-probe-20260904T095537Z` proves
`phase4om_probe=PASS`, `commands_exit_failures=0`, `failed_assertions=0`,
`probe_result=RECOVERY_SUPPORTED_WITH_PLAN_HOP`,
`concrete_implementation_gap_found=false`, `task_why_closure_chains=1`,
`ac_why_closure_chains=1`, `plan_why_closure_chains=1`,
`goal_why_closure_chains=0`, `goal_why_contains_plan_id=true`,
`goal_why_contains_verification_id=false`,
`plan_why_verification_id_matches=true`, `plan_why_resource_id_matches=true`,
`cache_refresh_from_plan_why_id_applicability=applicable`,
`ac_status_after_plan_refresh=verified`, `doctor_required_valid_exit=0`,
`release_flag_positive_hits=0`, and `repo_git_dirty_lines_after=0`.

An earlier attempt at
`/tmp/workvcs-4om-goal-plan-recovery-probe-20260904T095105Z` is retained only
as a stale-binary failed attempt and is not used as behavioral evidence.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OM proves that Goal ancestor rollup is not the next necessary minimal
implementation slice for the tested closeout recovery route: a continuation
operator can start from Goal `why`, see the direct Plan, query Plan `why`, and
recover through the existing basis-aware cache refresh path. Broader
context/Resource resolver maturity, full relation-subject traversal,
multi-hop/full evolution traversal, broader causal traversal, and
release-candidate validation remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4OI / Phase 4OH Evidence Refresh

Phase 4OI refreshes this matrix after the read-only Phase 4OH public CLI probe.
The probe tested the existing Phase 4OG Resource-backed Task and Acceptance
Criterion `why` closure path without changing code, schema, CLI flags,
ContextPacket behavior, Verification semantics, Resource observation,
applicability, cache-refresh behavior, Task closeout semantics, tests, or ADR
authority.

The probe at
`/tmp/workvcs-4oh-recovery-from-why-probe-20260904T011704Z` proves
`probe_execution_status=PASS`, `probe_result=RECOVERY_SUPPORTED`,
`commands_exit_failures=0`, `failed_assertions=0`,
`critical_failure=none`, `task_show_done=true`,
`ac_status_before_closeout=verified`, `ac_status_after_closeout=stale`,
`task_why_verification_id_present=true`,
`ac_why_verification_id_present=true`,
`task_ac_why_same_verification_id=true`, `task_why_resource_basis=true`,
`ac_why_resource_basis=true`, `task_why_basis_matches_verification=true`,
`ac_why_basis_matches_verification=true`,
`cache_refresh_from_why_id=true`, `post_refresh_ac_status=verified`, and
`post_refresh_ac_status_verified=true`.

This evidence strengthens the operator-recovery reading of Phase 4OG: after
Resource-backed Task closeout makes the Acceptance Criterion stale, the
operator can recover using the Verification id and Resource basis surfaced by
Task/Acceptance Criterion `why`, without a separate `verification show` lookup.
No implementation gap was found for this narrow path.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
The Operator discoverability and actionable recovery gate remains `Pass`.
Phase 4OH does not close broader context/Resource resolver maturity, full
relation-subject traversal beyond direct endpoint slices and the current
Task/Acceptance Criterion closure projection, multi-hop/full evolution
traversal, broader causal traversal, or release-candidate validation. The
overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4OG Update

Phase 4OG refreshes this matrix after making Resource basis fields visible
inside current VR-backed Task and Acceptance Criterion `why` closure chains.
The change was driven by the Phase 4OF public CLI probe where a temporary Store
had one closed Task, one Acceptance Criterion, one Verification Requirement,
one Evidence item, one local-file Resource, one Resource observation, and one
passed Resource-backed Verification. `verification show` exposed the Resource
basis, but Task and Acceptance Criterion `why` omitted it.

The Phase 4OG binary keeps Store schema, CLI flags, ContextPacket behavior,
Verification semantics, Evidence semantics, Resource observation,
applicability, cache-refresh, direct relation endpoint evolution behavior, and
Task closeout semantics unchanged. It adds only read-only Resource basis fields
inside existing `verification_closure_chains` key-value output for current Task
and current Acceptance Criterion subjects.

The public CLI dogfood run at
`/tmp/workvcs-4og-resource-basis-why-closure-20260903T113715Z` proves
`phase4og_dogfood=PASS`, `commands_exit_failures=0`,
`failed_assertions=0`, `task_show_done=true`,
`ac_status_verified=true`, `ac_status_after_closeout=stale`,
`verify_resource_basis=true`, `verify_resource_observation=true`,
`verification_show_resource_basis=true`, `task_why_resource_basis=true`,
`ac_why_resource_basis=true`, `task_why_scope_payload=true`,
`ac_why_scope_payload=true`, `task_why_baseline_observation=true`,
`ac_why_baseline_observation=true`, `task_why_baseline_fingerprint=true`, and
`ac_why_baseline_fingerprint=true`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OG closes this concrete Resource-backed Task/Acceptance Criterion
closeout explanation gap, but broader context/Resource resolver maturity, full
relation-subject traversal beyond direct endpoint slices, multi-hop/full
evolution traversal, broader causal traversal, and release-candidate
validation remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4OE Update

Phase 4OE refreshes this matrix after making current VR-backed Verification
closure chains visible from Task and Acceptance Criterion `why`. The change was
driven by a corrected public CLI probe where a temporary Store had one closed
Task, one verified Acceptance Criterion, one Verification Requirement, one
passed Verification, and one Evidence item. Direct VR, Verification, and
Evidence endpoint `why` controls passed, but the Task and Acceptance Criterion
queries could not identify the AC, VR, Verification, or Evidence ids for the
closeout chain.

The Phase 4OE binary keeps Store schema, CLI flags, ContextPacket behavior,
Verification semantics, Evidence semantics, Resource observation,
cache-refresh, direct relation endpoint evolution behavior, and Task closeout
semantics unchanged. It adds only read-only
`verification_closure_chains` key-value output for current Task and current
Acceptance Criterion subjects.

The public CLI dogfood run at
`/tmp/workvcs-4oe-task-closeout-why-closure-chain-20260903T105642Z` proves
`phase4oe_dogfood=PASS`, `commands_exit_failures=0`,
`failed_assertions=0`, `task_show_done=true`,
`ac_status_verified=true`, `task_why_verification_closure_chains=true`,
`ac_why_verification_closure_chains=true`, `task_why_chain_has_ac=true`,
`task_why_chain_has_vr=true`, `task_why_chain_has_verification=true`,
`task_why_chain_has_evidence=true`, `ac_why_chain_has_vr=true`,
`ac_why_chain_has_verification=true`, and
`ac_why_chain_has_evidence=true`. The same run keeps the direct
VR/Verification/Evidence `verifies` and `evidenced_by` endpoint controls
passing.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4OE closes this concrete Task/Acceptance Criterion closeout explanation
gap, but broader context/Resource resolver maturity, full relation-subject
traversal beyond direct endpoint slices, multi-hop/full evolution traversal,
broader causal traversal, and release-candidate validation remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4OC Update

Phase 4OC refreshes this matrix after closing a Session focus contract
mismatch. The change was driven by a current public CLI probe where a temporary
Store had one Resource-backed Verification Requirement. Unfocused full context
and valid Task-focused brief context both surfaced the VR Resource basis and
basis-aware recovery hint, but `session focus-set --focus
<verification_requirement>` accepted that unsupported focus. Later `context`
and `runnable tasks` failed with `session_invalid` because focused runtime
resolution only supports current Goal, Plan, and Task focus entities.

The Phase 4OC binary keeps Store schema, CLI flags, ContextPacket JSON fields,
snapshot schema, Resource observation/cache-refresh semantics, runnable
selection, Claim behavior, `claim next`, `next`, and `why` behavior unchanged.
It adds only a write-boundary `session focus-set` validation that rejects a
present focus entity unless it resolves at the active Branch head as a Goal,
Plan, or Task.

The public CLI dogfood run at
`/tmp/workvcs-4oc-focus-set-unsupported-kind-20260903T082308Z/dogfood`
proves `phase4oc_dogfood=PASS`, `commands_exit_failures=0`,
`failed_assertions=0`, `focus_set_vr_exit=expected_failure`,
`focus_set_vr_error_code=session_invalid`,
`focus_set_vr_error_category=runtime`,
`focus_set_vr_message_contains_supported_kind=true`,
`session_after_reject_focus=none`, `context_unfocused_succeeded=true`,
`task_focus_succeeded=true`, `context_task_focus_has_requirement=true`,
`context_task_focus_has_resource_basis=true`, and
`context_task_focus_has_refresh_hint=true`.

The Session, Claim, Runnable, `claim next`, and `next` gate remains `Pass`.
Phase 4OC closes this concrete focus contract mismatch, but it does not add
new supported focus entity kinds or broaden context/Resource resolver behavior.
The Context resolver, packets, and `why` explanations gate remains `Partial`,
and the Candidate release operation gate remains `Blocked`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NZ Update

Phase 4NZ refreshes this matrix after making Resource-backed Verification
Requirement recovery visible in normal/full focused context for runnable peer
Tasks whose direct parent Plans share the same direct parent Goal as the
focused Task's direct parent Plan. The change was driven by a current public
CLI probe where unfocused full context and peer-focused brief context both
reported the cross-Plan peer VR with `resource_basis=1` and a basis-aware
refresh hint, but current-focused normal/full context and focused
`claim next --context-profile full` omitted that peer VR while full context
reported `context_omitted_items=0`.

The Phase 4NZ binary keeps Store schema, CLI flags, ContextPacket JSON fields,
snapshot schema, Resource observation/cache-refresh semantics, runnable
selection, Claim behavior, `claim next`, `next`, and `why` behavior unchanged.
It reuses the existing read-only workspace-wide runnable projection for
normal/full focused Task context and appends only existing
`verification_requirement` items with summary text beginning
`same_goal_peer_task=<peer> parent_goal=<goal> focused_plan=<plan> peer_plan=<plan>`.

The public CLI dogfood run at
`/tmp/workvcs-4nz-focused-same-goal-cross-plan-resource-context-20260903T071452Z/dogfood`
proves `phase4nz_dogfood=PASS`, `commands_exit_failures=0`,
`unfocused_runnable_candidates=2`,
`context_current_focus_brief_has_cross_plan_vr_key=false`,
`context_current_focus_normal_has_cross_plan_vr_key=true`,
`context_current_focus_normal_has_cross_plan_resource_basis=true`,
`context_current_focus_normal_has_cross_plan_refresh_hint=true`,
`context_current_focus_full_has_cross_plan_vr_key=true`,
`context_current_focus_full_has_cross_plan_resource_basis=true`,
`context_current_focus_full_has_cross_plan_refresh_hint=true`,
`context_current_focus_full_omitted_items=0`,
`claim_next_packet_has_cross_plan_vr_key=true`,
`claim_next_packet_has_cross_plan_resource_basis=true`,
`claim_next_packet_has_cross_plan_refresh_hint=true`, and
`claim_next_same_goal_peer_hint_count=1`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NZ closes this concrete focused same-Goal cross-Plan peer Resource
context gap, but broader context/Resource resolver maturity outside direct peer
slices, full relation-subject traversal beyond direct endpoint slices,
multi-hop/full evolution traversal, broader causal traversal, and
release-candidate validation remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NY Update

Phase 4NY refreshes this matrix after making direct
`knowledge_exposure_derived_from` Relation creation visible as endpoint
evolution in `why` for queried source Knowledge and target KnowledgeExposure
endpoints. The change was driven by a current public CLI probe where both
endpoint queries reported one current `knowledge_exposure_derived_from` edge,
with outgoing direction from the Knowledge endpoint and incoming direction from
the KnowledgeExposure endpoint, but both reported
`evolution_change_operations=0`; `--expected-evolution-change-operations 1`
failed with `error_code=query_invalid`.

The Phase 4NY binary keeps Store schema, CLI flags, ContextPacket behavior,
Knowledge semantics, KnowledgeExposure semantics, and adoption semantics
unchanged. It lets KnowledgeExposure subjects enter the existing direct
relation evolution pass while keeping Entity-specific direct Entity evolution
unchanged, and resolves operation subject detail only for current
`knowledge_exposure_derived_from` Relations whose source or target endpoint
matches the query subject.

The public CLI dogfood run at
`/tmp/workvcs-4ny-why-knowledge-exposure-derived-from-evolution-20260903T063526Z/dogfood`
proves `phase4ny_dogfood=PASS`, `source_relation_edges=1`,
`source_relation_kind=knowledge_exposure_derived_from`,
`source_relation_direction=outgoing`,
`source_evolution_change_operations=1`,
`source_evolution_match_expected=true`,
`source_evolution_operation_type=knowledge.relation.create`,
`source_evolution_subject_relation_kind=knowledge_exposure_derived_from`,
`source_evolution_source_matches_knowledge=true`,
`source_evolution_target_matches_exposure=true`,
`exposure_relation_edges=1`,
`exposure_relation_kind=knowledge_exposure_derived_from`,
`exposure_relation_direction=incoming`,
`exposure_evolution_change_operations=1`,
`exposure_evolution_match_expected=true`,
`exposure_evolution_operation_type=knowledge.relation.create`,
`exposure_evolution_subject_relation_kind=knowledge_exposure_derived_from`,
`exposure_evolution_source_matches_knowledge=true`, and
`exposure_evolution_target_matches_exposure=true`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NY closes this concrete KnowledgeExposure derived-from direct endpoint
evolution gap for queried Knowledge and KnowledgeExposure endpoints, but full
relation-subject traversal beyond direct endpoint slices, multi-hop/full
evolution traversal, broader causal traversal, broader context/Resource
resolver maturity, and release-candidate validation remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NX Update

Phase 4NX refreshes this matrix after making Evidence endpoint incoming
`evidenced_by` Relation creation visible as direct endpoint evolution in `why`.
The change was driven by a current public CLI probe where one Evidence reused
by two Verifications reported `evidence_relation_edges=2`, both
`evidenced_by` and `incoming`, but `evidence_evolution_change_operations=0`;
`--expected-evolution-change-operations 2` failed with
`error_code=query_invalid`.

The Phase 4NX binary keeps Store schema, CLI flags, ContextPacket behavior,
Verification semantics, and Evidence semantics unchanged. It lets Evidence
subjects enter the existing direct relation evolution pass while keeping
Entity-specific direct Entity evolution unchanged and keeping Knowledge
Exposure subjects outside this direct relation pass.

The public CLI dogfood run at
`/tmp/workvcs-4nx-why-evidence-subject-evidenced-by-evolution-20260903T052944Z/dogfood`
proves `phase4nx_dogfood=PASS`, `evidence_relation_edges=2`,
`evidence_relation_0_kind=evidenced_by`,
`evidence_relation_0_direction=incoming`,
`evidence_relation_1_kind=evidenced_by`,
`evidence_relation_1_direction=incoming`,
`evidence_evolution_change_operations=2`,
`evidence_evolution_match_expected=true`,
`evidence_evolution_subject_relation_kind_count=2`,
`evidence_evolution_target_evidence_id_count=2`, and
`evidence_evolution_sources_match=true`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NX closes one concrete `evidenced_by` endpoint evolution gap for queried
Evidence endpoints, but full relation-subject traversal beyond direct endpoint
slices, multi-hop/full evolution traversal, broader causal traversal, broader
context/Resource resolver maturity, and release-candidate validation remain
open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NW Update

Phase 4NW refreshes this matrix after making Evidence-backed `evidenced_by`
Relation creation visible as direct endpoint evolution in `why` for queried
Verification endpoints. The change was driven by a current public CLI probe
where the queried Verification endpoint reported `verification_relation_edges=1`
and `verification_has_evidenced_by_edge=true`, but only
`verification_evolution_change_operations=1`; the visible operation was the
same `verification.record` `verifies` operation, and
`verification_has_evolution_subject_relation_kind_evidenced_by=false`.
`--expected-evolution-change-operations 2` failed with
`error_code=query_invalid`.

The Phase 4NW binary keeps Store schema, CLI flags, ContextPacket behavior,
Verification semantics, and Evidence semantics unchanged. It resolves direct
operation subject detail only for current `evidenced_by` Relations when the
queried Entity is the source Verification endpoint.

The public CLI dogfood run at
`/tmp/workvcs-4nw-why-evidenced-by-relation-evolution-20260903T024218Z/dogfood`
proves `phase4nw_dogfood=PASS`, `verification_relation_edges=1`,
`verification_relation_kind=evidenced_by`,
`verification_evolution_change_operations=2`,
`verification_evolution_match_expected=true`,
`verification_evolution_subject_relation_kind_0=verifies`,
`verification_evolution_subject_relation_kind_1=evidenced_by`,
`verification_evidenced_by_source_matches_verification=true`, and
`verification_evidenced_by_target_matches_evidence=true`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NW closes one concrete `evidenced_by` endpoint evolution gap for queried
Verification endpoints, but full relation-subject traversal beyond direct
endpoint slices, multi-hop/full evolution traversal, broader causal traversal,
broader context/Resource resolver maturity, and release-candidate validation
remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NV Update

Phase 4NV refreshes this matrix after making the defining Verification
`verifies` Relation creation visible as direct endpoint evolution in `why`. The
change was driven by a current public CLI probe where the queried Verification
Requirement endpoint reported `vr_relation_edges=1`,
`vr_has_verifies_edge=true`, and `vr_has_verifies_relation_id=true`, but
`vr_evolution_change_operations=0`; `--expected-evolution-change-operations 1`
failed with `error_code=query_invalid`.

The Phase 4NV binary keeps Store schema, CLI flags, ContextPacket behavior,
Verification semantics, and Evidence semantics unchanged. It includes
`verification.record` in the existing direct relation evolution pass and resolves
operation subject detail only for the defining `verifies` Relation when the
queried Entity is the source Verification or target Acceptance Criterion or
Verification Requirement endpoint.

The public CLI dogfood run at
`/tmp/workvcs-4nv-why-verifies-relation-evolution-20260903T012528Z/dogfood-v3`
proved `phase4nv_dogfood=PASS`, `vr_relation_edges=1`,
`vr_relation_kind=verifies`, `vr_evolution_change_operations=1`,
`vr_evolution_match_expected=true`,
`vr_evolution_operation_type=verification.record`,
`vr_evolution_subject_relation_kind=verifies`,
`vr_evolution_source_matches_verification=true`,
`vr_evolution_target_matches_requirement=true`,
`vr_deferred_relation_families=1`, and
`vr_deferred_relation_family_0=evolution`.

The Context resolver, packets, and `why` explanations gate remains `Partial`.
Phase 4NV closes one concrete `verifies` endpoint evolution gap, but
`evidenced_by` endpoint evolution, full relation-subject traversal beyond direct
endpoint slices, multi-hop/full evolution traversal, broader causal traversal,
broader context/Resource resolver maturity, and release-candidate validation
remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NU Update

Phase 4NU refreshes this matrix after making Resource-backed Verification
Requirement recovery visible for runnable same-Plan peer Tasks in focused
normal/full ContextPacket output. The change was driven by the post-4NR
context/Resource resolver gap where clearing focus could show a runnable
sibling Task's Resource-backed VR, but keeping focus on the current Task hid
that same peer recovery path even in full context.

The pre-change public CLI probe used a temporary Store and showed
`task_count=2`, `plan_task_containment_relations=2`,
`unfocused_runnable_candidates=2`,
`unfocused_full_has_sibling_resource_basis=true`,
`unfocused_full_has_sibling_refresh_hint=true`,
`current_normal_has_sibling_resource_basis=false`,
`current_full_has_sibling_resource_basis=false`,
`current_full_has_sibling_refresh_hint=false`, and
`current_full_context_omitted_items=0`.

The Phase 4NU binary keeps brief context focused and keeps runnable selection,
`claim next`, `next`, ContextPacket schema, snapshot schema, CLI flags, and
`why` behavior unchanged. For focused Task context in `normal` or `full`
profile, it uses a read-only workspace-wide runnable projection to append
existing `verification_requirement` items for runnable peer Tasks sharing the
focused Task's direct parent Plan. The summary includes
`same_plan_peer_task`, `parent_plan`, `peer_runnable=true`, `criterion`,
`local_key`, the existing Resource basis fields, and the basis-aware
`refresh_hint`.

The public CLI dogfood run at
`/tmp/workvcs-4nu-focused-plan-peer-resource-context-20260902T084000Z/post-dogfood`
proved `phase4nu_dogfood=PASS`, `unfocused_runnable_candidates=2`,
`brief_has_same_plan_peer_hint=false`,
`normal_has_same_plan_peer_hint=true`,
`normal_has_peer_refresh_hint=true`,
`full_has_same_plan_peer_hint=true`, and
`full_has_peer_refresh_hint=true`.

This advances the Context resolver, packets, and `why` explanations gate by
closing one focused same-Plan peer Resource recovery gap, backed by
`docs/provenance/phase-4nu-focused-plan-peer-resource-context.md`. The gate
remains `Partial` because broader context/Resource resolver maturity outside
this direct same-Plan peer slice, full relation-subject traversal beyond the
direct endpoint slices, multi-hop/full evolution traversal, and broader causal
traversal remain open. The overall release decision remains false.

## Phase 4NR Update

Phase 4NR refreshes this matrix after making primary containment relation
creation visible as direct endpoint evolution from
`workvcs why --entity <Goal/Plan/Task endpoint>`. The change was driven by the
post-4NQ gap where `why` could show a current `primary_containment` relation
edge for a contained Task endpoint, but reported no direct
`primary_containment.create` evolution operation for that endpoint.

The pre-change public CLI probe used a temporary Store and showed
`relation_edges=1`,
`relation.0.relation_kind=primary_containment`,
`relation.0.direction=incoming`,
`evolution_change_operations=0`, `deferred_relation_families=0`, and an
expected-count query failing for the missing evolution output.

The Phase 4NR binary reports direct `primary_containment.create` endpoint
evolution with relation subject detail for queried Goal, Plan, and Task
endpoints. The public CLI dogfood run at
`/tmp/workvcs-4nr-after-dogfood.MyYclp` proved `goal_relation_edges=1`,
`goal_direction=outgoing`, `goal_evolution_change_operations=1`,
`goal_evolution_subject_relation_kind=primary_containment`,
`plan_child_direction=incoming`, `plan_child_evolution_change_operations=1`,
`task_relation_edges=1`, `task_direction=incoming`,
`task_evolution_change_operations=1`, and
`task_evolution_subject_relation_kind=primary_containment`.

This advances the Context resolver, packets, and `why` explanations gate by
closing one primary containment endpoint evolution gap, backed by
`docs/provenance/phase-4nr-why-primary-containment-relation-evolution.md`.
The gate remains `Partial` because broader context/Resource resolver maturity,
full relation-subject traversal beyond the direct endpoint slices,
multi-hop/full evolution traversal, and broader causal traversal remain open.
The overall release decision remains false.

## Phase 4NQ Update

Phase 4NQ refreshes this matrix after making Task scheduling relations visible
from `workvcs why --entity <Task endpoint>`. The change was driven by the
post-4NP gap where `task scheduling-list` could show a `depends_on` relation,
but `why` reported no relation edge and no direct create evolution operation
for either Task endpoint.

The pre-change public CLI probe used a temporary Store and showed
`dependent_relation_edges=0`,
`dependent_evolution_change_operations=0`, `prereq_relation_edges=0`,
`prereq_evolution_change_operations=0`, and expected-count queries failing for
the missing relation/evolution output.

The Phase 4NQ binary reports current Task scheduling relation edges as
`task_depends_on` and `task_ordered_before`, with `entity_kind=task` endpoints.
It also reports direct `task.scheduling_relation.create` endpoint evolution
with relation subject detail. The public CLI dogfood run at
`/tmp/workvcs-4nq-after-dogfood.fECY7l` proved
`dependent_relation_edges=1`,
`dependent_evolution_subject_relation_kind=task_depends_on`,
`prereq_direction=incoming`, `earlier_relation_edges=1`,
`earlier_evolution_subject_relation_kind=task_ordered_before`, and
`later_direction=incoming`.

This advances the Context resolver, packets, and `why` explanations gate by
closing one Task scheduling endpoint explanation gap, backed by
`docs/provenance/phase-4nq-why-task-scheduling-relation-evolution.md`. The
gate remains `Partial` because broader context/Resource resolver maturity,
full relation-subject traversal beyond the direct endpoint slices,
multi-hop/full evolution traversal, and broader causal traversal remain open.
The overall release decision remains false.

## Phase 4NP Update

Phase 4NP refreshes this matrix after making Resource basis recovery visible in
brief ContextPacket `blocked_dependency` items for focused Tasks blocked by a
prerequisite Task. The change was driven by the post-4NN context/Resource
resolver gap where current-task VR items had recovery hints, but a focused
dependent Task saw only the prerequisite Task id and description.

The pre-change public CLI probe used a temporary Store and showed
`blocked_item_count=1`, `verification_requirement_item_count=0`,
`prereq_vr_visible_in_focused_context=false`,
`context_has_any_refresh_hint=false`,
`blocked_dependency_has_resource_basis=false`, and
`blocked_dependency_has_refresh_hint=false`.

The Phase 4NP binary keeps the ContextPacket schema unchanged and appends
summary text to the existing `blocked_dependency` item:
`dependency_resource_requirements=1`, `dependency_acceptance_criterion`,
`dependency_verification_requirement`, `dependency_vr_local_key`,
`resource_basis=1`, `verification_id`, `resource_id`, `adapter=local-file@1`,
`scope=path@1`, `baseline_observation_id`, and a `refresh_hint` for
`verification cache-refresh --verification <id> --resource-content-from-basis`.

The public CLI dogfood run at `/tmp/workvcs-4np-post-dogfood.NTxC4a` proved
`blocked_dependency_items=1`, `verification_requirement_items=0`,
`has_dependency_resource_requirements=true`, `has_dependency_vr_id=true`,
`has_resource_basis=true`, `has_baseline_observation=true`, and
`has_basis_refresh_hint=true`.

This advances the Context resolver, packets, and `why` explanations gate by
closing one focused blocked-dependency Resource recovery gap, backed by
`docs/provenance/phase-4np-blocked-dependency-resource-context.md`. The gate
remains `Partial` because broader context/Resource resolver maturity, full
relation-subject traversal beyond the direct create/remove/restore endpoint
slices, multi-hop/full evolution traversal, and broader causal traversal remain
open. The overall release decision remains false.

## Phase 4NO Update

Phase 4NO refreshes this matrix after making direct relation create operations
visible as endpoint evolution from `workvcs why --entity <Record or Knowledge
endpoint>` for the currently recognized Record-to-Record,
Record-to-Knowledge, and Knowledge-to-Knowledge relation shapes. The change was
driven by the post-4NM gap where a newly created supports relation was visible
as a current `relation_edges=1` result, but its create ChangeOperation was not
reported in `evolution_change_operations`.

The pre-change public CLI probe used a temporary Store and showed
`pre_change_relation_edges=1`, `pre_change_evolution_change_operations=0`,
`pre_change_deferred_relation_families=0`, and
`pre_change_expected_evolution_1_status=1` for
`--expected-evolution-change-operations 1`.

The Phase 4NO binary preserves the existing relation edge and proves
`new_rr_evolution_change_operations=1`,
`new_rr_operation_type=record.relation.create`,
`new_rr_subject_relation_kind=record_supports`,
`new_rk_evolution_change_operations=1`,
`new_rk_operation_type=record.relation.create`,
`new_rk_subject_relation_kind=record_supports`,
`new_kk_evolution_change_operations=1`,
`new_kk_operation_type=knowledge.relation.create`, and
`new_kk_subject_relation_kind=knowledge_supersedes`.

This advances the Context resolver, packets, and `why` explanations gate by
removing the direct create endpoint evolution gap for relation shapes already
covered by the direct remove/restore endpoint slices, backed by
`docs/provenance/phase-4no-why-relation-create-endpoint-evolution.md`. The
gate remains `Partial` because full relation-subject traversal beyond these
direct create/remove/restore endpoint slices, multi-hop/full evolution
traversal, broader causal traversal, and broader context/Resource resolver
maturity are still open. The overall release decision remains false.

## Phase 4NN Update

Phase 4NN refreshes this matrix after making Resource basis recovery visible in
brief ContextPacket `verification_requirement` items for current Tasks. The
change was driven by the post-4MN context/Resource resolver gap where
Resource-basis refresh was implemented, but a continuation operator reading
`workvcs context --profile brief` still had to inspect verification detail to
find the Resource-backed Verification id, Resource basis, and basis-aware
refresh command.

The pre-change public CLI probe showed the current
`verification_requirement` item but reported
`has_resource_basis_in_context=false`: the summary contained only the
criterion, local key, and requirement statement.

The Phase 4NN binary keeps the ContextPacket schema unchanged and appends
summary text for Resource-backed current-task Verification Requirements:
`resource_basis=1`, `verification_id`, `resource_id`, `adapter=local-file@1`,
`scope=path@1`, `baseline_observation_id`, and a `refresh_hint` for
`verification cache-refresh --verification <id> --resource-content-from-basis`.
When multiple matching Resource-backed Verifications exist, the summary counts
all matching Resource basis entries and selects a basis-refreshable
Verification for the displayed recovery command when one is available.

The public CLI dogfood run at `/tmp/workvcs-phase4nn-dogfood.XS378N` proved
`phase4nn_dogfood=PASS`, `context_items=6`, and a brief context
`summary_json` containing `verification_id=01a0600e-464e-7561-be8a-fcce6e72b9f0`
and the concrete basis-aware cache-refresh hint.

This advances the Context resolver, packets, and `why` explanations gate by
closing one concrete current-task Resource-backed VR recovery hint gap, backed
by `docs/provenance/phase-4nn-context-resource-basis-packet.md`. The gate
remains `Partial` because broader context/Resource resolver maturity, full
relation-subject traversal beyond the direct remove/restore endpoint slices,
multi-hop/full evolution traversal, and broader causal traversal remain open.
The overall release decision remains false.

## Phase 4NM Update

Phase 4NM refreshes this matrix after making direct Record-to-Knowledge and
Knowledge-to-Knowledge relation remove/restore operations visible as endpoint
evolution from `workvcs why --entity <Knowledge endpoint>`. The change was
driven by the post-4NL gap where a removed relation changed a queried Knowledge
endpoint's neighborhood, but the operation subject was the Relation, not the
Knowledge Entity, so `why` reported no evolution operation.

Pre-change probes used temporary Stores and public CLI commands. The previous
4NL baseline proved `rk_relation_edges=0`,
`rk_causal_anchor_changesets=0`, `rk_evolution_change_operations=0`,
`rk_deferred_relation_families=0`, `kk_relation_edges=0`,
`kk_causal_anchor_changesets=0`, `kk_evolution_change_operations=0`, and
`kk_deferred_relation_families=0`. Both expected-count queries exited 1 for
`--expected-evolution-change-operations 1`.

The Phase 4NM binary preserved `relation_edges=0` after removal and proved
`rk_remove_evolution_change_operations=1`,
`rk_remove_subject_relation_kind=record_supports`,
`kk_remove_evolution_change_operations=1`,
`kk_remove_subject_relation_kind=knowledge_supersedes`, and
`*_remove_expected_match=true`. Restore queries for both paths reported
`relation_edges=1`, `evolution_change_operations=2`, and restore operation
type before remove operation type.

This advances the Context resolver, packets, and `why` explanations gate by
removing two concrete direct Knowledge endpoint relation remove/restore
evolution gaps, backed by
`docs/provenance/phase-4nm-why-knowledge-relation-endpoint-evolution.md`. The
gate remains `Partial` because full relation-subject traversal beyond the
direct remove/restore endpoint slices, multi-hop/full evolution traversal,
broader causal traversal, and broader context/Resource resolver maturity are
still open. The overall release decision remains false.

## Phase 4NL Update

Phase 4NL refreshes this matrix after making direct Record-to-Record relation
remove/restore operations visible as endpoint evolution from
`workvcs why --entity <Record endpoint>`. The change was driven by the post-4NK
gap where a removed supports relation changed a queried Decision endpoint's
neighborhood, but the operation subject was the Relation, not the Decision
Entity, so `why` reported no evolution operation.

The old-main/new-binary dogfood run used one Store and one relation removal
commit. The previous main binary proved `old_relation_edges=0`,
`old_causal_anchor_changesets=0`, `old_evolution_change_operations=0`,
`old_deferred_relation_families=0`, and
`old_expected_status=1` for `--expected-evolution-change-operations 1`. The
Phase 4NL binary preserved `new_relation_edges=0` after removal and proved
`new_evolution_change_operations=1`, `new_expected_match=true`,
`new_subject_family=relation`, `new_subject_relation_kind=record_supports`,
matching operation-local relation version, and matching Finding source plus
Decision target endpoint ids.

This advances the Context resolver, packets, and `why` explanations gate by
removing one concrete direct Record relation remove/restore endpoint evolution
gap, backed by
`docs/provenance/phase-4nl-why-relation-subject-endpoint-evolution.md`. The
gate remains `Partial` because full relation-subject traversal beyond this
direct Record remove/restore slice, multi-hop/full evolution traversal,
broader causal traversal, and broader context/Resource resolver maturity are
still open. The overall release decision remains false.

## Phase 4NK Update

Phase 4NK refreshes this matrix after making direct queried-Entity evolution
operation detail operation-local for Entity after-versions. The change was
driven by the post-4NJ gap where `why` could report two direct Entity
operations for one Assumption status workflow, but the older validated
operation still rendered the final invalidated Entity version.

The old-main/new-binary dogfood run used one Store and one invalidated commit.
The previous main binary proved `old_evolution_change_operations=2`,
`old_op1_actual_matches_final_invalidated=true`, and
`old_op1_actual_matches_operation_validated=false`. The Phase 4NK binary kept
`new_evolution_change_operations=2` and proved
`new_op1_actual_matches_final_invalidated=false` and
`new_op1_actual_matches_operation_validated=true`.

This advances the Context resolver, packets, and `why` explanations gate by
removing one concrete operation-local direct Entity detail gap, backed by
`docs/provenance/phase-4nk-why-operation-local-entity-detail.md`. The gate
remains `Partial` because relation-subject traversal, multi-hop/full evolution
traversal, broader causal traversal, and broader context/Resource resolver
maturity are still open. The overall release decision remains false.

## Phase 4NJ Update

Phase 4NJ refreshes this matrix after making `workvcs why --entity <changed
Entity>` project the queried Entity's own direct first-parent evolution
ChangeOperation. The change was driven by the post-4NG/4NH gap where a
superseded prior Decision had an incoming `record_supersedes` relation and a
direct operation in the same supersede ChangeSet, but `why` still reported
`evolution_change_operations=0` because the prior Decision was not the causal
anchor.

The old-main/new-binary dogfood run used one Store and one supersede commit.
The previous main binary proved
`old_prior_evolution_change_operations=0`. The Phase 4NJ binary proved
`new_prior_evolution_change_operations=1`,
`new_prior_evolution_match_expected=true`,
`new_prior_operation_type=record.decision.supersede`,
`new_prior_subject_family=entity`,
`new_prior_subject_statement_json="Use optimistic writes"`, and
`new_prior_deferred_relation_family_0=evolution`. It also preserved the
existing causal-anchor behavior with `new_finding_causal_anchor_changesets=1`
and `new_finding_evolution_change_operations=3`.

This advances the Context resolver, packets, and `why` explanations gate by
removing one concrete direct Entity-subject evolution projection gap, backed by
`docs/provenance/phase-4nj-why-subject-evolution-projection.md`. The gate
remains `Partial` because relation-subject traversal, multi-hop/full evolution
traversal, broader causal traversal, and broader context/Resource resolver
maturity are still open. The overall release decision remains false.

## Phase 4NI Update

Phase 4NI refreshes this matrix after adding and running the local
`scripts/operator-recovery-maturity-v0.1.sh` harness. The slice targeted the
remaining Operator discoverability and actionable recovery blocker: current
errors were stable and documented, but no single current workflow proved the
broader recovery matrix through script-readable CLI output.

The proof run reported `phase4ni_operator_recovery_maturity=PASS`,
`core_error_codes=41`, `guide_error_codes=42`,
`guide_coverage_missing=0`, `guide_coverage_extra=0`,
`guide_retryability_matches_core_rule=true`,
`cli_parse_error_json_recovery=passed`,
`branch_head_conflict_recovery=passed`,
`resource_drift_recovery=applicable`,
`resource_unavailable_recovery=applicable`,
`resource_error_recovery=applicable`, `claim_takeover_recovery=passed`,
`merge_unresolved_recovery=completed`, and
`final_integrity_valid_required=true`.

This closes the Operator discoverability and actionable recovery gate for the
bounded V1-local release scope, backed by
`docs/provenance/phase-4ni-operator-recovery-maturity-dogfood.md`.
The overall release decision remains false because the Context resolver,
packets, and `why` explanations gate remains `Partial`, and the candidate
release operation gate remains `Blocked`.

## Phase 4NH Update

Phase 4NH refreshes this matrix after making Phase 4NG evolution operation
subjects self-explanatory for current recognized subjects. The change was
driven by the remaining post-4NG lookup gap: `why` could report three direct
operation subjects for the causal-anchor ChangeSet, but an operator still had
to perform separate lookups to understand the prior Record statement and the
relation kinds/endpoints.

The dogfood run created one Store with the previous main binary and queried the
same Store through both the previous main binary and the Phase 4NH binary. The
previous output proved `old_evolution_change_operations=3`,
`old_subject_statement_json_present=false`, and
`old_subject_relation_kind_present=false`. The Phase 4NH output kept
`new_evolution_change_operations=3`, proved `new_evolution_match=true`, and
rendered `new_op0_subject_statement_json="Use optimistic writes"`,
`new_op1_subject_relation_kind=record_supersedes`, and
`new_op2_subject_relation_kind=record_derived_from` with source/target detail.

This advances the Context resolver, packets, and `why` explanations gate by
removing one concrete post-4NG subject-detail lookup gap, backed by
`docs/provenance/phase-4nh-why-evolution-subject-detail.md`. The gate remains
`Partial` because full evolution traversal, broader causal traversal, and
broader context/Resource resolver maturity are still open. The overall release
decision remains false.

## Phase 4NG Update

Phase 4NG refreshes this matrix after making causal-anchor ChangeSet operation
subjects visible from `workvcs why`. The change was driven by the remaining
post-4ML evolution gap: `why` could report the anchoring commit and ChangeSet,
but an operator still had to run `changeset operations` to learn which direct
Entity or Relation subjects were changed by that ChangeSet.

The post-change dogfood run created a local temporary Store with a prior
Decision, causal Finding, replacement Decision, and Decision supersede with
`--because-record`. It proved `changeset_operations=3`,
`why_prior_evolution_change_operations=0`,
`why_finding_causal_anchor_changesets=1`,
`why_finding_evolution_change_operations=3`, direct operation subjects for the
prior Decision entity, supersedes relation, and derived_from causal relation,
and `why_filtered_evolution_change_operations=3` after
`--relation-kind record_derived_from --relation-limit 1`.

This advances the Context resolver, packets, and `why` explanations gate by
removing one concrete single-command visibility gap, backed by
`docs/provenance/phase-4ng-why-evolution-operation-projection.md`.
The gate remains `Partial` because full evolution traversal, broader causal
traversal, and broader context/Resource resolver maturity are still open. The
overall release decision remains false.

## Phase 4NF Update

Phase 4NF refreshes this matrix after making the maintained Store portability
validator's successful preserved run log self-contained. The change was driven
by the Phase 4NE independent review finding that final result values were in
stdout and governance evidence, while `run.log` only contained per-command
output.

A small opt-in run with two cycles, one seed Task, two Tasks per cycle, one
Verification per cycle, and one relation pair per cycle passed. The run
reported five final Tasks, two Verification records, four script-counted
scheduling relation versions, two same-target Bundle applies, two source
reopens, two lineage-list checks, one target-local restore check, final
source/target head and WorkState digest convergence, and elapsed time of 18
seconds. The captured stdout summary matched the final same-length `run.log`
tail byte-for-byte, and the log contained no failure markers.

This advances the Operator discoverability and actionable recovery gate by
removing one concrete review and handoff ambiguity, backed by
`docs/provenance/phase-4nf-self-contained-run-log-summary.md`.
The gate remains `Partial` because broader recovery maturity is still open, and
the overall release decision remains false.

## Phase 4NE Update

Phase 4NE refreshes this matrix after a larger maintained Store workload
validation. The run used the existing opt-in
`scripts/maintained-store-portability-v0.1.sh` script with five cycles, 12 seed
Tasks, 20 Tasks per cycle, four Verifications per cycle, and eight relation
pairs per cycle.

The final output reported 112 Tasks, 20 Verification records, 80
script-counted scheduling relation versions, five same-target Bundle applies,
five source reopens, five lineage-list checks, one target-local restore check,
647 final Bundle payload files, 1,817 final payload references,
source/target head convergence at
`01a05ea0-8e7e-7e62-8169-e6b3be863b78`, source/target WorkState digest
convergence at
`437c61bd224372024e31b98c23db218f7eeac371e2cdd058d59c7964aa8c6762`, and
elapsed time of 331 seconds. Log checks found five `bundle apply-dir` commands,
five `store lineage-list` commands, ten `doctor` commands, twelve
`store integrity` commands, one `restore` command, and no failure markers.

This closes the Larger Store and performance evidence gate for the bounded
V1-local release scope, backed by
`docs/provenance/phase-4ne-larger-maintained-store-workload-validation.md`.
The evidence is still not a general benchmark, index-design basis, external
Store proof, remote/cloud proof, or V2 proof.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4ND Update

Phase 4ND refreshes this matrix after maintained Store portability dogfood. The
run initialized one source Store, copied one target Store after a seed
baseline, then kept the same source and target Stores through three maintenance
cycles. Each cycle reopened the source through public CLI commands, added V1
semantic state, created a Checkpoint, exported and validated a Bundle
directory, preflighted and applied the Bundle to the same target Store, proved
target Branch head and WorkState digest convergence, validated the imported
Checkpoint and import metadata, checked same-Store copied-target lineage-list
output, and ran source and target integrity plus source and target doctor with
require-valid mode.

The same run proved target-local post-apply maintenance by forking a
target-local Branch inside the same target Store, adding local work, restoring
that Branch to the imported Bundle head, and confirming the target main Branch
remained at the imported head so later Bundle cycles could continue. The final
source and target head commit both ended at
`01a05e8a-f88a-75d0-bb07-863de3b26c08` with WorkState digest
`c05438072a235a9b6008f07d04ef879666a5fe9cc0126164fe91730358844484`.

This closes the Core Store, lineage, integrity, and local portability gate for
the bounded V1-local release scope, backed by
`docs/provenance/phase-4nd-maintained-store-portability-dogfood.md`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## V2 Boundary

The matrix does not authorize V2 work. The V2 exclusions in the readiness
ledger remain unchanged, including transcript parsing, LLM-generated semantic
records, embeddings/vector search, semantic merge, automatic knowledge
distillation, hooks, Agent orchestration, cloud synchronization, federation,
destructive compaction, and required TUI/GUI/human-first storage.

## Phase 4MP Validation

PASS: docs-only validation for this matrix and its index updates.

Covered checks:

```text
git diff --check
cargo fmt --all -- --check
file existence checks for the matrix, ADR-0457, readiness ledger, and README
literal link and release-state checks across docs/
```

Independent review found no blocker/high/medium issues. The review confirmed
that the matrix does not overclaim release readiness, preserves the broader
recovery-maturity blocker after removing the obsolete open action to create the
matrix, and keeps release/release-candidate operations behind explicit
authorization.

## Phase 4MQ Update

Phase 4MQ refreshes this matrix after real Branch/diff dogfood. The
Workspace, Branch, history, diff, show-at, and restore gate is now `Pass` for
the bounded V1-local scope, backed by
`docs/provenance/phase-4mq-branch-diff-dogfood.md`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MS Update

Phase 4MS refreshes this matrix after real write-mode Handoff consumption
dogfood. The Handoff consumption gate is now `Pass` for the bounded V1-local
scope, backed by
`docs/provenance/phase-4ms-handoff-consumption-write-mode-dogfood.md`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MT Update

Phase 4MT refreshes this matrix after generated external local Git write-mode
merge dogfood. The run pairs an actual external Git conflict and two-parent
merge commit with a WorkVCS merge over the mirrored external project states,
backed by
`docs/provenance/phase-4mt-external-merge-write-mode-dogfood.md`.

This narrows the merge gate but does not close it. The external project was
generated for this dogfood rather than a pre-existing business repository or
broader operator-owned workflow, so merge lifecycle and conflict recovery
remain `Partial` and blocking for broad release maturity.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MU Update

Phase 4MU refreshes this matrix after pre-existing external-project write-mode
merge dogfood. The run cloned the real local `agent_soul` repository into
`/tmp`, changed existing `README.md` content on target/source branches, added
one source-only file, observed a real Git conflict, resolved the Git merge to
the source branch, and completed the mirrored WorkVCS merge lifecycle. The
original `agent_soul` repository remained unchanged.

This closes the merge lifecycle and conflict recovery gate for the bounded
V1-local release scope. The row is now `Pass` and non-blocking. Remote,
distributed, cross-Store, semantic/LLM, Agent-orchestrated, and direct
original-repository mutation merge flows remain outside this V1 gate unless
separately authorized.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MV Update

Phase 4MV refreshes this matrix after Git worktree Resource rename-policy
dogfood. The run staged an actual Git rename in a `/tmp` clone of the
pre-existing local `agent_soul` repository, proved Git rename detection would
report the change as a rename, proved WorkVCS observes the same change under
its no-renames/delete-add policy, and refreshed Resource-backed applicability
to stale drift while leaving the original repository unchanged.

This closes the Git rename-policy subgap for the Resource registration,
observation, applicability, and drift gate, backed by
`docs/provenance/phase-4mv-git-rename-resource-policy-dogfood.md`. After Phase
4MV, the gate remained `Partial` and blocking because symlink/case policy,
submodule or sparse-checkout Git semantics, and background re-observation
scheduling remained unproven.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MW Update

Phase 4MW refreshes this matrix after Git worktree Resource symlink-policy
dogfood. The run staged a tracked symlink in a `/tmp` clone of the
pre-existing local `agent_soul` repository and proved WorkVCS records summary
metadata declaring tracked symlinks are represented through Git index/diff
material. The same run added an untracked symlink and proved refresh projects
`unknown` / `resource_error` with no new ResourceObservation while leaving the
original repository unchanged.

This closes the Git symlink-policy subgap for the Resource registration,
observation, applicability, and drift gate, backed by
`docs/provenance/phase-4mw-git-symlink-resource-policy-dogfood.md`. After Phase
4MW, the gate remained `Partial` and blocking because case-folding policy,
submodule or sparse-checkout Git semantics, and background re-observation
scheduling remained unproven.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MX Update

Phase 4MX refreshes this matrix after Git worktree Resource submodule-policy
dogfood. The run added a temporary local submodule inside a `/tmp` clone of the
pre-existing local `agent_soul` repository, proved parent Git index contains
gitlink mode `160000`, proved submodule-internal untracked files are not listed
as parent untracked file content, and proved cache refresh projects
`stale` / `resource_drift` through parent status while the original
`agent_soul` repository remained unchanged.

This closes the Git submodule-policy subgap for the Resource registration,
observation, applicability, and drift gate, backed by
`docs/provenance/phase-4mx-git-submodule-resource-policy-dogfood.md`. After
Phase 4MX, the gate remained `Partial` and blocking because case-folding
policy, sparse-checkout Git semantics, and background re-observation scheduling
remained unproven.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MY Update

Phase 4MY refreshes this matrix after Git worktree Resource sparse-checkout
policy dogfood. The run enabled sparse checkout for `src` inside a `/tmp` clone
of the pre-existing local `agent_soul` repository, proved a tracked excluded
file was absent from the working tree but still present in parent Git index
material and marked sparse by Git, and proved cache refresh projects
`stale` / `resource_drift` through parent status and diff material while the
original `agent_soul` repository remained unchanged.

This closes the Git sparse-checkout policy subgap for the Resource
registration, observation, applicability, and drift gate, backed by
`docs/provenance/phase-4my-git-sparse-checkout-resource-policy-dogfood.md`.
The gate remains `Partial` and blocking because case-folding policy and
background re-observation scheduling remain unproven.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NB Update

Phase 4NB refreshes this matrix after real-project clone Goal/Plan/Task and
AC/VR recovery dogfood. The run used a `/tmp` clone of the pre-existing local
`agent_soul` repository, created one Goal, one Plan, three contained Tasks, two
`depends_on` relations, and two `ordered_before` relations, then proved blocked
dependency context, pre-verification Task closeout refusal, focused Handoff
consumption by a separate Session, baseline Resource-backed verification,
sandbox-only `README.md` mutation, explicit foreground batch refresh,
`stale` / `resource_drift` projection, stale closeout refusal, recovery
`verify`, Task B and Task C closeout, Plan completion, Goal achievement, and
required-valid integrity/doctor. The original `agent_soul` repository remained
unchanged.

This closes the Goal/Plan/Task and AC/VR recovery gates for the bounded
V1-local release scope, backed by
`docs/provenance/phase-4nb-goal-plan-task-ac-vr-recovery-dogfood.md`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NC Update

Phase 4NC refreshes this matrix after direct epistemic `why` explanation
dogfood. The run proved the pre-change gap on a temporary Store:
`why` reported one direct `record_supports` relation edge, while statement
fields were visible only through separate `record show` and `knowledge show`
commands.

The implementation adds read-only `epistemic_explanations` to
`WhyQueryResult` and stable CLI `epistemic_explanation.<i>.*` fields for
direct Record-to-Record and Record-to-Knowledge `supports`, `contradicts`,
`validates`, and `invalidates` relations. CLI filtering and `--relation-limit`
retain explanations only for rendered relation id/version pairs, and
`--expected-epistemic-explanations` gives dogfood scripts a direct assertion.

This closes the direct epistemic statement lookup subgap, backed by
`docs/provenance/phase-4nc-why-epistemic-explanation.md`. The Context resolver,
packets, and `why` explanations gate remains `Partial` and blocking because
full evolution traversal, broader causal traversal, and broader
context/Resource resolver maturity remain open.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4NA Update

Phase 4NA refreshes this matrix after Resource re-observation scheduling policy
dogfood. The V1-local policy is explicit foreground operator-triggered batch
refresh through `verification cache-refresh --all-resource-backed
--resource-content-from-basis`; background re-observation, daemons, watchers,
automatic polling, implicit refresh, and Agent orchestration remain disabled.

The run used a `/tmp` clone of the pre-existing local `agent_soul` repository,
created two Resource-backed Verifications and one non-resource-backed
Verification, mutated only the clone's `README.md`, and ran the explicit
foreground batch refresh. The command output reported
`reobservation_policy=explicit_operator_batch_refresh`,
`background_reobservation=disabled`, `reobservation_trigger=operator_explicit`,
`reobservation_execution=foreground_command`,
`selection_policy=current_head_resource_backed_verifications`, and
`branch_head_mutation=disabled`; it refreshed two Resource-backed caches,
skipped the non-resource-backed Verification, projected modified `README.md` as
`stale` / `resource_drift`, preserved unchanged `AGENTS.md` as `applicable` /
`all_basis_applicable`, kept the Branch head stable, and left the original
`agent_soul` repository unchanged.

This closes the Resource registration, observation, applicability, and drift
gate for the bounded V1-local release scope, backed by
`docs/provenance/phase-4na-resource-reobservation-scheduling-policy-dogfood.md`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MZ Update

Phase 4MZ refreshes this matrix after Resource case-folding policy dogfood. The
run used a `/tmp` clone of the pre-existing local `agent_soul` repository,
proved local-file exact path summary metadata for `README.md`, proved
case-sensitive glob matching by recording `README.*` with one file and
`readme.*` with zero files, and proved Git worktree Resource cache refresh
projects `stale` / `resource_drift` through parent Git reporting of `README.md`
while the original `agent_soul` repository remained unchanged.

This closes the case-folding policy subgap for the Resource registration,
observation, applicability, and drift gate, backed by
`docs/provenance/phase-4mz-resource-case-folding-policy-dogfood.md`. The gate
remains `Partial` and blocking because background re-observation scheduling
remains unproven.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.

## Phase 4MR Update

Phase 4MR refreshes this matrix after real shared-Claim write/read-write
dogfood. The Session, Claim, Runnable, `claim next`, and `next` gate is now
`Pass` for the bounded V1-local scope, backed by
`docs/provenance/phase-4mr-shared-claim-write-mode-dogfood.md`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.
