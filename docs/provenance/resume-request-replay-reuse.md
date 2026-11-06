# Resume request-local replay reuse

Date: 2026-11-06

## Scope and authority

The user requested diagnosis of the 30-second `workvcs resume` timeout in
`program:1.1`, then authorized the diagnosed request-local replay optimization.
The user separately authorized replacing `/Users/ld/.local/bin/workvcs` after
validation and read-only acceptance in `agent_buddy`. The initial delivery did
not authorize Push or cleanup. The user's subsequent instruction explicitly
authorizes committing and pushing this repair, then removing process build and
rollback artifacts. External release, registry/Store mutation, Skill
installation, and daemon restart remain outside this scope.

This is focused implementation/performance evidence, not a new domain or
architecture contract. ADR-0413, ADR-0425, and existing replay and Verification
applicability semantics remain unchanged. There is no schema, command surface,
output-budget, or timeout-policy change.

## Observed failure and mechanism

The source pi session recorded `workvcs resume --cwd "$PWD"` with a bash-tool
`timeout` of 30 seconds, followed by `Command timed out after 30 seconds`
(duration 30004 ms). The project was `agent_buddy`. The installed baseline was
3.0.2, commit `869b344c9e94de68ce327210a75598cea3f0c485`, clean build,
BLAKE3 `7a4b7b77b411a5cc3fcb0dc6c7acfa0e1fd6dab7de11e15ef928330fdc0a160a`.

Read-only reproduction completed successfully at approximately 45 seconds.
A separate `project discover` succeeded in 3.11 seconds. Live sampling placed
2349/2349 main-thread samples in the acceptance-status path during one
three-second window; this is a window-local count, not a whole-run percentage.
The reduced-output control (`--budget-items 1`) still took 43.95 seconds and
constructed 682 available context items before rendering one.

The active head's ancestry contained 2146 commits. Acceptance status resolution
repeatedly called criterion, requirement, and verification queries that each
created a fresh `ReplayCache`, reconstructing the same history. Context also
computed effective statuses before discarding non-current criteria. This
explains a CPU-bound successful command exceeding the caller's timeout; it is
not evidence of an unavailable service. Smaller output is not a computational
budget and was not adopted as a remedy.

## Implementation

- Context overview and packet assembly share an explicitly passed request-local
  `ReplayCache` across the existing snapshot readers, including nested
  criterion/requirement/effective-status reads.
- Current-task/focus filtering precedes effective-status resolution. Snapshot
  anchor checks and deterministic source ordering remain intact.
- Effective-status reads check the expected Branch head; a warmed replay cache
  cannot conceal head advancement or mix newer effective statuses with the
  overview's older anchor.
- The packet releases ancestor replay states after the last shared snapshot
  read, before collecting potentially independent peer/resource context.
- Resource applicability results are not memoized. They remain fresh,
  branch-sensitive derived reads even when the WorkState head is unchanged.
- Public standalone history APIs retain their signatures and use fresh caches.
  Existing integrity checks remain enabled. Runnable/currentness and other
  independent query paths are not claimed to have been reduced to one replay
  for the entire CLI invocation.

## Validation

- Full core regression: 833 passed, 0 failed, 1 explicitly ignored scale test.
- CLI `resume` selection: 6 passed, including cwd discovery, implicit Session
  ambiguity, focused/explicit scope, and non-default Session-selected Branch.
- Final focused context/currentness/applicability suites: 38 passed.
- Added deterministic unit coverage counts cold replay entries across **all**
  caches on the test thread, not only the cache passed by the test. Its positive
  control performs two independent replays and observes two; the selected
  multi-criterion/requirement batch observes one, and a warmed focus expansion
  observes zero. It also checks standalone status equivalence, new-head
  freshness, historical status, and stale-anchor rejection.
- Added same-head resource-applicability coverage cycles applicable, stale,
  unavailable, and applicable results through both overview and brief packet,
  comparing standalone effective status while proving the Branch head unchanged.
- `cargo fmt --all --check`, `git diff --check`, and
  `cargo clippy -p workvcs-core --lib -- -D warnings` passed.
- Active LSP checks were inconclusive (timeouts; the large task module also
  exceeded the diagnostic size limit). No LSP-clean claim is made. Compilation,
  tests, and clippy provide the available static/runtime evidence.

