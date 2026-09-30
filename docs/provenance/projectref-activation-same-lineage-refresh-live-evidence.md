# ProjectRef Same-Lineage Activation Refresh Live Evidence

Date: 2026-09-30
Scope: one explicitly selected local registry and its registry-sidecar control plane

## Claim boundary

This evidence proves that the installed WorkVCS build can safely refresh the
two stale activation markers for the explicitly selected registry below, and
that both markers read back as exact and active afterward. It does not claim
that every configuration alias is active, authorize another registry, or
authorize capture recovery, Store delivery, rollback, Push, release, or
deployment.

## Installed candidate

- Source commit before local uncommitted changes:
  `02aaa3513501e7ac1078f0ae1232bff6184d97ed`
- Installed binary: `$HOME/.local/bin/workvcs`
- Installed binary SHA-256:
  `1092c9836724fe9978b6562bffb89abc08d5489cd86d03df06421aef0529f187`
- Installed Skill entry SHA-256 at activation time:
  `a62a700917e78e99e836fc33436ead295d5bde86a5ad25ac041403f31356e8d9`

Before installation, `cargo fmt --all -- --check`, Clippy with warnings denied,
the 93-row ProjectRef acceptance-matrix validator, Skill validation, and
`cargo test --workspace --all-targets --locked` all passed. The CLI suite
included 262 passing tests. The new isolated fixture advanced one registry
through ProjectRef recovery, proved both old markers stale, injected failure
immediately after each atomic replacement, recovered through status, and
proved exact retries idempotent. Negative coverage rejected another registry
lineage and retained invalid/symlink fail-closed behavior.

## Fresh live observations before apply

- Registry path: `$HOME/.codex/workvcs/project-bindings.json`
- Registry ID: `01a0e2e6-f2e6-77a2-b858-7631e40f1ad5`
- Registry revision: `2`
- Registry digest:
  `blake3-256:608fbd17ca0469019062653c546bf8c8b56b66c9e5c0f1bad8b39a33fb87da64`
- Read marker path:
  `$HOME/.codex/workvcs/project-bindings.json.d/routing-activation-v1.json`
- Old read marker digest:
  `blake3-256:571b08a6b971fc929abf41eb519de1ff25512ff04bc6bc3702525bf2049a1150`
- New read candidate digest:
  `blake3-256:4a1d56d96f4c5f4c96b27b03ef5b3cd67ac322ede16e540717467d59e93794ec`
- Journal marker path:
  `$HOME/.codex/workvcs/project-bindings.json.d/journal-admission-activation-v1.json`
- Old journal marker digest:
  `blake3-256:d0e1ea4ca9650fe7987e20ba3389db68034cccbd9153ce80dd7be3efe53f3567`
- New journal candidate digest:
  `blake3-256:43fecf6805a629509cb31db49ba101bc32d4475e011eb730c70826dcd15286c8`

Both old markers named the same registry ID at revision 1. Both status calls
classified them as `stale`, not invalid, and no live apply began until these
values were observed again after installation.

## Applied operations and postconditions

The operator had explicitly authorized these two exact live operations. Read
routing was refreshed first with the registry, new candidate, and old marker
digests above. Its apply and subsequent status both reported:

- `activation_state=active`
- `routing_active=true`
- activation digest equal to the new read candidate
- `registry_written=false`, `journal_written=false`, `store_written=false`

Journal admission was then refreshed with its three exact digests. Its apply
and subsequent status both reported:

- `activation_state=active`
- `read_routing_active=true`
- `journal_admission_active=true`
- activation digest equal to the new journal candidate
- `registry_written=false`, `journal_written=false`, `store_written=false`

Thus both atomic replacements succeeded, and the registry remained revision 2
with the same digest.

## Configuration-selection finding

The first status probe omitted `--registry`. In this environment,
`WORKVCS_HOME=$HOME/.codex/workvcs/` makes that invocation select the
configured-home marker locations, which are absent. Supplying the already
verified registry explicitly selects its sidecar marker locations, where the
two expected stale markers existed. No marker had disappeared; the two command
forms selected different control-plane locations by contract.

Required-participation integrations must therefore inspect `workvcs config
show` and consistently select the already verified active route before
declaring WorkVCS unavailable. This is not permission to scan arbitrary
registries, treat both aliases as active, or activate the configured-home route
without separate authority.
