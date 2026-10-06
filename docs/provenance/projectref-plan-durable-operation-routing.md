# ProjectRef Plan Durable-Operation Routing Evidence

Status: Source, independent review, exact CI, local installation, live activation, and bounded dogfood complete
Date: 2026-10-07
Decision: [ADR-0516](../decisions/adr/0516-projectref-plan-durable-operation-routing.md)

## Problem reproduced

After registry-v2 cutover, cwd-based `plan admit` and `plan evolve` still used
legacy direct binding discovery. That route intentionally rejected registry
v2, even though product and operator documentation presented WorkVCS Plan as
the durable project carrier. Cognition already had journal-first routing and
missing-receipt recovery, so bypassing it for Plan would have restored the
command while weakening the control-plane durability boundary.

The original journal-admission marker also contained no capability list. An
already installed marker therefore could not safely be interpreted as consent
for newly introduced Work-State mutations.

## Implemented contract

- One physical `capture-journal/v1` now carries distinct
  `cognition_v2`, `legacy_cognition_v1`, `plan_admit_v1`, and
  `plan_evolve_v1` payload kinds.
- Plan manifests remain Plan manifests. They are never converted to cognition
  and cannot carry CaptureGroup state.
- Registry-v2 cwd Plan commands durably admit the typed intent, persist
  `delivery_started`, execute the existing atomic Plan engine, and persist
  `delivery_applied` before reporting success.
- Plan receipts name the delivered Goal/Plan/Task/AC/VR, Record, Evidence, and
  evolution relation versions. Existing-goal admission receipts omit the
  unchanged Goal rather than claiming it was delivered.
- Every Plan receipt carries the exact operation payload kind, and projection
  validates its complete local-ID/object-kind set against the admitted
  create/existing-Goal or in-place/supersede manifest variant.
- Plan record-kind aliases use the same domain normalization for placeholder
  sizing, expected receipt shape, and committed result; `unknown` therefore
  converges as canonical `question` for both admission and evolution.
- The compatibility recovery surface reports the payload kind and repairs a
  missing receipt through the target manifest's original idempotency key.
- Before any Plan Store write, recovery materializes the exact worst-case
  detailed receipt in a fixed maximum-length timestamp envelope and rejects an
  oversized result durably as `plan_receipt_too_large`. Timestamp parsing is
  capped at nine fractional digits, so an actual append cannot exceed the
  preflight envelope because of timestamp precision.
- Pure typed-manifest validation and explicit current-snapshot guard mismatch
  are the only sources of terminal `plan_manifest_rejected` and
  `plan_target_conflict`. Existing idempotent target results are recognized
  before an advanced head is considered a conflict. Wrong-kind Goal/Plan
  entity references are target conflicts, and any terminal disposition is
  persisted before receipt materialization can fail. Engine execution,
  storage, integrity, transaction, control-plane, and post-commit failures
  remain eligible for status-first recovery.
- A projected `delivery_failed` is monotonic for its `delivery_started`
  identity: the journal rejects both a different failure and a later receipt
  before event installation, leaving reconstruction on the original terminal
  state.
- Event append and projection reconstruction validate typed receipt/failure
  families against the admitted payload kind and enforce one recovery action
  per failure code.
- CLI output separates journal admission reuse from target delivery reuse, so
  a retry after only `delivery_started` reports target `created` while a
  commit-before-receipt replay reports target `reused`. Once a receipt is
  durable, result rendering uses a read-only target lookup rather than a
  second mutating Plan call.
- Journal-admission marker version 2 declares `cognition_capture`,
  `plan_admit`, and `plan_evolve`. Version 1 parses as cognition-only.
- An exact version-1 marker for the same registry snapshot can become version
  2 only through an old-digest-locked strict capability-superset refresh.

## Validation

The focused source checks pass:

- core Plan admission and Plan evolution suites;
- ProjectRef primary delivery and commit-before-receipt recovery;
- activation candidate round-trip plus version-1 capability compatibility;
- journal activation fault recovery;
- registry-v2 absent-gate durable-write zero-Store-write matrix;
- registry-v2 routed Plan admission and idempotent replay;
- registry-v2 in-place and supersede evolution through the same journal;
- Goal/Plan/Task recall after routed admission;
- target commit followed by injected pre-receipt failure and exact replay
  convergence; and
- stale Plan compare-and-swap conflict after durable admission with byte-stable
  target Store files;
- deterministic Plan validation rejection as a terminal journal event with no
  target Store mutation, including the invalid-record-kind path that cannot
  be masked by receipt construction;
- existing Plan-as-Goal and Goal-as-Plan guards as terminal target conflicts
  with byte-stable target Store files;
