# Linux and Codex setup

The supported reconstruction installation path is local to `rebuild/` until
cutover. From the repository root:

```text
rebuild/install.sh --setup-codex
```

The installer builds and installs `volicord`, `volicord-viewer`, and
`volicord-mcp` under `$HOME/.local/bin` by default. It initializes only the
current runtime at `$XDG_DATA_HOME/volicord` or
`$HOME/.local/share/volicord`. `--prefix` and `--runtime-dir` accept explicit
absolute alternatives. The installer does not look for or interpret any other
runtime.

Codex registration uses the installed CLI contract:

```text
codex mcp add volicord --env VOLICORD_RUNTIME_DIR=/absolute/runtime -- /absolute/bin/volicord-mcp
codex mcp get volicord
```

The host reports connection failure separately from Project capability
degradation. On stdin EOF or Codex restart, the stdio adapter exits without a
background daemon or child process.

Uninstall removes the three executables and the `volicord` Codex MCP entry:

```text
rebuild/install.sh --runtime-dir /absolute/runtime --uninstall
```

Uninstall deliberately preserves the runtime and canonical user data at the
displayed path. Re-running the install command reinstalls the executables and
reuses only that current-product runtime. Delete user data only through an
explicit, separately reviewed action.
