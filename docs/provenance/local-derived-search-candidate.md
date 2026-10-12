# Local Derived Search Candidate Evidence

Date: 2026-10-12
Scope: ADR-0525, opt-in local 1.1.0 source candidate

## Result and Authority

The approved first stage implements native Record/Knowledge lexical search,
complete source readback, and a separate immutable SQLite FTS5 derived index.
Default queries return every matching object, its complete canonical statement,
original scope/provenance and all verified matching ranges with leading context.
Token counts are observations. The user's later instruction, “不要人为设定Token等的
限制，先实现能力后再来考虑成本等”, supersedes the provisional default cost limits.
An explicit caller `--limit N` remains a top-k projection.

See [ADR-0525](../decisions/adr/0525-local-derived-fts-search-and-canonical-readback.md),
[the architecture](../architecture/local-derived-search.md) and
[the operator guide](../operator/local-derived-search.md).
The new commands are `search refresh` and `search query`; they resolve the
existing read binding and do not activate or change it.

This is a locally validated source candidate. The measured development binary
was built from the frozen uncommitted source before any implementation commit.
Its exact hashes identify the
measured code. A later clean-commit rebuild requires its own binary provenance
and cannot inherit the measured binary's identity. Stable installation, merge,
Push, release, Qdrant deployment, embeddings, background hooks and automatic
transcript extraction remain outside this stage. Canonical ContextPacket and
Task/runnable contracts are unchanged.

## Source and Measurement Identity

- Source repository: `/Users/ld/Repositories/CLI/WorkVCS`.
- Isolated native Git worktree: `/Users/ld/.codex/worktrees/fts-native-retrieval/WorkVCS`.
- Branch: `codex/fts-native-retrieval`; verified starting/default-branch commit:
  `b35cc740234f174cc30faf6d534c5d7916808aa3`.
- Candidate version: 1.1.0; stable entry remains 1.0.1 at the starting commit.
- Measured local Cargo release-profile binary: `target/release/workvcs`, 24,949,264 bytes,
  SHA-256 `d8500e54ea6372f0d535775e93c156e1d1d63c7a459d80087ebac5cdb71f265f`.
- Machine: macOS 26.6.2, Apple M4 Max, 16 CPUs, 128 GiB RAM.
- Source freeze records 15 source files. Independent review checked their
  hashes; subsequent evidence-document changes do not expand its source scope.

Local evidence root, abbreviated **E**:
`/Users/ld/.codex/.chatgpt-projects/g-p-6a8569f19e0481918d1c7222641a0ac1/research/workvcs-fts-implementation-20261012`.
Original frozen comparison root, abbreviated **R**:
`/Users/ld/.codex/.chatgpt-projects/g-p-6a8569f19e0481918d1c7222641a0ac1/research/workvcs-retrieval-eval-20261012`.
These are local audit artifacts, not imported live cognition.

## Original Local-Model Comparison

The original valid trial used 60 literal questions from 51 messages and 19
conversations, 491 reference documents and 2,314 common passages. Its known
targets are historical final answers, not 60 native Record ground truths.
Nine message pairs yield two related questions each; the other 42 messages
yield one question each. Labels were independently frozen before retrieval.
Exact inputs include the only non-null background in case-019; null is never
converted to the word `None`.

Original methods use the same raw MiniLM 112-token/16-overlap windows.
Hybrids use fixed untuned reciprocal rank fusion, k=60. This table reproduces
`R/comparison.csv` with display rounding. Tokens cover short returned context,
not the later complete native-item JSON or model billing. Memory is research
worker peak RSS; hybrids share their model worker. Timings exclude native
integrity/readback, CLI startup/routing and answer generation.

| Method | Hit@1/3/5 of 60 | Support range@5 | Mean context tokens | P50/P95 ms | Worker peak MiB |
|---|---:|---:|---:|---:|---:|
| Original FTS5 | 29/47/51 | 32 | 735.75 | 4.33/8.41 | 187.83 |
| Multilingual MiniLM | 11/24/29 | 15 | 710.88 | 8.01/16.41 | 971.83 |
| E5-small int8 | 21/33/40 | 23 | 679.65 | 5.50/11.91 | 853.81 |
| BGE-small zh | 20/33/40 | 23 | 720.43 | 3.42/6.35 | 513.09 |
| FTS + MiniLM | 26/38/44 | 23 | 739.55 | 12.62/24.80 | 971.83 |
| FTS + E5-small | 30/44/51 | 32 | 724.02 | 9.99/20.86 | 853.81 |
| FTS + BGE-small zh | 25/46/48 | 28 | 735.62 | 8.13/15.35 | 513.09 |

