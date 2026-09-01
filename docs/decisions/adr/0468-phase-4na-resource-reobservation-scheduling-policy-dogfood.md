# ADR-0468: Phase 4NA Resource Re-Observation Scheduling Policy Dogfood

Status: Accepted
Date: 2026-09-02

## Context

ADR-0448 added explicit basis-aware re-observation for one Verification.
ADR-0455 added explicit batch refresh for all current-head Resource-backed
Verifications on a Branch. ADR-0463 through ADR-0467 made the remaining
bounded Resource adapter policies explicit and dogfood-proven.

After Phase 4MZ, the Resource registration, observation, applicability, and
drift gate still listed background re-observation scheduling as open. A daemon,
watcher, automatic polling loop, implicit refresh during read commands, or
Agent-driven scheduler would exceed the V1-local boundary.

## Decision

For V1-local WorkVCS, Resource re-observation scheduling is explicit and
foreground-only. The supported scheduling point is an operator-invoked batch
refresh:

```text
workvcs verification cache-refresh STORE \
  --branch BRANCH \
  --all-resource-backed \
  --resource-content-from-basis
```

The command evaluates the current Branch head, selects current-head
Verifications with non-empty Resource basis, skips non-resource-backed
Verifications, validates supported Resource basis refresh inputs before writing
any refresh observations, records applicability caches, and does not move the
Branch head.

Batch refresh output now exposes the scheduling policy:

```text
reobservation_policy=explicit_operator_batch_refresh
background_reobservation=disabled
reobservation_trigger=operator_explicit
reobservation_execution=foreground_command
selection_policy=current_head_resource_backed_verifications
branch_head_mutation=disabled
```

## Non-Goals

- No daemon, watcher, automatic polling loop, cron integration, or implicit
  refresh.
- No read-command side effects.
- No Agent orchestration or automatic task scheduling.
- No Store schema, command-shape, manifest fingerprint profile, or cache
  applicability semantic change.
- No remote, distributed, cross-Store, V2, release, release-candidate, tag,
  Push, or deployment behavior.

## Evidence

- Implementation and focused tests:
  `crates/workvcs-cli/src/main.rs`.
- Current-behavior inspection:
  `/tmp/workvcs-4na-reobservation-scheduling-inspection-20260901T171900Z/summary.txt`.
- Focused validation:
  `/tmp/workvcs-4na-focused-validation-corrected-20260901T172740Z/summary.txt`.
- Real pre-existing project clone dogfood:
  `/tmp/workvcs-4na-resource-reobservation-scheduling-dogfood-20260901T173650Z/summary.txt`.
- Candidate full pre-merge validation:
  `/tmp/workvcs-4na-full-validation-candidate-20260901T174530Z/summary.txt`.
- Read-only independent review:
  `01a05e02-ea38-7990-bd14-754e630b8a61`.
- Final post-review validation:
  `/tmp/workvcs-4na-final-validation-post-review-20260901T175200Z/summary.txt`.
- Provenance document:
  `docs/provenance/phase-4na-resource-reobservation-scheduling-policy-dogfood.md`.
- Updated release gate matrix:
  `docs/provenance/v1-release-gate-matrix.md`.
- Updated readiness ledger:
  `docs/provenance/v1-readiness-ledger.md`.

## Consequences

The Resource registration, observation, applicability, and drift gate no longer
needs a separate background scheduling capability for the bounded V1-local
release scope. The release-scope policy is visible, test-covered, and
dogfood-proven as explicit foreground operator batch refresh.

Background daemons, watchers, automatic polling, implicit refresh, Agent
orchestration, and remote/distributed scheduling remain outside V1.
