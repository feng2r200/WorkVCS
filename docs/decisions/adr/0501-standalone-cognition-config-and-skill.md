# ADR-0501: Stable Configuration, Standalone Cognition, and Skill Distribution

Status: Accepted / Implemented (current capability)
Date: 2026-09-11

## Context

WorkVCS was previously coupled too closely to Plan admission and an ambient
`WORKVCS_HOME` variable. A fresh Codex process could miss the interactive-shell
environment, and “No-Plan” was incorrectly described as zero WorkVCS writing.
That prevented useful Knowledge, risks, evidence, and retrospective material
from being preserved for small or one-off tasks. Existing Evidence created from
raw bytes retained only digest and size, so its body could not be recovered.

WorkVCS also needs to teach its own usage without making a public governance
Plugin depend on this particular CLI.

## Decision

### Stable configuration and binding audit

Registry selection uses this precedence:

1. command-local `--registry PATH`;
2. `WORKVCS_HOME` and its `project-bindings.json`;
3. version-1 TOML at `$XDG_CONFIG_HOME/workvcs/config.toml`, falling back to
   `$HOME/.config/workvcs/config.toml`.

The TOML file defines exactly one of `home` or `registry`. `config show` reports
the effective source. `project list --require-valid` checks every binding for
duplicate identity, path drift, Store identity, Workspace, and Branch
consistency. Any future configuration or environment-locator change must update
`config.toml.example`, operator documentation, and focused tests together.

### Plan-independent cognition

`capture --cwd PATH --manifest FILE` writes a coherent group of new Records,
Knowledge, Evidence metadata, and supported semantic relations as one guarded,
idempotent Work-State commit. It does not require or create a Goal, Plan, Task,
Session, or Claim. Lifecycle-changing invalidation and supersession against
existing objects remain dedicated guarded operations.

`recall --profile brief|handoff|retrospective` is bounded and read-only. It
does not require a Session or Plan. The profiles progressively expose active
context, relations useful for continuation, and terminal/retrospective context.

A later No-Plan-to-Plan transition records why planning became valuable and
carries forward still-relevant cognition. Plan admission remains a governance
decision, not the prerequisite for durable recording.

### Inspectable Evidence bodies

Evidence created from raw bytes persists those bytes in a Store-adjacent,
Store-ID-scoped, content-addressed object area. The database records an
available `workvcs.local-object-v1` storage location. `evidence extract`
verifies size and digest before writing the requested output. Digest-only
content remains a valid external reference and has no fabricated local body.

### WorkVCS owns its integration Skill

The repository packages `skills/workvcs` with the binary and installs it to an
Agent Skills directory during an explicit installation. The Skill explains
configuration, semantic capture, Plan evolution, evidence recovery,
retrospective use, and multi-Agent read/single-writer coordination. This keeps
WorkVCS optional: a governance Plugin can stay tool-neutral, while an installed
WorkVCS teaches Codex how to use the available enhancement.

## Authority boundary

Creating or reading WorkVCS records does not need a separate authorization
ceremony. A mechanical receipt records an authority claim; it does not grant
the underlying external, destructive, production, release, push, deployment,
or credential action. Those real actions remain governed by their own current
authority boundaries.

## Consequences

- Project records resolve consistently across fresh processes and Git
  worktrees without depending on an interactive shell.
- Small tasks may yield reusable knowledge without inventing a Plan.
- Retrospectives can reconstruct semantic reasoning and verify original
  Evidence bodies when those bodies were supplied locally.
- Multiple Agents and models may share bounded WorkVCS context while avoiding
  overlapping semantic writers.
- WorkVCS and any governance Plugin remain independently usable modules.
