# Phase 4LX Resource Scope Path Normalization Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LX closes a bounded V1 usability gap for explicit path scopes:

- Context path-scoped Knowledge matching now handles common lexical variants;
- common CLI workflows can pass paths without hand-authoring JSON; and
- the top-level `verify` wrapper can create a path scope payload from
  `--scope-path` or `--scope-path-prefix`.

This evidence does not claim Resource adapter maturity, glob semantics,
symlink resolution, case normalization, rename detection, automatic
re-observation, or automatic path extraction from Agent text.

## Implementation Proof

The implementation keeps the existing canonical scope object model. CLI
shortcuts construct the same `{"path":...}` or `{"path_prefix":...}` payloads
that advanced callers can still provide through JSON.

`verify --scope-path` defaults to `scope_kind=path` and
`scope_schema_version=1`; explicit `--scope-payload-json` callers still provide
their own kind and schema version.

Relative path shorthands stay relative after lexical normalization; callers
who need absolute scope pass an absolute path.

Focused regression coverage:

```text
cargo test -q -p workvcs-core --test context_profile_budget_phase4kx context_packet_path_scope_matching_normalizes_lexical_variants
cargo test -q -p workvcs-cli path_scope_shorthand
cargo test -q -p workvcs-cli relative_scope_json
```

The tests prove:

- lexical variants using repeated separators, `.`, `..`, and trailing
  separators match consistently;
- lexically adjacent paths do not leak across slash boundaries;
- `knowledge create --scope-path` and `context --scope-path` interoperate;
- relative `--scope-json` and relative path shorthands interoperate;
- `context-packet save --scope-path-prefix` persists canonical packet scope;
  and
- `verify --scope-path` records Evidence, one ResourceObservation, one
  Verification resource basis, and an applicable cache.

## Dogfood Run

Successful Store and log directory:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lx-resource-scope-normalization/.work-governance/runtime/dogfood/phase-4lx-20260901T024704Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lx-resource-scope-normalization/.work-governance/runtime/logs/phase-4lx/resource-scope-normalization.20260901T024704Z
```

The dogfood target was the real local `agent_soul` project:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

The run proved:

- the target project can be represented as a bound `git-worktree` Resource and
  associated with a Workspace;
- matching Knowledge created with `--scope-path` remains visible when context
  is requested with a lexically different `--scope-path`;
- unrelated path-scoped Knowledge is filtered from that ContextPacket;
- `claim next --context-scope-path` emits a scoped packet for the selected
  Task;
- `verify --scope-path` records Resource-backed evidence and makes the AC
  verified; and
- `context-packet save --scope-path-prefix` persists the resolved scoped
  packet.

Summary:

```text
phase4lx_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lx-resource-scope-normalization/.work-governance/runtime/dogfood/phase-4lx-20260901T024704Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4lx-resource-scope-normalization/.work-governance/runtime/logs/phase-4lx/resource-scope-normalization.20260901T024704Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
workspace_id=01a05adc-f41c-7cf1-8d9b-8845b568504e
branch_id=01a05adc-f41c-7cf1-8d9b-8870b2eefa7e
resource_id=01a05adc-f433-7692-bf53-d4e169097d64
task_id=01a05adc-f469-7b63-9f7e-d5bf2eb902bf
acceptance_criterion_id=01a05adc-f483-7c40-a8f3-fe9ff23bd9f2
session_id=01a05adc-f4c4-7a82-8415-4d214fc4a3ed
verification_id=01a05adc-f53b-7ff0-a3ee-e1cd3cc3923a
observation_id=01a05adc-f539-7b30-83a2-71c6078857af
context_scope_json={"path":"/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md"}
claim_next_context_scope_json={"path":"/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/SYSTEM.md"}
packet_scope_json={"path_prefix":"/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack"}
target_status_unchanged=PASS
```

## Target Project Evidence

The run captured before/after `git status --short` snapshots for the target
project and compared them byte-for-byte. The comparison passed.

The target project remained dirty because it was already dirty before this
slice. That dirty state is unrelated to Phase 4LX.

## Findings

- Lexical path normalization is now part of explicit path scope matching.
- Operators can use path shorthands for the common local-file Context and
  Resource-backed verification path.
- The implementation remains local and deterministic: no hidden filesystem,
  Git, adapter, or LLM lookup is performed while matching scope selectors.
- Remaining Resource work is narrower: glob semantics and adapter-backed
  re-observation remain Open.

## Independent Review

Independent review found one High compatibility issue: the first
implementation converted relative path shorthands into absolute paths, which
would have prevented interoperation with existing relative `--scope-json`
Knowledge. The implementation was corrected so relative path shorthands remain
relative after lexical normalization, and the regression test
`cli_path_scope_shorthand_interoperates_with_relative_scope_json` now covers
both JSON-to-shorthand and shorthand-to-JSON matching.

No blocker issues were reported.

## Final Validation

Final validation passed after the independent-review fix:

```text
git_diff_check=PASS log=/tmp/workvcs-4lx-final-validation-20260901T025425Z/git_diff_check.log
cargo_fmt=PASS log=/tmp/workvcs-4lx-final-validation-20260901T025425Z/cargo_fmt.log
schema=PASS log=/tmp/workvcs-4lx-final-validation-20260901T025425Z/schema.log
cargo_clippy=PASS log=/tmp/workvcs-4lx-final-validation-20260901T025425Z/cargo_clippy.log
cargo_test=PASS log=/tmp/workvcs-4lx-final-validation-20260901T025425Z/cargo_test.log
cli_smoke=PASS log=/tmp/workvcs-4lx-final-validation-20260901T025425Z/cli_smoke.log
```