E5 hybrid gained four known targets and lost four relative to FTS; the grouped
paired interval includes zero. These observations supported the approved
lexical stage and do not establish a vector-layer benefit. They impose no cost
cap on future model or Qdrant selection.

## Frozen 60-Question Implementation Regression

The final local regression harness runs the native adapter over an isolated canonical
fixture containing the same 491 reference documents. Historical answers and
reports remain references, never live Store facts. It requests five objects
for rank comparison; five is not the product default. The implementation uses
Jieba HMM=false, Chinese bigrams, ASCII/technical identifiers and fixed generic
stopwords, with continuous o200k ordinary 128-token/24-overlap windows.
The 81 lexical/token controls have zero mismatches.

| Measurement | Original FTS reference | Final native-adapter fixture |
|---|---:|---:|
| Hit@1/3/5 | 29/47/51 | 31/46/50 |
| MRR@5 | 0.636111 | 0.640000 |
| Returned-range support@5 | 32/60 | 48/60 |
| Complete-statement support@5 | Not measured | 50/60 |
| Mean observed context/item tokens | 735.75, short context | 14,207.03, complete items |
| P50/P95 query ms | 4.33/8.41, derived retrieval only | 187.84/211.72, native API |

Support means at least 50% **character** overlap with the frozen literal quote
in a returned verified match/lead range among the first five objects. A quote
elsewhere in the full statement is reported separately, without inflating the
range metric. The new output covers more labelled evidence ranges but loses
one top-five known target. Window, payload and runtime boundaries differ;
no overall ranking superiority, speedup or comparable cost reduction is
established by this table.

Native API timing includes full source and derived checks plus materialization,
but excludes CLI startup/routing and answer generation. It is one timed query
per case, without separately randomized cache state. Standalone harness peak
RSS is 175,767,552 bytes (167.63 MiB) while loading/building/querying the fixture;
total process elapsed time is 12.02 s. This scope differs from the old worker
and the live CLI probe.

Exact input composition, full text, version identity, UTF-8 ranges, complete
item JSON tokens and object uniqueness pass `E/historical60-unrestricted-audit.json`.
Raw output is `E/historical60-unrestricted-top5.json`. Explicit top-five
projection omits 15,348 matching objects across all queries, not because of
tokens or a default limit.

## Real Native Store CLI Probe

Additional labels are 17 literal Record-supported questions from 16 messages
and 11 conversations, independently frozen before retrieval. They are partial
known-target labels, not answer accuracy; unlabelled useful results are not
counted as errors. Each quote and native identity was also read back with the
stable CLI at the selected snapshot.

The candidate and fair original FTS baseline use the same 318 objects:
311 Records and seven Knowledge statements. The earlier 311-Record-only
baseline is preserved but is not the final comparison. Explicit historical
scope is required because some labelled targets were later superseded.

- ProjectRef: `01a0e2e6-f2e6-77a2-b858-7680d26d9eb0`.
- Store: `01a08b3a-022f-70d1-9ff7-3393f5f28d0d`.
- Workspace: `01a08b3a-1fc0-71f0-b3f9-fa11ce2c36dd`.
- Branch: `01a08b3a-1fc0-71f0-b3f9-fa4d9df53dcd`.
- As-of commit: `01a12969-4c45-73be-88c6-f2e0941782b8`.
- Source state digest: `897774ab1fbbef60611727351dea57d194f13e84328c386605a62f376d4fd730`.

| Measurement | Original FTS, same 318 sources | Candidate explicit top five | Candidate default complete |
|---|---:|---:|---:|
| Hit@1/3/5 of 17 | 3/4/4 | 3/4/4 | 3/4/4 |
| Returned-range support@5 | 4/17 | 4/17 | 4/17 |
| Known target at any rank | Different projection | 4/17 | 13/17 |
| Mean observed item/context tokens | 579.65, short context | 3,447.53, complete items | 88,399.76, complete items |
| Mean actual CLI stdout tokens | Not applicable | 4,668.59 | 99,078.94 |
| CLI P50/P95 elapsed ms | Not measured | 687.55/700.28 | 716.18/735.85 |
| Query-child peak RSS MiB | Different worker scope | 138.22 | 138.56 |
| Objects omitted over 17 queries | Different projection | 2,375 | 0 |

Default mode returns 27–237 objects per question. Thirteen known targets at any
rank do not imply useful ranking or that the four absent targets have no other
relevant evidence. Native top-five ranking improvement is not demonstrated.

Every CLI query starts a fresh process, three repetitions per question per mode;
filesystem caches warm after the first call. P50/P95 use each question's median
child elapsed time, including startup, routing, whole-source integrity, full
generation/FTS shadow checks, readback, token counting and stdout delivery.
RSS excludes the parent Python evaluator and Cargo. Refresh takes 722.482 ms,
peaks at 145,539,072 bytes (138.80 MiB), and yields a 942,080-byte index.
These are finite observations, not guarantees.

