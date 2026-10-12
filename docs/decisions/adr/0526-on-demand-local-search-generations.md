# ADR-0526: On-demand Local Search Generations

Status: Accepted; local candidate implementation and validation complete
Date: 2026-10-12
Authority: The user's “推动下一阶段进度”, following the stated next-stage
priorities of on-demand index updates, routine retrieval integration, and fresh
real-question semantic/reranking evaluation in conversation
`01a12e00-5166-7223-8a8d-77bbddab089c`.

## Decision

An explicit search request may prepare its local derived index on demand.
ADR-0525's immutable publication, native source verification, full output and
current/historical isolation remain the governing contract. A healthy matching
generation is reused. Source changes or damaged generations require a new
pathname; previous and unrelated files are preserved. There is no background
watcher or automatic cleanup.

The implementation uses a registry-adjacent derived directory by default,
with an optional caller-selected directory. Its generation names are an
engineering convention derived from the complete native source basis and
search policy. They do not establish identity or correctness: every reused
generation must still pass complete native/index validation. Existing explicit
`--index` queries retain their read-only, refresh-required behavior.

Prepare and query share one verified native source snapshot and replay cache.
The final head/state check continues to reject current results if the source
advances during the operation. Explicit as-of queries remain historical.
Derived writes are reported separately from canonical writes.

The WorkVCS Skill routes evidence lookup to search when native recovery or
exact identifiers do not answer the request. Search does not redefine Recall,
ContextPacket selection, lifecycle, project ownership or permission to act.
It does not add a default token/result/resource cap.

## Local validation result and deferred choices

The 1.2.0 source candidate implements and locally validates managed immutable
generations, healthy-generation reuse, complete native readback and routine
Skill routing. The frozen 50-question local study compared lexical search, two
E5 models, two lexical/vector hybrids, full-corpus BGE reranking and two RRF
variants without imposing a Token, result, candidate, time or resource cap.

Only five questions have known-positive native targets. Full-corpus BGE placed
three of those targets in the first five results; the five lexical/vector base
methods placed none there, and each BGE/RRF combination placed one there. The
other 45 questions remain unknown rather than negative. This evidence validates
the experiment and shows a narrow reranking signal, but it is insufficient to
select or enable a semantic default.

This ADR does not authorize transcript import or expand the prior local
adoption into new merge, Push or stable-install authority. Exact local evidence
and its interpretation limits are recorded in
[the phase-2 candidate evidence](../../provenance/on-demand-local-search-phase2-candidate.md).
