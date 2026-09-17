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

The registry is an index, not the work history. Each binding pins Store,
Workspace, and Work Branch identities. Git projects use the common Git
directory as identity, so linked worktrees resolve to the same record set even
when their checkout paths differ.

Keep the registry and Store outside the project repository. A project move does
not change Git common-directory identity; a Store move must be an explicit,
verified binding update rather than silent path fallback.

The default is one Store per logical project. Use explicit `project bind` only
when separate projects intentionally share a chosen Store, Workspace, and Work
Branch. Ensure never infers that coordination topology.
