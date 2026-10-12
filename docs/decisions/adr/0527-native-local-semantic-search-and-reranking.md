# ADR-0527: Native Local Semantic Search and Full-Corpus Reranking

Status: Accepted; implementation and delivery authorized
Date: 2026-10-13
Accepted by: The current user's explicit confirmation, “确认接受，并授权推动实现”,
after review of the complete confirmation gate in conversation
`01a12e00-5166-7223-8a8d-77bbddab089c`. The acceptance is persisted as
WorkVCS decision Record `01a13117-6b1e-74bc-b94c-c1b6d10480e3` at commit
`01a13117-6b81-7499-ae45-3fcc820e75f6`.

## Context

ADR-0525 and ADR-0526 established an opt-in local lexical search with immutable
derived generations, complete native source verification, canonical readback,
explicit current/historical scope and no default Token, result, candidate,
time or resource ceiling. They explicitly deferred embeddings, vector search
and reranking.

The subsequent 50-question local study compared native FTS, two multilingual
E5 models, lexical/vector hybrids, full-corpus BGE reranking and two RRF
variants over 318 native objects and 484 passages. Only five questions have
known-positive native targets. Full-corpus BGE placed three of those targets
in the first five results. FTS, both E5 variants and both lexical/vector
hybrids placed none there; the two BGE/RRF variants each placed one. The other
45 questions remain unknown rather than negative. This is enough to justify
an explicit capability and continued evaluation. It is not enough to select a
global semantic default or claim broad answer quality.

The user has authorized continued implementation of unfinished capabilities
and has required capability delivery before cost optimization. Repository
rules still require a specifically accepted ADR before a material architecture
choice becomes confirmed.

Current local probes provide these implementation facts:

- qdrant-edge 0.8.0 compiles under the repository's Rust 1.98.1 toolchain on
  macOS arm64 and completed create, named-vector upsert, query, explicit
  flush, drop, load and repeat-query operations in process.
- A follow-up Rust 1.98.1 offline dependency probe set
  `SearchParams.exact=true`, requested all three fixture points, called
  `optimize()`, flushed, dropped, reopened and repeated the query. Both exact
  responses matched an independent brute-force cosine score and rank
  calculation for the complete fixture.
- fastembed 7.1.1 and ort 2.0.0-rc.13 compile under Rust 1.98.1.
  MultilingualE5Large and BGERerankerV2M3 are represented by the crate.
- The exact evaluated E5-large graph and BGE reranker graph completed real
  Rust inference from local files. The product query path can use fastembed
  with default features disabled and without hf-hub.
- The final FastEmbed probe used only user-defined local model loaders. Its
  dependency lock contains no hf-hub, and its macOS binary inspection showed
  only system libraries and frameworks. Final release provenance must repeat
  these checks for the release target; the probe does not substitute for that
  evidence.
- Hugging Face snapshot symlinks are not a safe product model layout for ONNX
  external data. ONNX Runtime rejects a graph whose graph and external-data
  symlinks resolve to different blob paths outside the apparent model
  directory. Materializing the hash-verified files as regular clone files in
  one immutable directory resolves the failure.

Probe artifacts are persisted as WorkVCS Evidence
01a13107-835b-706c-82cd-1817ebf72ce4 and
01a13109-85ef-7617-a025-f03b4d27f865. The evaluated result report remains
the quality boundary; dependency probes do not expand its five known-positive
labels.

Qdrant documents Edge as an in-process Rust/Python shard with explicit
create/load/query/flush lifecycle and one-open-shard-per-directory ownership:
<https://qdrant.tech/documentation/edge/> and
<https://qdrant.tech/documentation/edge/edge-api/shard-lifecycle/>.
FastEmbed documents local embedding and reranking with ONNX Runtime:
<https://docs.rs/fastembed/7.1.1/fastembed/>.

## Proposed Decision

### 1. Product boundary and activation

