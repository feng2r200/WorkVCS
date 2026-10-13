# Local Derived Search

This opt-in local surface is governed by ADR-0525 through ADR-0530. It searches
delivered Record and Knowledge statements from verified read bindings, and may
also search an explicitly imported non-authoritative local history archive. It
produces source evidence; existing `context`, `recall`, Task readiness,
exact Record filters and mutation authorization retain their own contracts.

Stable adoption and each later upgrade have separate merge, Push, CI and
installation evidence. Use `workvcs runtime status --require-stable` to
inspect the actual installed path, commit and binary digest; version text alone
does not identify a build.

## Query with the Stable Entrypoint

Ordinary work uses `/Users/ld/.local/bin/workvcs`. Use an existing verified
project binding and a new derived-file destination:

```sh
workvcs search refresh \
  --cwd /path/to/verified/project \
  --index /path/to/new-derived-search.sqlite \
  --format json

workvcs search query \
  --cwd /path/to/verified/project \
  --index /path/to/new-derived-search.sqlite \
  --query 'historical evidence or technical identifier' \
  --format json
```

Only WorkVCS development validation may use a source candidate from its own
checkout, for example after `cargo build -p workvcs-cli`. That candidate is a
separate runtime identity and does not replace the stable entrypoint for
ordinary project work.

## On-demand Lexical Generations

ADR-0526 queries can omit `--index`:

```sh
workvcs search query --cwd /path/to/verified/project \
  --query 'evidence question' --format json
```

This prepares or reuses a derived generation in the verified registry's
`<registry-file>.d/search-indexes` directory. `--index-dir /absolute/directory`
selects another local derived directory; directory components must be actual
directories, without symlinks. `--index` selects the existing read-only mode
and cannot be combined with `--index-dir`.

`index_maintenance` reports `refreshed` or `reused`, while
`derived_index_written` reports the local write effect. Canonical source,
registry and routing marker remain read-only. A new source head, policy or
historical scope selects another complete basis. Damaged or incompatible
generations are preserved and a new file is built. The lexical path has no
incremental object-update claim. The on-demand command itself has no resident
dependency, mutable pointer, or automatic cleanup. ADR-0529 adds a separate
disabled-by-default local coordinator described below; it publishes only new
immutable derived artifacts.

Prepare and query use one native snapshot with the existing final freshness
guard. Head advancement during verification yields no current items; repeat
the request against a fresh source snapshot rather than trusting old hits.
The source Skill routes evidence questions to this supplemental read and keeps
the installed-runtime provenance boundary explicit.
If a complete generation was published before the head advanced, the
`refresh_required` response reports that generation and
`derived_index_written=true`, with no stale items. Publication/storage errors
may also leave a generation named in the error; do not assume that any failed
command had no local derived effect.

The CLI resolves the normal read-only ProjectRef routing and exact existing
Store/Workspace/Branch binding. An optional `--registry` is a one-command
locator override, not authority to create a binding or activate a marker.
An unbound or unsafe route remains a routing error.

The final local implementation has two independently verified real-question
sets: 50 frozen questions with five known-positive targets and 45 coverage-only
unknowns, plus 17 additional known-positive questions. Full-corpus BGE rerank
reaches Hit@5 3/5 and 9/17; native FTS reaches 0/5 and 4/17; E5-large vector
ranking reaches 0/5 and 2/17. Tested fusion does not improve on independent
reranking. See [the Phase-3 validation](../provenance/local-derived-search-phase3-validation.md)
for exact accuracy, Token, latency, memory, completeness and artifact evidence.
Native FTS remains the ordinary default. Use explicit `rerank` for high-value
recall when its observed local cost is acceptable; no semantic mode is implicit.

Current mode captures the selected branch's native eligible cognition. Record
status remains active/running/unverified/validated as applicable, and Knowledge
must be active; a returned uncertainty/validation status is not upgraded. To search
an explicit historical snapshot, pass the same `--commit COMMIT_ID` to refresh
and query. The commit must be in the selected workspace and reachable from
its branch. The result remains labelled historical even when a statement was
active at that commit. Use separate index paths for independently retained
current and historical generations.

