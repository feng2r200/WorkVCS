# V1 Release Gate Matrix

Status: current release-maturity gate matrix
Last refreshed: 2026-09-01 by ADR-0459 / Phase 4MR

This matrix is an evidence map for deciding whether the local Rust V0.1
implementation can support a V1 release-maturity claim. It is not a product
specification and does not replace the confirmed product, architecture, schema,
or accepted ADR authorities.

The source-state basis at the start of Phase 4MR was main commit
`c1130e7fd5fdf95cc66931daa02394d74b0f1746` and the V1 readiness ledger last
refreshed by ADR-0458 / Phase 4MQ. Historical governance Plans and logs are
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
| Resource registration, observation, applicability, and drift | Resource-backed verification covers explicit basis refresh, unavailable/error states, drift projection, adapter boundaries, and re-observation policy. | Phases 4LV, 4LX through 4MG, and 4MN cover exact path, path-prefix, glob, Git worktree, persisted-basis, and batch basis refresh scenarios. | Partial | Yes | Decide and prove broader symlink/case/rename policy, broader Git adapter semantics, and background re-observation scheduling only when a real workflow demands them. |
| Session, Claim, Runnable, `claim next`, and `next` | Continuation, focus, and Claim guard flows remain usable across stale recovery and multi-operator scenarios. | Ledger marks the current Session/Runnable/claim-next surface dogfood-proven. Phase 4LW and Phase 4MI cover Claim transfer, stale takeover, and shared read-only collaboration. Phase 4MR covers shared-Claim write/read-write coordination in this repository: non-unique shared Claims block protected writer mutation, then reader release restores unique-writer closeout. | Pass | No | Keep as regression foundation; automatic ownership arbitration, distributed collaboration, and remote multi-operator coordination remain outside V1 unless explicitly authorized. |
| Context resolver, packets, and `why` explanations | Context packets and `why` output expose enough focused, causal, and explanatory state for continuation Agents without speculative LLM extraction. | Phases 4LR through 4LT and 4LX cover scoped packets and rationale projection; Phases 4MA, 4MH, and 4ML cover selected `why` relationships and causal anchors. | Partial | Yes | Prove broader context/Resource resolver behavior, full evolution traversal, epistemic explanation, and broader causal traversal only through concrete dogfood gaps. |
| Handoff consumption | Handoff creation, display, focus consumption, blocked recovery, and continuation work across varied project and write-mode workflows. | Current evidence covers focused Handoff smoke, read-only external project dogfood, and stale Claim recovery through Phase 4LG, Phase 4LV, and Phase 4LW. | Partial | Yes | Consume Handoffs in varied project and write-mode flows, then refresh this matrix with the observed recovery and closeout evidence. |
| Merge lifecycle and conflict recovery | Divergent Work Branch resolution, freeze/continue/abort/restart recovery, and final WorkState proof hold in realistic write-mode external-project work. | Phase 4LN and Phase 4MM prove merge behavior in durable local Stores, including larger conflict sets and two-parent merge commits. | Partial | Yes | Repeat merge in a real write-mode external-project workflow before a broad release-maturity claim. |
| Operator discoverability and actionable recovery | Operators and scripts can identify failures, choose recovery, and parse error output without source inspection. | Phase 4LZ, Phase 4MJ, Phase 4MK, and Phase 4MO cover stable key-value and JSON error output plus per-code recovery guidance for current error codes. | Partial | Yes | Prove broader recovery maturity in realistic workflows and reduce command friction only where repeated dogfood blockage appears. |
| Larger Store and performance evidence | Candidate release behavior is bounded by workload evidence that is larger and more varied than smoke, with integrity/doctor proof. | Phase 4LQ validates portability on a bounded larger Store; Phase 4MM validates a larger merge-path Store. | Partial | Yes | Run a meaningfully larger or more varied workload only when the next real workload justifies it; do not design indexes from the current bounded runs alone. |
| Candidate release operation | A named candidate commit has a fresh full validation matrix, clean git state, closed governance Plan, refreshed release gate matrix, and explicit release authorization. | No candidate release has been authorized or prepared in this slice. | Blocked | Yes | After all functional/dogfood gates pass, run a release-candidate validation from the exact candidate commit and obtain explicit release authorization. |

## Next Highest-Value Work

Use the blocking rows above as the release-oriented queue. The next local slice
should name the exact gate it advances and should prefer real dogfood evidence
over speculative broadening.

Priority candidates:

1. Consume Handoffs in varied project and write-mode flows.
2. Repeat merge in a real write-mode external-project workflow.
3. Close Resource adapter policy or background re-observation gaps only when
   the next continuation or verification workflow proves the need.
4. Refresh this matrix after each blocking gate changes status and before any
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

## Phase 4MR Update

Phase 4MR refreshes this matrix after real shared-Claim write/read-write
dogfood. The Session, Claim, Runnable, `claim next`, and `next` gate is now
`Pass` for the bounded V1-local scope, backed by
`docs/provenance/phase-4mr-shared-claim-write-mode-dogfood.md`.

The overall release decision remains false because other blocking gates remain
`Partial` or `Blocked`.
