# ADR-0210: Phase 4DC Canonical Digest CLI

Status: Accepted

Date: 2026-08-30

## Context

Phase 1 implemented WorkVCS canonical semantic JSON, domain-separated
EntityVersion and RelationVersion digests, and raw-byte ContentObject digests.
Those capabilities were available to core callers and indirectly used by later
store workflows, but the CLI had no direct tool surface for validating or
computing those values during manual WorkVCS operation.

## Decision

1. Add top-level `workvcs canonical encode --json JSON`.
2. Add `workvcs canonical digest --domain entity-version|relation-version --json JSON`.
3. Add `workvcs canonical content-digest` for raw content bytes from text or
   hex input.
4. Reuse the core canonical parser and digest functions; do not duplicate
   canonical JSON rules in the CLI.
5. Keep these commands store-independent because they validate deterministic
   encoding and hashing rather than SQLite state.

## Non-Goals

- This slice does not add file or stream input.
- This slice does not add WorkState mapping digest CLI.
- This slice does not add import or bundle behavior.
- This slice does not change canonical encoding rules.

## Consequences

- Users can manually verify canonical JSON and digest values without writing
  Rust code.
- Domain separation can be checked directly from the CLI.
- Raw ContentObject digest computation is explicit and distinct from canonical
  semantic JSON encoding.
