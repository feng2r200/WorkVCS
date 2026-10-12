# ADR-0525: Local Derived FTS Search and Canonical Readback

Status: Accepted; locally validated source candidate, stable adoption pending
Date: 2026-10-12
Accepted by: The current user's explicit approval, “方案确认门通过，开始推动”,
following the first-stage proposal in conversation
`01a12e00-5166-7223-8a8d-77bbddab089c`.

Amended by the same user's subsequent instruction: “不要人为设定Token等的限制，
先实现能力后再来考虑成本等”. This supersedes the proposed default token budget
and cost-based query/result/snippet ceilings before the first release.

## Context

A frozen comparison used 60 real historical questions from 51 user messages
and 19 conversations. SQLite FTS5 found the labelled historical answer in its
top five for 51 questions and returned the labelled support span for 32.
E5-small hybrid retrieval matched those counts, recovering four questions
while losing four, with higher measured query latency and research-worker
memory. These are historical known-target metrics, not answer correctness,
current project truth, or production performance guarantees.

The user approved the first stage: native WorkVCS source verification with a
separate SQLite FTS5 derived index, improving evidence snippets before deciding
whether a vector layer adds value. The approved proposal is identified by
SHA-256 `209eb8e087aefc2b39f3c00638786d70071d469871df12bc43f87dcb58fffba6`.

ADR-0081 keeps `record list --statement-contains` an exact substring filter.
ADR-0413 and INV-027 keep current ContextPacket selection deterministic. This
new opt-in post-V1 lexical capability does not replace either contract.

## Decision

1. Add an explicitly requested local derived-search surface for canonical
   Record and Knowledge statements. A separate SQLite FTS5 file ranks candidate
   snippets with BM25. No canonical schema, history, entity lifecycle,
   authorization, Task readiness, or ContextPacket priority changes are made.
2. The native source is opened read-only with its existing identity and complete
   integrity checks. One immutable source snapshot and request-scoped replay
   cache supply all exported objects and candidate readbacks. Do not open or
   replay the source separately for every hit, or weaken validation to improve
   latency.
3. The CLI resolves the existing binding through the current read-only project
   routing. It does not treat a caller-supplied display name or UUID as proof of
   ProjectRef ownership, activate markers, create bindings, bootstrap Projects,
   or sweep operations. The initial CLI uses the routed cwd surface rather
   than inventing an unverified direct-Store ProjectRef association.
4. The lower-level core API carries ProjectRef, Store, Workspace, Branch,
   snapshot commit, state digest, and current/historical scope explicitly. Its
   caller is responsible for verified ProjectRef routing; core additionally
   checks actual Store and branch/workspace identity. The ProjectRef anchor
   describes the read binding, not a reconstructed semantic owner of every
   legacy statement. Original scope/provenance remains available, including
   unknown historical ownership.
5. Current searches use the selected branch's current, native eligible
   Record/Knowledge state. Records retain active/running/unverified/validated
   status as applicable to their native kind; Knowledge must be active. Their
   returned status does not certify a business claim. Historical retrieval requires an explicit as-of
   commit and is labelled historical even if that object's status is active.
   A historical commit must belong to the selected workspace and be reachable
   from the selected branch. Cross-branch federation is outside this stage.
   Superseded or invalidated statements never become current through ranking;
   the existing context resolver alone retains any causal-summary exception.
6. Index manifests fix format, tokenizer, snippet-policy, source identity,
   temporal scope, commit/state digest, and object-version basis. Rows retain
   exact statement text, entity/version identity and digest, status and UTF-8
   byte ranges. Before returning a candidate, native readback must agree on
   identity, membership, version, digest, full statement and snippet bytes.
   Index text, recency and similarity never establish fact correctness or
   permission to act.
7. Only delivered canonical Store state enters this index. An admitted Capture
   without target delivery is absent. Index refresh is a derivation, not a new
   Finding, Decision, Knowledge or semantic extraction operation. Historical
   conversation text is not imported into live Work-State by this capability.
8. Refresh uses an exclusively created temporary file, validates it, and
   atomically publishes a complete derived generation to a new destination
   with the platform's no-replace primitive. A healthy existing generation
   with the same complete basis is reused; any other existing generation is
   preserved and requires a new destination. Do not overwrite an existing
   pathname on the strength of a prior stat check, or call exchange plus a
   later identity check an inode compare-and-swap. Reject source Store and its
   SQLite sidecar names, registry, marker, symlink/hardlink aliases and unrelated
   existing files. Cooperative publishers share a directory lock; current
   source freshness is checked before publication. Head advancement after that
   check can make the result immediately stale, which the next query detects.
   Unsupported atomic-file platforms fail closed rather than inheriting Unix
   guarantees. No automatic old-generation cleanup is introduced.
