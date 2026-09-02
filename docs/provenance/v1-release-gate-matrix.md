# V1 Release Gate Matrix

Status: current release-maturity gate matrix
Last refreshed: 2026-09-02 by ADR-0483 / Phase 4NP

This matrix is an evidence map for deciding whether the local Rust V0.1
implementation can support a V1 release-maturity claim. It is not a product
specification and does not replace the confirmed product, architecture, schema,
or accepted ADR authorities.

The source-state basis at the start of Phase 4NP was main commit
`b661733b7808c4f79310fb8f23bbf81372321d94` and the V1 readiness ledger last
refreshed by ADR-0482 / Phase 4NO. Historical governance Plans and logs are
treated only as provenance unless their conclusions are reflected in current
project documents or current validation evidence.

## Current Decision

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false` without a later matrix refresh that turns
  every blocking gate below to `Pass` using fresh evidence from the candidate
  commit.

V0.1 dogfood should continue. This matrix records local operator recovery
maturity proof, direct queried-Entity evolution operation projection,
operation-local direct Entity evolution detail for `workvcs why`, direct
Record-to-Record relation remove/restore endpoint evolution projection, and
direct Record-to-Knowledge plus Knowledge-to-Knowledge relation remove/restore
endpoint evolution projection, direct relation create endpoint evolution
projection for those recognized relation shapes, plus brief ContextPacket
Resource basis recovery hints for current-task Verification Requirements and
focused blocked-dependency prerequisite Verification Requirements. It does not
close the remaining full relation-subject traversal beyond those direct
create/remove/restore endpoint slices, multi-hop/full evolution traversal,
broader context/Resource resolver, broader causal traversal, or
release-operation gaps named by the gates.

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
| Session, Claim, Runnable, `claim next`, and `next` | Continuation, focus, and Claim guard flows remain usable across stale recovery and multi-operator scenarios. | Ledger marks the current Session/Runnable/claim-next surface dogfood-proven. Phase 4LW and Phase 4MI cover Claim transfer, stale takeover, and shared read-only collaboration. Phase 4MR covers shared-Claim write/read-write coordination in this repository: non-unique shared Claims block protected writer mutation, then reader release restores unique-writer closeout. | Pass | No | Keep as regression foundation; automatic ownership arbitration, distributed collaboration, and remote multi-operator coordination remain outside V1 unless explicitly authorized. |
| Context resolver, packets, and `why` explanations | Context packets and `why` output expose enough focused, causal, and explanatory state for continuation Agents without speculative LLM extraction. | Phases 4LR through 4LT and 4LX cover scoped packets and rationale projection; Phases 4MA, 4MH, and 4ML cover selected `why` relationships and causal anchors. Phase 4NC covers direct Record-to-Record and Record-to-Knowledge epistemic statement projections in `why`. Phase 4NG covers direct ChangeOperation Entity/Relation subject projection for causal-anchor ChangeSets in `why`. Phase 4NH covers current recognized subject detail for those direct operation subjects, including Record/Knowledge statements and Relation kind/source/target detail. Phase 4NJ covers direct first-parent Entity-subject evolution operations for queried changed Entities that are not causal anchors. Phase 4NK covers operation-local Entity detail for multiple direct queried-Entity evolution operations. Phase 4NL covers direct Record-to-Record relation remove/restore operations as endpoint evolution for queried Entity endpoints, including operation-local relation detail when a removed relation is absent from current `relation_edges`. Phase 4NM covers direct Record-to-Knowledge and Knowledge-to-Knowledge relation remove/restore operations as endpoint evolution for queried Knowledge endpoints. Phase 4NO covers direct Record-to-Record, Record-to-Knowledge, and Knowledge-to-Knowledge relation create operations as endpoint evolution for queried Entity endpoints. Phase 4NN covers brief ContextPacket Resource basis recovery hints for current-task Verification Requirements with current-head Resource-backed Verifications, without changing packet schema. Phase 4NP covers brief ContextPacket Resource basis recovery hints on focused `blocked_dependency` items when the blocking dependency Task has current-head Resource-backed Verifications, without changing packet schema. | Partial | Yes | Prove broader context/Resource resolver behavior beyond current-task and focused blocked-dependency Resource-backed VR recovery hints, full relation-subject traversal beyond direct Record-to-Record, Record-to-Knowledge, and Knowledge-to-Knowledge create/remove/restore endpoint evolution, multi-hop/full evolution traversal, and broader causal traversal only through concrete dogfood gaps; broaden epistemic traversal only if a future concrete dogfood gap requires it. |
| Handoff consumption | Handoff creation, display, focus consumption, blocked recovery, and continuation work across varied project and write-mode workflows. | Current evidence covers focused Handoff smoke, Handoff consumption, blocked recovery, read-only external-project Handoff creation/show, external-project continuation-adjacent Claim work, and Phase 4MS write-mode Handoff consumption with continuation Claim, VR-backed verification, Task closeout, and SessionDiff closeout. | Pass | No | Keep as regression foundation; remote/cloud Handoff, cross-Store synchronization, automatic takeover, and Agent orchestration remain outside V1 unless explicitly authorized. |
| Merge lifecycle and conflict recovery | Divergent Work Branch resolution, freeze/continue/abort/restart recovery, and final WorkState proof hold in realistic write-mode external-project work. | Phase 4LN and Phase 4MM prove merge behavior in durable local Stores, including larger conflict sets and two-parent merge commits. Phase 4MT adds generated external local Git project write-mode merge proof. Phase 4MU repeats merge against pre-existing real `agent_soul` project content cloned into a write-mode sandbox with an actual Git conflict on existing `README.md`, WorkVCS conflict and auto merge items, unresolved freeze guard, explicit source-side resolutions, freeze/continue, a two-parent WorkVCS merge commit, final WorkState proof, Branch diff, SessionDiff closeout, original-project unchanged proof, and required-valid integrity/doctor. | Pass | No | Keep as regression foundation; semantic/LLM merge, remote or distributed merge, cross-Store synchronization, Agent orchestration, and direct mutation of an original external repository remain outside the bounded V1-local release gate unless separately authorized. |
| Operator discoverability and actionable recovery | Operators and scripts can identify failures, choose recovery, and parse error output without source inspection. | Phase 4LZ, Phase 4MJ, Phase 4MK, and Phase 4MO cover stable key-value and JSON error output plus per-code recovery guidance for current error codes. Phase 4NF makes the maintained Store portability validator's successful preserved `run.log` self-contained by appending the final stdout summary to the log tail, proving stdout/log-tail equality in a real opt-in run. Phase 4NI adds a local recovery maturity matrix proving guide coverage for all 41 core business error codes plus `cli_parse_error`, the retryability rule, parse-error JSON recovery, branch-head retry, Resource drift/unavailable/error recovery to applicable, stale-gated Claim takeover, merge unresolved recovery, and final Store integrity. Phase 4NN exposes Resource-backed VR basis-aware cache-refresh hints directly in brief current-task context. Phase 4NP exposes the same recovery path in focused `blocked_dependency` context for blocking prerequisite Tasks. | Pass | No | Keep as regression foundation; reduce command friction only where future dogfood exposes repeated workflow blockage. |
| Larger Store and performance evidence | Candidate release behavior is bounded by workload evidence that is larger and more varied than smoke, with integrity/doctor proof. | Phase 4LQ validates portability on a bounded larger Store; Phase 4MM validates a larger merge-path Store; Phase 4NE validates a larger maintained Store with five cycles, 112 final Tasks, 20 Verifications, 80 script-counted scheduling relation versions, five same-target applies, 647 final payload files, 1,817 final payload references, and source/target required-valid integrity/doctor in 331 seconds. | Pass | No | Keep as bounded regression evidence; do not design indexes or claim general performance maturity without future workload-specific profiling. |
| Candidate release operation | A named candidate commit has a fresh full validation matrix, clean git state, closed governance Plan, refreshed release gate matrix, and explicit release authorization. | No candidate release has been authorized or prepared in this slice. | Blocked | Yes | After all functional/dogfood gates pass, run a release-candidate validation from the exact candidate commit and obtain explicit release authorization. |

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
