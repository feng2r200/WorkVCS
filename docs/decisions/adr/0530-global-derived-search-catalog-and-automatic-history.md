# ADR-0530: Global Derived Search Catalog and Automatic History

Status: Accepted; implementation and local delivery authorized
Date: 2026-10-13
Accepted by: The user's integrated WorkVCS post-V1 and Token-efficiency
authorization after independent review. The execution contract is persisted in
WorkVCS Plan `01a13391-7d89-7336-9cf3-e6d199896922`.

## Context

Explicit source archives and request-scoped federation in ADR-0528 are correct
but require a caller to repeat the source set and import steps. Automatic local
discovery can make that evidence cheaper to use only if historical conversation
text remains visibly non-authoritative and cross-project retrieval never turns
into a mutable global Knowledge Space.

## Decision

### Automatic history remains an immutable reference lane

An explicitly registered rollout root may be reconciled automatically. The
registration fixes its ProjectRef mapping and role policy (`user_only` or
`user_and_assistant`). Discovery accepts only recognized rollout files below
that exact root. New/append/rewrite/replace/delete and permission changes are
classified; unsupported or truncated schema, unresolved ownership, binding
drift and unsafe paths are quarantined with exact counters and errors.

Successful import creates a new immutable source archive using ADR-0528. It
never extracts canonical Findings, Decisions, Knowledge, Tasks, currentness,
ownership, acceptance, or permission. Missing or rewritten source bytes do not
rewrite or delete prior archives.

The private reconciliation inventory retains the last successful archive
reference for each rollout source. Unchanged files revalidate that archive
without re-import. A deleted source or failed replacement keeps the prior
archive available in `historical_reference` and adds a separate failed source
observation; historical evidence remains readable while the round stays
explicitly incomplete. A failed root scan or file fingerprint cannot be used as
proof of deletion: that root's prior entries and validated archives remain in
inventory while successfully scanned roots reconcile independently.

### The global catalog is derived, immutable and lane-separated

Each catalog round is an immutable manifest over an explicit registration
revision and a fixed ordered set of ProjectRefs and source anchors. It has two
lanes:

- `canonical_current`: verified current canonical generations whose final
  native currentness guard succeeds; and
- `historical_reference`: immutable source archives and explicit historical
  canonical generations, always labelled reference-only.

The catalog contains references and source reports, not copied canonical
objects and not mutable search documents. Query verifies every referenced
manifest and source anchor, preserves ProjectRef/Store/Workspace/Branch and
temporal identity, ranks within each source, and combines complete source-local
rankings deterministically. Revoked registrations are filtered from new
rounds. A missing, stale, damaged, quarantined or inaccessible source is
reported; the round or query is `partial_failure` with `complete=false`. If no
source succeeds it is `failed`.

Readback recomputes completeness from source statuses and rejects a manifest
that claims otherwise. The newest catalog for the exact registry basis is
reused only when ordered sources and completeness are identical. If an older
matching state reappears after a different newer state, reconciliation
publishes a new immutable round with a monotonic basis-local timestamp; this
keeps `latest` truthful while avoiding a new round for repeated no-change
scans.

There is no mutable canonical global KnowledgeSpace, cross-Store semantic
write, replication protocol, implicit adoption, or authority transfer. Search
results cannot authorize action or become current facts merely by ranking.

## Immutability and cleanup

Reference generations and catalog rounds use private temporary construction,
complete manifests, content digests, no-replace publication and directory
sync. Existing archives, generations and catalog rounds are never updated in
place or automatically removed. Cleanup remains an explicit, separately
authorized operation that proves exact target identity and reference absence.

## Rejected alternatives

- A mutable global canonical KnowledgeSpace or one shared source-of-truth
  index.
- Automatic semantic distillation from conversation history.
- Silent omission of failed projects or quarantined files.
- Updating one catalog in place or deleting superseded source material.
- Treating Token reduction as a quality or completeness gate.

## Validation obligations

Validation covers new/append/rewrite/replace/delete and permission transitions;
unsupported, malformed and truncated rollout schemas; role filtering; source
mapping and binding drift; same-id cross-project collisions; fixed membership
and revision digests; current/reference lane separation; partial and total
failure; immutable no-replace publication; query equivalence with the same
explicit federation source set; forged completeness rejection; stable
no-change catalog reuse and changed-then-returned latest selection; and
byte-stable canonical Stores, registries, Exposure state and source files.
