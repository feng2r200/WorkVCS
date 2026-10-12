# ADR-0528: Explicit History, Federated Read, and Incremental Local Search

Status: Accepted; implementation and delivery authorized
Date: 2026-10-13
Accepted by: The user's authorization to continue all undelivered retrieval
capabilities, including the explicit confirmations “授权，并授权推动未交付能力推动”
and “确认接受，并授权推动实现”. The admitted Phase 3 Plan
01a130ee-f1ab-75bd-a127-818b90208670 fixes the source-ingestion,
cross-store-search and incremental-local-runtime acceptance boundaries.
The decision is persisted as WorkVCS Record
01a1317c-45a9-74be-852c-540035fdb39e at commit
01a1317c-4616-75ff-9ede-bfa7a629e6c6.

## Context

ADR-0525 and ADR-0526 define canonical-readback lexical search. ADR-0527 adds
explicit local semantic and full-corpus reranking with immutable Qdrant Edge
generations. Those decisions intentionally leave three practical gaps:

1. useful local conversation history is outside canonical WorkVCS state;
2. one evidence question may name several separately governed projects; and
3. rebuilding every unchanged passage vector after each source change wastes
   local inference work.

The user authorized capability completion before cost optimization and rejected
artificial Token, candidate, result, time and resource ceilings. This decision
therefore improves source coverage and repeat work while retaining complete
rankings, native identity, currentness and authorization boundaries.

## Decision

### 1. History enters only through an explicit reference archive

Add an explicit search source-import operation for caller-selected local Codex
rollout files. It extracts only message text and exact provenance. By default it
accepts user and assistant messages; --user-only is a narrower caller choice.
Developer, system, tool and non-text blocks are counted as ignored and do not
become documents.

The archive is a private, immutable, content-addressed derived directory. Its
manifest binds:

- the selected ProjectRef, Store, Workspace and Branch read identity;
- every source path, byte length and SHA-256 digest;
- source thread and message identifiers, source line and role;
- every extracted document digest and the complete ordered document basis;
- fixed labels historical_reference_only, unknown_historical and
  action_authority=none.

Import never writes a Record, Knowledge, Task, WorkStateCommit, Event, registry
binding or source rollout. It does not infer present truth, lifecycle,
ownership, acceptance or permission. search source-query searches the archive's
complete extracted document set and returns its fixed historical and
non-authoritative labels with exact provenance.

Same-basis repeat import reuses the archive. Changed source bytes create a
different basis. A source later missing or changed is reported on query without
erasing the immutable archive. Wrong project identity, unexpected archive
members, digest mismatch, symbolic links, broad permissions, interrupted
publication and publication collision fail closed.

There is no automatic conversation crawl, background import, semantic
distillation or promotion into canonical WorkVCS state.

### 2. Federation is an explicit read over verified ProjectRefs

Add separate lexical and semantic federated query commands. The caller must
provide at least two unique --source-project-ref values. Each source is
resolved independently through the active registry-v2 read route, opened
read-only and checked against the binding StoreId before retrieval.

Each source first produces its own complete, unbounded local ranking under the
existing lexical or semantic contract. Federation then assigns deterministic
one-based source-local ranks and combines them with
source-reciprocal-rank-v1, using 1 / (60 + rank). A caller --limit is
applied only after every successful source has completed its local ranking.

Every returned item retains:

- ProjectRef, Store, Workspace and Branch identity;
- current or historical scope;
- source commit and state digest;
- source-local rank and score;
- federated score;
- original native payload and readback result.

The same entity identifier in different verified sources remains two
source-qualified results. Duplicate objects within one source, a stale Store
binding or a source whose resolved ProjectRef differs from the explicit request
fails that source.

One source failure is never silently omitted. The response becomes
partial_failure, sets complete=false, and includes the requested ProjectRef,
stable error code and error text. If every source fails, the response is
failed. Federation has no cross-Store write path and cannot use one source's
result to mutate another source.

This is request-scoped evidence federation. It is not a shared Knowledge Space,
replication protocol, live cross-Store reference or distributed transaction.

### 3. Incremental semantic work uses a content-addressed vector cache

Each distinct source/model basis still produces a fresh immutable semantic
generation with its own complete catalogue and Qdrant Edge shard. No generation
is updated in place and no mutable current-generation pointer is added.

An explicit embedding cache may reuse only the expensive passage-vector
calculation. Its cache key is a domain-separated digest over:

- the complete provisioned embedding-model manifest digest; and
- the complete model input, including the versioned passage prefix and every
  UTF-8 byte of the passage window.

The binary cache record repeats the key and model digest, fixes the dimension,
stores every little-endian float and includes a digest of the complete vector
bytes. Cache roots, model directories, prefix directories, temporary files and
published vector files are private to the user. Symbolic links, non-files,
wrong identity, wrong dimension, non-finite values, digest mismatch and group
or other-user file access fail validation.

