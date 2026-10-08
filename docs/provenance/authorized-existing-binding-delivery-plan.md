# Authorized Existing-Binding Delivery Design and Change Plan

Status: Stage B source implemented and locally validated; Stage C local adoption complete; Stage D separately gated
Observed: 2026-10-08T06:37:47Z
Authority: ADR-0519

## Why this change is justified

The selected live ProjectRef control plane was healthy when inspected: registry
revision 3, all seven bindings valid, both activation markers current, and all
three typed journal capabilities active. The recurring gap was therefore not a
damaged control plane. It was the expected result of public cognition capture
stopping after admission even when resolution and the existing binding were
already valid.

At the observation time the registry-derived journal contained:

| Measure | Count |
| --- | ---: |
| Immutable intents | 806 |
| Stored projections | 784 |
| Completed projections | 780 |
| Terminal legacy-upgrade projections | 1 |
| Terminal semantic-invalid projections | 3 |
| Intents without a projection | 22 |
| No-projection operations currently requiring ProjectRef bootstrap | 12 |
| No-projection operations currently requiring only `apply_binding_receipt` | 10 |
| Intents admitted on 2026-10-08 UTC | 22 |
| Those already completed | 16 |
| Those still without a projection | 6 |

Every one of the ten resolved no-projection operations reported
`binding_state=valid` and `recovery_action=apply_binding_receipt`. The twelve
other operations were genuinely unbound and reported
`recovery_action=apply_project_bootstrap`; they are deliberately outside the
fast path. Counts are a point-in-time snapshot and may change as concurrent
tasks admit or recover operations.

The snapshot used registry digest
`blake3-256:b5ec12377cbe536bce8cb53e5378c95d019a5c8dde1584843b843e68868aa6c9`
and journal root
`/Users/ld/.codex/workvcs/project-bindings.json.d/capture-journal/v1`.
Intent/projection totals came from the canonical file sets and their parsed
`recovery_state`; each CaptureId missing a projection was then inspected with
read-only `project operation-recovery --status`. No recovery apply, registry,
marker, journal, projection, or Store write was part of the snapshot.

The earlier guidance-only checkpoint proposal did not alter command behavior,
so recurrence is expected. ADR-0519 changes the product contract while keeping
the existing safety and authorization gates.

## Delivery stages and stop conditions

### Stage A — design contract (completed)

- Add ADR-0519.
- Align confirmed product, domain, and architecture authority with an explicit
  implementation-pending target.
- Freeze the command spelling, error semantics, file ownership, test matrix,
  and later authorization gates.
- Validate links, formatting, and repository consistency.

Stop after a reviewed local documentation boundary. Do not change runtime,
installed artifacts, activation, live registry/journal/Store state, historical
backlog, or any remote state in this stage.

### Stage B — source implementation (authorized; candidate complete)

The accepted contract is implemented in the repository, with focused and full
local validation plus independent review recorded in
[Authorized Existing-Binding Delivery Candidate Evidence](authorized-existing-binding-delivery-candidate.md).
This stage does not install the binary or Skill and does not mutate the
configured live control plane.

### Stage C — local adoption and bounded canary (completed)

Exact source/package/runtime parity was proven, the reviewed candidate and
Skill were installed, and one newly admitted eligible capture completed through
intent-first delivery, exact target receipt, semantic readback, and zero-write
replay. Current activation markers already satisfied the installed contract, so
no activation refresh was required or performed. Exact hashes and durable IDs
are recorded in
[Authorized Existing-Binding Delivery Local Adoption Evidence](authorized-existing-binding-delivery-local-adoption.md).

### Stage D — historical reconciliation (separately authorized)

Use `--list-open` to classify the backlog. Apply only an explicitly selected
set under fresh status evidence. Do not batch bootstrap unbound owners or treat
standing same-binding authority as historical-sweep authority.

Push, tag, public release, remote deployment, rollback, and destructive cleanup
remain outside every stage above unless named explicitly.

## Exact source change map for Stage B

