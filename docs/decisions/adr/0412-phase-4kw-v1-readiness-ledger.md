# ADR-0412: Phase 4KW V1 Readiness Ledger

Status: Accepted
Date: 2026-08-31

## Context

The WorkVCS local Rust implementation has moved beyond the original
design-entry and Phase 1 boundaries. The CLI now exposes a broad V0.1 local
surface, and repository smoke checks exercise Store bootstrap, Work State
mutation, runtime coordination, Verification, Merge, Checkpoint, and Bundle
flows.

Recent implementation slices have usefully strengthened process-level smoke
checks, expectation flags, and query details. Continuing that pattern as the
default path would risk optimizing WorkVCS into a highly testable CLI kernel
without enough proof that the tool is usable for real Agent handoff and
continuation workflows.

The project needs a single current ledger that distinguishes confirmed design,
implemented behavior, smoke proof, dogfood proof, and remaining V1 Open work.

## Decision

Phase 4KW adds `docs/provenance/v1-readiness-ledger.md` as the current
implementation-readiness ledger for V1.

The ledger is not a new product model and does not supersede the accepted
product, architecture, schema, or ADR authorities. It maps the current evidence
state so future slices can be selected from concrete readiness gaps.

Subsequent V1 implementation slices should prioritize areas where the ledger
shows missing dogfood proof or unresolved V1 Open work. In particular, the next
implementation queue should favor:

- `context` profile and hard-budget behavior;
- a deterministic single-target `verify` wrapper that captures execution
  Evidence and Resource state;
- Handoff workflows beyond explicit `Record(kind=handoff)` creation;
- Claim stale takeover, transfer, and force provenance;
- install and use documentation that a local operator can follow;
- actionable error recovery paths; and
- larger Store performance validation.

Narrow smoke expectation, list/detail, count, and display-only slices remain
allowed only when they close a concrete ledger gap. They should not remain the
default implementation path merely because they are easy to test.

The ledger must explicitly preserve the V1/V2 boundary. Transcript parsing,
LLM semantic extraction, embeddings or vector search, cloud synchronization,
distributed collaboration, GUI/TUI work, and Agent orchestration remain beyond
V1.

Stale stage wording should be corrected where repository documents still imply
that no runtime implementation exists. The correction must not claim release
readiness, finalized CLI spelling, final protocol encoding, or completion of
all V1 behavior.

## Non-Goals

- This slice does not change Engine behavior, Store schema, WorkState
  semantics, Runtime Coordination, Merge behavior, Bundle behavior, smoke
  workflow behavior, command spelling, or command output.
- This slice does not implement the dogfood gaps named above.
- This slice does not promote any V2 capability into V1.

## Consequences

Future planning has a repository-native checkpoint for balancing implementation
work against usability proof. A slice that adds more script expectations or
query detail should explain which ledger row it advances.

The project can continue local implementation while acknowledging the current
state accurately: the core is runnable and smoke-proven in many areas, but V1
is not dogfood-complete or release-ready.
