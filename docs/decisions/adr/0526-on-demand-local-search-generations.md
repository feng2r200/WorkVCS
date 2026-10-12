# ADR-0526: On-demand Local Search Generations

Status: Accepted scope; local implementation and validation in progress
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

## Deferred choices

Fresh real-question evaluation determines whether a stronger local embedding
or reranker adds useful recall/ranking. This ADR does not select or enable a
semantic default, authorize transcript import, or expand the prior local
adoption into new merge, Push or stable-install authority.
