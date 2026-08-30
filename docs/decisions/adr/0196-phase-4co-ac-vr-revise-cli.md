# ADR-0196: Phase 4CO Acceptance Criterion And Verification Requirement Revise CLI

- **Status:** Accepted for implementation
- **Accepted by:** current long-running WorkVCS implementation goal.

## Context

The core Engine already supports revision of Acceptance Criterion and
Verification Requirement semantic state. CLI tooling could create, show, and
list both entities, but could not yet drive the confirmed revision operations
without dropping to Rust API usage.

## Decision

1. Add `ac revise` and `vr revise` CLI commands.
2. Require branch, expected head commit, entity id, expected entity version id,
   and the next statement for both revision commands.
3. For `ac revise`, accept optional `--classification`; when omitted, preserve
   the classification visible at the expected head commit.
4. Accept optional rationale JSON through the existing canonical semantic JSON
   object parser and pass it to the existing Engine revision options.
5. Render revision results with previous/current entity version ids,
   previous/current statement JSON, state digest, and work-state digest.

## Non-Goals

- This slice does not change AC or VR revision invariants.
- This slice does not add partial JSON patching or policy DSLs.
- This slice does not alter effective verification projection semantics.
- This slice does not introduce new persistence tables.

## Consequences

- CLI users can evolve AC and VR statements through confirmed semantic
  operations.
- AC revision avoids accidental classification changes by preserving the current
  classification unless explicitly overridden.

## Implementation Findings

- None.
