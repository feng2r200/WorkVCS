# Local Derived Search Phase 3 Validation

Date: 2026-10-13
Scope: WorkVCS 1.3.0 local retrieval implementation before remote delivery

## Result

The accepted local-retrieval scope is implemented and locally validated. It
adds explicit native semantic ranking, full-corpus reranking, caller-selected
historical source archives, explicit multi-ProjectRef federation, and exact
passage-vector reuse across complete immutable semantic generations. None of
these paths writes canonical WorkVCS cognition.

The ordinary default remains native FTS. Full-corpus BGE reranking is the
recommended explicit mode when the caller needs higher recall and accepts its
observed latency and memory. E5-large vector ranking and the tested reciprocal-
rank fusions remain available explicit modes, but the evidence does not support
making either a default. There is no automatic transcript import, implicit
semantic query, background watcher, resident daemon, global Knowledge Space or
cross-Store mutation.

The validated clean source commit is
`eebc6cd2c575fbc4abcea35792bb04ece0218a23`. Its release binary is version
1.3.0, SHA-256
`601004aa44345e38f88adbadcd53bc865bcfb8a11ef39c26340b814d9b450687`,
and reports `build_dirty=false`. Final main, remote, CI and stable-install
identity are separate delivery evidence.

## Model comparison retained from Phase 2

The frozen Phase 2 study compared 50 literal real questions, 318 native
objects and 484 fixed passages before the native semantic implementation was
selected. Only five questions had independently known native targets; the
other 45 remain coverage-only unknowns.

| Method | known-target Hit@5 | Mean observed method time | Mean full-ranking JSON tokens |
|---|---:|---:|---:|
| FTS | 0/5 | 2.95 ms | 7,546.46 |
| E5-small INT8 | 0/5 | 33.71 ms | 13,554.44 |
| FTS + E5-small | 0/5 | 37.01 ms | 13,626.68 |
| E5-large FP32 | 0/5 | 236.13 ms | 13,554.30 |
| FTS + E5-large | 0/5 | 239.25 ms | 13,627.80 |
| full-corpus BGE reranker | 3/5 | 212,943.68 ms | 13,660.64 |
| E5-large + BGE RRF | 1/5 | 213,179.96 ms | 13,601.76 |
| FTS/E5-large + BGE RRF | 1/5 | 213,183.08 ms | 13,602.04 |

Those are research-runner component measurements rather than native CLI
end-to-end timings. Independent process peaks were 570,195,968 bytes for FTS,
1,819,295,744 for E5-small, 5,184,815,104 for E5-large and 15,256,420,352 for
the BGE stage. No Token, candidate, result, time or resource ceiling removed
evidence. Exact Phase 2 identities and limits remain in
[the Phase 2 candidate evidence](on-demand-local-search-phase2-candidate.md).

## Native final-code evaluation

Phase 3 ran two disjoint real-question sets against the same historical native
snapshot. Every semantic request returned all 318 native objects, omitted zero
objects and verified every canonical object and passage readback. The first set
retains 50 frozen questions with five known targets and 45 unknowns. The second
set contains 17 independently verified known targets from 16 original messages
and 11 threads. Exact query keys do not overlap between the sets.

| Method | Frozen 5 Hit@1 / Hit@5 | Expanded 17 Hit@1 / Hit@5 | Expanded MRR |
|---|---:|---:|---:|
| Native FTS | 0/5 / 0/5 | 3/17 / 4/17 | 0.2260 |
| E5-large vector | 0/5 / 0/5 | 0/17 / 2/17 | 0.0497 |
| Full-corpus BGE rerank | 1/5 / 3/5 | 4/17 / 9/17 | 0.4143 |
| E5-large + FTS RRF | 0/5 / 0/5 | 2/17 / 2/17 | 0.1468 |
| BGE + E5-large RRF | 0/5 / 0/5 | 0/17 / 1/17 | 0.0929 |
| BGE + dense/FTS RRF | 0/5 / 0/5 | 3/17 / 4/17 | 0.2398 |

An independent verifier reread the raw semantic and FTS JSON, recomputed all
six rankings and target ranks, recalculated aggregate accuracy, parsed external
time/RSS evidence, and checked complete native readback. Both sets passed with
no mismatch. Unknown questions were never counted as misses.

The 50-question native semantic run consumed 6,864,419 ms in total, with
49,883 ms P50, 597,150 ms P95 and 8,626,257,920 bytes maximum RSS. It observed
32,943 query-model tokens, 36,819 passage-model tokens per generation,
13,928,316 reranker-pair tokens and 31,164 complete query-window/passage pairs.
Native FTS consumed 40,635 ms total, 803 ms P50, 873 ms P95 and 161,464,320
bytes maximum RSS, while serializing 5,362,476 complete result tokens.

The 17-positive native semantic run consumed 512,090 ms total, with 28,638 ms
P50, 45,512 ms P95 and 7,123,533,824 bytes maximum RSS. FTS consumed 13,545 ms
total and at most 158,482,432 bytes. Model Token counts are local tokenizer
observations, not billing Tokens or product limits.

