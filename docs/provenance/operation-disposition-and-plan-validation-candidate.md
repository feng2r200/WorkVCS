# Operation Disposition and Plan Validation Candidate Evidence

Status: Source implemented and locally validated; installation and live adoption not performed
Date: 2026-10-08
Decisions: [ADR-0520](../decisions/adr/0520-operation-disposition-and-global-inventory-classification.md), [ADR-0521](../decisions/adr/0521-plan-manifest-validation-and-routed-rejection-diagnostics.md)

## Problem boundary

Two related runtime gaps were confirmed without weakening journal-first Plan
delivery:

- the global open-operation view could not retain completed and explicitly
  closed history or terminally classify an obsolete pre-delivery operation;
- an intrinsically invalid Plan manifest could correctly become the durable
  zero-Store-write `plan_manifest_rejected` result, but callers had no public
  zero-write validator and the routed error omitted the CaptureId and cause
  already known to that invocation.

The deterministic rejection itself remains valid audit authority. The
optimization is to prevent avoidable rejected intents and make any remaining
terminal result directly diagnosable and globally auditable.

## Implemented contract

- `operation_disposition_recorded` adds immutable `superseded` and `abandoned`
  terminal states before target-bearing authority exists.
- `--dispose superseded` requires a distinct later Capture; `--dispose
  abandoned` forbids a successor. Registry and reconstructed-projection
  digests are mandatory compare-and-swap guards.
- Disposition uses the registry and journal quiescence locks, opens no Store,
  writes no routing state, and is idempotently reusable only for an identical
  payload.
- `--list-open` preserves the recovery-action membership contract while
  `--list-all` retains completed, deterministic-terminal, superseded, and
  abandoned operations. Scope is included in the paging digest.
- Inventory rows distinguish projected state, effective state, lifecycle,
  disposition, successor, and bounded receipt/failure metadata. Terminal
  failure and disposition states cannot be resurrected by registry drift.
- `workvcs plan validate --operation admit|evolve --manifest PATH` runs the
  same strict typed parsing and intrinsic validation used by delivery without
  resolving a project or reading or writing registry, marker, journal,
  projection, or Store state.
- A routed deterministic Plan failure now returns
  `capture_delivery_incomplete` with the durable CaptureId,
  `journal_persisted=true`, terminal failure code, canonical recovery action,
  and bounded same-invocation detail. The detail is not persisted in the
  journal or inventory.

## Local validation

The cumulative candidate passed:

- `cargo fmt --all -- --check`;
- `cargo check --workspace --all-targets --locked`;
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
- `cargo test --workspace --all-targets --locked`;
- `scripts/validate-schema-v0.1.sh`;
- `scripts/smoke-v0.1-cli-workflow.sh`;
- `scripts/operator-recovery-maturity-v0.1.sh`;
- `scripts/validate-projectref-acceptance-matrix.sh`, with 98 exact matrix
  rows and both new acceptance probes passing; and
- `git diff --check`.

Focused regressions prove zero Store writes for validation, rejection, and
disposition; exact disposition replay; stale-digest and target-bearing
refusal; later-successor enforcement; scope-bound inventory paging;
legacy-projection rebuild compatibility; structured routed failure detail;
and replay without an additional intent or event.

## Adoption boundary

This candidate did not install or activate a binary or Skill, mutate any live
historical operation, sweep a backlog, Push, release, tag, or deploy. Those
actions remain separately gated. In particular, the previously rejected Plan
Capture remains immutable audit history and was not rewritten or disposed by
this source change.
