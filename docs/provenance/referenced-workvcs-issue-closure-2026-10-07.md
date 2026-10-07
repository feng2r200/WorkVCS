# Referenced WorkVCS Issue Closure Matrix

Status: Every confirmed defect from the two referenced investigations is fixed,
delivered, installed, and verified in live use
Date: 2026-10-07

## Scope and decision rule

This matrix closes the confirmed defects found in these two Codex
investigations:

- `01a1112f-c5cc-7962-b2af-81bd40e75b6c`, which investigated slow discovery,
  Recall, and capture recovery plus the relationship between those symptoms;
- `01a11141-b717-7c92-bdb8-d52a08b26dc4`, which investigated registry-v2
  `plan admit` and `plan evolve` routing and then adversarially reviewed the
  implementation.

An item is marked closed only when the source behavior, regression coverage,
delivered commit, exact remote CI, installed artifact, and any required live
control-plane adoption agree. Suggestions that would intentionally weaken or
change the safety contract are listed separately and are not silently counted
as defects.

## Performance and recovery investigation

| Confirmed issue | Closure | Evidence |
| --- | --- | --- |
| Complete integrity validation replayed every commit by recursively rebuilding its ancestry, producing quadratic repeated work. | Closed. Complete validation now shares a request-scoped replay cache while still checking every commit. | [`phase-5i-integrity-replay-performance.md`](phase-5i-integrity-replay-performance.md) |
| Repeated verified Store opens amplified recovery cost; a pending capture apply performed three full validations. | Closed. Preparation reuses the already validated read-only Engine, while the separately required writable-open validation remains fresh; apply now performs two full validations. | Phase 5I correctness and safety-boundary evidence |
| Recall replayed the same immutable HEAD separately for each requested semantic family. | Closed. One cached semantic snapshot serves the requested Goal, Plan, Task, Record, Knowledge, and relation families. | Phase 5I exact old/new profile comparison |
| Recursive replay overflowed the test-thread stack at 5,000 commits. | Closed. Replay uses an explicit `Enter`/`Exit` frame stack; the 500/1,000/5,000-commit release-mode regression passes. | Phase 5I scale evidence |
| A transient journal-lock observation might have been mistaken for the root cause. | Resolved as a diagnosis, not a code defect. Measurements isolated complete integrity replay and repeated Store validation as the cause; journal admission was not the bottleneck. | Phase 5I problem statement and same-machine measurements |

