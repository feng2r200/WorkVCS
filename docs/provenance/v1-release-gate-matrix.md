# V1 Release Gate Matrix

Status: current release-maturity gate matrix
Last refreshed: 2026-09-02 by ADR-0468 / Phase 4NA

This matrix is an evidence map for deciding whether the local Rust V0.1
implementation can support a V1 release-maturity claim. It is not a product
specification and does not replace the confirmed product, architecture, schema,
or accepted ADR authorities.

The source-state basis at the start of Phase 4NA was main commit
`a2ce415b16e54efded620ec0ad41b03858b779ae` and the V1 readiness ledger last
refreshed by ADR-0467 / Phase 4MZ. Historical governance Plans and logs are
treated only as provenance unless their conclusions are reflected in current
project documents or current validation evidence.

## Current Decision

- `V1_RELEASE_READY=false`
- `V0_1_DOGFOOD_COMPLETE=false`
- `RELEASE_CANDIDATE_ALLOWED=false` without a later matrix refresh that turns
  every blocking gate below to `Pass` using fresh evidence from the candidate
  commit.

V0.1 dogfood should continue. This matrix closes the documentation gap of
having no explicit release gate view; it does not close the implementation,
dogfood, or release-operation gaps named by the gates.

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
| Core Store, lineage, integrity, and local portability | Store bootstrap/open/manifest/lineage/doctor and local Bundle/Checkpoint portability work across ordinary and maintained Stores. | Smoke coverage plus Phase 4LO, Phase 4LP, and Phase 4LQ provenance, including bounded copied-target portability and larger Store portability. | Partial | Yes | Prove broader long-lived Store maintenance and repeated portability/doctor behavior beyond the bounded local profile runs. |
| Workspace, Branch, history, diff, show-at, and restore | Branch and state navigation workflows are proven in real implementation work, not only narrow smoke or post-Bundle inspection. | Current ledger marks this area implemented with partial smoke and dogfood evidence. Phase 4LO dogfoods `restore` and `show-at` against a target Store. Phase 4MQ dogfoods Branch fork, bidirectional Branch diff, Branch history, Branch `show-at`, and two-Branch integrity in a real repository delivery Store. | Pass | No | Keep as regression foundation; broaden only if a future real Branch/diff workflow exposes a concrete gap. |
| Goal, Plan, Task, ordering, dependencies, and containment | Work graph planning and dependency semantics are repeatedly used in real project workflows through closeout. | Phase 4LV dogfoods Goal/Plan/Task for a bounded external-project review; ordering, dependencies, and containment remain broader repetition gaps. | Partial | Yes | Repeat planning, dependency, ordering, and containment usage in varied real-project continuation or implementation loops. |
| AC, VR, Verification, Evidence, and verification wrapper | Acceptance and verification records can close obligations through the CLI and remain understandable in recovery and handoff scenarios. | Ledger marks AC/VR/Verification/Evidence and the top-level `verify` wrapper as dogfood-proven for current covered scenarios. | Partial | Yes | Repeat obligation closure in recovery and handoff-consumption scenarios, including Resource-backed stale/recovery behavior where relevant. |
| Resource registration, observation, applicability, and drift | Resource-backed verification covers explicit basis refresh, unavailable/error states, drift projection, adapter boundaries, and re-observation policy. | Phases 4LV, 4LX through 4MG, and 4MN cover exact path, path-prefix, glob, Git worktree, persisted-basis, and batch basis refresh scenarios; Phase 4MV covers explicit no-renames/delete-add Git rename-policy scenarios; Phase 4MW covers explicit tracked-symlink Git index/diff and untracked-non-regular Resource error-policy scenarios; Phase 4MX covers explicit parent-Git submodule gitlink/status/diff and disabled-recursion policy scenarios; Phase 4MY covers explicit parent-Git sparse-checkout index/status/diff and disabled-expansion policy scenarios; Phase 4MZ covers explicit no-WorkVCS-case-folding scenarios for local-file exact path, path-prefix, glob, and Git worktree Resource observations; Phase 4NA covers explicit foreground operator-triggered re-observation scheduling through current-head Resource-backed batch refresh, with background re-observation disabled. | Pass | No | Keep as regression foundation; background daemons, watchers, automatic polling, implicit refresh, and Agent orchestration remain outside V1. |
| Session, Claim, Runnable, `claim next`, and `next` | Continuation, focus, and Claim guard flows remain usable across stale recovery and multi-operator scenarios. | Ledger marks the current Session/Runnable/claim-next surface dogfood-proven. Phase 4LW and Phase 4MI cover Claim transfer, stale takeover, and shared read-only collaboration. Phase 4MR covers shared-Claim write/read-write coordination in this repository: non-unique shared Claims block protected writer mutation, then reader release restores unique-writer closeout. | Pass | No | Keep as regression foundation; automatic ownership arbitration, distributed collaboration, and remote multi-operator coordination remain outside V1 unless explicitly authorized. |
| Context resolver, packets, and `why` explanations | Context packets and `why` output expose enough focused, causal, and explanatory state for continuation Agents without speculative LLM extraction. | Phases 4LR through 4LT and 4LX cover scoped packets and rationale projection; Phases 4MA, 4MH, and 4ML cover selected `why` relationships and causal anchors. | Partial | Yes | Prove broader context/Resource resolver behavior, full evolution traversal, epistemic explanation, and broader causal traversal only through concrete dogfood gaps. |
| Handoff consumption | Handoff creation, display, focus consumption, blocked recovery, and continuation work across varied project and write-mode workflows. | Current evidence covers focused Handoff smoke, Handoff consumption, blocked recovery, read-only external-project Handoff creation/show, external-project continuation-adjacent Claim work, and Phase 4MS write-mode Handoff consumption with continuation Claim, VR-backed verification, Task closeout, and SessionDiff closeout. | Pass | No | Keep as regression foundation; remote/cloud Handoff, cross-Store synchronization, automatic takeover, and Agent orchestration remain outside V1 unless explicitly authorized. |
| Merge lifecycle and conflict recovery | Divergent Work Branch resolution, freeze/continue/abort/restart recovery, and final WorkState proof hold in realistic write-mode external-project work. | Phase 4LN and Phase 4MM prove merge behavior in durable local Stores, including larger conflict sets and two-parent merge commits. Phase 4MT adds generated external local Git project write-mode merge proof. Phase 4MU repeats merge against pre-existing real `agent_soul` project content cloned into a write-mode sandbox with an actual Git conflict on existing `README.md`, WorkVCS conflict and auto merge items, unresolved freeze guard, explicit source-side resolutions, freeze/continue, a two-parent WorkVCS merge commit, final WorkState proof, Branch diff, SessionDiff closeout, original-project unchanged proof, and required-valid integrity/doctor. | Pass | No | Keep as regression foundation; semantic/LLM merge, remote or distributed merge, cross-Store synchronization, Agent orchestration, and direct mutation of an original external repository remain outside the bounded V1-local release gate unless separately authorized. |
| Operator discoverability and actionable recovery | Operators and scripts can identify failures, choose recovery, and parse error output without source inspection. | Phase 4LZ, Phase 4MJ, Phase 4MK, and Phase 4MO cover stable key-value and JSON error output plus per-code recovery guidance for current error codes. | Partial | Yes | Prove broader recovery maturity in realistic workflows and reduce command friction only where repeated dogfood blockage appears. |
| Larger Store and performance evidence | Candidate release behavior is bounded by workload evidence that is larger and more varied than smoke, with integrity/doctor proof. | Phase 4LQ validates portability on a bounded larger Store; Phase 4MM validates a larger merge-path Store. | Partial | Yes | Run a meaningfully larger or more varied workload only when the next real workload justifies it; do not design indexes from the current bounded runs alone. |
| Candidate release operation | A named candidate commit has a fresh full validation matrix, clean git state, closed governance Plan, refreshed release gate matrix, and explicit release authorization. | No candidate release has been authorized or prepared in this slice. | Blocked | Yes | After all functional/dogfood gates pass, run a release-candidate validation from the exact candidate commit and obtain explicit release authorization. |

## Next Highest-Value Work

Use the blocking rows above as the release-oriented queue. The next local slice
should name the exact gate it advances and should prefer real dogfood evidence
over speculative broadening.

Priority candidates:

1. Repeat Goal/Plan/Task ordering and containment or AC/VR closure only when a
   real continuation or recovery workflow exposes the release-maturity gap.
2. Expand context/Resource resolver or `why` behavior only when a dogfood
   continuation exposes a concrete causal, evolution, or epistemic explanation
   gap.
3. Refresh this matrix after each blocking gate changes status and before any
   release-ready or release-candidate claim.

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
