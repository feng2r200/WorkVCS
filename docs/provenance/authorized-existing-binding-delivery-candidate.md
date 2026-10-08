# Authorized Existing-Binding Delivery Candidate Evidence

Status: Stage B source implemented and locally validated; Stage C adoption pending
Date: 2026-10-08
Decision: [ADR-0519](../decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md)
Plan: [Authorized Existing-Binding Delivery Design and Change Plan](authorized-existing-binding-delivery-plan.md)
Implementation commit: `c89a75a21b4f84dda6215b565511edf444a78541`
Implementation base: `6ce00060c1d469e8cc6e8aa082b2a84e675b6df7`

## Delivered source boundary

The implementation commit adds the explicitly authorized
`capture --deliver-existing-binding` continuation, the read-only
`project operation-recovery --list-open` inventory, and the dedicated
`project_owner_unbound` and `capture_delivery_incomplete` machine contracts.
Default registry-v2 capture remains journal-only.

The fast path reuses the existing recovery engine and remains limited to one
resolved, fully valid, non-shared existing binding. It admits the immutable
intent before delivery and refuses bootstrap, CaptureGroup, registry-v1,
conflict, unresolved ownership, invalid or shared bindings, changed targets,
unproven historical target continuity, and terminal failure state. Marker and
capability checks remain fast-path-only; exact manual recovery of an already
admitted operation remains marker-independent.

The inventory reconstructs state from immutable intent/event authority,
validates every classification-relevant distinct binding exactly once, treats
stored projections as cache, provides deterministic filtered and digest-locked
paging, and renders only the bounded metadata allowlist. Backlog presence does
not alter project-health classification or authorize recovery.

The same commit aligns source Skill, product, domain, architecture, operator
documentation, acceptance probes, and the isolated operator audit. It changes
no Store schema, registry schema, journal envelope, activation version, or
capability marker.

## Focused behavioral evidence

The final implementation input passed these focused CLI regressions:

- nine `capture_existing_binding` tests covering explicit success and
  zero-write replay, fast-path-only marker guards, changed registry without
  full target proof, physical Store-path continuity, shared target, conflict,
  unresolved invocation, invalid binding, CaptureGroup, and registry-v1;
- clean-unbound health and flagged-capture coverage in key-value and JSON;
- semantic-locator unbound discovery coverage in key-value and JSON;
- inventory coverage for deterministic filters, paging, allowlist, alias and
  immutable-authority failures, projection-cache independence, missing
  bindings, exact validation telemetry, and terminal-action preservation on a
  shared binding; and
- existing recovery fault injection, including commit-before-receipt and
  projection-install interruption, through the unchanged recovery engine.

The refusal fixtures compare registry, journal-event, projection, and SQLite
Store snapshots at the relevant boundary. Ineligible fast-path attempts retain
the admitted intent when admission is valid but create no new recovery event,
projection, or target mutation. Unsupported CaptureGroup and registry-v1
forms fail before admission. Completed replay retains the same CaptureId and
receipt with zero new event, projection, semantic object, or target commit.

## Full local validation

The exact implementation input that became commit
`c89a75a21b4f84dda6215b565511edf444a78541` passed:

- `git diff --check`;
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --locked -- -D warnings`;
- `cargo test --workspace --all-targets --locked`: all 296 CLI tests and all
  core unit/integration targets passed with zero failures; the existing
  opt-in 5,000-commit scale test remained ignored by its declared contract;
- `scripts/validate-schema-v0.1.sh`: `schema-v0.1 validation ok`;
- `scripts/smoke-v0.1-cli-workflow.sh`: `smoke_result=passed`; and
- `scripts/operator-recovery-maturity-v0.1.sh`:
  `phase4ni_operator_recovery_maturity=PASS`, 60 core error codes plus the CLI
  parse error covered, zero missing or extra guide entries, and retryability
  matching the core rule; and
- `scripts/validate-projectref-acceptance-matrix.sh`: all 98 contract and
  ledger rows matched, every status was valid, the authorized existing-binding
  delivery probe passed, and the generic core remained tool-neutral.

The operator audit is behavioral rather than help-only. In one disposable
root it performs v1 binding, digest-locked registry migration, exact read and
journal activation, default journal-only capture, filtered read-only open
inventory, and an unbound flagged capture rendered in both key-value and JSON.
It proves the pending operation is visible, the raw idempotency key is absent,
the binding validation counts are truthful, and partial delivery exposes its
CaptureId, durable-journal fact, cause code, and status-first recovery action.
The configured live registry, journal, markers, and semantic Stores are not
used by this audit.

## Independent review

An independent read-only review challenged authorization scope, target
continuity, shared-binding handling, immutable-authority inventory,
machine-output truth, documentation timing, and validation completeness. Its
final result reported no Blocker, High, or Medium finding.

The review's earlier Medium gates were closed before this evidence was
created:

1. source documents stopped claiming an exact Commit or candidate file before
   the implementation Commit existed; and
2. conflict, unresolved, invalid-binding, CaptureGroup, registry-v1,
   key-value/JSON partial and unbound regressions plus a real operator audit
   were added and passed.

The implementation Commit was then created and read back before this document
recorded its exact hash. This file does not infer that exact identity from a
dirty working tree.

## Remaining authority and risk boundary

This evidence closes Stage B source implementation and local validation only.
It does not claim that the user-local binary or installed Skill contains this
commit, that any configured live control plane has used the new route, or that
the historical backlog has been reconciled.

Stage C remains limited to exact-source packaging, atomic replacement of the
existing local binary and installed Skill, installed/source parity checks, and
one newly admitted same-binding canary. If current marker inspection shows
that activation refresh is required, Stage C must stop for separate user
authorization; this evidence does not authorize refresh.

Historical backlog recovery, Push, tag, release, remote deployment, rollback,
and destructive cleanup remain outside the authorized boundary.
