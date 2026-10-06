# Phase 5I Integrity Replay Performance Evidence

Date: 2026-10-06
Status: source implementation and local validation complete; installation not performed

## Problem

Read-only project discovery, Recall, and capture-recovery status had become slow
on maintained Stores even though journal admission itself remained fast. The
dominant cost was complete Store integrity validation: it replayed every commit
independently, and every replay recursively rebuilt its full ancestry without
sharing already verified states. A linear history therefore repeated the same
ancestor work many times. Recall then replayed the same immutable HEAD once per
semantic category, while capture recovery repeated one already completed
read-only validation before its separately required writable-open validation.

The two observed symptoms had one core cause and complementary amplification:

- discovery and Recall paid repeated full-history and same-HEAD replay costs;
- capture-recovery status paid one slow full validation, while apply paid three;
- none of the evidence implicated journal intent admission, SQLite
  `PRAGMA integrity_check`, or the size of one newly admitted cognition packet.

## Preserved safety boundary

This slice does not replace complete integrity validation with head-only or
sampled validation. It preserves both control-plane checks:

1. every discovered Store is identity-checked and fully integrity-validated
   before use; and
2. capture recovery performs another complete integrity and identity validation
   after opening the target writable, so a path replacement between the
   read-only preparation and mutation cannot inherit trust.

The optimization is request-scoped. It creates no persisted cache, does not
reuse validation across commands or Store opens, and changes no Store schema,
digest, canonical replay, registry, journal, or recovery contract. No new ADR
is required because the public semantics and safety invariants are unchanged.

## Implemented boundary

- Work-State replay now uses an explicit `Enter`/`Exit` frame stack rather than
  recursive Rust calls. Cycle detection, commit-parent shape checks, workspace
  checks, canonical operation replay, and digest comparison remain fail-closed.
- One request-scoped replay cache is shared across all branch-head and commit
  checks in a complete integrity pass. Every commit is still covered, but an
  already replayed ancestor is not rebuilt for each descendant.
- Recall builds its requested Goal, Plan, Task, Record, Knowledge, and relation
  families from one cached semantic snapshot of the selected immutable HEAD.
  Profile contents, ordering, budgets, and output fields are unchanged.
- Capture recovery reuses the Engine that already passed the initial read-only
  binding validation for delivery preparation. It still drops that handle and
  performs a fresh full validation after opening the target writable. A
  pending apply therefore performs two full validations instead of three.

## Correctness and scale validation

The first cached prototype still used recursive ancestry replay and overflowed
the test thread stack at the 5,000-commit checkpoint. It was rejected and
replaced with the explicit frame stack before delivery.

The accepted implementation passed:

- focused replay and integrity suites, including a corrupted parent-cycle
  regression that fails closed without recursion;
- focused Recall profile and capture-recovery regressions;
- exact old/new output comparison for `brief`, `handoff`, and `retrospective`
  Recall on the maintained Store;
- exact old/new output comparison for a completed capture-recovery status;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo fmt --check`; and
- the ignored release-mode scale regression at 500, 1,000, and 5,000 commits.

The fixed-state scale fixture alternates two valid versions of one Entity. It
reported 67 ms, 134 ms, and 672 ms respectively. This demonstrates bounded
commit-replay scaling and the absence of recursive stack growth for that
fixture; it does not characterize memory use when Work-State itself grows
materially at every commit.

Run the opt-in regression with:

```sh
cargo test -p workvcs-core --release --test integrity_replay_scale -- --ignored --nocapture
```

## Same-machine operational measurements

The old figures below are one run of the currently installed pre-fix binary.
The new discovery and Recall figures are medians of three immediate runs of the
new release build, with their observed range. Capture-recovery status was
measured once per binary. All measurements used the same machine and current
local control plane; they are operational evidence, not a portable benchmark.

| Store and command | Installed old binary | New release build | Observed change |
| --- | ---: | ---: | ---: |
| Shared WorkVCS Store `project discover` | 12.16 s | 0.20 s median (0.20–0.21 s) | 60.8x faster |
| Shared WorkVCS Store `recall --profile brief` | 24.79 s | 0.44 s median (0.43–0.44 s) | 56.3x faster |
| Shared WorkVCS Store completed `capture-recovery --status` | 12.17 s | 0.20 s | 60.9x faster |
| HXAP Store `project discover` | 35.92 s | 0.30 s median (0.30–0.31 s) | 119.7x faster |
| HXAP Store `recall --profile brief` | 72.71 s | 0.75 s median (0.74–0.75 s) | 96.9x faster |

On the shared Store, the new `handoff` profile was 0.44 s median across three
runs; `retrospective` was 0.45 s median. On the HXAP Store, both profiles were
0.76 s median. No old-profile comparison was recorded for those four cases.

## Delivery and remaining boundary

This phase updates source, tests, and project documentation. It does not install
the new binary or Skill, change the active registry or Store format, migrate
live data, create a release or tag, or deploy anything. Adoption must separately
verify the exact installed artifact if and when installation is authorized.

The main residual performance risk is histories whose Work-State grows at most
commits: the request cache retains replayed states for the duration of one
integrity pass. Add a growing-state scale fixture before making a broad memory
or asymptotic-performance claim. Keep the current complete validation contract
unless a separate, evidence-backed contract decision explicitly changes it.