Missing entries are embedded without a count, time or resource cutoff.
Publication uses a unique private temporary file, file sync, a kernel
no-replace hard link and directory sync. Concurrent publishers accept the
single valid winner. Orphan temporary files are ignored and never selected as
cache entries.

If any selected cache entry is invalid, the builder discards all hits from that
attempt, records cache_read_invalid and embeds the complete passage catalogue.
It never mixes an untrusted partial cache with new vectors. A cache publication
failure is recorded but does not make a fully computed immutable generation
incomplete. --full-rebuild explicitly bypasses both cache reads and writes.

The generation manifest records cache policy, enabled state, read hits,
computed vectors, successful and failed writes, fallback reason and explicit
full-rebuild state. Validation requires hit plus computed counts to cover every
point and rejects inconsistent observations.

Source head and state guards remain unchanged. A generation built from an old
snapshot never returns stale current items. A changed source builds a new
generation, reusing only vectors whose exact model input is unchanged.

### 4. No resident service in this delivery

The accepted Plan permits an incremental or resident mode. Content-addressed
incremental generation satisfies the current recovery and efficiency need
without adding a daemon. This delivery adds no watcher, background model
process, port, startup item, retry loop or automatic cleanup.

A later resident process would add ownership, shutdown, upgrade, stale-process,
memory-residency and supervision decisions. It requires a separate ADR and
evidence. Absence of a daemon does not reduce the foreground command's complete
coverage or final currentness checks.

## Failure and Recovery

- Preserve canonical Stores, registries, source rollouts, prior archives,
  generations and valid cache entries.
- Retry a source archive only after resolving the reported identity, content or
  permission failure; do not rewrite the source to fit an old digest.
- Treat federated partial failure as incomplete evidence. Use the successful
  source results only with their explicit source report and do not describe the
  request as complete.
- Use a new derived destination after generation damage. A damaged cache causes
  a recorded complete recomputation; remove or repair cache data only under an
  explicit cleanup decision.
- Use --full-rebuild to prove that a generation can be produced without cache
  assistance. This is a foreground validation action, not a cleanup command.

## Rejected Alternatives

### Automatic transcript ingestion

Background discovery would silently expand the evidence corpus and could turn
untrusted history into apparent project authority.

### Copying history into canonical Records

Conversation text lacks the explicit semantic judgment, currentness and
authorization required for canonical Records. A fixed non-authoritative archive
preserves that distinction.

### One mutable global cross-project index

It would blur ProjectRef and Store identity, complicate currentness and create a
cross-project write and cleanup surface.

### Silent best-effort federation

Returning successful sources as a complete answer would hide missing evidence.
Partial failure must remain part of the result contract.

### In-place Qdrant updates

Mutable shards would mix source snapshots and weaken crash recovery, provenance
and final-currentness reasoning. Cache reuse is confined to exact passage
vectors; generation publication remains immutable and complete.

### Qdrant Server or a local semantic daemon

Neither is required for deterministic foreground incremental generation. Both
add a service lifecycle and authority surface that the current need does not
justify.

## Validation Obligations

- Repeated, changed, missing and damaged source files; ignored roles and content
  blocks; wrong identity; symbolic links; private permissions; concurrent and
  interrupted archive publication; complete provenance readback.
- Shared-Store and isolated-Store ProjectRefs; same-named branches; stale
  bindings; unknown or inaccessible sources; partial and total failure;
  duplicate objects; cross-source identifier collisions; complete local
  ranking before final projection; unchanged source Stores.
- Cache encode/decode identity, damage and finite-value checks; private
  permissions; orphan temporary files; concurrent no-replace publication;
  changed source with exact reuse counts; corrupted-cache complete fallback;
  explicit full rebuild; generation integrity and final currentness.
- Real lexical and semantic multi-Store probes plus model-offline and canonical
  byte-stability checks.
- Strict formatting, compile, test and lint checks; release provenance,
  independent review, exact CI and installed-command probes before completion.

The local implementation and real probes satisfy the pre-delivery parts of
this strategy. Exact accuracy, resource, source-archive, federation and cache
results are fixed in
[the Phase-3 validation evidence](../../provenance/local-derived-search-phase3-validation.md).
Remote CI and stable installation remain delivery evidence rather than inputs
to this architecture decision.

## Consequences

WorkVCS can search explicitly selected history, combine evidence from several
verified local projects and avoid recomputing unchanged passage vectors. Every
new surface remains derived, local, inspectable and subordinate to canonical
identity, lifecycle, currentness and authorization.

Private archives and caches consume local storage, federation can produce very
large results, and foreground semantic ranking remains CPU- and memory-heavy.
Those are measured resource observations. They do not authorize hidden
coverage limits or automatic deletion.
