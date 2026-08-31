# ADR-0430: Phase 4LO Bundle Portability Dogfood

Status: Accepted
Date: 2026-09-01

## Context

The V1 readiness ledger marked Checkpoint and Bundle portability as
design-confirmed, implemented, and smoke-proven, but not dogfood-proven. Recent
Phase 4K/4L work had built useful smoke expectations, but the next slice needed
to correct the risk of over-focusing on narrow output assertions by proving a
real operator portability loop.

The current local Bundle implementation exports a directory containing
`manifest.json`, `payload-index.json`, and content-addressed payload files.
Current apply semantics distinguish copied-target fast-forward from same-Store
divergence and avoid overwriting a diverged target Branch.

## Decision

Accept Phase 4LO as the local dogfood proof for copied-target Checkpoint and
Bundle portability.

The dogfood run used the real CLI and local SQLite Stores to prove:

- source Checkpoint creation and validation for the exported head;
- Bundle directory export using `workvcs-local-payload-index-v1` and payload
  index version `1`;
- target `bundle preflight-dir` acceptance for a copied-target fast-forward;
- target `bundle apply-dir --require-applied` importing the missing commit,
  entity version, Checkpoint, and Checkpoint status while updating the Branch
  head;
- target `checkpoint latest`, `checkpoint show`, and `checkpoint validate` for
  the imported Bundle head;
- target-local post-apply work followed by `workvcs restore` back to the
  imported Bundle head;
- restored Work State inspection with two Tasks and no target-local extra Task;
- same-Store divergence preflight refusal, recorded import outcome, non-applied
  apply result, and unchanged target Branch head.

No core or CLI behavior change is required by this dogfood slice.

## Non-Goals

- No schema change.
- No command spelling change.
- No packaged Bundle archive/container format.
- No external Store federation or cross-Store apply claim.
- No cloud sync or distributed collaboration.
- No Agent protocol encoding.
- No GUI/TUI.
- No larger-Store or release maturity claim.

## Consequences

The V1 readiness ledger can mark Checkpoint and Bundle portability
dogfood-proven for local copied-target operation. Remaining V1 work is narrower:
formalize the Bundle container/profile contract, prove or explicitly defer
external Store import/apply semantics, and run a larger Store portability
workload before making scale or release maturity claims.

Operator recovery documentation now names the concrete local loop:
create/validate a Checkpoint, export and validate a Bundle directory, preflight
the target, apply only when `can_apply=true`, validate the imported Checkpoint,
and use `restore` plus `show-at` for Work State recovery. It also records that
`checkpoint latest` is a commit-anchored selector, not a state-digest lookup
for later restore commits.
