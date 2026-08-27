# Confirmed State v0.2 Provenance

This document records the promotion evidence for WorkVCS Architecture
Specification decisions 81–188. It is an audit ledger, not a parallel
normative specification. Current authority remains the Accepted ADRs and the
product, domain, and architecture documents linked from
[`docs/README.md`](../README.md).

## Source and promotion authority

The decision sequence was discussed and explicitly confirmed in the ChatGPT
conversation `WorkVCS需求沟通-v1`
(`6a847cdb-7f00-83e8-8870-a8acee174a4c`). The current user request explicitly
authorizes promoting all final confirmed conclusions from decisions 81–188,
preserving Open questions, reviewing repository consistency, and stopping
before SQLite Logical Schema design or runtime implementation.

Conversation data is supporting provenance. The exact confirmed rule is the
numbered option the user accepted, not every example or surrounding assistant
recommendation.

## Confirmation turns

| Decision range | User confirmation turn | Response |
|---|---|---|
| 81–84 | `3e66d1a4-50be-4941-b300-df7443efcba2` | `81A 82接受 83是 84是` |
| 85–88 | `67d5e0f4-7156-4a8c-ae5e-acb65b4566e3` | `85允许 86认可 87下一轮专门设计 88认可` |
| 89–96 | `071133db-f4a6-4107-b1c3-e11bd3ea9874` | `89是 ... 96认可` |
| 97–102 | `63f90d5b-99e4-4ba1-97a1-e07301098abc` | `97是 ... 102是` |
| 103–111 | `cb525af6-57cf-4bc0-9a28-40340885c81b` | `103是 ... 111是` |
| 112–120 | `720c2494-7a6c-46e1-980f-9e03ca5f86bb` | `112是 ... 120是` |
| 121–130 | `5653e8a8-da77-4efe-840e-2ee7df9b2f70` | `121是 ... 130是` |
| 131–140 | `2d4839ad-2177-4806-a641-6a6935477a5b` | `131是 ... 140是` |
| 141–152 | `ab97fe13-6ca6-40a4-8949-69ca6065d119` | `141是 ... 152是` |
| 153–169 | `52a93d6f-26b7-4bfb-940c-11dde28cd526` | `153是 ... 169是` |
| 171–188 | `804bc570-7f00-83e8-8870-a8acee174a4c` | `171是 ... 188是` |

Decision 170 was not submitted for confirmation. It identified the Knowledge
Space identity/reference question as Open; decisions 171–188 then resolved its
high-level architecture. No independent “decision 170” is promoted.

## Decision-to-authority map

### Verification, requirements, and drift

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 81 | A Verification is an immutable judgment instance; re-verification creates another instance and does not automatically supersede history. | [Verification and Resource Drift](../architecture/verification-and-resource-drift.md) |
| 85 | A validated Assumption may later become invalidated. | [Semantic Operations and State Machines](../architecture/semantic-operations-and-state-machines.md) |
| 86 | Decision supersession is explicit and does not require equal `scope+subject`; equality alone does not imply supersession. | [Semantic Operations and State Machines](../architecture/semantic-operations-and-state-machines.md) |
| 87 | The interim applicability question was intentionally deferred to a dedicated round; its final answer is decisions 89–111. | This provenance ledger; no separate normative rule |
| 88 | Coordination force cannot bypass a mandatory AC gate; a future exception must be explicit semantic provenance. | [Verification and Resource Drift](../architecture/verification-and-resource-drift.md) |
| 89 | Immutable Verification result is separate from branch-sensitive derived applicability. | [Verification and Resource Drift](../architecture/verification-and-resource-drift.md) |
| 90 | V1 applicability distinguishes `applicable`, `stale`, and `unknown`. | Same |
| 91 | Verification stores a structured Verification Basis. | Same |
| 92 | Resource Basis supports resource, path, and explicit artifact/input scope without requiring maximum precision every time. | Same |
| 93 | Applicability considers Resource drift and Work-State semantic dependencies. | Same |
| 94 | Work-State Basis includes verification WorkStateCommit and explicit semantic dependencies. | Same |
| 95 | AC effective status is `unverified`, `verified`, `failed`, `stale`, or `conflicted`. | Same |
| 96 | An AC may have zero or more required Verification Requirements; V1 has no general verification-policy DSL. | Same |
| 97 | A Verification Requirement has stable AC-local identity. | Same |
| 98 | Verification targets a Requirement when present, otherwise it may target the AC. | Same |
| 99 | A Requirement states what must be proven, not the execution command. | Same |
| 100 | A V1 Verification judgment has one target; multiple judgments may share immutable Evidence. | Same |
| 101 | V1 supports a deterministic single-target command wrapper; automatic multi-target result mapping is deferred. | Same |
| 102 | A referenced Requirement's historical identity cannot be erased by current structural edits. | Same |
| 103 | WorkVCS Core uses a Resource Adapter abstraction instead of embedding Git semantics. | Same |
| 104 | Resource has stable logical identity and an environment-specific rebindable locator. | Same |
| 105 | Resource and immutable ResourceObservation are separate. | Same |
| 106 | Resource Basis binds Resource, scope, and baseline observation/fingerprint rather than only Git SHA or one global digest. | Same |
| 107 | An Adapter provides deterministic scoped fingerprinting and difference reporting. | Same |
| 108 | Git-backed Verification represents the actually verified working state, including relevant uncommitted changes, not HEAD alone. | Same |
| 109 | Unavailable or incomparable Resource state yields `unknown`. | Same |
| 110 | Resource observation/drift alone creates no WorkStateCommit. | Same |
| 111 | Applicability combines conservatively: stale dominates unknown, and unknown dominates applicable. | Same |