The original 50-question binary is preserved by SHA-256. The final clean
1.3.0 binary reran representative case `phase2-029` and matched all 318 object
orders, vector ranks, reranker ranks, lexical ranks and scores exactly; maximum
score delta was zero. The Store bytes, size and mtime were unchanged.

## Historical source boundary

The real source-archive probe imported one explicitly selected 563,233-byte
Codex rollout. It retained seven user/assistant text messages, excluded three
developer messages and all non-text fields, reused the same archive on repeat,
and returned four matches from seven candidates with zero omission. The
archive and every result remain `historical_reference_only`,
`unknown_historical`, and `action_authority=none`. The source bytes are not
copied and no text is promoted to a Record or Knowledge object.

Tests also cover changed and missing source files, forged document identity,
wrong project identity, corruption, symlinks, exposed permissions, invalid
input, orphan staging and concurrent publication. Private directories are
0700 and files are 0600. The canonical Store hash, size and mtime were unchanged.

## Federated read boundary

The real lexical probe explicitly selected the WorkVCS and HXAP ProjectRefs.
Both sources succeeded: 585 lexical candidates produced 492 matching complete
objects and 364,793 serialized item tokens with zero omission. A separate
unknown-ProjectRef probe returned visible `partial_failure` while preserving
the valid source; it did not fall back to another binding.

The final semantic probe selected those same two ProjectRefs and returned all
2,469 ranked objects with zero omission and 1,820,392 serialized item tokens.
It took 333,600 ms and reached 8,040,398,848 bytes maximum RSS. Every source and
item retained ProjectRef, Store, Workspace, Branch, source commit, state and
temporal scope, and passed native readback. Both Store hashes, sizes and mtimes
were identical before and after the request.

## Incremental generation boundary

The accepted incremental form is a private content-addressed cache of exact
passage vectors plus a new complete immutable Qdrant generation for every
distinct source/model basis. It is not a mutable partial generation or daemon.

| Probe | Points | Cache hits | Computed | Observed wall |
|---|---:|---:|---:|---:|
| Historical cold build | 318 | 0 | 318 | 26,754 ms |
| Changed current source | 328 | 272 | 56 | 8,812 ms |
| Explicit full rebuild | 328 | 0 | 328 | 26,135 ms |
| Corrupt-cache fallback | 318 | 0 | 318 | 22,681 ms |
| Existing generation reuse | 318 | n/a | 0 | 3,099 ms |

The changed source reused only exact model-and-prefixed-passage matches. The
full rebuild read and wrote no cache entries. Damage triggered the recorded
`cache_read_invalid` complete rebuild rather than mixing trusted and untrusted
vectors. Existing-generation reuse wrote no derived file and did not create
the unused cache path. Every generation was complete and readback-verified;
the Store was unchanged.

## Evidence identity

The local evidence root is
`/Users/ld/.codex/.chatgpt-projects/g-p-6a8569f19e0481918d1c7222641a0ac1/execution/workvcs-retrieval-phase3-20261013`.
Its consolidated validation summary SHA-256 is
`feba396343f6aec96adfbd24a00bb9660c522c7e46c0aeb7a84c7f1a48db1102`.
Primary artifact hashes are:

- frozen-50 result and independent verification:
  `ad61d574da697a0a2b7aa9303cdace6d5aa9816c71ea344872ae65a2ca74acd3`
  and `274d3e04a09ec3c16c643119749dbd5715c6bf0295a662f3851cc8658165a484`;
- expanded-17 result and independent verification:
  `b60a9ee86b168449901e51748a4fb8323bfea571d6c441888f344a59e232661e`
  and `482408d0e78ba3a352c69f1d946897c3278bf89a6d3d918663a51e6769bf7dd9`;
- final-binary equivalence:
  `095a992e41b347e70c094d4d717ca8a2c1b67eefffb605afca0f4f6bfb110b30`;
- source archive, federated lexical, federated semantic and incremental probes:
  `3e87379ee38c4e2cf422602900d7c0ff9e35e31e5f70d604bdd720687748de6c`,
  `5847a2f7ee62620979c6adde875fba50325bd8f87159362e36cefd535f33f9a4`,
  `eec1da55e606c577540af631e47831079ce8676fa4fbcd3dacf48b7629b06896`
  and `47c5019927f3334415b6160aed06405b156910314b79870a12c31e2f038815ba`.

WorkVCS persists the semantic evaluation as Record
`01a131b7-5492-7084-95a7-5aea9da0fde1` and the post-ADR capability validation
as Record `01a131b7-5492-7084-95a7-5aec0b9f5a35`, both at commit
`01a131b7-54fc-7568-aa89-11ddc738b640`.

This evidence proves local implementation and bounded acceptance. It does not
by itself prove final main ancestry, remote Push, exact CI or stable installed
identity; those states require their own current receipts.
