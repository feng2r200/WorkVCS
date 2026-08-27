# Confirmed State v0.3 Provenance

Later decisions 443–513 close several items listed as Open at this historical
baseline. Their current provenance and authority are recorded in
[Confirmed State v0.4](confirmed-state-v0.4.md) and
[ADR-0006](../decisions/adr/0006-sqlite-physical-schema-v0.1.md); the Open list
below records only the state at v0.3 promotion time.

This document records promotion evidence for WorkVCS decisions 189–318:
SQLite Logical Schema Rounds 1–6 and the closing Consolidation Review. It is an
audit ledger, not a parallel normative specification. Normative authority is
[ADR-0005](../decisions/adr/0005-logical-schema-family-boundaries.md),
[Logical Schema Boundaries](../architecture/logical-schema-boundaries.md), and
the linked product/domain documents in [`docs/README.md`](../README.md).

## Source and promotion authority

The sequence was discussed and explicitly confirmed in the ChatGPT
conversation `WorkVCS需求沟通-v1`
(`6a847cdb-7f00-83e8-8870-a8acee174a4c`). The current user request explicitly
authorizes promotion of final confirmed decisions 189–318, requires the Open
boundary to remain Open, and forbids starting Logical DDL or runtime work.

Conversation data is supporting provenance. The promoted rule is the numbered
conclusion explicitly accepted by the user, not surrounding examples,
recommendations, proposed SQL spellings, or later-stage suggestions.

## Confirmation turns

| Decision range | User confirmation turn | Response |
|---|---|---|
| 189–198 | `ba0b8f79-229b-4ba5-883c-ec7fc2e40b2e` | `189是 ... 198是` |
| 199–215 | `b22f7f25-0126-4899-bd4a-1e69d4362498` | `199是 ... 215是` |
| 216–235 | `1664af5e-c02f-4387-a3d7-b7164532e889` | `216是 ... 235是` |
| 236–258 | `7899ee56-190d-494a-82b2-a67d8a0c9bea` | `236是 ... 258是` |
| 259–291 | `977e8720-e69e-415b-ac3b-949e43100433` | `259是 ... 291是` |
| 292–314 | `94203023-df0c-4c6a-aa38-254aa0659059` | `292是 ... 314是` |
| 315–318 | `f7ee9ed5-77cb-45d5-a4ab-5cb12bc751bf` | `315是 ... 318是` |

All 130 integers from 189 through 318 are represented below exactly once.

## Decision-to-authority map

