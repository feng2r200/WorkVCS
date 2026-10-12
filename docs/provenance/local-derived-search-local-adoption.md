# Local Derived Search: Authorized Local Adoption

Date: 2026-10-12
Scope: Stable local WorkVCS 1.1.0 and the existing verified read binding

The user explicitly requested “合并、Push、安装” after the validated candidate
was delivered. This supersedes the earlier delivery boundary for integration,
ordinary Push and local installation. It does not activate a Qdrant layer,
background hooks, a new binding, registry migration, tags or a public release.

This is a completed **local adoption snapshot**. Exact subsequent Git delivery,
CI and final build identities are retained in WorkVCS and the local delivery
receipts. A documentation-only descendant requires its own clean package
identity; it does not change the validated search implementation or inherit a
different binary's measured performance.

## Integration and Exact Initial Installation

- Default branch `main` was confirmed against the live origin HEAD.
- Clean primary checkout fast-forwarded from
  `b35cc740234f174cc30faf6d534c5d7916808aa3` to the accepted implementation
  `e9b8e112b127cea2e59c3c659e1544e4d82f0129`.
- `scripts/package-workvcs.sh --install --dest /Users/ld/.local/bin/workvcs
  --skills-dir /Users/ld/.agents/skills` performed the explicit atomic install.
- Initial installed `runtime status --require-stable` reports version 1.1.0,
  `stable_entry=true`, `build_dirty=false`, and that exact implementation commit.
- Installed and packaged binary SHA-256 both equal
  `f610c9c7131b71d88f76b31fcb19b5ab81a152511d35f81ec6f2ffe919a2a873`.
- The complete seven-file installed WorkVCS Skill tree passes the package tree
  manifest check; the old installation matched source before replacement.
- `workvcs` resolves to `/Users/ld/.local/bin/workvcs` on PATH.
- The new CLI's read-only registry compatibility probe reports `v2_readable`,
  no migration required, and no registry/journal/Store writes. Existing
  canonical registry revision/digest and binding are preserved.

## Actual Installed Search

The stable entrypoint was invoked against the existing verified WorkVCS
ProjectRef/Store/Workspace/Branch route, with new derived-file destinations.

| Scope | Source commit | Complete objects | Omitted | Observed complete-item tokens |
|---|---|---:|---:|---:|
| Explicit historical | `01a12969-4c45-73be-88c6-f2e0941782b8` | 157 | 0 | 96,647 |
| Current, query `合并、Push、安装` | `01a12f26-6a77-776e-a53a-8418cefc682c` | 93 | 0 | 69,175 |

The historical result matches the previously validated case item-for-item,
including original statements, identities, versions, metadata, ranges and
token counts. The current result contains the exact authorization Decision
`01a12f26-6a1b-758c-89c7-8b8093223de0`, digest
`68fbda20f71c042e758c023c5bddf413f4cf768afab924f67b0d28543fc46974`.
Every returned byte range decodes to the exact original UTF-8 snippet. Native
readback is verified. Store, registry and routing-marker file SHA-256 values
are equal before and after both queries.

The default has no result/Token cost cap and omits zero matching objects.
These single-case adoption probes do not establish general ranking accuracy,
speed or memory savings. The original 60-question regression and 17-query
comparison retain their own boundaries in
[the candidate evidence](local-derived-search-candidate.md), including the
unchanged native top-five known-target result of 4/17.

## Evidence and Recovery Boundary

Local receipt root:
`/Users/ld/.codex/.chatgpt-projects/g-p-6a8569f19e0481918d1c7222641a0ac1/execution/workvcs-fts-adoption-20261012`.

- `authority-{capture,readback}.txt` preserve the current direct user authority.
- `preflight.json` and `merge.txt` preserve the target, clean state and merge.
- `install-code-commit.txt` and its stderr preserve the official package and
  install receipt; `installed-code-proof-verified/proof.json` preserves exact
  source/binary/package/Skill identities and the actual source/control hashes.
- `installed-registry-compatibility.txt` is the new stable CLI's zero-write
  compatibility result.
- The first adoption helper used shortened range-field names and failed with
  `KeyError: start`. The failed helper and native stdout are preserved under
  `verify_installation-schema-assumption-failed.py` and `installed-code-proof`.
  The corrected helper uses actual `snippet_byte_start/end` fields. This was a
  measurement-helper error; product source was unchanged.
- Final exact commit, package/install receipts, installed queries, remote refs
  and CI are retained as subsequent receipts in this same delivery root and
  persisted through the established WorkVCS binding.

Use [the operator guide](../operator/local-derived-search.md) for ordinary
stable-entry search. Refresh does not overwrite an existing incompatible or
stale generation; preserve it and choose a new destination. Search remains an
explicit supplemental read, with original lifecycle/temporal scope and
canonical readback. It does not make a related statement externally true,
grant action authority or inject results into deterministic ContextPacket.