Add a separate, explicitly requested native semantic-search surface. Existing
lexical search query behavior remains unchanged. A semantic request must name
its model profile and ranking mode. No semantic, hybrid or reranking mode
becomes an implicit default.

Source implementation, local validation, commit, main integration, Push,
package and stable installation remain separately evidenced delivery states,
even where the user has already authorized those actions.

The current delivery Plan targets 1.3.0 after this decision is accepted. That
version is delivery provenance rather than part of the architecture decision.

### 2. In-process runtime

Use Rust qdrant-edge 0.8.0 as the dense-vector store inside a semantic
derived generation. Use Rust fastembed 7.1.1 with ort 2.0.0-rc.13 for
embedding and reranking. Product query builds disable FastEmbed default
features and enable only the required local ONNX Runtime binary feature. They
do not link hf-hub or a product model-download path.

Every Qdrant query sets `SearchParams.exact = true`, uses the manifest's fixed
named-vector field, and requests the complete manifest point count. The first
generation format uses 1024-dimensional cosine vectors with quantization
disabled. The manifest records qdrant-edge version, vector name, dimension,
distance, exact-search flag, request-limit rule, HNSW/quantization settings and
the fully resolved `EdgeConfig`. Thread and provider settings are observed
runtime configuration; they must not introduce a coverage or work cutoff.

Do not introduce Qdrant Server, Docker, a local daemon, a Python sidecar or a
canonical vector table. The shard is private to one derived generation and has
no authority over Work-State.

### 3. Initial explicit model profiles

The first embedding profile is:

- logical ID: multilingual-e5-large-fp32-v1;
- evaluated source revision:
  Qdrant/multilingual-e5-large-onnx@ac6781cd1cf88b8306a536d7c9d18a5bd57cc14b;
- graph SHA-256:
  1c09780c907c8a91a77a6ab1fd231f79e090d2907ca431223703dfebeed3d36c;
- external-data SHA-256:
  0cf1883fee81c63819a44e2ba0efa51d4043d9759685a4ebebbde97e0623d15c;
- dimension 1024, cosine distance, mean pooling and 512 model-token context;
- the exact tokenizer/config file digests are part of the provisioned model
  manifest;
- query and passage prefixes are explicit versioned profile behavior.

The first optional reranking profile is:

- logical ID: bge-reranker-v2-m3-fp32-v1;
- evaluated source revision:
  onnx-community/bge-reranker-v2-m3-ONNX@6f5ff65298512715a1e669753bc754d2bc8f367b;
- graph SHA-256:
  faae32b124a9d54afb7e89b5e9896e03c18a9552d56d1d6b273a709a83012486;
- external-data SHA-256:
  f009aa6c6cf21986fd7e0021fa66b20ccce27abc6900a57c7109c8496811bcbe;
- 8192 tokenizer-token context;
- the exact tokenizer/config file digests are part of the provisioned model
  manifest.

These are explicit supported profiles, not a global default or a quality
winner. Adding or changing a model, precision, dimension, tokenizer, prefix,
pooling rule, output key or context length creates a new profile and
generation basis.

FastEmbed's built-in `MultilingualE5Large` enum is a profile label and
compatibility reference only. It is not a download or load authority. Product
embedding loads the verified E5 files only through
`TextEmbedding::try_new_from_user_defined`. Product reranking likewise loads
the verified BGE files only through
`TextRerank::try_new_from_user_defined`; registry metadata such as an upstream
model code is descriptive and never a source location. Ordinary query must
not call a loader such as `try_new` that can perform registry/cache discovery.

### 4. Provisioning and offline query

Model acquisition is a separate explicit provision operation. It may consume a
caller-selected local Hugging Face cache or another explicit local directory.
Any later network-backed acquisition requires its own explicit provision
request; ordinary query never downloads.

Provisioning:

1. identifies the exact profile and revision;
2. copies into a newly and privately created staging directory with no-follow
   file opens, then requires every member to be a regular file with link count
   one;