### Logical families and identity/version core

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 189 | Goal, Plan, Task, Decision, Knowledge, Record, AcceptanceCriterion, VerificationRequirement, and Verification share the Entity/EntityVersion family. | [Versioned Entity family](../architecture/logical-schema-boundaries.md#versioned-entity-family) |
| 190 | AC and VR use internal Entity identity while public display identity remains scoped. | Same |
| 191 | Verification uses Entity infrastructure but its semantic judgment is immutable and normally has one version. | [Verification persistence boundary](../architecture/logical-schema-boundaries.md#verification-persistence-boundary) |
| 192 | Relation/RelationVersion is a separate versioned family, not Entity. | [Versioned Relation family](../architecture/logical-schema-boundaries.md#versioned-relation-family) |
| 193 | Store, Workspace, Branch, Commit, ChangeSet, ChangeOperation, Event, and Checkpoint are not Entity. | [Identity families](../architecture/logical-schema-boundaries.md#identity-families) |
| 194 | Session, Claim, and Merge runtime/provenance use independent families, not Entity. | [Runtime and provenance families](../architecture/logical-schema-boundaries.md#runtime-and-provenance-families) |
| 195 | Evidence, Resource, and ResourceObservation are not Entity. | [Evidence and content objects](../architecture/logical-schema-boundaries.md#evidence-and-content-objects) |
| 196 | KnowledgeSpace and KnowledgeExposure remain an independent federation family outside Workspace Entity. | [Knowledge federation families](../architecture/logical-schema-boundaries.md#knowledge-federation-families) |
| 197 | ObjectIdentity is the lower cross-family address registry; Object is not Entity. | [ObjectIdentity registry](../architecture/logical-schema-boundaries.md#objectidentity-registry) |
| 198 | Typed Relation uses ObjectIdentity endpoints with Engine-enforced kind, Workspace, and deterministic semantics. | [Versioned Relation family](../architecture/logical-schema-boundaries.md#versioned-relation-family) |
| 199 | ObjectIdentity is the Store-wide addressable registry and stores identity/kind only. | [ObjectIdentity registry](../architecture/logical-schema-boundaries.md#objectidentity-registry) |
| 200 | Object ID is unique within one Store. | Same |
| 201 | Entity and ObjectIdentity are 1:1 and share one stable ID. | [Versioned Entity family](../architecture/logical-schema-boundaries.md#versioned-entity-family) |
| 202 | Entity Workspace ownership and kind are immutable identity properties. | Same |
| 203 | EntityVersion has no Branch or Commit ownership. | Same |
| 204 | EntityVersion has no single previous-version chain; Commit transitions express evolution. | Same |
| 205 | EntityVersion contains Entity-owned semantic state, not relations, Runtime, or derived state. | Same |
| 206 | EntityVersion stores complete canonical state, not a patch. | Same |
| 207 | EntityVersion digest covers canonical semantic state only; algorithm/encoding remain Open. | Same |
| 208 | Different EntityVersion IDs may have equal state digests; semantic dedup is not required. | Same |
| 209 | Relation and ObjectIdentity are 1:1 and share one stable ID. | [Versioned Relation family](../architecture/logical-schema-boundaries.md#versioned-relation-family) |
| 210 | Relation Workspace/type/source/target are immutable identity properties. | Same |
| 211 | RelationVersion contains relation metadata state and does not repeat endpoints/type as mutable state. | Same |
| 212 | Relation presence/absence is Work-State membership, not RelationVersion lifecycle. | [Work-State membership and projections](../architecture/logical-schema-boundaries.md#work-state-membership-and-projections) |
| 213 | Entity presence/absence is Work-State membership and differs from semantic terminal state. | Same |
| 214 | Current absence does not ordinarily physically delete canonical identity/version history. | Same |
| 215 | Object kind uses a controlled vocabulary; extension registration remains later work. | [ObjectIdentity registry](../architecture/logical-schema-boundaries.md#objectidentity-registry) |

### Work-State membership and projection

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 216 | Work State is Entity-to-EntityVersion plus Relation-to-RelationVersion membership/version-selection mappings. | [Work-State membership and projections](../architecture/logical-schema-boundaries.md#work-state-membership-and-projections) |
| 217 | Canonical Change Operations change those mappings; final primitive spelling remains Open. | Same |
| 218 | Change precondition distinguishes expected absence from expected version. | Same |
| 219 | Normal Commit state is parent mapping plus ChangeSet delta. | Same |
| 220 | Merge Commit records the primary-parent-to-merged-state delta. | Same |
| 221 | Branch HEAD is the sole canonical current Work-State pointer. | Same |
| 222 | Branch Entity/Relation current mappings are materialized HEAD projections. | Same |
| 223 | Projection completeness and projected Commit are explicit; only a valid complete projection gives absence semantics. | Same |
| 224 | Ordinary HOT-Branch history, HEAD, and complete projection advance atomically. | Same |
| 225 | A non-materialized Branch need not build a projection merely to exist. | Same |
| 226 | Mutation without a projection must reconstruct a deterministic HEAD base without lowering correctness. | Same |
| 227 | Typed current projection identifies its source EntityVersion. | Same |
| 228 | Typed projections are workload-driven, not one-per-Entity-kind mirroring. | Same |
| 229 | Relation current projection may use verifiable denormalization for graph traversal. | Same |
| 230 | Relation endpoint validity is relation-type-specific. | [Versioned Relation family](../architecture/logical-schema-boundaries.md#versioned-relation-family) |
| 231 | Removing an Entity must atomically remove any Relations that would be invalid in resulting state. | [Work-State membership and projections](../architecture/logical-schema-boundaries.md#work-state-membership-and-projections) |
| 232 | Semantic terminal state does not trigger generic Relation removal. | Same |
| 233 | Branch creation does not copy a source projection. | Same |
| 234 | Fast-forward/import may invalidate projection instead of rebuilding it synchronously. | Same |
| 235 | A complete projection validates its state digest against HEAD. | Same |

### Commit, ChangeSet, ChangeOperation, Event, and Branch

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 236 | Commit belongs to Workspace, not Branch; originating Branch is provenance only. | [Commit, ChangeSet, ChangeOperation, Event, and Branch](../architecture/logical-schema-boundaries.md#commit-changeset-changeoperation-event-and-branch) |
| 237 | Parent records have explicit primary/secondary roles; V1 parent count is 0/1/2. | Same |
| 238 | Each Workspace has one zero-parent Genesis Commit for empty Work State. | Same |
| 239 | Genesis has one initialization ChangeSet and may have zero ChangeOperations. | Same |
| 240 | ChangeSet and WorkStateCommit are non-reusable 1:1 occurrences. | Same |
| 241 | Semantic operation descriptor/rationale belongs primarily to ChangeSet. | Same |
| 242 | One ChangeSet has at most one normalized transition per membership key. | Same |
| 243 | ChangeOperation names subject and before/after presence/version with optional field delta. | Same |
| 244 | Before/after versions must belong to the named logical subject. | Same |
| 245 | Correctness is set-based and does not depend on operation ordinal. | Same |
| 246 | All before conditions precede candidate-state construction and global invariant validation. | Same |
| 247 | Event and ChangeOperation do not have 1:1 semantics. | Same |
| 248 | Runtime/infrastructure Events may exist without ChangeSet or Commit. | Same |
| 249 | Events are immutable. | Same |
| 250 | Branch is stable identity plus movable HEAD; identity is not name. | Same |
| 251 | Branch HEAD movement uses expected-head compare-and-swap. | Same |
| 252 | Branch HEAD points only to a Commit in the same Workspace. | Same |
| 253 | Branch creation creates ref plus provenance and no WorkStateCommit. | Same |
| 254 | Every Branch HEAD movement records immutable old/new provenance. | Same |
| 255 | Commit retention does not depend only on current Branch reachability. | Same |
| 256 | Merge parent roles are explicit and not inferred from insertion order. | Same |
| 257 | Merge runtime writes no canonical history until successful continue atomically creates it. | Same |
| 258 | Restore is a normal single-parent Commit+ChangeSet; fast-forward import moves a ref without a new Commit. | Same |

### Runtime, provenance, content, Resource, Verification, and federation

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 259 | Session is ObjectIdentity-backed. | [Session](../architecture/logical-schema-boundaries.md#session) |
| 260 | Stable Session occurrence and mutable SessionRuntime are separate. | Same |
| 261 | Session Context Set uses separate Workspace and KnowledgeSpace membership families. | Same |
| 262 | Current Context membership has no embedded history; Events retain changes. | Same |
| 263 | Focus is structured runtime state with a verifiable path, not an opaque string. | Same |
| 264 | SessionDiff is independent immutable provenance and may be ObjectIdentity-backed. | Same |
| 265 | Each Claim ownership/mode period is an ObjectIdentity-backed occurrence. | [Claim](../architecture/logical-schema-boundaries.md#claim) |
| 266 | Claim occurrence and active ClaimRuntime are separate. | Same |
| 267 | Claim mode change ends the old occurrence and creates another. | Same |
| 268 | Active Claim set is empty, one exclusive, or one-or-more shared. | Same |
| 269 | MergeAttempt is ObjectIdentity-backed and separate from MergeRuntime. | [Merge](../architecture/logical-schema-boundaries.md#merge) |
| 270 | MergeItem and provisional resolution are structured runtime children, not one opaque blob. | Same |
| 271 | Completed/aborted MergeAttempt remains permanent provenance. | Same |
| 272 | Evidence and ObjectIdentity are 1:1. | [Evidence and content objects](../architecture/logical-schema-boundaries.md#evidence-and-content-objects) |
| 273 | Evidence is separate from ContentObject and may reference zero or more blobs. | Same |
| 274 | ContentObject may be reused by multiple Evidence objects and deduplicates by digest. | Same |
| 275 | Resource and ObjectIdentity are 1:1. | [Resource families](../architecture/logical-schema-boundaries.md#resource-families) |
| 276 | Resource kind is stable identity state; kind change creates another Resource. | Same |
| 277 | Resource locator belongs to environment-local ResourceBinding, not canonical Resource identity. | Same |
| 278 | V1 has at most one active binding per Resource per Store environment. | Same |
| 279 | ResourceObservation is ObjectIdentity-backed and immutable. | Same |
| 280 | Large ResourceObservation detail may use ContentObject storage. | Same |
| 281 | VerificationBasis is an immutable Verification-owned component, not Entity/ObjectIdentity. | [Verification persistence boundary](../architecture/logical-schema-boundaries.md#verification-persistence-boundary) |
| 282 | Resource Basis and Work-State semantic dependencies remain structurally queryable, not opaque-only. | Same |
| 283 | Verification applicability is derived/cache state, not a canonical Verification field. | Same |
| 284 | KnowledgeSpace is ObjectIdentity-backed and outside Workspace Work State. | [Knowledge federation families](../architecture/logical-schema-boundaries.md#knowledge-federation-families) |
| 285 | KnowledgeExposure is ObjectIdentity-backed. | Same |
| 286 | Exposure source Store/Workspace/Knowledge/KnowledgeVersion binding is immutable. | Same |
| 287 | Exposure lifecycle uses append-only transition plus current projection. | Same |
| 288 | Source-stale is a separate derived projection, not an Exposure transition. | Same |
| 289 | Knowledge adoption uses ordinary Knowledge Entity plus `derived_from` Exposure. | Same |
| 290 | ExternalObjectRef represents unresolved/foreign stable provenance and is not ObjectIdentity. | Same |
| 291 | ExternalObjectRef is restricted to named provenance/federation locations and cannot become a Work Graph escape hatch. | Same |

### Store, Workspace, object storage, Checkpoint, Bundle, and import

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 292 | Store has independent stable identity above its ObjectIdentity registry. | [Store and Workspace identities](../architecture/logical-schema-boundaries.md#store-and-workspace-identities) |
| 293 | StoreManifest is current Store format/schema/capability infrastructure, not Work State. | [Store, Checkpoint, Bundle, and import infrastructure](../architecture/logical-schema-boundaries.md#store-checkpoint-bundle-and-import-infrastructure) |
| 294 | Migration may update current manifest; migration history is immutable provenance. | Same |
| 295 | Workspace has an independent Store-level identity outside ObjectIdentity. | [Store and Workspace identities](../architecture/logical-schema-boundaries.md#store-and-workspace-identities) |
| 296 | Workspace identity is independent of mutable display name and path. | Same |
| 297 | Workspace-to-Resource association is many-to-many. | [Resource families](../architecture/logical-schema-boundaries.md#resource-families) |
| 298 | WorkspaceResource is infrastructure/configuration, not Branch Work State; change emits provenance, not Commit. | Same |
| 299 | WorkspaceResource may have metadata/role while role taxonomy remains Open. | Same |
| 300 | ResourceBinding is environment-local current config outside ObjectIdentity. | Same |
| 301 | Portable import does not restore the source active ResourceBinding. | Same |
| 302 | ContentObject metadata and storage backend/locator are separate. | [Evidence and content objects](../architecture/logical-schema-boundaries.md#evidence-and-content-objects) |
| 303 | ContentObject uses content digest identity, not ObjectIdentity. | Same |
| 304 | Missing/archived storage does not delete digest metadata or provenance. | Same |
| 305 | Checkpoint uses an independent typed identity outside ObjectIdentity. | [Store, Checkpoint, Bundle, and import infrastructure](../architecture/logical-schema-boundaries.md#store-checkpoint-bundle-and-import-infrastructure) |
| 306 | One Commit may have zero or more Checkpoints. | Same |
| 307 | Checkpoint corruption makes it unusable/rebuildable and does not alone prove Commit corruption. | Same |
| 308 | Bundle is a transport artifact, not automatically a long-lived Store Object/Entity. | Same |
| 309 | BundleManifest is distinct from StoreManifest. | Same |
| 310 | ImportAttempt is immutable infrastructure provenance. | Same |
| 311 | Import ingests/validates immutable candidates before atomic ref activation. | Same |
| 312 | Immutable import is idempotent only for equal content; unequal same-identity content is integrity conflict. | Same |
| 313 | Store fork creates a new Store ID and preserves internal local IDs by default under the new namespace. | [Store and Workspace identities](../architecture/logical-schema-boundaries.md#store-and-workspace-identities) |
| 314 | Portable complete identity is `(store_id, local_id)`; V1 adds no Workspace fork/clone operation. | Same |

### Consolidation closure

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 315 | Every committed ObjectIdentity has exactly one kind-matching family owner. | [ObjectIdentity registry](../architecture/logical-schema-boundaries.md#objectidentity-registry) |
| 316 | Verification result, target, Basis, Evidence set, method/state, and defining edges form an immutable judgment closure. | [Verification persistence boundary](../architecture/logical-schema-boundaries.md#verification-persistence-boundary) |
| 317 | Relation logical key reuses one identity across remove/re-add and includes an immutable discriminator. | [Versioned Relation family](../architecture/logical-schema-boundaries.md#versioned-relation-family) |
| 318 | ExternalObjectRef is restricted metadata; canonical Relation endpoints remain local and Bundle export computes required local reference closure. | [Knowledge federation families](../architecture/logical-schema-boundaries.md#knowledge-federation-families) |

## Preserved Open boundary

The following are intentionally not promoted as final choices:

- final equal-candidate stable tie-breaker for `next`;
- final persistent ID encoding or UUID scheme;
- hash algorithm and canonical serialization;
- final CLI spelling and protocol encoding;
- programming language;
- final SQLite column types and complete Logical DDL;
- concrete indexes and exact foreign-key enforcement mechanism;
- checkpoint creation/selection/retention/eviction strategy;
- exact number and shape of typed projection tables;
- Knowledge exchange/access API and authorization protocol;
- cross-Store live federation, subscriptions, and distributed synchronization;
- physical storage/transaction tuning beyond the confirmed logical authority
  and atomicity contracts.

Logical DDL Round 1, runtime implementation, merge, Push, release, and
deployment are outside this promotion.