Throughout the probe, source-file SHA-256 remains
`2b4872c729176798ceaa2ad9c6be93d636b0314dc44178003077a25b556bf1ab`,
and the binary hash remains unchanged. Separate audits verify exact compact
complete-item JSON tokens, full original metadata and every source range.
Actual stdout includes the manifest, diagnostics and pretty formatting; it is
reported separately. No cost observations discard evidence.

Artifacts are `E/native-cli-final/native-{top5,full}.json`, their `-audit.json`
files, `E/native-cli-final/resource-measurements.json`, each child stdout/stderr/RSS file,
`E/native-baseline-complete/results.json` and `E/native-source-verification.json`.

## Correctness, Failure Discovery and Independent Review

- `cargo test --workspace`: 177 suites, 1,016 passed, zero failed, one opt-in
  scale test ignored. Includes CLI 305/305 and native FTS integration 12/12.
  Frozen files and totals are recorded in `E/source-freeze.json`,
  `E/workspace-tests-final.log` and `E/test-totals-final.json`.
- Strict workspace/all-targets Clippy, formatting, whitespace and version
  gate passed. Version increment is 1.0.1 → 1.1.0; corresponding final logs
  preserve the exact check boundary.
- Focused checks cover lifecycle/current/historical membership, full-DAG
  ancestry including merged secondary parents, cross-workspace/branch/store
  rejection, corrupted versions/digests, undelivered-state exclusion, and
  source-readonly behavior. Workspace regression covers existing context and
  task behavior.
- Controlled barriers cover head advancement during query/reuse/publication,
  different-head publishers, one derived snapshot under WAL mutation, and a
  destination inserted before kernel NOREPLACE. Mismatched/damaged/unknown
  existing files are preserved. Unsupported atomic platforms fail closed.
- Exact snippet membership and FTS shadow checks reject slice substitution and
  corrupt postings even on no-match queries. FTS checks use an isolated exact
  backup, leaving the persistent derived index read-only.
- Path protection rejects Store/registry/marker aliases and reserved Store
  journal/WAL/SHM names, including hard-link and case aliases. RAII cleanup
  removes only owned temporary files after failed publication.
- `E/review/review.md` closes all known blocker/high/medium/low source findings;
  its 15 final source hashes remain unchanged. It did not run the live probe.
- Independent final metric review in `E/final-metric-review.{json,md}` checks
  the 60 and 17 inputs, versions/digests, UTF-8 ranges, exact complete JSON
  tokens, rank prefixes, repeated timings and resource/source identity. It
  finds no blocking error and retains fixture/truth, partial-label and timing
  boundaries. JSON SHA-256:
  `bf8a19ed00b95a132fcb889fd0c4d01f8997517cecfb48b3cddcf2ab9c87a0d2`;
  readable report SHA-256:
  `c2612d8ca58408220b4a3d7aea5fcf6be6adafab00b84aa173ef01ca275e55cb`.
- Exact-checkout CodeGraph final sync is complete, extraction version 27,
  pending zero, no mismatch; this supports structure reading only.

Original bad-input and 600/80 snippet failures are retained. Initial native
support is 13/60 by character overlap, not the earlier mistaken byte-based
count of 15. Intermediate constrained trials remain in their original
directories; they are superseded by the capability-first instruction and
are not current defaults or final resource results. No failure is overwritten.

## Durable Work and Remaining Boundary

Goal `01a12e5c-c26f-7289-8d37-65fea0e53547` and Plan
`01a12e5c-c26f-7289-8d37-660027da8774` track this stage. The same Plan was
amended in place after the capability-first instruction. Its implementation
criterion requires complete output without default cost limits.

The real CLI checkpoint was admitted as Capture
`01a12edf-dbc9-7327-8226-57d7dbb88f85`, delivered to commit
`01a12edf-e1ba-70b9-a04b-7fd04800ac85`, then exactly read back as Finding
`01a12edf-e160-7097-b87a-f30caba54b1c`, digest
`5ca5511e0b6ff91c14e029fe4287861f29f757f4db1c39d6dd593e98511b6e29`.
Admission, delivery and readback remain separate evidence.

Readback verifies correspondence with stored state at an observation anchor.
It does not prove external truth, grant action authority, infer unknown legacy
ownership or keep a result fresh after its branch advances. Historical
conversation answers are not imported into live work records.

The candidate delivers complete payloads for every lexical match and verified
failure behavior.
Better native ranking, semantic retrieval and broad adoption remain distinct
future work. Resources above inform that work without capping the capability.
