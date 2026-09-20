# WorkVCS

**Version control for Agent work and knowledge state.**

[![CI](https://github.com/feng2r200/WorkVCS/actions/workflows/ci.yml/badge.svg)](https://github.com/feng2r200/WorkVCS/actions/workflows/ci.yml)

WorkVCS gives coding Agents a durable, inspectable state that survives a chat,
a process, or a particular Agent product. It versions not only what the work
contains, but also why the current state should be believed: goals, plans,
tasks, decisions, attempts, evidence, verification, reusable knowledge, and
their typed relationships.

Git versions files. Task trackers describe assignments. Transcripts preserve
conversation. WorkVCS covers the missing layer between them: the evolving
**Work & Knowledge State** an Agent needs to continue, explain, branch, merge,
review, and restore work.

> **Project status:** the repository contains a locally release-ready bounded
> V1 implementation. The Rust packages remain at `0.1.0`, and no public
> release has been made. The source is licensed under Apache-2.0; selecting a
> release version, publishing artifacts, and making release commitments remain
> separate decisions.

## What makes it different

- **Semantic history, not transcript replay.** WorkVCS stores explicit Goals,
  Plans, Tasks, Decisions, Findings, Risks, Knowledge, Acceptance Criteria, and
  Verification as versioned domain objects.
- **Real branching and merging.** Work branches can diverge, diff, merge with
  explicit conflict resolution, restore earlier state, and retain causal
  lineage independently of Git branches.
- **Deterministic recovery.** `context`, `resume`, `next`, and `why` reconstruct
  bounded context from structure, state, evidence, and provenance rather than
  guessing from a long conversation.
- **Coordination without orchestration.** Sessions, focus, claims, handoffs,
  and merge-in-progress state help multiple Agents coordinate while WorkVCS
  remains independent of any Agent runtime.
- **Evidence-bearing state.** Immutable events, changesets, evidence, resource
  observations, and verification records make completion claims auditable.
- **Local-first and portable.** SQLite-backed stores, BLAKE3-addressed objects,
  checkpoints, and bundles keep the system inspectable and movable.

## The model

WorkVCS deliberately separates three kinds of state:

| Layer | What it contains | Versioned in Work-State history? |
| --- | --- | --- |
| Work State | Work, cognition, knowledge, acceptance criteria, relations | Yes |
| Runtime Coordination | Sessions, focus, claims, handoffs, active merges | No; updated atomically |
| Provenance | Changesets, events, evidence, observed resource basis | Immutable audit trail |

That separation avoids two common failure modes: treating transient Agent
coordination as project truth, and treating a transcript as a database.

## Quick start

Prerequisites: Git and Rust `1.98.1` or newer.

```bash
git clone https://github.com/feng2r200/WorkVCS.git
cd WorkVCS
cargo build --release --locked -p workvcs-cli
./target/release/workvcs --version
./target/release/workvcs --help
```

Create the local configuration:

```bash
mkdir -p "$HOME/.config/workvcs"
cp config.toml.example "$HOME/.config/workvcs/config.toml"
```

Bind a project and recover its current work context:

```bash
./target/release/workvcs config show
./target/release/workvcs project ensure --cwd /path/to/project
./target/release/workvcs resume --cwd /path/to/project
```

The packaging script builds a checksummed local artifact without installing
anything by default:

```bash
scripts/package-workvcs.sh
```

Installation is explicit:

```bash
scripts/package-workvcs.sh --install --bin-dir "$HOME/.local/bin"
```

By default, `--install` also installs the bundled Agent skill under
`$HOME/.agents/skills/workvcs`. Pass `--no-install-skill` for a binary-only
installation.

See the [operator quickstart and recovery guide](docs/operator/quickstart-and-recovery.md)
before using WorkVCS as durable project infrastructure.

## Explore the design

- [Product definition](docs/product/product-definition.md) — the problem,
  product promise, user model, and boundaries.
- [System boundaries](docs/architecture/system-boundaries.md) — how WorkVCS
  relates to Agents, Git, resources, and knowledge spaces.
- [Versioning engine](docs/architecture/versioning-engine.md) — Work-State
  commits, branches, diffs, merge, restore, and lineage.
- [Semantic operations and state machines](docs/architecture/semantic-operations-and-state-machines.md)
  — the Agent-facing behavioral contract.
- [Persistence model](docs/architecture/persistence-model.md) and
  [schema contract](docs/architecture/physical-schema-v0.1.md) — local durable
  storage and rebuildable projections.
- [V1 readiness ledger](docs/provenance/v1-readiness-ledger.md) and
  [release gate matrix](docs/provenance/v1-release-gate-matrix.md) — evidence
  behind the bounded maturity claim.
- [Documentation map](docs/README.md) — the full authority and evidence index.

## Scope boundaries

WorkVCS is intentionally not:

- a replacement for Git or a source-code remote;
- an Agent launcher, planner, or orchestration framework;
- a chain-of-thought recorder or transcript parser;
- an LLM-based semantic inference engine;
- a cloud synchronization or hosted collaboration service.

Those boundaries are part of the architecture, not missing marketing checkboxes.
They keep the engine portable, testable, and independent of any one Agent.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
```

Read [CONTRIBUTING.md](CONTRIBUTING.md) before proposing a change. Security
reports follow [SECURITY.md](SECURITY.md); general help belongs in
[SUPPORT.md](SUPPORT.md).

## License

WorkVCS is licensed under the [Apache License 2.0](LICENSE).

Copyright © 2026 feng2r200.