3. parses every ONNX graph's external-data locations before ONNX Runtime sees
   the graph and rejects absolute locations, parent traversal, duplicate or
   undeclared references, paths that escape the staging directory, and an
   external-data file not named by the profile manifest;
4. walks the complete staging tree and rejects symlinks, hard links,
   non-regular members, unexpected files and required files that are absent;
5. verifies every loader-consumed graph, external-data, tokenizer, vocabulary,
   tokenizer-config, model-config and special-token file against the exact
   profile manifest digest;
6. materializes only those verified regular files into the new model directory
   rather than retaining external snapshot links or paths;
7. writes a complete model manifest containing the profile, file digests,
   dimensions, prefixes, pooling/output policy, tokenizer context, runtime and
   format versions;
8. reopens and performs local embedding/reranking inference canaries through
   the same user-defined loaders used by ordinary query;
9. publishes the immutable model directory with no-replace semantics.

Ordinary query accepts only a complete matching local manifest. A missing,
changed, symlinked, incompatible or unreadable model fails before inference
with exact provision/recovery guidance. It never falls through to a remote
lookup. User-defined FastEmbed loaders consume only the verified local bytes
and paths. The manifest covers the complete loader file set rather than only
the ONNX graph and external-data pair.

### 5. Complete semantic generations

A semantic generation is a separate immutable directory next to, but not
inside, the canonical Store. It contains:

- a Qdrant Edge shard;
- a passage catalogue that maps every vector point to native entity identity,
  entity version, object digest, complete statement digest, temporal scope and
  exact canonical-statement UTF-8 byte range, plus derived tokenizer window
  coordinates;
- a generation manifest;
- integrity data needed to prove complete point/catalogue correspondence.

Its manifest binds:

- semantic format and ranking-policy versions;
- ProjectRef, Store, Workspace and Branch identity;
- current or explicit historical scope;
- source commit, source state digest and object-version basis;
- complete eligible object and passage counts;
- model profile and every model/tokenizer/config digest;
- dimension, distance, prefix, pooling, output and window policy;
- qdrant-edge, fastembed, ort, Rust target and provider identity;
- the fixed vector name, exact-search parameters, request-limit rule,
  HNSW/quantization policy and resolved `EdgeConfig`;
- a digest over the complete semantic basis and an observational creation
  time that is explicitly excluded from that basis digest.

Reuse requires the whole basis and integrity checks to match. A model change,
source change or damaged generation creates a new destination. Existing
generations are preserved.

The canonical UTF-8 byte range is the only range used for native readback.
Tokenizer positions are derived coordinates tied to the exact tokenizer and
window policy; they never replace or authorize a byte range.

Each generation and staging shard has one-open ownership enforced across
threads and processes by an exclusive operating-system file lock held for the
entire `EdgeShard` lifetime. Lock contention fails closed. Recovery does not
delete or steal a lock, remove another builder's staging directory, or retry
by opening the same shard concurrently.

Build uses a uniquely and exclusively created staging directory. The builder
exports from the same verified native source snapshot used for final readback,
embeds every passage, writes every point, validates catalogue/point
correspondence, always calls `optimize()` after upserts and records its returned
result and resolved configuration, explicitly flushes, drops, reloads and
re-queries the shard, and only then publishes the complete directory with
platform no-replace semantics. Reload validation requires the complete unique
point-ID set rather than accepting a short successful response. Unsupported
atomic-directory platforms fail closed. There is no mutable current pointer,
watcher, source hook or automatic cleanup in this stage.

### 6. Full text coverage without artificial ceilings

Model context length is an architecture constraint, not a cost budget.
Statements and queries longer than a model context are split with the exact
model tokenizer into deterministic overlapping windows. All windows are
processed; none is silently truncated or discarded. Prefix and special-token
space is accounted for before constructing each window. The manifest versions
the window and overlap rules.

Normal semantic execution has:

- no default Token budget;
- no default result or object limit;
- no candidate prefetch ceiling;
- no passage-count, pair-count, time or resource cutoff.