## Explicit Semantic and Reranking Queries

Provision the pinned E5 and optional BGE models separately, then name the local
model directories and one explicit ranking mode:

~~~sh
workvcs search semantic-query \
  --cwd /path/to/verified/project \
  --embedding-model /private/models/multilingual-e5-large-fp32-v1 \
  --reranker-model /private/models/bge-reranker-v2-m3-fp32-v1 \
  --mode rerank-hybrid-rrf \
  --query 'evidence question' \
  --format json
~~~

Supported modes are `vector`, `rerank`, `hybrid-rrf`,
`rerank-vector-rrf` and `rerank-hybrid-rrf`. Modes that use reranking
require `--reranker-model`; hybrid modes also prepare or reuse a lexical
generation. There is no implicit semantic mode. Ordinary query never downloads
a model and fails before inference if the provisioned model manifest or any
consumed file is missing, changed, linked or incompatible.

The semantic result covers the complete eligible object and passage catalogue.
It reports model, source and generation identity, exact ranking policy,
generation maintenance, Token and resident-memory observations, and complete
native readback. `--limit N` is applied only to final object delivery and
reports the omitted count.

Use `--embedding-cache-dir /absolute/private/path` to reuse exact unchanged
passage vectors across new immutable source generations. The cache never
replaces the new catalogue, Qdrant shard or final source-currentness check.
Use `--full-rebuild` on a missing generation to bypass cache reads and writes.
The manifest reports hits, computed vectors, writes, write failures and any
complete-rebuild fallback. One damaged selected cache entry causes a recorded
full passage recomputation; no partial untrusted cache is mixed into a result.

## Explicit Historical Source Archives

Import only rollout files the caller selected:

~~~sh
workvcs search source-import \
  --cwd /path/to/verified/project \
  --source /absolute/path/to/rollout.jsonl \
  --archive-dir /absolute/private/source-archives \
  --format json

workvcs search source-query \
  --cwd /path/to/verified/project \
  --archive /absolute/private/source-archives/<basis> \
  --query 'what was discussed' \
  --format json
~~~

Repeat `--source` for more explicitly selected rollouts. Import includes user
and assistant text by default; `--user-only` narrows it. Every result remains
`historical_reference_only`, has `currentness=unknown_historical`, and has
no action authority. Exact thread, message, role, line, file and content-digest
provenance is returned. No source text becomes a canonical Record or Knowledge
object.

Same source bytes and identity reuse the archive. Changed bytes choose a new
basis. If an original source later changes or disappears, query reports that
state while retaining immutable archive readback. Preserve damaged or
incompatible archives and diagnose the reported identity, digest or permission
failure.

## Explicit Multi-Project Federation

Select every source ProjectRef explicitly:

~~~sh
workvcs search federated-query \
  --cwd /path/to/one/verified/project \
  --source-project-ref <PROJECT_REF_ONE> \
  --source-project-ref <PROJECT_REF_TWO> \
  --query 'cross-project evidence' \
  --format json

workvcs search federated-semantic-query \
  --cwd /path/to/one/verified/project \
  --source-project-ref <PROJECT_REF_ONE> \
  --source-project-ref <PROJECT_REF_TWO> \
  --embedding-model /private/models/multilingual-e5-large-fp32-v1 \
  --mode vector \
  --query 'cross-project evidence' \
  --format json
~~~

At least two unique ProjectRefs are required. Each source is independently
routed, opened and StoreId-checked, then completes its local ranking before
source-level reciprocal-rank fusion. `--limit` applies only after federation.
Semantic federation also accepts the reranker, cache and full-rebuild options
from ordinary semantic query; cache directories are source-qualified.

Inspect `status` and `complete`. A missing, stale or inaccessible source is
reported with its requested ProjectRef and stable error code. Successful
sources remain visible, but the overall result is `partial_failure` and
`complete=false`; do not present it as complete evidence. Every hit retains
its own ProjectRef, Store, Workspace, Branch, source commit, state and temporal
scope. The command never writes across Stores.

