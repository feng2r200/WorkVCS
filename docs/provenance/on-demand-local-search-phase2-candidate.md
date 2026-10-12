# On-demand Local Search Phase-2 Candidate Evidence

Date: 2026-10-12
Scope: ADR-0526, local 1.2.0 source candidate and frozen local retrieval study

## Result

The local source candidate implements on-demand immutable search generations
and daily WorkVCS Skill routing without changing canonical WorkVCS state. Its
managed query either reuses a completely verified generation or publishes a
new pathname from one verified source snapshot. Damaged, stale, incompatible
and unrelated files are preserved. There is no watcher, mutable current pointer,
automatic cleanup, incremental-update claim or default Token/result/resource
limit.

The capability is locally complete at Git commit
`0f33fa306845ae234309ecfbf40c77fbb1814f15` on branch
`codex/fts-native-retrieval`. The primary and remote-tracking main commit remains
`a89327d9faae0d2e772e2dba0e846a35105dad9d`, which is the parent of the
candidate. The candidate was not merged or pushed and did not replace the
stable 1.1.0 installation.

## Source and validation identity

- Candidate version: 1.2.0.
- Candidate debug binary BLAKE3:
  `8345c2ea7c87489a94f230676a4ec990c4ec1715dbfd3396a28bc222f74f5ff5`;
  size: 69,758,008 bytes.
- Final source-focused checks: 14 search unit tests and 15 native FTS
  integration tests, plus strict workspace/all-targets Clippy, formatting,
  whitespace and version-bump validation.
- Broader pre-final regression: 177 suites, 1,020 passed, zero failed and one
  explicitly ignored scale test.
- Managed-query probes verified new generation, healthy reuse, explicit-index
  equivalence and historical scope. Registry, Store and both activation-marker
  hashes were unchanged; every returned UTF-8 range matched its source bytes.
- Independent source review closed without blocker, high, medium or low
  findings. Exact source hashes remain those in the implementation evidence.

The local implementation evidence root is
`/Users/ld/.codex/.chatgpt-projects/g-p-6a8569f19e0481918d1c7222641a0ac1/execution/workvcs-retrieval-phase2-20261012`.
It is audit evidence, not live project cognition.

## Frozen real-question study

The final study uses 50 literal questions from 49 original messages and 17
threads. Eighteen questions retain their complete original pre-query context.
The source is 318 native WorkVCS objects rendered as 484 fixed passages. The
dataset, corpus and source metadata were frozen before ranking:

- dataset SHA-256:
  `ea3cc4390ffcfcf458cbd3e383705f4a0b8801381be1b1a5a449c04335e6db47`;
- input-audit SHA-256:
  `95179251a5080938b22d4a4647f25969d93b7d7e80b6825650fbef8074f8769b`;
- corpus SHA-256:
  `5077aaca8bffa6653211162f46bd83b00a2a0cbd97d7844ac40d9ab258771de2`;
- native-object metadata SHA-256:
  `9ead936919c4f02315dea7b505267e8a0d4181b8e5d0788711d0ff46db89c723`.

Only five questions have known-positive native targets. The remaining 45 are
`no_known_native_support`: they participate in completeness, latency, Token and
memory observations but are not negative labels or accuracy misses.

| Method | known-target Hit@5 | MRR@5 |
|---|---:|---:|
| FTS | 0/5 | 0.0000 |
| E5-small | 0/5 | 0.0000 |
| FTS + E5-small | 0/5 | 0.0000 |
| E5-large | 0/5 | 0.0000 |
| FTS + E5-large | 0/5 | 0.0000 |
| full-corpus BGE reranker | 3/5 | 0.3067 |
| E5-large + BGE RRF | 1/5 | 0.0500 |
| FTS/E5-large + BGE RRF | 1/5 | 0.1000 |

Both E5 runs cover 92 literal query windows and return all 484 unique Qdrant
points for every window. BGE scores all 50 x 484 = 24,200 query/passage pairs;
the largest actual pair is 4,237 tokens under the configured 8,192-token
window. Batch size two is only a scheduling choice. No candidate, query,
passage or result was removed to meet a cost target.

The study records complete input, ranking and renderer Token observations. They
are not billing Tokens or native WorkVCS CLI end-to-end output. The independent
process peaks are 570,195,968 bytes for FTS, 1,819,295,744 for E5-small,
5,184,815,104 for E5-large and 15,256,420,352 for BGE. Sequential stage wall
time totals 10,695.718 seconds; the BGE stage accounts for 10,652.465 seconds.
Final allocated study artifacts occupy 182,464,512 bytes. These are local
measurements, not product guarantees or limits.

## Frozen result and independent review

The original formal run remains the primary research artifact:

- report SHA-256:
  `3423310f85b984d0b55211532c7da4a9ec9f4f993f36ce1e23df41f182fb2235`;
- raw results SHA-256:
  `fb96e7fa0684420c8908cb45c6fa60a8093669f9f02db90b1d137661ad9bfe8c`;
- input/result manifest SHA-256:
  `e047972646f8db1a7ff126cb743a73afbf3bc399c63875ffd0a30603652007c9`;
- post-hoc validation SHA-256:
  `f7cfa3518f3d3385304d0d09ea2fff0088b26b3fb35a14a1e146be9fe60cc2ac`;
- independent review SHA-256:
  `d80d8b68748b3b3cd7bef549e29a27ab059ba823157e6085ebaf798a7bb2e5e4`.

Independent review recalculated all 24,200 BGE scores into 318-document max
rankings, 200 fused rankings and 400 method/case metrics with zero mismatch.
It also confirmed that every metric for the 45 unknown questions remains null.
No model inference or timing was rerun for that review.

## Durable stage closeout

WorkVCS Goal `01a12f61-3bc8-720f-a47d-dedbb16e3a92` is achieved and Plan
`01a12f61-3bc8-720f-a47d-dedd72bc897a` is completed at commit
`01a13088-76e0-7071-8a7b-51c506289174`. The semantic-evaluation and local-
delivery acceptance criteria are verified. Their shared Evidence is
`01a13088-62f7-7485-9b66-717276150767`, content digest
`4870f0ed406f1d37cb4d6a41249584695fb9812ab9411f8f254256cc67066ed1`.
Exact extraction verifies that recovery report byte-for-byte; it binds the four
original stage-result hashes, dataset/corpus/model identities and source
candidate commit. The recovery report is a closeout artifact and does not
replace the original formal run hashes listed above.

## Interpretation and delivery boundary

The five known-positive questions show that full-corpus BGE reranking can
recover some labelled native evidence missed by the tested lexical/vector base
rankings. Five labels are too few to establish general accuracy, choose a
default embedding or reranker, or claim that RRF improves on independent BGE.
Historical relevance also does not prove current truth, authorization or answer
correctness.

This stage does not add native semantic search, automatic transcript import,
cross-Store federation, background services or lifecycle inference. It does
not authorize merge, Push, stable installation, release or deployment. The
stable local entrypoint remains version 1.1.0 until a separate delivery action
is explicitly authorized and verified.