## Real-data isolation and performance evidence

A SQLite read-only backup of the affected Store was used for candidate testing
from inside the WorkVCS source checkout. The source-tree candidate did not read
or mutate the live project's control plane. Both binaries used the same copied
Store and Session `01a181bb-854d-752e-887c-a18c1fc6bf43`, head
`01a1a2aa-0f73-725d-a40c-bcb89f3776ed`.

| Measurement | Wall seconds | Result |
| --- | ---: | --- |
| Stable baseline, snapshot `resume STORE --session ID` | 52.08 | exit 0 |
| First optimized candidate, same input | 6.64 | exit 0; byte-identical stdout |
| Candidate with explicit head guard and early cache release | 6.76 | exit 0; byte-identical stdout |
| Final candidate after counter positive-control verification | 6.91 | exit 0; byte-identical stdout |
| First optimized candidate, output budget 1 | 5.14 | exit 0 |

These are local single-run observations, including system scheduling, not a
latency distribution or an SLA. The final 6.91-second run reported 3.25 user
seconds, 1.73 system seconds, and a 346129176-byte peak memory footprint.
Earlier candidate footprints ranged from approximately 260 to 264 MB. Snapshot
SHA-256 readback stayed unchanged. No claim of a memory reduction is made.

Reproduce using an isolated backup, the selected active Session, and identical
arguments for the stable and source-checkout candidate binaries:

```sh
/usr/bin/time -l workvcs resume "$SNAPSHOT" --session "$SESSION" > before.out
/usr/bin/time -l target/release/workvcs resume "$SNAPSHOT" --session "$SESSION" > after.out
cmp before.out after.out
```

Transient evidence is under `/tmp/workvcs-resume-*`, including baseline and
candidate stdout, timings, the snapshot checksum, samples, and test/build logs.
Those files are local evidence, not portable test fixtures or permanent project
truth. Later observations take precedence over these dated measurements.

## Local delivery

The authorized package/install completed using:

```sh
scripts/package-workvcs.sh --install \
  --dest /Users/ld/.local/bin/workvcs --no-install-skill
```

The installed executable and verified candidate are byte-identical:

- version: `3.0.2` (local patched build, not a new external release);
- build base: `869b344c9e94de68ce327210a75598cea3f0c485`;
- `build_dirty=true`: the binary was built before this repair was committed;
- SHA-256: `c414dce8b35b636686c04fc181618b6cd61ab487cbc27f1ff8a68f895be989b5`;
- BLAKE3: `8afb3ad0fc196671c509e41d00ae93a672d9dfa803c5fcc89e713e0fd1d050c1`.

`runtime status --require-stable` confirms the specified stable entrypoint.
The original `workvcs resume --cwd "$PWD"` from `agent_buddy` completed in
**7.89 seconds**, exit 0, under a 30-second monitor inactivity boundary with
command output redirected to files. Its full stdout is byte-identical to the
pre-optimization original cwd invocation: 682 available items, the default 12
retained items, 16 current tasks, 18 criteria, and 23 requirements. This is the
original project/locator path, not just the isolated direct-Store benchmark.

SHA-256 readback confirms the live registry, routing marker, configuration, and
Store main file were unchanged. The daemon retained PID 14389 and its original
start time; no restart was performed. No Skill was installed, no timeout was
increased, and no Git commit or Push was performed during that initial
installation stage.

At initial acceptance, the old executable and checksum/runtime identity were
saved at `/tmp/workvcs-resume-rollback.LLhLvA/`, and the package was saved under
`target/package/workvcs-3.0.2-aarch64-apple-darwin-869b344c9e94-20261106T064047Z-78832/`.
These are historical artifact locations, not retention commitments.

## Commit, Push, and cleanup boundary

The subsequent user instruction requests committing and pushing the coherent
implementation, tests, and this evidence, followed by removing process build
and rollback artifacts. The cleanup scope is this checkout's generated build
and package outputs plus the identified temporary resume snapshots, rollback
copy, samples, and validation logs. Preserve the installed stable executable,
source, Git history, registry, Store, Skill, daemon, and unrelated global caches.
The measurements and installed binary digests above remain recorded here even
when their transient raw artifacts are removed. Do not rebuild or reinstall
merely to relabel the already-verified binary's build-base metadata.