9. Missing, stale, incompatible or damaged derived state reports an explicit
   reason and safe native-read/refresh guidance. It must not return stale
   candidates, present an empty successful search as a valid current result,
   or repair unrelated files. Native source/ownership failures remain errors;
   derived failure cannot bypass them.
10. Implement the retrieval capability before imposing cost limits. By default
    return all matching source objects, all verified matching ranges with
    document-leading context, complete canonical statement text, and original
    provenance. No default token budget, byte ceiling, candidate prefetch
    ceiling or result ceiling discards evidence. An explicit caller-selected
    top-k remains a query projection; the regression benchmark requests five
    objects to compare known-target ranks. Count exact complete items with
    fixed o200k_base ordinary encoding as an observation, not an omission rule
    or model-billing claim. Preserve the existing mandatory ContextPacket.
11. Preserve the tested lexical policy: Chinese words with HMM disabled,
    Chinese bigrams, ASCII words/technical identifiers and fixed generic
    stopwords. Pin the Rust implementation and verify sample terms and counts
    against the frozen Python policy. Any difference is versioned and measured
    explicitly; old retrieval scores cannot be inherited after a policy change.
    Snippet indexing uses a fixed continuous 128-token/24-overlap policy with
    adjacent evidence, independent of gold labels, and exposes exact source
    ranges. This is indexing granularity, not truncation of delivered source
    statements. Do not impose a two-range cost cap on a matched object.
12. Add the capability as a compatible minor release, following ADR-0523. The
    development candidate is 1.1.0. Source implementation, local validation,
    local commit, merge, stable installation, Push and release remain distinct
    evidence and authorization boundaries.

## Implementation Placement

Implement a core `search` adapter with source snapshot, index and tokenizer
components, and thin `search refresh` / `search query` CLI paths using the
existing routed read helpers. The manifest is derived-file versioning, not a
canonical Store migration. Concrete API names and internal layout are ordinary
implementation choices subject to conformance tests, rather than additional
user-confirmed product requirements.

Use the current bundled SQLite dependency and measure FTS5 availability in the
actual build. Jieba and o200k implementations must be local and pinned; no
model service or Python runtime is required for the product command. Record
their binary, startup and resident-memory cost in local validation.

## Validation Obligations

- Positive Chinese and technical-identifier retrieval with actual supporting
  snippets, exact provenance and canonical readback.
- Current and explicit historical state; active, superseded and invalidated
  membership; workspace and full-DAG branch ancestry guards (including merged
  secondary parents); unchanged ContextPacket and Task/runnable behavior.
- Cross-ProjectRef/Store/Workspace/Branch identity rejection, source-head
  advance, undelivered Capture exclusion, and corrupted versions/digests or
  snippet identities.
- Missing, unknown, malformed and incompatible indexes; rejected destination
  aliases and reserved SQLite sidecars; no-replace publication, different-head
  concurrent refresh and current-query head drift with controlled barriers;
  idempotent same-source reuse and byte-stable read-only canonical source.
- Exact complete-item token observation including full statement/provenance,
  Unicode boundaries, unrestricted defaults and explicit caller top-k, without
  token-based omission or changes to existing ContextPacket budgets.
- Exact native-derived document/snippet/term membership and FTS5 inverted-index
  integrity; a damaged unmatched row or shadow posting cannot return a false
  successful no-match result. Persistent derived files remain read-only during
  query, including integrity probes performed on an exact isolated copy.
- Frozen 60-question regression with the actual implementation and additional
  independently labelled real queries supported by native Record statements.
  Benchmark fixtures are isolated references, never live project facts.
- Independent review and appropriate workspace checks. Preserve original
  failures and report exact measurement boundaries rather than repeatedly
  tuning the frozen questions.

## Non-Goals

Embeddings, vector retrieval, Qdrant deployment, rerankers, automatic transcript
extraction, semantic merge, lifecycle inference, cross-branch/Store federation,
background services, hook activation and automatic actions remain deferred.
No stable installation, merge, Push or release is authorized by this ADR.

## Consequences

Canonical Work-State remains the authority for stored identity and current
state. A replaceable derived index narrows historical evidence cheaply without
turning a related statement into a current fact. An unavailable index remains
diagnosable and rebuildable, and bounded source reads remain usable.

Local search creates a new dependency and output surface, so its actual native
startup, token and memory costs must be measured before any adoption claim.
Vector search may be considered later using new real questions and a separately
accepted net-benefit threshold.
