# Operation Disposition and Plan Validation Local Adoption Evidence

Status: Exact source pushed, CI-verified, packaged, installed, and active locally
Date: 2026-10-08
Decisions: [ADR-0520](../decisions/adr/0520-operation-disposition-and-global-inventory-classification.md), [ADR-0521](../decisions/adr/0521-plan-manifest-validation-and-routed-rejection-diagnostics.md)
Source candidate: [Operation Disposition and Plan Validation Candidate Evidence](operation-disposition-and-plan-validation-candidate.md)

## Scope and authority

This evidence closes the separately authorized Push and local-adoption stage
for source commit `382cef20d383195be49b7c8c799c9ae8102e00ce`. It covers
exact remote-ref and CI verification, the repository's official local package
and install path, package-to-install parity, and read-only runtime probes for
Plan validation, all-operation inventory, and composite health.

The stage did not release, tag, deploy, rewrite history, refresh an activation
marker, dispose a historical operation, recover a historical operation, or
run an automatic backlog sweep. The previously rejected Plan Capture remains
immutable terminal audit history.

## Exact Push and CI boundary

Local `main`, `origin/main`, and GitHub `refs/heads/main` all resolved to
`382cef20d383195be49b7c8c799c9ae8102e00ce` after Push. GitHub Actions
[CI run 37781935760](https://github.com/feng2r200/WorkVCS/actions/runs/37781935760)
completed successfully for that exact head SHA in 7 minutes 13 seconds. Its
formatting, strict lint, full tests, schema validation, CLI smoke workflow, and
98-row ProjectRef acceptance matrix steps all passed.

## Exact package and installed parity

The repository's `scripts/package-workvcs.sh --install` path reported a clean
source and built the release package:

- package directory:
  `target/package/workvcs-aarch64-apple-darwin-382cef20d383-20261008T131539Z-86016`;
- package archive SHA-256:
  `76205f7b611587f908516c82ed34e38e7553dec85dce9699df4ee6762e65b27c`;
- manifest SHA-256:
  `10ee7294982dbcde2e38014e0193ee1ceaf02a6b1ce5f17009bdb7c875d13df9`;
- packaged and installed binary SHA-256:
  `2d2ebd8720615703b41e4ed1c6ac6bb7259f7617fe41e93f6700d60c9c32d0a2`;
- installed binary path: `$HOME/.local/bin/workvcs`;
- packaged and installed Skill entry SHA-256:
  `0fbb6b5fd15f9527643dc30e4fc65bdf97b877f3f6ccfae9ef5a943a7cc79689`;
- installed Skill path: `$HOME/.agents/skills/workvcs`; and
- seven-file Skill tree manifest SHA-256:
  `d529e453e85c3764e8bbdfca7611c9db6aef00e11b2bff506d2789e253bd2650`.

Independent SHA-256 comparison proved that the package and installed binaries
are byte-identical. The installed Skill passed the package's seven-file tree
manifest verification and recursively matched the source Skill tree. The
installed CLI reports `workvcs 0.1.0` and exposes both `plan validate` and
`operation-recovery --list-all|--dispose`.

## Installed runtime evidence

The installed binary validated the admitted delivery-stage Plan manifest with
`valid=true` and payload digest
`6c473499321f9daff0b6f4272f52c8be80ef7b1d4cafe641c72861e206c2d2db`.
The command reported `registry_read=false`, `journal_read=false`,
`store_opened=false`, and all write flags false, proving the public validator
is the intended intrinsic zero-write preflight rather than a second admission
path.

The installed read-only global `--list-all` probe returned
`inventory_scope=all`, `total_matching=836`, and lifecycle counts of 810
completed, 21 open, and 5 terminal-failed operations. The same probe scoped to
the WorkVCS ProjectRef returned 46 rows: 45 completed and one terminal-failed
`plan_manifest_rejected` operation. Both probes reported no registry, journal,
projection, or Store writes. These results prove that the all-lifecycle
inventory is active; they do not authorize treating every historical row as
actionable or mutating it automatically.

## Existing control-plane activation

Composite health after installation reported:

- `health=healthy` and `read_only=true`;
- registry revision 3 with semantic digest
  `blake3-256:b5ec12377cbe536bce8cb53e5378c95d019a5c8dde1584843b843e68868aa6c9`;
- seven of seven bindings valid and 57 local content objects verified;
- read routing active;
- journal admission version 2 active; and
- cognition capture, Plan admit, and Plan evolve capabilities active.

The registry and both activation-marker files retained the same SHA-256 before
and after the installed probes:

| Artifact | SHA-256 |
| --- | --- |
| ProjectRef registry | `e30b185dfa61071b4e6083eee86a570836b112e76f3e2e2d1be07bb2b3a0455f` |
| Read-routing activation | `5c93245fbaf790bee2ad677d06ef566937448f9efcf594470512382aa2fd6a96` |
| Journal-admission activation | `84132fa9610cac33327d9d7ef3d3fbed7d902a2f00e7c6bee6f1464a078c3b66` |

The existing markers already satisfied the installed contract. Local
activation therefore required no marker refresh and caused no control-plane or
Store mutation.

## Remaining boundary

Local adoption establishes that ADR-0520 and ADR-0521 are available through
the installed CLI and bundled Skill. `--dispose` remains an explicit,
digest-locked operation for a separately selected pre-delivery Capture; this
stage did not select or dispose one. Release, tag, deployment, historical
reconciliation, rollback, and destructive cleanup remain separate actions.