### Operation contract and state machines

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 82 | SessionDiff is cross-Workspace immutable Session provenance; Handoff is 0..N Workspace+Branch-scoped Versioned Work State with optional path. | [Semantic Operations and State Machines](../architecture/semantic-operations-and-state-machines.md) |
| 83 | Engine semantic operation contracts are designed separately from final CLI spelling. | Same |
| 84 | Operation coverage is developed through complete work lifecycles before enumerating a command tree. | Same |
| 112 | Task has confirmed terminal/non-terminal states; explicit atomic `pending -> done` retains start/completion provenance. | Same |
| 113 | `failed`, `cancelled`, and `done` may explicitly reopen/retry; superseded work requires supersession-aware resolution. | Same |
| 114 | Explicit blocker is Task state; dependency blocking is derived readiness. | Same |
| 115 | Plan states are `active`, `completed`, `abandoned`, and `superseded`, with supersession-aware reopening rules. | Same |
| 116 | Goal states are `active`, `achieved`, and `abandoned`; replacement uses relation. | Same |
| 117 | Assumption transition graph forbids ordinary `invalidated -> validated`. | Same |
| 118 | A terminal Attempt is never reopened; another try creates another Attempt. | Same |
| 119 | Decision states are `active`, `superseded`, and `withdrawn`; old choices return through a new Decision. | Same |
| 120 | Operation validation separates structural, state, and coordination preconditions and returns actionable categorized errors. | Same |
| 121 | Session lifecycle is `starting -> active -> ending -> ended` with recoverable `potentially_stale`. | Same |
| 122 | Workspace/Branch switch is atomic; Workspace switch clears Focus by default and Branch switch preserves only valid entity+path Focus. | Same |
| 123 | SessionEnd atomically creates deterministic diff, releases default claims, clears Focus, and ends; Handoff is separate. | Same |
| 124 | Active Claims are none, one exclusive, or one-or-more shared; exclusive/shared cannot coexist. | Same |
| 125 | Claim mode change and takeover are explicit atomic provenance-bearing operations; force takeover requires rationale. | Same |
| 126 | Claiming does not change Task status and claim-next does not imply TaskStart. | Same |
| 127 | MergeRuntime persists `active`, `completed`, or `aborted`; resolving/ready are derived. | Same |
| 128 | One target Workspace+Branch has at most one active merge. | Same |
| 129 | Resolutions remain provisional until continue creates one atomic ChangeSet and two-parent commit. | Same |
| 130 | V1 merge freezes base/source/target heads, rejects moved inputs, and does not Branch-lock. | Same |

