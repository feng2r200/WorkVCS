# Local Derived Search

This opt-in development surface is governed by ADR-0525. It searches delivered
Record and Knowledge statements from one verified read binding. It produces
source evidence; existing `context`, `recall`, Task readiness, exact Record
filters and mutation authorization retain their own contracts.

The source candidate is version 1.1.0. Local candidate validation and stable
installation are separate: an installed 1.0.1 CLI does not acquire these
commands merely because the source worktree contains them.

## Build and Query

Build the candidate from its WorkVCS checkout:

```sh
cargo build -p workvcs-cli
```

Run the resulting candidate only for bounded WorkVCS development validation.
The examples assume the default Cargo target directory. Use an existing
verified project binding and a new derived-file destination:

```sh
./target/debug/workvcs search refresh \
  --cwd /path/to/verified/project \
  --index /path/to/new-derived-search.sqlite \
  --format json

./target/debug/workvcs search query \
  --cwd /path/to/verified/project \
  --index /path/to/new-derived-search.sqlite \
  --query 'historical evidence or technical identifier' \
  --format json
```

The CLI resolves the normal read-only ProjectRef routing and exact existing
Store/Workspace/Branch binding. An optional `--registry` is a one-command
locator override, not authority to create a binding or activate a marker.
An unbound or unsafe route remains a routing error.

Current mode captures the selected branch's native eligible cognition. Record
status remains active/running/unverified/validated as applicable, and Knowledge
must be active; a returned uncertainty/validation status is not upgraded. To search
an explicit historical snapshot, pass the same `--commit COMMIT_ID` to refresh
and query. The commit must be in the selected workspace and reachable from
its branch. The result remains labelled historical even when a statement was
active at that commit. Use separate index paths for independently retained
current and historical generations.

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

No embeddings, Qdrant service, transcript import, automatic extraction,
cross-branch search, semantic lifecycle inference or automatic action is
included. Merge, stable installation, Push and release require their separate
delivery authorization and evidence.
