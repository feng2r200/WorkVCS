# ADR-0533: Snapshot-bound Historical Occurrence and Lineage

Status: Accepted; bounded implementation and local delivery authorized
Date: 2026-10-14
Accepted by: The user's direct confirmation of the bounded software update,
including independent review, main integration, ordinary push, stable
installation and daemon health verification. The user subsequently clarified
that historical recovery is not the deliverable and cancelled real historical
sample validation. Execution is tracked in WorkVCS
Plan `01a13a01-e69c-77d8-8f41-bd58a3fddf9e`; the prior recovery Plan remains
retired history.

## Context

The first confirmed batch contains 24 legacy rollout files with 68 user-text
messages (4 native message IDs and 64 missing IDs) and 8 StockLens rollout
files whose two raw CWD spellings resolve to the same verified current member
tuple. The latter files contain 28 user-text messages with native IDs. Reused
turn IDs cannot identify distinct messages, and copied fork headers cannot
prove a message's original thread.

The previous importer and automatic owner resolver read the source separately.
An end-of-import fingerprint comparison detects ordinary changes, but does not
structurally bind an intermediate metadata read to the imported body. The
single-thread/single-message-ID archive format also cannot represent missing
native IDs and all observed metadata without inventing provenance.

## Decision

### One immutable byte snapshot binds the automatic pipeline

One captured source snapshot supplies the source digest, all observed
`session_meta` records, owner-resolution inputs, role/content extraction and
message occurrence identity. Automatic reconciliation must not re-read live
metadata independently to select an owner for a different body snapshot.
Source changes and relevant owner/policy/locator drift remain explicit failures;
prior validated immutable history may be retained without adopting the changed
source. An ABA rewrite cannot substitute another owner's metadata into a
snapshot whose body and digest belong to the original source.

### All metadata must agree on a verified current owner

Raw CWD spellings may differ only when each independently passes the existing
safe native identity/alias checks and resolves to the same full
ProjectRef/Store/Workspace/Branch tuple. Any missing, unsafe, unbound, non-member,
excluded, drifted or genuinely conflicting context is quarantined before body
indexing. This does not widen locator precedence, directory prefixes, semantic
project routing, or historical ownership fallback.

### New archives distinguish native IDs from derived occurrences

Versioned new registry-verified automatic archive provenance preserves a native
message ID whenever it exists. Legacy explicit caller-mapped imports retain
the existing v1/native-ID contract. In the new format, an absent native ID
explicitly identifies a `derived_occurrence_v1`, bound
to the identity scheme, exact source snapshot digest, physical line, role and
text digest. A derived key is not a claimed native message ID. A turn ID may be
retained as lineage metadata but is not a uniqueness key, and copied messages
across forks are not globally deduplicated without proof.

All observed metadata retain their physical line, thread ID and raw CWD. The
container/source identity remains separately identifiable. Per-message origin
is not inferred from the closest header or silently collapsed to the first
header; where origin cannot be established it remains unknown.

### Automatic ownership evidence is digest-bound and non-authoritative

New automatic attribution evidence binds the exact source digest, all observed
metadata, verified complete member tuple, registry identity/revision/digest,
source-policy basis, locator/alias/native identity observations, resolution
scheme and as-of time. It is derived provenance, not an active registry locator,
historical ownership repair, ProjectRef adoption, or permission to write
canonical work state. Explicit source import keeps its caller-selected mapping
distinct from registry-verified automatic attribution.

The production publisher may issue verified automatic attribution only after
actual resolver/control-plane verification, not by accepting unchecked public
receipt fields. The public import boundary observes the registry selected by
the existing command-local configuration route, its exact activation marker,
source policy and native owner metadata before privately issuing attribution.
Explicit registry selection remains supported; activity is scoped to that
selected route rather than a new machine-global registry authority. A
caller-created offline bundle, arbitrary activation path or caller-selected
owner receipt cannot substitute for core resolution from all snapshot metadata.
The snapshot's policy root must equal the selected entry's root, and every
archived source path must be a normal component descendant of that root.
Archive readback rejects internally contradictory evidence,
including identity paths, alias-target hashes and single-hop expansion. Native
CWD, Git root/marker and linked-worktree common-directory/admin/backlink
relationships must close over the claimed owner, rather than merely name a
valid owner elsewhere in the selected registry.
Owner-private storage and a trusted local publisher are the trust boundary:
unkeyed content hashes establish internal consistency and detect accidental
tampering, not cryptographic authenticity against the same filesystem owner
re-authoring an entire archive and recomputing every digest. No signing key,
MAC, authenticated source-producer identity or new authority service is added.

