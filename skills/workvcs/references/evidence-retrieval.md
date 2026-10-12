# Evidence retrieval

Use search for a specific evidence question when bounded Recall or an exact
identifier is insufficient. It searches delivered native Record/Knowledge
statements. It does not search arbitrary conversation history, admit semantic
records, determine runnable work, or certify that a statement is true.

After normal stable-entry and project-ownership checks, on an installation
supporting managed search generations (1.2.0 onward), invoke:

```text
workvcs search query --cwd <verified-project> --query <question> --format json
```

This explicit request may prepare a local derived generation in the verified
registry's `<registry-file>.d/search-indexes` directory. `index_maintenance`
reports `refreshed` or `reused`; `derived_index_written` distinguishes those
effects from the always read-only canonical source. `--index-dir <absolute-dir>`
selects another local derived directory. No daemon, binding/marker change,
history cleanup, or canonical write is performed.

For an exact existing generation, use `--index <absolute-file>`. That mode
reads only and reports `refresh_required` if its basis is stale, damaged or
incompatible. On 1.1.0, `--index` is required; explicitly refresh a new pathname
with `search refresh --cwd ... --index ...` before querying. Preserve an
existing stale/damaged/unknown destination and choose a new one.

Current queries use the verified branch's current eligible native state. Add
`--commit <exact-commit-id>` only for a deliberate historical question. Its
result remains historical, including when an object's old status was active.
Never use ranking to reactivate superseded conclusions or turn historical
ownership names into a verified ProjectRef.

`ready` results include native entity/version identity, state and statement
digests, original scope/provenance, complete canonical statement, verified
UTF-8 ranges and source commit/state. Use those fields for an exact follow-up
read or citation; a separate per-hit source replay is unnecessary merely to
repeat the already verified bytes. A current head change during verification
returns no items and requires a fresh request. Do not silently use stale hits.

Default output includes all matching objects, full statements and all verified
matching ranges with lead context. There is no default token or result budget.
Use `--limit N` only when the user or a concrete comparison requests a ranking
projection, and retain the reported omitted count. Large output is a resource
observation, not authority to truncate evidence or change canonical context
selection. Embedding and reranking remain separate evaluated candidates until
explicitly supported by the installed release.
