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
volicord --repository /absolute/repository codex enable
```

## Repository workflow CLI

Run ordinary commands from a bound repository. Volicord resolves the Project
from the current directory; `--repository /absolute/path` selects another
repository and `--project PROJECT_ID` is an explicit disambiguation fallback.

```text
volicord init "Project name"
volicord status
volicord analyze
volicord recall
volicord questions
volicord decisions
volicord document preview handoff-resume
volicord viewer open
volicord context export --output /absolute/project.volicord.json
volicord privacy status
volicord doctor check
volicord doctor repair
volicord doctor reindex
```

The supported production background semantic provider identity is
`openai-codex`. It reuses the installed Codex CLI login and requires an
explicit model plus exact Project source scopes:

```text
volicord advanced records source --host codex --session <SESSION> --text "enable bounded background semantics"
volicord privacy enable openai-codex <MODEL> --source <USER_SOURCE_ID> --scope src/bounded.rs
```

Provider opt-in is not transmission approval. Each source-bearing operation
still requires the exact Guarded confirmation shown for that operation. The
adapter reads neither credential files nor tokens; an unavailable executable or
Codex login yields `provider_unavailable`, and the Codex CLI transport reports
provider-side deletion as unsupported. `VOLICORD_CODEX_EXECUTABLE` may select an
explicit Codex CLI path for a controlled installation; it does not carry or
authorize credentials.

Output is human-readable by default. Add `--json` for automation and
`--locale ko` for bundled Korean fixed strings. Use `volicord --help` and
command-level `--help` to discover options and examples. Canonical audit,
Candidate inspection, explicit Checkpoint recording, and the Guarded CLI
fallback are grouped under `volicord advanced`.

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

The SessionStart context resolves the repository and performs bounded Recall
before project work, or establishes a Goal and repository baseline for a new
Project. It does not require every task to create a Question. Repository facts
are researched, accepted contracts are reused, and delegated implementation
choices are handled by the agent. Research or no-question work,
prototype/research, and explicit deferment are valid outcomes. Only a genuinely
material, currently relevant, unresolved user-owned outcome stops before that
outcome is chosen and uses the source-grounded Question and current-host
Decision path.

Materiality follows consequence and ownership, not the number of possible code
implementations or whether a detail is public. After current owners, applicable
Decisions/contracts, and repository facts are inspected, stable public API
shape, user-visible defaults, external diagnostics or error contracts,
compatibility behavior, downstream generated/package/output defaults,
privacy/security policy, and support/maintenance policy are strong user-owned
signals when viable outcomes have materially different consequences and the
choice was not already decided or delegated. A narrow, conventional, simple,
backwards-looking, agent-recommended, or locally isolated option is not
authority to choose such an outcome. Trivial public details do not become
Questions, repository facts are never asked as user Questions, and exploratory
uncertainty may still lead to research, a bounded prototype, deferment, or a
revisit basis. Once the branch is resolved, ordinary repository edits require
no approval ceremony.

For Git worktrees, files newly created by Volicord are added only to that
worktree's repository-local Git exclusions. A tracked `.codex/config.toml` or
ownership manifest is a conflict. An untracked local config can be merged when
it has no `mcp_servers.volicord` entry or Volicord hook; unrelated settings and
hooks are preserved.

Remove only Volicord-owned repository activation with:

```text
volicord --repository /absolute/repository codex disable
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
volicord context compare --input INCOMING_BUNDLE [--base COMMON_BASE_BUNDLE]
volicord context merge --input INCOMING_BUNDLE [--base COMMON_BASE_BUNDLE]
volicord context resolve --input INCOMING_BUNDLE --conflict-set CONFLICT_SET --revision REVISION --source USER_SOURCE --mode MODE [--base COMMON_BASE_BUNDLE] [--merged-bundle EXPLICIT_MERGED_BUNDLE]
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