Commit `04100dfe110011179ae88a8b42b2a398ffe85142` delivered this repair.
Exact [CI run 37472069961](https://github.com/feng2r200/WorkVCS/actions/runs/37472069961)
succeeded. The accepted implementation preserves complete integrity validation,
identity checks, canonical replay, and the fresh validation after writable
open. On the maintained shared Store, completed recovery status fell from
about 12.5–13.0 seconds immediately before installation to about 0.2 seconds
after installation; the wider same-machine measurements are recorded in the
Phase 5I evidence.

### Deliberately unimplemented contract alternatives

The first investigation also discussed three possible contract changes:
removing the second full validation, adding a weaker routine-status validation
path, and adding broader integrated health/trace reporting. They were not
confirmed defects and were explicitly left behind a separate decision gate.
The delivered optimization made them unnecessary for the observed latency and
did not trade away the current integrity boundary. They remain possible future
product decisions, not incomplete fixes.

## Registry-v2 Plan control-plane investigation

| Confirmed issue | Closure in the delivered contract |
| --- | --- |
| Cwd-based `plan admit` and `plan evolve` had no registry-v2 durable journal route and safe refusal looked like a completed cutover. | Both commands now use typed `plan_admit_v1` and `plan_evolve_v1` journal-first delivery under registry v2. |
| The activation marker did not state which mutation capabilities were authorized. | Marker version 2 explicitly declares `cognition_capture`, `plan_admit`, and `plan_evolve`; version 1 remains cognition-only and can be upgraded only by an exact same-snapshot digest-locked refresh. |
| A detailed Plan receipt could exceed the journal event limit after the Store commit. | The exact worst-case receipt is materialized and size-checked before any target Store write; oversize is a durable `plan_receipt_too_large` failure. |
| Deterministic Plan rejection could remain an ambiguous pending primary delivery. | Pure manifest rejection and target conflict are durable, typed terminal failures with no target Store mutation. |
| Reusing a journal intent could misreport whether the target was created or reused. | Journal-admission reuse and target-delivery reuse are reported separately. |
| A typed intent was not fully bound to its typed receipt or failure family. | Event append and projection reconstruction enforce payload-kind-specific receipt shapes and failure codes. |
| Terminal classification was too broad and could terminalize engine, storage, control-plane, or post-commit uncertainty. | Only pure validation and explicit snapshot/target conflicts are terminal; operational and post-commit failures remain status-first recoverable. |
| Receipt preflight used a timestamp envelope that did not exactly bound the append. | Preflight uses the fixed maximum nanosecond timestamp, and timestamp parsing rejects more than nine fractional digits. |
| Receipt construction could shadow a deterministic terminal disposition. | The terminal failure is persisted before receipt materialization can fail. |
| Admission and evolution receipt families overlapped. | Complete local-ID/object-kind sets are validated against create/existing-Goal and in-place/supersede variants. |
| A retry after a durable receipt reran the mutating Plan engine. | Rendering a completed result uses a read-only target lookup. |
| Plan variants could reach panic-only `manifest()` and `into_manifest()` paths. | Typed Plan variants are handled explicitly and no longer enter the cognition-only accessors. |
| `record.kind=unknown` sized as one kind but committed as canonical `question`. | Placeholder sizing, expected receipt shape, and the Plan engine share canonical record-kind normalization. |
| A terminal delivery failure could be overwritten by another failure or receipt. | Terminal failure is monotonic for one delivery identity; both replacement paths are rejected before event installation. |
| A generic control-plane error obscured missing Plan authorization. | Capability-aware marker checks and status output identify the missing `plan_admit` or `plan_evolve` authorization while retaining the stable control-plane error family. |

The complete behavioral and regression evidence is in
[`projectref-plan-durable-operation-routing.md`](projectref-plan-durable-operation-routing.md).
The independent final review returned `APPROVE` with no remaining P0, P1, or P2
finding. Commit `501bb343651b1a3dd3ea649b8b446bc8f237513e` was pushed to
`main`, and exact [CI run 37498406205](https://github.com/feng2r200/WorkVCS/actions/runs/37498406205)
succeeded. The installed marker and live routed admission/evolution canaries
also completed and replayed idempotently.

## Cross-project state isolation and work-governance follow-up

The follow-up review found one additional live governance defect: the WorkVCS
and work-governance ProjectRefs resolved independently but named the same
unpartitioned Store/Workspace/Branch. A correct locator was therefore not proof
of isolated logical-project state.

This is now closed at both layers:

- WorkVCS commit `fa2597636410280b5049fc789468bc37c93602ed` adds the
  digest-locked `project isolate-shared-binding` control-plane operation,
  regression coverage, ADR, Skill guidance, and operator evidence. Exact
  [CI run 37553941190](https://github.com/feng2r200/WorkVCS/actions/runs/37553941190)
  succeeded.
- The installed WorkVCS binary and Skill are sourced from that clean commit.
  The work-governance ProjectRef now names a fresh deterministic dedicated
  Store, while the old Store and the WorkVCS peer binding remain byte-stable.
  Both activation markers were explicitly refreshed against registry revision
  3, all declared capabilities are active, and all seven registry bindings
  validate.
- A journal-backed Plan was then admitted through the isolated
  work-governance binding, proving that the repaired route is writable and
  recoverable without copying ambiguous history.
- work-governance commit `e178ee1c219430a8af00e9fac1260d5b28f51c55`
  makes provider isolation a project-startup invariant, bounds recovery
  packets, and invalidates snapshots only on typed receipts or relevant
  mutations. Exact [CI run 37553940420](https://github.com/feng2r200/work-governance/actions/runs/37553940420)
  succeeded, and plugin version `2.0.0+codex.20261007002945` is installed and
  enabled.

Detailed live before/after evidence is in
[`shared-project-binding-isolation.md`](shared-project-binding-isolation.md).

## Final boundary

All confirmed defects and review findings in scope are closed. No release,
tag, remote deployment, force push, old-Store deletion, or ambiguous history
copy was performed or inferred from this result.
