# ADR-0008: Implementation Contract and Phase 1 Canonical Core

- **Status:** Accepted
- **Accepted by:** explicit user confirmations for IC-01 through IC-10,
  CE-01 through CE-14, SE-01 through SE-15, and the current Work request to
  begin implementation.
- **Confirmation turns:** `cdfc2a94-a235-429d-b4eb-856666c31e57`,
  `1c34308a-69fd-4a2b-a553-b2a6409c74df`, `a35518d7-b7ba-475a-90b6-52fd1fce16fb`,
  and `2a5c4fa5-7e8c-4178-91b8-d3a8ec46edb7`.

## Context

ADR-0007 closed the executable SQLite schema assembly and left implementation
language, canonical JSON profile, module boundary, and first vertical-slice
coding scope outside the schema contract. The follow-up implementation
discussion closed those remaining development-entry decisions as IC, CE, and
SE contracts.

## Decision

1. WorkVCS implementation begins with a Rust 2024 workspace containing exactly
   `crates/workvcs-core` and `crates/workvcs-cli`.
2. Phase 1 implements typed UUIDv7 IDs, BLAKE3-256 digests, canonical semantic
   JSON, strict validation, WorkVCS JCS Profile v1 encoding, domain-separated
   version/state hashing, and raw-byte ContentObject digest.
3. `workvcs-core` is the authority boundary for canonical logic. `workvcs-cli`
   remains a thin compiling shell and does not implement business CLI behavior
   in Phase 1.
4. WorkVCS conformance vectors are the authority for canonical encoding
   behavior. External serializer helpers may be used only behind the canonical
   module boundary.
5. SQLite Store/bootstrap/Genesis and upper-domain features are out of Phase 1.

## Consequences

- Implementation Ready = YES.
- Development prerequisites = 0.
- Phase 2 may start only after Phase 1 canonical tests pass and no unresolved
  implementation finding blocks Store bootstrap.
- Any conflict discovered while implementing the frozen contract must be
  recorded as an implementation finding rather than resolved through silent
  architecture expansion.

## Implementation findings

- IF-0001: CE-13 defines WorkState as Entity and Relation mappings, so duplicate
  subject keys must be rejected at the WorkState construction boundary instead
  of being hashed as repeated entries. Phase 1 resolves this by keeping
  WorkState fields private and validating duplicate EntityId/RelationId keys in
  `WorkState::new`.
- IF-0002: Independent review found that the first typed ID implementation
  accepted non-v7 UUID values through public constructors and parsers. Phase 1
  resolves this by validating UUID version 7 in `from_uuid`, `from_bytes`, and
  canonical text parsing, and by adding positive and negative conformance tests.