| File | Required change | Boundary |
| --- | --- | --- |
| `crates/workvcs-core/src/error.rs` | Add stable `project_owner_unbound` and `capture_delivery_incomplete` codes/variants, categories, retryability, display contracts, and code tests. The incomplete-delivery variant carries CaptureId, durable-admission fact, underlying cause code, and recovery action. | No routing or Store mutation. |
| `crates/workvcs-core/src/control_plane/journal.rs` | Add a read-only authority enumeration API and bounded open-operation report types. Reconstruct from intent/events, treat projections as cache, sort deterministically, detect cross-alias CaptureId disagreement, expose ordered prior resolution and target-bearing authority for fast-path continuity checks, and compute the canonical full-inventory digest used by paging. | No journal/projection writes and no new journal schema. |
| `crates/workvcs-core/src/control_plane/mod.rs` | Export only the new read-only inventory types/API required by the CLI. | No semantic engine changes. |
| `crates/workvcs-cli/src/main.rs` | Add `capture --deliver-existing-binding`; factor routed admission so its structured result can feed the existing recovery apply function without parsing rendered text; before any new event, reject a reused intent whose prior ProjectRef or target tuple differs from the current binding and reject changed-registry resolution history without later same-chain proof of the current complete tuple; make marker/capability revalidation a fast-path-only in-lock guard; add `operation-recovery --list-open`, filters, digest-locked CaptureId cursor paging, exact-once distinct-binding validation, and allowlisted rendering; render success and partial-delivery errors; map clean unbound discovery and health to the new classification. | Reuse the current recovery engine and lock/CAS paths; manual recovery stays marker-independent; no second queue or delivery implementation. |
| `crates/workvcs-core/tests/projectref_journal_recovery.rs` | Cover deterministic enumeration, authority corruption, stale/missing projection handling, open/completed filtering, ordering, row limits, and alias collision behavior. | Fixture-only. |
| `crates/workvcs-cli/src/main.rs` test module | Cover Clap compatibility, default admission-only behavior, eligible delivery/replay, unbound/shared/conflict/target-change refusal, reused-intent prior-target mismatch, changed-registry ProjectRef-only resolution without continuity proof, all refusals before any new event, fast-path-only marker guards, partial-error fields, exact-once inventory validation, filters/counts/paging, allowlisted output, and discover/health classification. | Fixture-only, including existing recovery fault injection. |
| `scripts/validate-projectref-acceptance-matrix.sh` | Add static/behavioral obligations for the new explicit mode, inventory, error codes, and unchanged default. | Validation only. |
| `scripts/operator-recovery-maturity-v0.1.sh` | Add bounded CLI assertions for new error rendering and open-operation inventory after the implementation is stable. | Local isolated audit only. |
| `docs/operator/quickstart-and-recovery.md` | Document when standing authority permits the new option and when status/apply remains mandatory. | Must not imply bootstrap or batch authority. |
| `docs/operator/error-recovery-guide.md` | Add both new stable errors and status-first recovery guidance; distinguish v1 `project_binding_not_found`, v2 read `project_owner_unbound`, successful journal `resolution_status=unbound`, and flagged post-admission wrapping. | Documentation only. |
| `docs/operator/workvcs-tool-reference.md` | Add the normative flag/action syntax, mutual exclusions, filters, digest-locked cursor paging, success boundary, and partial-result fields. | Must continue to distinguish default admission from authorized delivery. |
| `skills/workvcs/SKILL.md` | Route an already authorized exact existing-binding checkpoint through the new option; preserve separate gates for every excluded case. | Source Skill only; installation is Stage C. |
| `skills/workvcs/references/workflows.md` | Replace the absolute “never automatic continuation” wording with the accepted default-plus-explicit-mode contract. | Default remains journal-only. |
| `skills/workvcs/references/checkpoint-delivery.md` | Define one-command eligible delivery, partial-result handling, and backlog inventory. | No authority inference. |
| `README.md`, `README.zh-CN.md`, `docs/README.md` | Keep the accepted target explicitly implementation-pending, then report source and installed/live status only when each has its own evidence. | No premature adoption claim. |
| `docs/product/product-definition.md`, `docs/product/v1-v2-boundary.md`, `docs/domain/invariants.md`, `docs/architecture/system-boundaries.md`, `docs/architecture/projectref-control-plane-v2.md` | After source implementation and its required tests pass, change only the ADR-0519 sections from implementation-pending to source-implemented; keep installed/live adoption explicitly pending until Stage C evidence exists. | Product, domain, and architecture authority must advance in the same source commit. |
| `docs/decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md`, this plan | After source implementation and its required tests pass, record source-implemented/adoption-pending status and link the exact candidate evidence; do not leave either document claiming implementation has not started. | Decision acceptance is not installed/live adoption. |
| `docs/architecture/projectref-registry-v2-migration-and-acceptance.md` | Add acceptance rows for eligible continuation, every fail-closed exclusion, read-only inventory, and clean unbound classification. | Migration semantics unchanged. |
| `docs/provenance/authorized-existing-binding-delivery-candidate.md` | Record exact revision, focused/full checks, fault evidence, byte-stability, independent review, and residual risks. | Created only after implementation evidence exists. |

No SQL migration, Store schema change, journal envelope change, capability-marker
version change, or new activation marker is expected. If implementation proves
one necessary, Stage B must stop and return to a material ADR revision instead
of silently expanding scope.