Qdrant's required request limit is set to the manifest's complete point count,
`SearchParams.exact` is true, and the response must contain that many unique
known point IDs. Vector scoring therefore covers the entire semantic
generation. Full-corpus BGE mode evaluates every query-window and
passage-window pair. An explicit caller top-k is applied only as a final query
projection and is reported as such.

Token, elapsed-time, resident-memory, model-load and index-size values are
observations. They do not discard evidence or redefine success.

### 7. Ranking modes

The explicit first-stage ranking modes are:

1. vector: exact E5 cosine scores over all passage points;
2. rerank: BGE scores over the complete passage catalogue;
3. hybrid-rrf: deterministic reciprocal-rank fusion of complete lexical and
   vector object ranks, using a versioned k=60 policy already exercised by
   the evaluation;
4. rerank-vector-rrf: deterministic RRF of the complete BGE and vector object
   ranks;
5. rerank-hybrid-rrf: deterministic RRF of the complete BGE object rank and
   the complete hybrid-rrf object rank.

The two BGE fusion modes mirror the evaluated BGE-plus-E5 and
BGE-plus-hybrid methods. Neither narrows BGE to a vector or lexical top-k.

For vector and BGE rankings, each passage score is the maximum score across
the complete query-window/passage-window pair set. An object's component score
is the maximum of all its passage scores. Every passage range and score for a
returned object remains in output, and every range exactly equal to the object
maximum is marked as a winning range. Passage details sort by score
descending, start byte ascending, end byte ascending and point ID ascending.
Component object ranks sort by score descending and then native identity
ascending.

RRF uses one-based complete object ranks and the exact versioned formula
`sum(1 / (60 + rank))`. Fusion output sorts by fused score descending and then
native identity ascending. These pre-rank and post-fusion tie rules make the
five named modes the complete first-generation ranking schema rather than
side-by-side experimental labels. Exact formulas and field names are
versioned in the generation/output contract.

The five-positive study does not select one mode as the semantic default. The
caller must choose, and output names the chosen mode and complete basis.

### 8. Canonical authority, currentness and readback

Semantic scores create candidates only. They do not establish currentness,
truth, permission, Task runnable state, lifecycle or ownership.

The implementation reuses ADR-0525/0526 source routing, immutable native
snapshot, delivered-membership export, replay cache, current/historical rules,
complete source integrity and final head/state freshness guard. Before return,
every output object is read back from the same native snapshot and must match:

- entity and version identity;
- object and statement digest;
- eligible status and native membership;
- complete statement bytes;
- exact returned range bytes;
- ProjectRef/Store/Workspace/Branch and temporal scope.

Current head drift returns no semantic items and keeps the derived generation
as an explicit stale artifact. Historical generations never become current.
No semantic operation writes a WorkStateCommit, Event, Task, Record, Knowledge,
registry binding, marker or canonical Store row.

### 9. Failure behavior

Missing model, digest mismatch, unsupported runtime/provider, model-load
failure, wrong dimension, damaged shard, point/catalogue mismatch, incomplete
flush, reopen/query failure, source identity mismatch, source drift, native
readback mismatch and publication collision are visible failures. They
preserve prior generations and canonical source bytes.

A semantic request does not silently return lexical results as if semantic
execution succeeded. A caller may explicitly request both modes and receive
separately identified outcomes. Recovery names the exact provision, rebuild or
native-read action; it does not rewrite unrelated files.

### 10. CLI and compatibility placement

Place model provisioning and semantic query under explicit search subcommands
without changing existing lexical command behavior. Exact CLI spelling is an
implementation detail, but the public contract must expose:

- selected model/reranker profile;
- selected ranking mode;
- model and generation paths/digests;
- source and temporal anchors;
- maintenance result (reused or refreshed);
- complete counts and observational Token/resource fields;
- canonical/derived write separation;
- per-item native readback and contributing ranges;
- explicit caller limit when present.

Core owns model/generation validation, embedding, shard lifecycle, ranking and
native readback. CLI owns verified ProjectRef routing and rendering. Do not
duplicate Store/history/currentness logic in the semantic module.

