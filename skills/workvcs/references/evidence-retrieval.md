# Evidence retrieval

Use search for a specific evidence question when bounded Recall or an exact
identifier is insufficient. Search is supplemental evidence lookup. It does
not determine runnable work, certify a statement as true, infer authorization,
or replace ownership discovery and recovery.

After normal stable-entry and project-ownership checks, choose the narrowest
source and ranking surface that can answer the question.

## Canonical single-project evidence

Start with the lower-cost complete lexical ranking unless the wording mismatch
requires an explicitly requested semantic mode:

~~~text
workvcs search query --cwd <verified-project> --query <question> --format json

workvcs search semantic-query --cwd <verified-project> \
  --embedding-model <verified-local-e5-directory> \
  --mode <vector|rerank|hybrid-rrf|rerank-vector-rrf|rerank-hybrid-rrf> \
  --query <question> --format json
~~~

Reranking modes require the verified local BGE directory. Ordinary semantic
query is offline and must not download. There is no implicit semantic default.
Do not represent the currently evaluated mode as broadly more accurate unless
the applicable frozen labels support that claim.

Managed lexical and semantic requests prepare or reuse immutable local derived
generations. Maintenance and derived-write fields describe that local effect;
canonical source remains read-only. For an exact historical question, add
--commit with a reachable commit in the selected workspace. Its result remains
historical even when the old object status was active.

Use --embedding-cache-dir only for an explicit private local cache. It reuses
vectors for an exact model-manifest plus complete prefixed passage input; it
does not reuse source currentness. A damaged selected cache entry causes a
recorded complete recomputation. Use --full-rebuild on a missing generation
when validation must bypass cache reads and writes.

## Explicit local history

Conversation history is outside canonical WorkVCS. Import only sources selected
for the current question:

~~~text
workvcs search source-import --cwd <verified-project> \
  --source <absolute-rollout-jsonl> --archive-dir <private-directory> \
  --format json

workvcs search source-query --cwd <verified-project> \
  --archive <published-archive> --query <question> --format json
~~~

Repeat --source for more selected files; --user-only excludes assistant text.
Archive results are always historical_reference_only with unknown historical
currentness and no action authority. Preserve their exact thread, message,
role, line, path and digest provenance. Never promote archive wording into a
current fact or permission without a separate semantic judgment and current
evidence.

## Explicit multi-project evidence

Use federation only when the question deliberately spans named projects:

~~~text
workvcs search federated-query --cwd <verified-project> \
  --source-project-ref <PROJECT_REF_ONE> \
  --source-project-ref <PROJECT_REF_TWO> \
  --query <question> --format json
~~~

Use federated-semantic-query only when the semantic capability is required.
Every source ProjectRef must be explicit and unique. Each source completes its
own ranking before the final source RRF; a caller limit applies only at the end.
Keep each hit's ProjectRef, Store, Workspace, Branch, commit, state and temporal
scope.

Check status and complete before using the response. partial_failure preserves
successful sources but proves that the requested evidence set is incomplete.
Report the failed ProjectRef and error; do not silently present the remaining
hits as a complete answer. Federation cannot authorize cross-Store writes.

## Readback and output boundary

Canonical ready results include entity/version identity, state and statement
digests, original scope/provenance, complete statement, verified ranges,
source commit/state and native readback. Use these fields for an exact follow-up
read or citation. A current head change returns no items and requires a fresh
request.

Default canonical search has no Token, object, passage, reranker-pair or result
ceiling. Semantic exact search and reranking cover the complete generation.
Use --limit N only when the caller asks for a final ranking projection, and
retain the omitted count. Large output, elapsed time and resident memory are
resource observations, not authority to discard evidence or alter ContextPacket.

Search never changes a Record or Knowledge lifecycle, Task readiness,
ProjectRef ownership, authorization or external truth. An admitted Capture that
was not delivered to the Store is absent from canonical search.
