# ADR-0455: Phase 4MN Batch Basis Refresh

Status: Accepted
Date: 2026-09-01

## Context

ADR-0448 added `verification cache-refresh --resource-content-from-basis` so an
operator can re-observe the persisted Resource basis for one Verification
without manually selecting the exact adapter flag. That removed adapter-flag
selection friction, but dogfood still has to copy each Verification ID and run
one command per Resource-backed Verification.

The V1 readiness ledger keeps background Resource re-observation scheduling
Open. A daemon, watcher, or implicit refresh policy would be too broad for V1
maturity work right now. The smaller V1-safe step is an explicit operator batch
mode that refreshes the current Branch head's Resource-backed Verifications on
demand.

## Decision

Add explicit batch mode to the existing command:

```text
workvcs verification cache-refresh STORE \
  --branch BRANCH_ID \
  --all-resource-backed \
  --resource-content-from-basis
```

Batch mode:

- selects Verifications visible at the current Branch head whose
  `resource_basis` is non-empty;
- skips non-resource-backed Verifications;
- supports only `--resource-content-from-basis`;
- pre-validates every selected Resource basis before recording any
  ResourceObservation or applicability cache;
- preserves `--expected-evaluated-commit` as a branch-head precondition;
- reports `refreshed_caches=<N>` and one cache summary per refreshed
  Verification.

The existing single-Verification command remains compatible:

```text
workvcs verification cache-refresh STORE \
  --branch BRANCH_ID \
  --verification VERIFICATION_ID \
  --resource-content-from-basis
```

## Non-Goals

- No daemon, scheduler, watcher, automatic polling, or implicit refresh during
  `ac status`.
- No semantic re-verification or Evidence creation.
- No WorkState commit creation.
- No new adapter semantics beyond existing local-file and Git worktree basis
  dispatch.
- No hidden best-effort partial refresh: unsupported selected basis entries
  fail before writes.

## Evidence

- Implementation: `crates/workvcs-cli/src/main.rs` adds
  `--all-resource-backed` and `--expected-refreshed` to
  `verification cache-refresh`.
- Focused validation:
  `/tmp/workvcs-4mn-focused-validation-rerun2-20260901T102321Z`.
- Related regression validation:
  `/tmp/workvcs-4mn-related-validation-20260901T102430Z`.
- Process-level dogfood:
  `/tmp/workvcs-4mn-batch-basis-refresh-dogfood-20260901T102546Z`.
- Final validation and independent review closeout are tracked in
  `docs/provenance/phase-4mn-batch-basis-refresh.md`.

## Consequences

Operators can refresh all currently Resource-backed Verifications for a Branch
head with one explicit command. This reduces real dogfood ID plumbing while
keeping V1 re-observation deliberate and auditable.
