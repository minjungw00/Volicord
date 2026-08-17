# Linux and Codex setup

The supported reconstruction installation path is local to `rebuild/` until
cutover. From the repository root:

```text
rebuild/install.sh
```

The installer builds and installs `volicord`, `volicord-viewer`, and
`volicord-mcp` under `$HOME/.local/bin` by default. It initializes only the
current runtime at `$XDG_DATA_HOME/volicord` or
`$HOME/.local/share/volicord`. `--prefix` and `--runtime-dir` accept explicit
absolute alternatives. The installer does not look for or interpret any other
runtime.

Installation does not register a user-global MCP server. Explicitly authorize
each repository that should expose Volicord to Codex:

```text
volicord codex enable /absolute/repository
```

`--runtime /absolute/runtime` or `VOLICORD_RUNTIME_DIR` selects the Runtime Home
recorded for that repository. Enable canonicalizes the repository, locates the
installed sibling `volicord-mcp`, and owns only
`mcp_servers.volicord`, one `hooks.SessionStart` matcher group for
`startup|resume|clear|compact`, and `.codex/volicord-integration.json` in the
repository-local `.codex/config.toml` layer. The MCP server is enabled and
required, so a broken installation fails Codex startup or resume.

Codex loads this project layer only after the user trusts the repository. Review
and trust the exact command hook through Codex `/hooks`; Volicord never marks a
repository or hook trusted. The CLI and IDE extension share these Codex config
layers, so no plugin or AGENTS.md change is needed.

For Git worktrees, files newly created by Volicord are added only to that
worktree's repository-local Git exclusions. A tracked `.codex/config.toml` or
ownership manifest is a conflict. An untracked local config can be merged when
it has no `mcp_servers.volicord` entry or Volicord hook; unrelated settings and
hooks are preserved.

Remove only Volicord-owned repository activation with:

```text
volicord codex disable /absolute/repository
```

Disable rejects changed ownership state instead of guessing, removes its exact
MCP table, SessionStart matcher group, manifest, and local exclusion block, and
preserves every unrelated config or hook. It does not delete Runtime Home or
canonical Project data.

The host reports connection failure separately from Project capability
degradation. On stdin EOF or Codex restart, the stdio adapter exits without a
background daemon or child process.

Portable conflict handling uses the canonical three-way comparison and merge
owner through these CLI commands:

```text
volicord portable compare INCOMING_BUNDLE [--base COMMON_BASE_BUNDLE]
volicord portable merge INCOMING_BUNDLE [--base COMMON_BASE_BUNDLE]
volicord portable resolve INCOMING_BUNDLE CONFLICT_SET REVISION USER_SOURCE MODE [--base COMMON_BASE_BUNDLE] [--merged-bundle EXPLICIT_MERGED_BUNDLE]
```

`MODE` is `choose-local`, `choose-incoming`, `context-branch`, or
`explicit-merged`; the last mode requires `--merged-bundle`. `compare` returns
the conflict-set identity/revision, base/local/incoming history bases, affected
record identities, consequences, uncertainty, automatic-resolution eligibility,
and Source availability. Every explicit resolution requires a current-host user
`Source` from the same Project. `merge` is only the no-judgment automatic path;
unresolved semantic conflicts remain unchanged.

Disable repository integrations before uninstalling their binaries. Uninstall
removes only the three executables:

```text
rebuild/install.sh --runtime-dir /absolute/runtime --uninstall
```

Uninstall deliberately preserves repository config, the runtime, and canonical user data at the
displayed path. Re-running the install command reinstalls the executables and
reuses only that current-product runtime. Delete user data only through an
explicit, separately reviewed action.
