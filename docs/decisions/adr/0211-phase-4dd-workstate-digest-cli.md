# ADR-0211: Phase 4DD WorkState Digest CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 1 defined the CE-13 WorkState mapping digest as an order-independent
mapping over EntityVersion and RelationVersion ids. Core already exposed
`WorkState` validation and `work_state_mapping_digest`, but operators still had
no direct CLI command for computing the digest from explicit mappings.

## Decision

1. Add `workvcs canonical work-state-digest`.
2. Accept repeated `--entity ENTITY_ID=ENTITY_VERSION_ID` mappings.
3. Accept repeated `--relation RELATION_ID=RELATION_VERSION_ID` mappings.
4. Parse all ids through the strongly typed canonical UUIDv7 parsers.
5. Reuse core `WorkState::new` and `work_state_mapping_digest` so duplicate
   subjects and digest ordering rules remain authoritative in core.
6. Keep the command store-independent; it computes canonical mapping digests
   from explicit inputs and does not inspect SQLite state.

## Non-Goals

- This slice does not add file or stream input.
- This slice does not add Store-backed WorkState extraction.
- This slice does not change CE-13 digest encoding.
- This slice does not add Replay, checkpoint, bundle, or import behavior.

## Consequences

- Users can compute and compare WorkState mapping digests from CLI inputs.
- CE-13 insertion-order independence is now covered at the CLI surface.
- Duplicate WorkState subjects fail through existing core validation.