## Acceptance matrix

| ID | Scenario | Required result |
| --- | --- | --- |
| ED-01 | Default registry-v2 cognition capture | Intent durable; Store byte-stable; `delivery_status=not_started`. |
| ED-02 | Flag plus resolved, valid, non-shared binding | Intent precedes target write; current verified receipt; success. |
| ED-03 | Add/remove flag with the same idempotency key | Admission identity is unchanged; only the flagged invocation may continue; a fully completed matching replay writes no event, projection, Store object, or commit. |
| ED-04 | Commit durable before receipt with a returned error | Non-zero partial result names CaptureId; status-first apply converges through target idempotency. |
| ED-04A | Process loss after admission, before a completed receipt, and before a response | The still-open operation appears in `--list-open` with CaptureId and idempotency-key digest, never the raw key; recovery uses that CaptureId without guessing rollback. |
| ED-04B | Process loss after a completed receipt but before a response | Repeating the caller-held idempotency key with the explicit option returns the same receipt and CaptureId with no new event, projection, Store object, or commit. The operation is not falsely listed as open. |
| ED-05 | Clean unbound owner | Intent remains durable; no registry/Store write; bootstrap remains separately gated. |
| ED-06 | Conflict or unresolved owner | Intent remains durable; no fallback or target write. |
| ED-07 | Invalid or exactly shared binding | No target write; stable partial result and explicit repair/isolation action. |
| ED-08 | Registry/target/activation changes after admission | Fresh guards fail closed; no cross-target delivery. |
| ED-08A | Reused open intent contains a different prior target tuple | Refuse before any new event or Store write and require a new Capture for the current target. |
| ED-08B | Admission/read marker becomes inactive or stale | Same-command fast path refuses; exact manual status/apply for the already admitted Capture remains available under its existing digest guards. |
| ED-08C | Reused intent has only a prior ProjectRef resolution, then its recorded registry digest changes | Unless later same-chain authority proves the exact current ProjectRef/Store/Workspace/Branch tuple, refuse before any new event or Store open and require a new Capture. |
| ED-09 | `--capture-group` plus fast-path option | Usage failure before journal or Store write. |
| ED-10 | Registry-v1/legacy route plus fast-path option | Unsupported failure before journal or Store write. |
| OL-01 | Mixed open/completed/terminal journal | Only action-not-`none` rows returned; aggregate counts exact. |
| OL-02 | ProjectRef/payload/action filters, limit, and multiple pages | Counts cover all matches; rows oldest-first; next cursor is explicit; later pages require the exact inventory digest; every matching row is reachable. |
| OL-03 | Many captures share a target | Registry loaded once and every distinct referenced binding that affects classification is fully validated exactly once, never zero or more than once. |
| OL-04 | Missing/stale projection cache | State derived from immutable authority; command remains zero-write. |
| OL-05 | Invalid event chain or alias disagreement | Inventory fails closed and identifies the affected CaptureId/alias. |
| OL-06 | Intent contains newline/control-bearing text and secret canaries | Key-value and JSON rows expose only the bounded metadata allowlist; raw idempotency key, semantic/value/provider/locator/free-text fields and canaries are absent. |
| UB-01 | Registry-v2 read with clean unbound semantic owner | Key-value and JSON return `project_owner_unbound`, recoverable, non-retryable, no fallback/write; v1 misses remain `project_binding_not_found`. |
| UB-02 | `project health --cwd` for same owner | Default output is `degraded` with `resolution_issue=project_owner_unbound`; key-value and JSON `--require-healthy` fail with top-level `project_owner_unbound`. |
| UB-02A | Default capture/status versus flagged capture for same owner | Default journal output retains `resolution_status=unbound`; flagged post-admission failure wraps `cause_error_code=project_owner_unbound` in `capture_delivery_incomplete`. |
| UB-03 | Conflict, malformed marker, or invalid registry | Remains `blocked`/`control_plane_invalid`. |

## Independent validation brief

The reviewer should challenge, rather than restate, these claims:

1. the option cannot accidentally authorize bootstrap, shared-state writes,
   CaptureGroup delivery, or target substitution;
2. post-admission failure remains visible and recoverable without converting a
   partial result into command success;
3. inventory performance improves by sharing validation without weakening full
   integrity or immutable-authority checks;
4. clean unbound classification does not let read-only callers treat absence of
   a binding as usable target state; and
5. the proposed source and adoption stages retain distinct authorization gates;
   and
6. reusing an older intent cannot carry standing authority across a prior
   ProjectRef or Store/Workspace/Branch target change.
