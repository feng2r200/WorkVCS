# ADR-0497: Work-Governance Cutover P0 Entrypoints

Status: Accepted; configuration and No-Plan recording clauses superseded by ADR-0501; first-use bootstrap extended by ADR-0511
Date: 2026-09-10

## Context

The WorkVCS cutover needs a small, deterministic entry boundary for local
governance state. The boundary must identify the project and its Store without
turning WorkVCS into a policy engine, and must remain usable when a prior
execution left useful discovery behind. Existing `workctl`, schema-v3/v4/v5
layouts, and `.work-governance` compatibility are not part of this cutover
contract.

The project binding must be independent of repository-local governance files.
Git identity therefore comes from the repository common directory, while the
Store is located through an explicit registry or configured locator. ADR-0501
adds XDG configuration and defines the current precedence.
The registry is external to the repository. A second Store validation is
required after locating the candidate, and ambiguity must fail closed rather
than silently selecting an active Session.

## Decision

The P0 capability matrix is:

| Slice | Status | Contract |
| --- | --- | --- |
| Project ensure/bind/discover plus read-only `resume --cwd` | Implemented; first-use bootstrap extended by ADR-0511 | Ensure one default external Store/Workspace/Branch binding idempotently; bind explicitly for intentional sharing; identify Git projects by common-directory identity; validate the Store before use; fail closed on ambiguous active Session selection. |
| P0-2a atomic/idempotent `plan admit` | Implemented | `workvcs plan admit [OPTIONS] --manifest <PATH> <STORE\|--cwd <PATH>` admits one manifest atomically and replays the same idempotency key without duplicating state. |
| P0-2b `plan evolve mode=in_place\|supersede` | Implemented; current | `workvcs plan evolve [OPTIONS] --manifest <PATH> <STORE\|--cwd <PATH>` supports atomic/idempotent in-place updates and supersede transitions with their explicit manifest semantics. |
| P0-2b2 `plan evolve mode=supersede` | Implemented; current | Supersede atomically transitions old active→superseded, creates a new active Plan under the same Goal with dual `contains` relations and a `new_plan→old_plan` `supersedes` relation; constraints use explicit `carry_all` or `replace`, and old Tasks/Records/Evidence are not migrated. |
| P0-3a receipt issue/show/list | Implemented; current | Dedicated AuthorizationReceipt Record subtype with mechanical branch/target/action/contract binding, redacted authority-ref type+digest, transaction, idempotency, and target guards. |
| P0-3b receipt consume | Implemented; current | Branch-scoped single-use consume with idempotency only from a committed `workstate_commit`; orphan ChangeSets never yield `reused`, and no Store-global lock across restore histories is promised. |
| Receipt revoke | Deferred; not current | No revoke capability is declared in the current P0 surface. |
| P0-4 read-only `closeout inspect` | Implemented; current | Explicit target kind/id via `--cwd` or `STORE` plus `--branch/--commit`; OS/query-only read, bounded direct projection, runtime aggregates, and before/after proof. |

For `plan admit`, the target is exactly one of `STORE` or `--cwd PATH`.
Explicit `STORE` requires `--branch BRANCH`; `--cwd` uses the bound branch and
must not be combined with `--branch`. `--registry PATH` is available with
`--cwd`, and `--manifest PATH` is required. The manifest carries
`idempotency_key`, `expected_head_commit_id`, optional `expected_state_digest`,
Goal/Plan/Task content, and optional records/evidence/rationale. Expected head,
state, and idempotency guards are manifest fields; there is no
`--expected-head` CLI flag.

The admit operation is one atomic WorkVCS transition and is idempotent for the
same manifest identity. A first admission may carry prior findings, decisions,
questions, constraints, and evidence in the manifest rather than discarding
useful preceding discovery.

In-place evolution uses the same target and guard pattern. It preserves Plan
fields that are not present in the manifest, updates only explicit
`description`, `strategy`, or `constraints` fields, and atomically appends
declared Tasks with Acceptance Criteria and Verification Requirements, Records,
and Evidence. It never implicitly deletes or replaces omitted state. Expected
head/state, target Plan identity/version/digest, and idempotency are manifest
fields; there is no evolve `--expected-head` CLI flag.

The P0 entrypoints do not infer, approve, or enforce the user's authorization
policy. WorkVCS records and exposes mechanical state; work-governance remains
responsible for admission judgment, confirmation gates, policy, validation
strength, and completion claims.

Discovery and `resume --cwd` are zero-write. ADR-0501 supersedes the broader
No-Plan rule: No-Plan creates no Plan but may explicitly capture standalone
cognition.

Project binding and Store discovery use these rules:

- `--registry PATH` is explicit authority for the registry locator;
- without it, the default locator is under `WORKVCS_HOME`;
- the registry and Store are not placed inside the repository as a hidden
  compatibility surface;
- Git identity is the canonical repository common-directory identity, not the
  current worktree path;
- after discovery, the Store undergoes complete identity, format, and
  integrity validation before use;
- the registry and Store are forced outside the project/repository boundary;
- registry updates use cross-process mutual exclusion and atomic replacement;
- zero, multiple, or otherwise ambiguous active Sessions fail closed; and
- no read-only query, including `resume --cwd`, may
  create or mutate WorkVCS state.

The external-registry placement, cross-process locking, atomic replacement, and
complete Store-integrity checks are current P0 contracts. Registry and Store
operations enforce those boundaries before state is used.

## Non-goals

- No `workctl` compatibility or migration shim.
- No schema-v3, schema-v4, or schema-v5 compatibility surface for this
  cutover, and no `.work-governance` compatibility surface.
- No automatic transcript parsing, policy inference, authorization approval,
  daemon, watcher, remote coordination, or repository-local registry.
- No receipt revoke or receipt `context`/`why` projection implementation is
  implied by this ADR.

## Consequences

The implemented P0-1, P0-2a, P0-2b in-place, and P0-2b2 supersede slices give
operators a stable, low-friction project boundary and atomic/idempotent Plan
admission/evolution
for WorkVCS-backed governance without reintroducing plugin-local Plan
controllers.
External registry placement avoids coupling project identity to a repository
checkout, and common-directory identity keeps worktree aliases from creating
duplicate projects. Fail-closed ambiguity makes recovery explicit but prevents
unsafe implicit Session selection. Receipt revoke and receipt `context`/`why`
projection remain deferred; closeout is current as a mechanical projection.