### Existing v1 archives remain immutable and strictly readable

New provenance has a versioned format and extraction policy. Existing v1
archives, manifest bytes, document bytes, paths and digests are preserved and
readable without migration or republication. Readers strictly distinguish the
supported versions and reject unknown versions or contradictory provenance.
Unchanged sources may reuse their validated prior archives. Changed source
bytes select new immutable artifacts; old artifacts are never removed.

## Software delivery and authority boundary

The deliverable is the WorkVCS software update. The 32 historical sources above
are motivating cases, not a data-recovery target or a required live acceptance
sample set. Additional real historical sample validation has been cancelled by
the user. Deleted-worktree historical mappings, semantic-project mirror
routing, new ProjectRefs/Stores, malformed-source repair and broader recovery
remain outside this delivery. Unresolved ownership continues to fail closed;
the update does not adopt historical mappings or expand source policy to repair
quarantined sources.

User text remains the current global source policy; assistant opt-in and the
unconditional developer/system/tool/non-text exclusion retain ADR-0531's
contracts. Archives remain `historical_reference_only`,
`unknown_historical`, and `action_authority=none`. The daemon writes only new
derived artifacts and evidence, never original rollouts, canonical Stores,
registry bindings, source policy, or existing archives. Task-state checkpoints
written by the responsible agent through WorkVCS are separate from daemon
source ingestion.

No force push, tag/public release, production change, extra system permission,
source rewrite, automatic cleanup or work-governance-wide branch rule is
authorized by this ADR.

## Validation obligations

- Native/missing-ID mixtures preserve the native IDs and create distinct,
  deterministic derived occurrences despite reused turn IDs and copied forks.
- Same-owner aliases are accepted only with complete native identity evidence;
  cross-owner, unbound, unsafe, excluded and drifted contexts fail closed with
  no new archive/body index and visible partial failure.
- Controlled metadata replacement/restore, append, alias retarget and registry
  drift cannot mix an owner and body from different snapshot/binding bases.
- Every live source receives a final object/type/permission/digest fence before
  inventory/catalog publication, including changes after its earlier local
  check while another source is processed. Only the changed source fails; a
  validated same-member prior archive may be retained, but a newly imported
  archive is not adopted as Ready for the failed live observation.
- Immediate reuse/post-import checks use the same descriptor-safe path as the
  final fence, including raced FIFOs and leaf/intermediate symlinks. Retained
  all-source fences keep only identity/digest metadata, not every source's body
  or parsed tree; live revalidation streams bytes without reparsing JSON.
- Source capture is anchored to the verified policy-root/directory descriptor,
  follows no leaf or intermediate symlink, and binds opened-object identity to
  the authorized path. A raced leaf or parent link cannot read outside a policy
  root, even when the outside file is otherwise valid JSONL.
- All observed headers are retained; ambiguous message origin stays unknown.
- User-only filtering excludes assistant/developer/system/tool/non-text data.
- Existing v1 artifacts are byte stable and query-compatible; both supported
  formats strictly read back, and unsupported/forged provenance is rejected.
- Registry/policy and archive-member reads use bounded, no-follow,
  nonblocking descriptor observations. A raced FIFO, leaf link or parent link
  cannot block inspection or substitute another object after a path check.
- The selected Home or registry-only route derives its own exact marker;
  matching marker bytes at another route cannot authorize import. Legitimate
  explicit registries and multiple independently selected control planes
  remain supported.
- Installed runtime identity must match the clean main commit. The existing
  daemon's LaunchAgent, autostart, OS/IPC process identity and responsiveness
  must be verified, with partial failure reported accurately. No real
  historical sample import, recovery or validation probe is required or run
  for this delivery.

## Rejected alternatives

- Filling missing IDs with turn IDs or pretending derived IDs are native.
- Selecting the first/last metadata CWD without checking all contexts.
- Guessing fork message origin from header ordering.
- Recreating deleted directories or widening path-prefix routing to recover
  old owners.
- Migrating or editing existing v1 archives or original source files.
- Dropping failed sources or broadening roles to make the catalog complete.