## Optional per-user automation and global derived catalog

Automation is explicit and disabled by default. Installing WorkVCS does not
install or enable a LaunchAgent, login item, service unit, or privileged
component. Registration is a separate compare-and-swap operation against an
exact derived registry basis:

```sh
workvcs automation register-preview \
  --cwd /path/to/verified/project \
  --project-ref <PROJECT_REF> \
  --rollout-root /absolute/private/rollout-root \
  --format json

workvcs automation register-apply \
  --cwd /path/to/verified/project \
  --project-ref <PROJECT_REF> \
  --rollout-root /absolute/private/rollout-root \
  --expected-revision <PREVIEW_REVISION> \
  --expected-digest <PREVIEW_DIGEST> \
  --format json
```

Use `--role-policy user-only` when assistant messages must not enter the
historical reference archive. Roots must be absolute, real, owner-controlled,
and non-overlapping. Registration never changes the ProjectRef registry or the
canonical Store.

Run one finite reconciliation before considering a resident coordinator:

```sh
workvcs automation reconcile --once --format json
workvcs automation catalog latest
workvcs automation catalog show --catalog /absolute/catalog/path
workvcs automation catalog query \
  --query 'evidence question'
```

Reconciliation re-resolves each exact ProjectRef binding, classifies rollout
files as new, appended, unchanged, rewritten, or deleted, and publishes or
reuses the newest semantically identical immutable catalog round.
`canonical_current` contains verified native current
generations. `historical_reference` contains immutable source archives and is
never action authority. A failed or quarantined source remains visible and
makes the round or query incomplete. A catalog whose registration revision or
digest is no longer current is rejected rather than silently queried. `latest`
and a query without `--catalog` scan immutable manifests and deterministically
select the newest round for the current exact registration basis; there is no
mutable `current` pointer. Readback rejects a manifest whose `complete` flag
does not equal the conjunction of all source statuses. Repeated no-change
reconciliation reuses the newest exact round; a changed state, including a
return to older source semantics after an intervening round, publishes a new
monotonically newer immutable round before it can be reused.

The private source inventory retains the last successful immutable archive
reference for each discovered rollout. An unchanged source revalidates that
archive instead of re-importing the rollout. If a source later disappears or a
new version cannot be imported, the prior archive remains queryable with its
per-result source-file state while a separate failure entry keeps the catalog
and query at `partial_failure`; source loss is never turned into success. Each
root is reconciled independently: a missing or unreadable root cannot authorize
deletion classification for that root, but successful roots still detect their
own deletions. A canonical routing, Store-open or generation failure does not
skip this historical reconciliation lane.

The foreground coordinator is optional:

```sh
workvcs automation daemon run --periodic-seconds 900
workvcs automation daemon status
workvcs automation daemon reconcile
workvcs automation daemon stop
```

`daemon run` stays attached to its terminal. It holds one owner-private lock
and Unix socket, verifies same-user clients, bounds protocol frames, and starts
a short-lived reconciliation worker at startup, on a manual request, after a
coalesced watcher dirty/overflow signal, and periodically for anti-entropy.
The watcher only marks dirty through a capacity-one signal; status separately
reports observed events, coalescing, backend errors and the last watcher error.
A setup or overflow error triggers a full scan and does not masquerade as a
healthy event stream. The coordinator owns no Store handle, model, or Qdrant
shard. A worker is terminated and reported failed if it exceeds its bounded
runtime. Its bounded stdout and stderr are retained under
`runtime/worker-evidence/` with exact byte counts, full-stream hashes and
explicit truncation state. The daemon accepts success or partial success only
after parsing that report and matching the exact current-basis catalog
readback; nonzero exit, timeout, malformed output, unexpected stderr and output
overflow remain failed. Daemon status keeps worker outcome separate from protocol
health: rejected or disconnected clients increment `rejected_requests` and
preserve `last_protocol_error` instead of terminating the coordinator or being
reported as successful reconciliation.

