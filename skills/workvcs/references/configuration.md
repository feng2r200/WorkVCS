# Configuration

WorkVCS selects its project registry in this order:

1. command-local `--registry PATH`;
2. `WORKVCS_HOME`, using `$WORKVCS_HOME/project-bindings.json`;
3. `$XDG_CONFIG_HOME/workvcs/config.toml`, or
   `$HOME/.config/workvcs/config.toml` when `XDG_CONFIG_HOME` is unset.

The TOML file has `version = 1` and exactly one of:

```toml
home = "~/.codex/workvcs"
```

```toml
registry = "/absolute/path/to/project-bindings.json"
```

Run `workvcs config show` to see the effective source and path. Run
`workvcs project list --require-valid` to detect missing Stores, Store-ID or
Workspace/Branch mismatches, path drift, and duplicate project identities.

`workvcs project discover --cwd <path>` is always read-only. When it reports
`project_binding_not_found`, `workvcs project ensure --cwd <path>` creates a
default binding idempotently. A configured `home` or `WORKVCS_HOME` places the
identity-derived Store under `<home>/stores/projects`. When the effective
locator is a direct registry path, pass `--store-root PATH`; WorkVCS will not
invent a Store location from the registry's parent.

In registry v1, the registry is an index, not the work history. Each
binding pins Store, Workspace, and Work Branch identities. Git projects use the
common Git directory as identity, so linked worktrees resolve to the same
record set even when their checkout paths differ.

ADR-0513 accepts a successor registry v2 in which stable ProjectRef is separate
from semantic Project, Git, and CWD locators. A known WorkVCS home will hold the
registry and capture journal; registry-only configuration will derive the
journal sidecar as `<canonical-registry-path>.d`. The CLI can inspect
the v1-to-v2 mapping with `workvcs project registry-migrate --preview`; an
explicit `--repair-manifest PATH` may add a strictly bound historical ownership
repair to that preview. Preview creates no sidecar. Migration apply, explicit
rollback, and the read-only rollback probe use the same registry precedence
and introduce no new TOML key. They have isolated-fixture validation; command
availability does not prove installation, current state, or live authority.
For rollback of the standard registry filename, the probe enumerates both the
home-root and registry-sidecar marker/journal aliases even when only one input
form selected the registry. Canonically equivalent home paths are deduplicated.
Apply and rollback currently require Unix atomic-replace semantics. Journal
admission and rollback share the canonical-registry quiescence lock; always
inspect exact marker and lock state before a live operation.

Versioned ordinary reads and `project routing-activation` use the same
precedence. With `home`, the marker path is
`<home>/routing-activation-v1.json`; registry-only configuration uses
`<canonical-registry-path>.d/routing-activation-v1.json`. Marker absence means
off. An exact marker activates only ProjectRef-v2 reads and is bound to the
registry ID, revision, and digest. It does not activate the capture journal or
Store writes. Determine live state only from the selected registry and its
exact marker; activation remains a separately authorized operation.

The separate journal marker uses
`<home>/journal-admission-activation-v1.json` or the corresponding registry
sidecar. Its filename remains stable, while marker version 2 carries the
strictly sorted `cognition_capture`, `plan_admit`, and `plan_evolve`
capabilities. A readable version-1 marker authorizes cognition only. Enabling
Plan on the same registry snapshot requires an explicit strict-superset
refresh with the exact installed marker digest; it is never inferred from a
new binary or source checkout.

Concrete locator adapters do not add a configuration key. A trusted tool
integration passes one bounded invocation explicitly with
`--locator-adapter-context FILE`; `codex_home` inside the current
`codex-app-project-metadata/v1` context is used only to derive its stable local
namespace and verify a canonical Project-mirror path. It does not select the
WorkVCS registry or Store. The registry continues to use the precedence above.

Keep the registry and Store outside the project repository. A project move does
not change Git common-directory identity; a Store move must be an explicit,
verified binding update rather than silent path fallback.

The default is one Store per logical project. Use explicit `project bind` only
when separate projects intentionally share a chosen Store, Workspace, and Work
Branch. Ensure never infers that coordination topology.