- routed admission and in-place evolution with `record.kind=unknown`, both
  converging to a durable `record:question` receipt and idempotent replay;
- exact oversized-receipt preflight before Store mutation, with terminal
  `plan_receipt_too_large`; and
- exact event-size boundaries at limit-minus-one, limit, and limit-plus-one;
- cross-family receipt/failure rejection, exact create/existing and
  in-place/supersede receipt-shape validation, and exact failure-action
  pairing;
- nanosecond timestamp type bounds plus equality between the preflight event
  size and an actual maximum-precision append;
- rejection of both a different failure and a receipt after terminal Plan
  failure, with unchanged event count and reconstructed failure code;
- a retry after `delivery_started` that reuses the journal intent but creates
  exactly one target Plan commit; and
- explicit same-snapshot version-1-to-version-2 marker refresh, including
  Plan denial before refresh, same-version lateral expansion rejection, and
  success after the explicit version upgrade.

The fresh cumulative candidate also passed on 2026-10-07:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`;
- `cargo test --workspace --all-targets --locked`, including 274 CLI tests,
  13 core unit tests, and every non-opt-in core integration target;
- `scripts/validate-schema-v0.1.sh`;
- `scripts/validate-projectref-acceptance-matrix.sh`, with 98 exact unique
  matrix rows and the typed-Plan/tool-neutral probes passing;
- `scripts/smoke-v0.1-cli-workflow.sh`; and
- `git diff --check`.

The independent review of this exact candidate returned `APPROVE` after
re-checking the two last blocking counterexamples and running the relevant
regression coverage.

## Delivery and local adoption evidence

| Gate | Exact evidence |
| --- | --- |
| Source delivery | Commit `501bb343651b1a3dd3ea649b8b446bc8f237513e` was pushed normally to `main`; local HEAD and `origin/main` matched with a clean worktree. |
| Remote verification | Exact [CI run 37498406205](https://github.com/feng2r200/WorkVCS/actions/runs/37498406205) succeeded with formatting, lint, tests, schema, CLI smoke, and the ProjectRef acceptance matrix all passing. |
| Installed binary | The official clean-source package installed `/Users/ld/.local/bin/workvcs` with SHA-256 `b35f54a4abd587aa388ae4aa18c3314413ab030f7694388bed8e4a7c6ed8b386`. |
| Installed Skill | The official package installed the WorkVCS Skill with entry SHA-256 `1a760b8c76f8123a28fd9cf4e4f267735623ad9196cfe2e7a6acd6634a2b37dc` and tree SHA-256 `11f658a0fe940e7dd2b1de5c04812832d665e7b67f4d99fe83240772ee60b414`. |
| Live marker | Exact same-registry refresh replaced version 1 digest `blake3-256:43fecf6805a629509cb31db49ba101bc32d4475e011eb730c70826dcd15286c8` with version 2 digest `blake3-256:ef6206fe7a52e9878ae238f5aab07a8707e4a31ea7f234e6c369f896d65ef548`; `cognition_capture`, `plan_admit`, and `plan_evolve` are all active. |
| Live admission | Capture `01a11225-b7e1-7079-861a-e1ae0470281d` (`plan_admit_v1`) completed at target commit `01a11225-ba3c-7675-ad3c-5c12b7755004`, creating Goal `01a11225-ba3c-7675-ad3c-5c08d4de8251`, Plan `01a11225-ba3c-7675-ad3c-5c0a1c7d1865`, and Task `01a11225-ba3c-7675-ad3c-5c0c5cb985d5`. Exact replay reused the same capture and target commit. |
| Live evolution | Capture `01a11226-3dda-757e-b90c-166f532fb03c` (`plan_evolve_v1`) completed an in-place update at target commit `01a11226-7790-7701-9e21-b9583f6595dc`. Exact replay reused the same capture and target commit. |

Both capture-recovery projections are current, terminally `completed`, and hold
durable receipts. The live Task tracks publication and exact-CI verification of
this provenance update. After that gate succeeds, Task/Plan/Goal closure is
recorded in the WorkVCS Store rather than generating a circular follow-up
documentation commit merely to quote its own CI result.

The installed artifact also contains the independently delivered replay-cache
optimization from commit `04100dfe110011179ae88a8b42b2a398ffe85142`.
On this same Store, completed recovery status dropped from about 12.5–13.0
seconds immediately before installation to about 0.2 seconds after
installation. That bounded observation links the two repair slices without
claiming a portable benchmark; the full performance evidence remains in
[`phase-5i-integrity-replay-performance.md`](phase-5i-integrity-replay-performance.md).

## Boundaries

This work did not create a release or tag, deploy remotely, force push, perform
an unrelated registry migration, or change work-governance. The local package
installation and exact live capability refresh are separately identified above
with their before/after digests.