## Rejected Alternatives

### Qdrant Server or Docker

This adds a daemon, port, service lifecycle, authentication boundary and
separate recovery surface without improving the local immutable-generation
authority model.

### Python/FastEmbed sidecar

This adds a second runtime, environment and subprocess protocol, and weakens
single-binary provenance and failure/readback integration.

### Vectors in the canonical SQLite Store

This turns replaceable model output into canonical schema/state and couples
model upgrades to Store migration.

### FastEmbed automatic model retrieval in ordinary query

Even a cache-first loader may attempt remote retrieval after a missing file or
race. Product query instead omits hf-hub and uses user-defined local loaders.

### A top-k vector candidate stage before BGE

The accepted capability-first boundary and the only positive reranking signal
come from full-corpus BGE. A hidden candidate cap could erase that signal and
would contradict the user's instruction.

### Semantic or hybrid default activation

Five known-positive labels cannot justify changing ordinary retrieval
semantics. Explicit activation preserves accurate evidence boundaries while
the corpus and labels grow.

## Validation Obligations

- Exact dependency/toolchain lock and local model digest verification.
- A committed Cargo lock with dependency checksums; release target triple,
  ONNX Runtime binary/archive identity and acquisition source; absence of
  hf-hub from the product dependency graph; and a clean offline build from a
  prepared dependency cache. Final package inspection must record all dynamic
  libraries for the release target.
- Model provision success, missing file, changed hash, symlink/external-data
  escape/undeclared/duplicate reference rejection, hard-link and unexpected
  file rejection, interrupted provision, publication collision and offline
  query tests. With model/cache/Hugging-Face environment variables pointed at
  denied endpoints and network access denied, an already provisioned ordinary
  query must still succeed without a network attempt.
- Complete model-token window coverage, long query/statement coverage,
  prefix/special-token accounting and no silent tokenizer truncation.
- Qdrant create/upsert/count/exact-query/flush/drop/load/query, point catalogue
  equality, unique complete result IDs, exclusive-lock contention, wrong
  dimension, damaged WAL/segment/manifest and incomplete flush. A deterministic
  fixture independently computes brute-force cosine scores from the same
  vectors and requires Qdrant exact-search scores and complete rank order to
  match within a versioned numeric tolerance.
- Current and explicit historical state, full-DAG ancestry, source-head drift,
  undelivered Capture exclusion and unchanged ContextPacket/Task behavior.
- Cross-ProjectRef/Store/Workspace/Branch rejection and exact native
  identity/version/digest/statement/range readback.
- Complete vector, full-corpus rerank and hybrid lists without default limits;
  deterministic aggregation, RRF and ties; explicit top-k projection only.
- Frozen 50-question replay, additional independently labelled positives and
  preservation of all 45 unknown labels as unknown.
- Binary provenance, package/Skill parity, source/derived byte checks and
  independent review before any completion claim.

## Consequences

The design supplies a real local semantic and reranking capability while
keeping canonical Work-State authoritative and the existing lexical surface
stable. Exact model and source basis make generations reproducible and
replaceable. Ordinary query remains offline and failure is diagnosable.

The dependency graph, binary, local model set, startup, memory and full-corpus
latency will be materially larger. Those costs must be observed and may inform
a later optimization ADR, but they do not reduce this first capability's text,
candidate or result coverage.

## Accepted Confirmation Gate

Accepting this ADR confirms all of the following together:

1. Rust in-process Qdrant Edge plus local-only Rust FastEmbed;
2. explicit E5-large and optional full-corpus BGE profiles;
3. separate hash-verified model provisioning with no ordinary-query download;
4. immutable semantic generations with complete currentness and native
   readback;
5. no semantic default and no artificial Token/result/candidate/time/resource
   ceiling.

The user accepted this complete gate on 2026-10-13 and authorized continued
implementation. Implementation and delivery evidence remain required before
claiming the capability complete or installed.