### Store and versioned-state persistence

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 131 | Store persistence separates logical identity, versioned state, and immutable history. | [Versioned-State Persistence Model](../architecture/persistence-model.md) |
| 132 | WorkStateCommit does not contain a full Workspace snapshot. | Same |
| 133 | A Commit has exactly one ChangeSet as its semantic-change carrier. | Same |
| 134 | Machine-applicable Change Operations and semantic Events are distinct. | Same |
| 135 | Typed Relations are first-class versioned graph data, not embedded-only Entity fields. | Same |
| 136 | Deterministic fields are structurally queryable; extensions may use a structured document. | Same |
| 137 | Immutable large objects use content-addressed object storage. | Same |
| 138 | ResourceObservation metadata is queryable in SQLite while large manifests/diffs may be objects. | Same |
| 139 | Current Runtime Coordination is stored separately from immutable provenance. | Same |
| 140 | Store is self-describing for identity, format, schema, capabilities, and object format. | Same |
| 141 | Commit + ChangeSet + Change Operations are reconstruction truth; Events are provenance. | Same |
| 142 | Change Operations form a deterministic replayable low-level patch vocabulary. | Same |
| 143 | Updates carry expected-before and after state or equivalent structural conditions. | Same |
| 144 | Merge ChangeSet is relative to primary parent=target; source is secondary parent. | Same |
| 145 | Replay follows primary-parent history and never reruns merge. | Same |
| 146 | Checkpoint is a rebuildable derived snapshot, not a commit or authority. | Same |
| 147 | Checkpoint scheduling remains implementation tuning. | Same |
| 148 | Branch creation is O(1) ref creation without copying Workspace state. | Same |
| 149 | Materialized projection is rebuildable cache, not canonical state. | Same |
| 150 | Only HOT Branches must remain materialized; others may reconstruct lazily. | Same |
| 151 | Commit records a resulting-state digest for replay/checkpoint/integrity verification; algorithm/encoding remain Open. | Same |
| 152 | Commit identity and state digest are distinct; different histories may reach the same state. | Same |
| 153 | Logical Entity identity and semantic state are separate. | Same |
| 154 | Every Entity state change creates an immutable EntityVersion. | Same |
| 155 | EntityVersion is not Branch-owned; Branches may share it. | Same |
| 156 | Relation uses stable identity plus immutable RelationVersion. | Same |
| 157 | Removing Entity/Relation from Work State does not physically delete canonical history. | Same |
| 158 | HOT current projection maps Branch Entity/Relation identity to active version. | Same |
| 159 | Current projection contains head mapping, not temporal history. | Same |
| 160 | Change Operations bind before/after EntityVersion or RelationVersion and may include field delta. | Same |
| 161 | EntityVersion carries complete canonical semantic state; typed current projection indexes deterministic hot fields. | Same |
| 162 | EntityVersion state is schema-versioned and read-time upcast does not rewrite history. | Same |
| 163 | Change Operation payload is schema-versioned. | Same |
| 164 | Canonical relation types use stable controlled identifiers; custom meaning uses labeled `related_to`. | Same |
| 165 | Explicit sibling order remains domain semantics; physical representation remains Open. | Same |
| 166 | Primary containment has at most one parent; multi-Plan reuse uses `references`. | Same |
| 167 | Containment is acyclic. | Same |
| 168 | Task dependency remains a DAG. | Same |
| 169 | A Work Graph Relation and both endpoints belong to one Workspace. | Same |

### Knowledge federation and portability

| ID | Confirmed conclusion | Normative authority |
|---:|---|---|
| 171 | Knowledge Space uses stable KnowledgeExposure identity referencing source Knowledge and one KnowledgeVersion. | [Knowledge Federation and Store Portability](../architecture/knowledge-federation-and-portability.md) |
| 172 | Exposure binding is immutable and does not follow source `latest`. | Same |
| 173 | V1 uses append-only Exposure history plus current projection and no independent Knowledge Space DAG. | Same |
| 174 | Consulting shared Knowledge does not copy it; explicit adoption creates Workspace Knowledge. | Same |
| 175 | Adopted Knowledge preserves Exposure/source-version provenance and is not auto-updated. | Same |
| 176 | Exposure may leave current availability, but history is never deleted; exact enum remains Open. | Same |
| 177 | Source drift/invalidation changes a derived warning/status and does not silently mutate Exposure semantic state. | Same |
| 178 | Knowledge Space queries distinguish current, historical, and source-stale sets. | Same |
| 179 | Copy/move/export/import/backup/restore preserve Store identity by default. | Same |
| 180 | Explicit Store fork creates new identity and source-store lineage. | Same |
| 181 | Bundle is self-contained for full/compact profiles and cannot omit data required to reconstruct and validate included canonical history. | Same |
| 182 | Bundle interchange is distinct from physical SQLite and may omit rebuildable projections/caches. | Same |
| 183 | Import preserves runtime provenance but does not resurrect active Session/Claim/Merge Runtime. | Same |
| 184 | Same-Store import distinguishes fast-forward from divergence and forbids last-write-wins. | Same |
| 185 | Object transport verifies and deduplicates by content hash. | Same |
| 186 | Resource identity is portable while locator may import unresolved and be rebound. | Same |
| 187 | Cross-Bundle Knowledge lineage remains resolved or a portable unresolved external provenance reference. | Same |
| 188 | V1 Knowledge Space is Store-local; live cross-Store federation is deferred. | Same |

## Preserved Open and deferred boundary

The following are intentionally not promoted as final implementation choices:

- the final equal-candidate tie-breaker for `next`;
- final CLI spelling, protocol encoding, and complete operation/error catalogue;
- complete SQLite schema, DDL, indexes, foreign-key strategy, and transaction
  layout;
- programming language;
- final persistent ID format, hash algorithm, canonical encoding, EntityVersion
  payload encoding, and object/archive format;
- physical sibling-order representation and checkpoint/eviction schedules;
- exact Resource path/glob normalization and non-Verification observation
  capture policy;
- exact KnowledgeExposure lifecycle/source-status enum, source-stale Context
  policy, access/security model, physical tables, exchange API, Bundle container
  and profile detail, and import recovery states;
- cross-Store live federation, remote subscriptions, and distributed sync;
- exact semantic operation for any future mandatory-AC waiver.

At the v0.2 promotion boundary, SQLite Logical Schema design and runtime
implementation were the next separate stages and were not part of that
promotion. Decisions 189–318 later closed the logical-family stage; see
[Confirmed State v0.3 Provenance](confirmed-state-v0.3.md). Logical DDL and
runtime implementation still remain separate.
