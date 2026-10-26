# ADR-0536: Compressed Rollout Source Identity

Status: Accepted; bounded implementation authorized
Date: 2026-10-26
Accepted by: The user's confirmation of the WorkVCS and Work Governance
optimization plan, including compressed Codex rollout discovery and correct
classification of archived, moved, deleted and damaged sources.

## Context

Codex may retain a rollout as either a plain `.jsonl` file or a zstd-compressed
`.jsonl.zst` file, and may move it below another directory while preserving the
same JSONL content. Treating only the on-disk path and encoded bytes as source
identity makes a compression or archival move look like an unrelated new file
plus a deletion. Treating encoded bytes as JSONL also prevents historical
reconciliation from reading a valid compressed rollout.

The archive contract must continue to preserve immutable, source-qualified
message occurrences. A copied rollout at a second live path remains a distinct
container; recognizing a move must not globally deduplicate physical
occurrences or weaken the existing owner and policy checks.

## Decision

### Discover and decode exact rollout formats

Rollout discovery accepts only `.jsonl` and `.jsonl.zst` regular files below an
authorized rollout root. Symbolic-link, ownership, permission and descriptor
fences remain unchanged. Zstd decoding uses the maintained `zstd` Rust crate;
WorkVCS does not shell out to another process or implement a decoder.

Malformed or truncated zstd data fails at source capture before JSON parsing or
archive publication. Reconciliation reports that exact source failure while
continuing independent valid sources. It never rewrites, repairs or deletes the
input file.

Source capture enforces explicit 256 MiB limits independently on the encoded
container and decompressed logical JSONL. The encoded length is checked from the
opened file descriptor before allocating for its contents, and the maintained
zstd streaming decoder caps its back-reference window at the same 256 MiB
boundary and stops once the logical limit is exceeded. An over-limit source is
reported with its path, representation and applicable byte limit; other sources
continue, and a previously valid archive for the same path remains available as
historical evidence.

### Separate logical content from stored-container identity

`source_sha256` and `source_bytes` identify the decompressed JSONL bytes for
both formats. A compressed source additionally records:

- compression algorithm `zstd`;
- encoded container byte count; and
- encoded container SHA-256.

The archive manifest digest binds both identities. Message and attribution
provenance use the logical snapshot digest, while the live publication fence
checks the exact encoded file object captured from disk. Archive readback
rejects a compressed suffix without matching storage provenance, a plain
suffix with compressed provenance, an unknown digest, or contradictory
compression metadata.

Recompressing the same logical JSONL at the same path remains logically
unchanged for append/rewrite classification, but it is not eligible to reuse an
archive whose encoded-container identity differs. Reconciliation publishes a
new immutable archive basis carrying the current encoded digest and byte count.

Existing manifests without compressed storage provenance remain byte-stable
and readable. No migration or republishing of an existing immutable archive is
performed.

### Recognize a unique relocation without collapsing containers

During one reconciliation round, a previously known path that disappeared is
classified as relocated only when the complete successful scan contains
exactly one missing old path and exactly one newly observed path in the same
source-policy, verified ProjectRef-member and logical-fingerprint group.
For an explicit legacy registration, this comparison spans all successfully
scanned rollout roots owned by that registration, so a move from an active
session root into a separate archive root is still one relocation. A root that
could not be scanned is excluded because absence was not established there.
The old path is removed from the current inventory and is not reported as
deleted. The new path is imported as a new source-qualified container, so its
path-bound occurrence identities and immutable archive remain explicit.

If either side of the candidate group is ambiguous, including one old path to
two new paths or two old paths to one new path, WorkVCS does not infer a move.
The same applies when logical content differs, the source resolves to a
different member, or the new source cannot be decoded and verified. Every
missing old source remains a deletion observation and every new or failed
source remains its own observation. A previously validated archive is retained
when it can be associated with the same path or a uniquely relocated source, so
a failed refresh does not erase successful historical evidence.

## Validation obligations

- Plain and compressed copies of one rollout have equal logical byte counts,
  logical SHA-256 values and parsed messages.
- A compressed archive manifest records and readbacks both the decompressed
  identity and encoded-container identity.
- Same-path recompression preserves logical identity while replacing the
  remembered archive with one bound to the new encoded-container identity.
- Damaged zstd input fails capture and explicit import without publishing an
  archive; other discovered sources in the round still publish successfully.
- Encoded and logical byte limits fail before unbounded allocation, name the
  exact source and exceeded representation, preserve a same-path prior archive,
  and do not suppress independent successful sources.
- A uniquely relocated same-member rollout is not counted or catalogued as a
  deletion, including across two registered roots, and the current inventory
  contains only its new path.
- One-to-many and many-to-one logical matches remain separate new and deletion
  observations in both registry-derived and legacy reconciliation.
- Byte-identical files that remain simultaneously present at different paths
  continue to produce distinct source containers and occurrence identities.
- Existing uncompressed v1 and automatic v2 archives remain readable without
  mutation.

## Rejected alternatives

- Treating compressed bytes as JSONL or using the encoded digest as message
  content identity.
- Invoking an external `zstd` executable from the runtime.
- Considering every matching digest a move without a unique path and verified
  member match.
- Keeping a relocated old path as a permanent deletion tombstone.
- Repairing corrupted compressed input or hiding its failure behind a retained
  archive.
