# ADR-0414: Phase 4KY Verification Wrapper

Status: Accepted
Date: 2026-08-31

## Context

The V1 readiness ledger identifies the deterministic `verify` wrapper as the
highest-priority dogfood-biased gap after Phase 4KX. The current implementation
already has lower-level commands for Evidence creation, Resource observation,
Verification creation, and Verification applicability cache recording. That
surface is scriptable, but a real Agent must still manually transfer generated
Evidence, Observation, Verification, fingerprint, and commit identifiers across
multiple command invocations.

Continuing to add only expectation flags or list/show filters would improve test
assertions without materially improving the local WorkVCS tool loop. Phase 4KY
therefore adds one bounded wrapper around the existing semantic operations.

## Decision

1. Add a high-level `Engine::verify(...)` runtime operation and an Agent-facing
   `workvcs verify` CLI command.
2. The wrapper records exactly one Verification target per invocation:
   `AcceptanceCriterion` or `VerificationRequirement`.
3. The wrapper always creates one Evidence object for the invocation and attaches
   it to the created Verification through the existing immutable
   `evidenced_by` closure.
4. The wrapper may record one Resource Observation in the same invocation. When a
   Resource Observation is provided, the created Verification stores one matching
   Resource Basis entry whose baseline fingerprint and optional baseline
   observation point at that newly captured observation.
5. When the wrapper records a Resource Observation, it also records one
   branch-scoped Verification applicability cache entry at the new Verification
   commit, using an observed Resource stamp for that same fingerprint and
   observation.
6. The wrapper preflights the Branch HEAD and target existence before creating
   invocation Evidence or Resource Observation, so common stale-head or missing
   target failures do not leave new provenance objects.
7. The wrapper output renders the Evidence, optional Resource Observation,
   Verification, and optional applicability cache identifiers and state needed by
   later `show`, `list`, `context`, or handoff commands.
8. Existing lower-level `evidence`, `resource observe`, and `verification`
   commands remain supported for explicit, multi-step, diagnostic, and batch-like
   workflows.

## Non-Goals

- No schema change.
- No arbitrary command execution.
- No shell adapter, Git adapter, filesystem adapter, or path/glob normalization
  decision.
- No automatic transcript parsing, LLM extraction, embeddings, orchestration, or
  background cache refresh.
- No multi-target, multi-Evidence, or multi-Resource batch wrapper.
- No new Acceptance Criterion waiver, retry policy, or Verification result
  semantics.
- No replacement of the lower-level Verification command family.

## Consequences

The local CLI now has a practical single-command path for a common Agent
workflow: attach bounded Evidence, persist Resource state when supplied, create a
single immutable Verification judgment, and make the Resource-backed judgment
immediately applicable when the observed fingerprint matches its baseline. This
directly advances V1 dogfood readiness instead of only making smoke scripts
denser.

The first implementation remains intentionally conservative. It composes the
existing semantic operations through runtime/Engine APIs and lets Core compute
applicability. Future work can add adapter-backed observation production or
multi-resource ergonomics only after the Resource normalization and adapter
contracts are confirmed.

## Implementation Findings

- The existing lower-level Engine operations are sufficient for the first
  wrapper without a schema change.
- Verification-time Resource observations can be captured by the wrapper because
  the current ResourceObservation model already stores adapter kind, adapter
  schema version, fingerprint, source session, summary, and optional detail
  content.
- Full atomicity across Evidence creation, ResourceObservation creation,
  Verification creation, and applicability cache persistence is not provided by
  the existing public operation boundaries. Phase 4KY reduces common partial
  failure risk with preflight checks and records full atomic wrapper persistence
  as a possible later implementation improvement rather than expanding this
  slice into a broad storage refactor.
- Independent review found that Core direct callers could otherwise pass
  non-empty cache detail without a Resource Observation and have that intent
  silently ignored. The runtime preflight now rejects that combination before
  creating Evidence, Resource Observation, Verification, or cache rows.
- The monolithic CLI parser is close to the default Rust test thread stack
  limit. Phase 4KY keeps `verify` and the already large `record` command family
  behind top-level raw argument forwarding so unrelated commands do not expand
  those large subcommand trees during every parse.
- Local dogfood confirmed the wrapper can make a Resource-backed Acceptance
  Criterion immediately `verified` and allow Task completion. It also exposed a
  recovery gap: the Task completion commit advances the branch head, so the same
  Resource-backed Verification is `stale` at the new head until applicability is
  refreshed for that commit.
