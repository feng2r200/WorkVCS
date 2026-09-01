# Phase 4LY Resource Scope File Observation Evidence

Status: current local evidence
Date: 2026-09-01

## Scope

Phase 4LY reduces Resource-backed verification friction by letting
`workvcs verify` read ResourceObservation content directly from the explicit
`--scope-path` when the caller opts in with
`--resource-content-from-scope-path`.

This is a bounded local-file observation path. It is not a general Resource
adapter, glob resolver, Git diff observer, symlink resolver, automatic
re-observation scheduler, or Agent-text path extractor.

## Implementation Proof

The new flag joins the existing mutually exclusive Resource fingerprint source
group:

```text
--resource-fingerprint
--resource-content
--resource-content-file
--resource-content-from-scope-path
```

When selected, the wrapper reads bytes from `--scope-path`, hashes them with
the existing content digest algorithm, records one ResourceObservation, records
one Verification resource basis, and records an applicable cache for that
same observation.

Focused regression coverage:

```text
cargo test -q -p workvcs-cli cli_verify_accepts_path_scope_shorthand_for_resource_observation
cargo test -q -p workvcs-cli cli_lazy_record_and_verify_commands_render_nested_help
cargo test -q -p workvcs-cli cli_verify_content_errors_use_verify_flag_labels
```

The tests prove:

- `verify --resource-content-from-scope-path` reads a real file named by
  `--scope-path`;
- the rendered ResourceObservation fingerprint equals the content digest of
  that file;
- the help surface exposes the opt-in flag; and
- missing `--scope-path` produces a CLI error instead of recording a bogus
  ResourceObservation.

## Dogfood Run

Successful Store and log directory:

```text
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ly-resource-scope-file-observation/.work-governance/runtime/dogfood/phase-4ly-20260901T031203Z.sqlite
/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ly-resource-scope-file-observation/.work-governance/runtime/logs/phase-4ly/resource-scope-file-observation.20260901T031203Z
```

The dogfood target was the real local `agent_soul` project:

```text
/Users/example/Projects/HeXun/Hernes/agent_soul
```

The run proved:

- the target project can remain a read-only bound Resource;
- `verify --scope-path ... --resource-content-from-scope-path` reads the
  actual target file;
- the ResourceObservation fingerprint equals an independent
  `canonical content-digest --content-file` result for that file;
- `ac status` becomes `verified`; and
- the target project `git status --short` snapshot is unchanged.

Summary:

```text
phase4ly_dogfood_result=PASS
dogfood_store=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ly-resource-scope-file-observation/.work-governance/runtime/dogfood/phase-4ly-20260901T031203Z.sqlite
log_dir=/Users/example/Repositories/CLI/WorkVCS/.work-governance/worktrees/phase-4ly-resource-scope-file-observation/.work-governance/runtime/logs/phase-4ly/resource-scope-file-observation.20260901T031203Z
target_project=/Users/example/Projects/HeXun/Hernes/agent_soul
scope_path=/Users/example/Projects/HeXun/Hernes/agent_soul/dianjin-research-skill-pack/../dianjin-research-skill-pack/SYSTEM.md
workspace_id=01a05af3-d36f-7740-b327-3d259d9659ae
branch_id=01a05af3-d36f-7740-b327-3d511712a05c
resource_id=01a05af3-d388-75a2-9e5f-9d8123b492ea
task_id=01a05af3-d3c3-78c3-8525-d3700235b1b8
acceptance_criterion_id=01a05af3-d3dd-75e0-b4e0-8364ce300fcb
session_id=01a05af3-d3f6-7431-95c8-53636d7e18ee
verification_id=01a05af3-d40d-70e3-8496-d85b8dcacfc1
observation_id=01a05af3-d40b-7860-8987-57720f4456db
resource_fingerprint=5b07052923bb60b5a9be5dca7079ca90fd4ae78856fdb77349ce98ccef5b0a3b
expected_resource_fingerprint=5b07052923bb60b5a9be5dca7079ca90fd4ae78856fdb77349ce98ccef5b0a3b
target_status_unchanged=PASS
```

## Target Project Evidence

The run captured before/after `git status --short` snapshots for the target
project and compared them byte-for-byte. The comparison passed.

The target project remained dirty because it was already dirty before this
slice. That dirty state is unrelated to Phase 4LY.

## Findings

- Resource-backed verification can now observe the common explicit local-file
  case without a separate manual content or fingerprint input.
- The opt-in flag keeps hidden file reads out of default `verify` behavior.
- Prefix scopes and arbitrary JSON scopes remain intentionally unsupported as
  file content sources.
- Adapter-backed re-observation remains Open.

## Independent Review

Independent review found no code blocker/high/medium issues. It found one
Medium documentation/governance consistency issue: the Phase 4LY Plan was
marked completed before final validation, commit, merge, and cleanup had
actually happened. The Plan was corrected back to active and now records those
actions as remaining before closeout.

## Final Validation

Final validation passed after the independent-review fix:

```text
git_diff_check=PASS log=/tmp/workvcs-4ly-final-validation-20260901T031644Z/git_diff_check.log
cargo_fmt=PASS log=/tmp/workvcs-4ly-final-validation-20260901T031644Z/cargo_fmt.log
schema=PASS log=/tmp/workvcs-4ly-final-validation-20260901T031644Z/schema.log
cargo_clippy=PASS log=/tmp/workvcs-4ly-final-validation-20260901T031644Z/cargo_clippy.log
cargo_test=PASS log=/tmp/workvcs-4ly-final-validation-20260901T031644Z/cargo_test.log
cli_smoke=PASS log=/tmp/workvcs-4ly-final-validation-20260901T031644Z/cli_smoke.log
```