The automation layer does not write canonical Stores, the ProjectRef registry,
KnowledgeExposure, source status, rollout files, existing archives, existing
generations, or an existing catalog. It does not clean up automatically.
Revoke removes a registration only from later rounds while retaining history:

```sh
workvcs automation list --format json
workvcs automation revoke \
  --registration-id <REGISTRATION_ID> \
  --expected-revision <CURRENT_REVISION> \
  --expected-digest <CURRENT_DIGEST> \
  --format json
```

Treat binding drift, unsafe permissions, malformed or truncated history,
worker timeout, protocol failure, and source disappearance as explicit failed
or quarantined evidence. Do not edit the derived registry or catalog by hand to
turn those states green. Repair or re-register only with separate authority,
then perform a new complete reconciliation.

## Complete Results and Observed Cost

The user directed capability-first implementation. Default queries have no
token budget, byte ceiling, prefetch ceiling or result ceiling. They return all
matching source objects. Use an explicit `--limit N` only when the caller wants
a top-k projection; the comparison benchmark requests five to preserve its
rank metric, without imposing that choice on normal use.

Each returned item carries original scope/provenance, entity/version/digest,
source commit, lifecycle and temporal scope, native readback status, full
canonical statement text, and all verified matching UTF-8 source ranges.
Match ranges and document-leading context are explicitly
distinguished. Leading context retains a statement's scope and qualifications;
it does not certify a retrieved conclusion as true.

The continuous 128-token/24-overlap windows are indexing granularity; full
statement delivery is not truncated to those windows, and the former two-match
cap is removed. Token observation uses exact compact JSON of each complete
item, including provenance, full statement and the count field. No result is
omitted for exceeding a token target. Result-level manifest, query, diagnostics
and pretty-print whitespace add to the total CLI output, which is measured
separately. Model billing and whole-conversation savings are not claimed.

The existing mandatory ContextPacket is never truncated to make room for
search. Search is an explicit supplemental read; this stage does not
automatically inject results into `context` or activate background hooks.

## Refresh and Recovery

Refresh opens the canonical source read-only, validates its complete integrity,
and builds an exclusively created temporary derived generation before kernel
no-replace publication to a new destination. Repeating refresh at the same
healthy source basis reuses the generation.
Refresh cannot target the source Store, registry, routing marker, their path
aliases, reserved source SQLite sidecar names, or an unrelated existing file.
It cannot repair canonical history. Atomic-file capability is platform-gated;
the local validation machine is macOS. Unsupported targets fail closed.

An advanced head, different identity, changed policy, missing file or damaged
derived index reports an explicit `refresh_required` reason and guidance.
No stale current candidates are returned. A valid search with no lexical
matches instead reports `ready` with zero items; inspect status before treating
an empty item list as a genuine no-match result. A current query rechecks its
head/digest before returning; its source commit states the observation anchor.

Preserve every existing stale, damaged, incompatible or different-basis file
and choose a new destination. Explicit refresh does not overwrite it. Do not
delete or overwrite unrelated data to silence a failure. Native routing,
ownership or source-integrity failures remain errors and require their own
authorized recovery. Available native bounded reads remain useful while a
derived index is unavailable.

## Evidence Boundary

Native readback verifies that stored text, identity and ranges correspond to
the selected snapshot. It does not establish current external truth, business
correctness, permission to act or production readiness. Binding ProjectRef is
the read anchor and does not reconstruct unknown legacy semantic ownership.
An admitted Capture without Store delivery is absent from this index.

Embedding and reranking use an in-process, per-generation Qdrant Edge shard and
verified local model files. There is no Qdrant Server, network model lookup,
resident model/index process, canonical transcript extraction, semantic
lifecycle inference or automatic action. The optional coordinator may discover
files only below explicitly registered roots and creates reference-only
immutable archives. Source archives, federation, and the derived catalog widen
evidence reads without widening authority. Source, installation, Push and
release states retain their own exact delivery evidence.
