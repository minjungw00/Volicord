# Runtime Boundaries

This document owns filesystem-location and process boundaries among the
`Product Repository`, `Volicord Runtime Home`, installed Volicord executables,
managed Codex configuration, and the stdio MCP child process.

## Component Model

| Component | Boundary |
|---|---|
| Product Repository | The canonical Git work tree containing user product files and explicitly managed project configuration. |
| Volicord Runtime Home | Local registry, project state, authoritative operational sessions, structured diagnostic findings, and runtime-owned artifacts. |
| Volicord installation | The selected `volicord` executable and its build identity. It is not the Runtime Home. |
| Managed Codex configuration | User- or project-owned configuration that starts the exact hidden host launcher. It contains no reusable launch authority and is not Core authority. |
| Hidden host launcher and stdio adapter | One local process that validates current managed configuration, consumes a one-time launch lease, and binds managed stdio to one current Agent Connection. It is not a network service. |

## Product Repository

A Product Repository is resolved from the canonical current Git work tree. The
first release requires the owner-defined native filesystem topology, including
the WSL2 ext4 boundary in [System Requirements](system-requirements.md#wsl2-topology).
Repository identity is not derived from a display name, cwd alone, a parent
directory scan, or a copied path string.

Inside WSL2, canonical Linux spelling is not sufficient. Registration and
execution validate the exact pinned distribution coordinate and observe that
the repository root is on its ext4 filesystem. WSL1, DrvFS, and unavailable or
conflicting topology observations fail closed before repository use.

Explicitly requested managed files may include:

- `.codex/config.toml` for the shared Codex entry;
- `.volicord/policy.json` for project-owned workflow policy; and
- Guard hook configuration, dispatch and phase wrappers, and rule instructions;
- the Volicord-managed block in `AGENTS.md`; and
- the optional managed block in `.git/info/exclude`.

Setup, repair, and removal preserve unrelated file content. Product source,
build output, test output, and user configuration do not become Runtime Home
state merely because Volicord can observe their paths.

<a id="product-repository-api-path-normalization"></a>
## Product Repository API Path Normalization

Public product paths use repository-relative slash-separated UTF-8 text.
Absolute paths, drive or UNC prefixes, backslashes, empty components, `.` or
`..` components, NUL, and paths that escape the canonical repository root are
invalid. A normalized path never begins or ends with `/` and contains no
repeated separator.

Lexical normalization does not establish filesystem containment. Any method
that reads or writes a path must resolve the current repository root and apply
the owner-defined symlink and canonicalization checks before effects. Paths in
stored records remain normalized repository-relative identities.

## Managed Codex Configuration

A personal connection writes user-owned managed configuration with the selected
canonical absolute Runtime Home as static `VOLICORD_HOME` and no forwarded
environment names. A shared connection writes project-owned configuration and
forwards only `VOLICORD_HOME` without embedding a machine-local Runtime Home
path or lifecycle coordinate. Both are projections of the one canonical
managed launch contract defined by
[Agent Connection](agent-connection.md#managed-mcp-launch-contract). The
personal hidden-launcher command, arguments, and Runtime Home binding select the
registered Connection at startup without a project selector; current
project associations come from Store-owned Connection Project memberships. The
shared launch resolves its Connection and project through repository discovery.
The static configuration contains no lease, nonce, reusable secret, or raw handle.
These values are cooperative launch context, not identity
credentials.

The hidden launcher reloads and strictly validates that exact current entry,
Connection revision, and fingerprint before it creates a short-lived Registry
lease. It passes the lease only through the in-memory transition to MCP
bootstrap. Store consumes it exactly once and creates the `managed_host`
runtime atomically. This is an evidence-integrity boundary, not an OS actor-
identity or adversarial security guarantee.

The stored managed fingerprint identifies the Volicord-managed host
configuration that setup, repair, staged activation, or another explicit
configuration owner last successfully applied or adopted. Those mutation
paths record it only after the host apply succeeds. A different applied
fingerprint changes the Connection integration revision and clears the prior
verification report. Operational verification observes the current file and a
newly generated Host Plan but never applies or adopts that plan's fingerprint.
Its report-only write is guarded by the exact revision observed before the
probe.

Inside WSL2, the Codex executable, Volicord executable, configuration target,
and each generated managed artifact are independently resolved and checked for
the same distribution ext4 boundary. A repository root on ext4 does not
authorize a nested file on another mount.

Configuration presence does not prove Codex trust, reload, initialization,
tool discovery, safe tool behavior, Guard observations, or a current
operational session. Those facts remain separate.

A Connection mode transition does not rewrite managed Codex configuration or
Product Repository files. Its coherent revision transition is confined to
Registry state: the Connection mode and generation, verification report, and
the integration revision in every owned strict Guard manifest. The CLI emits
one reload action after a real transition so a newly started managed host can
establish current runtime evidence; a same-mode no-op emits none.

The Runtime Home Guard manifest is an ownership inventory for its exact
Guard-managed subset of those files and its typed runtime commands. Managed script entries
require executable behavior on every platform, while filesystem inspection and
permission repair remain platform-specific. The manifest does not claim
ownership of unrelated repository content. It owns the current policy hash,
integration revision, typed runtime commands, complete managed-file
expectations, and required hook phases used by audit and observation.

Operational connection verification discovers the actual `codex` command on
`PATH`, canonicalizes the observed executable path under the platform topology
rules, and runs its version command. It records the path and version as
diagnostics. A changed observed version makes current host behavior pending for
renewed operational observation.

## Volicord Runtime Home

The Runtime Home contains only Volicord-owned runtime state: registry storage,
per-project storage, authoritative operational sessions, structured diagnostic
findings, and runtime-managed artifact bytes. It is selected explicitly or
through the platform rule owned by
[Administrative CLI](admin-cli.md#runtime-home-selection).

The Runtime Home must not be placed inside a Product Repository. Product files,
maintained docs, release working output, test results, screenshots,
credentials, and transcripts are not Runtime Home records.

Inside WSL2, validation checks the Runtime Home or its nearest existing
ancestor against the exact distribution ext4 boundary before initialization.
Project homes and runtime-managed artifacts remain within that same boundary;
Linux-looking `/mnt/*` or other non-ext4 locations are unsupported.

The Registry is the durable Runtime Home carrier for structured diagnostic
findings and their cause edges. A finding may correlate to its Connection,
project, runtime session, and integration revision, while an MCP runtime
session may point to its terminal finding. These records are diagnostic
evidence and do not grant authority.

A failure before the Registry can be opened may emit one bounded single-line
stderr fallback in the exact form `VOLICORD_DIAGNOSTIC_V1 <bounded-json>`.
`bounded-json` is the current shared `DiagnosticFinding` representation, not a
second error shape. The parser requires the exact prefix, one line, the shared
finding bounds, and the complete shared typed model. This fallback must not
contain an environment dump, unrestricted process output, raw request body, or
other unprojected input. It is not a substitute for a successful Registry
write, and the Store does not render it.

### Platform, Runtime Home, and installation findings

Operational classification uses closed owner enums. It never derives a code or
recommended action from rendered error text. Platform observation emits these
stable codes:

| Code | Condition |
|---|---|
| `platform.operating_system.unsupported` | The operating system has no supported release cell. |
| `platform.target.unsupported` | The executable target, including an incompatible WSL2 target, is unsupported. |
| `platform.wsl1.unsupported` | The process is running under WSL1. |
| `platform.wsl2.distribution_identity_unavailable` | The WSL2 distribution identity could not be observed. |
| `platform.wsl2.distribution_unsupported` | The observed WSL2 distribution is outside the pinned cell. |
| `platform.filesystem.unsupported` | A selected path is outside the supported filesystem boundary. |
| `platform.filesystem.observation_failed` | Filesystem identity could not be observed. |
| `platform.observation.failed` | Another required platform observation failed. |

Runtime Home findings use `runtime_home.path.missing`,
`runtime_home.path.empty_or_relative`, `runtime_home.path.invalid`,
`runtime_home.registry.missing`, `runtime_home.permission.denied`,
`runtime_home.filesystem.unsupported`, and
`runtime_home.boundary.owner_mismatch`. An explicit `VOLICORD_HOME` must be a
nonempty absolute path. A relative explicit CLI `--home` remains a CLI path
input and is resolved before this environment-value rule is applied.

Installation findings use `installation.executable.missing`,
`installation.executable.not_runnable`,
`installation.build_identity.unavailable`, and
`installation.managed_config.inconsistent`. Findings retain bounded categorical
facts only; they do not retain environment dumps, full environment values,
filesystem contents, or unrestricted path discovery.

`volicord mcp preflight` stays inside the selected Runtime Home's read boundary:
it reads the canonical managed configuration, Registry, project state,
protocol profiles, tool schemas, and host contracts without creating files,
opening a write transaction, or starting a runtime session. Active connection
verification is a different boundary. It may perform rollback-only
writeability probes in the selected stores and creates its protocol-conformance
sessions only in a disposable Runtime Home and Product Repository removed
after the command.

## Baseline MCP Process

Managed configuration starts the hidden host launcher, which enters the stdio
adapter in the same process after lease consumption. The public manual process
is `volicord mcp serve`. The adapter reads JSON-RPC from stdin
and writes responses to stdout. It opens no TCP, HTTP, Unix-domain socket, or
other network transport listener. Exact startup, binding, and protocol behavior
belongs to [MCP Transport](mcp-transport.md).

One process is bound to one enabled Agent Connection and receives a new
Volicord-generated Registry runtime-session ID at process start. Its project set is the
stored allowlist or an explicitly selected member. It does not discover
authority from arbitrary filesystem proximity.

The Registry owns process lifecycle milestones, structured runtime diagnostic
findings and cause edges, terminal-finding links, and cross-project runtime/host
session reservations. Each project database owns normalized host sessions,
turns, hook tool invocations, Guard observations, and MCP-only session
anchors. The explicitly selected `codex-mcp-2025-06-18-v1` profile supplies
session/thread/turn correlation. The separately selected `codex-hooks-v1`
profile supplies prompt session/turn or tool session/turn/tool-use/tool-name
correlation; command hooks have no thread coordinate.

MCP retains its typed native coordinates until an actual project is selected;
Store then derives the local project session coordinate from the Connection,
current project integration revision, and native session. A Guard observation
can establish shared normalized host rows but cannot establish the MCP-only
thread or runtime anchor. The first actual managed MCP tool call validates the
current managed runtime without mutation and creates or validates that exact
anchor. Only after project ownership validation commits does the Registry
revalidate current owner facts and reserve cross-project uniqueness with the
exact project revision. A final project transaction attaches the runtime.
Project ownership conflicts leave no Registry reservation. An unbound MCP
anchor and a Registry reservation without project attachment are independently
non-authoritative. Exact replay under unchanged owner state repairs an
interrupted final attach. A process row is not a lease or liveness signal, so a
crashed apparently open row and concurrent processes never select or block
Guard correlation. `diagnostics.sqlite` is a separate best-effort carrier and
is never an operational authority source.

## Location And Authority Boundaries

- Product Repository writes still require the applicable Core authority.
- Runtime Home write access is not Product Repository write permission.
- Managed configuration selects cooperative routing context, while the one-time
  lease preserves source evidence across bootstrap; neither supplies a user
  decision, Write Ticket, OS actor identity, or human identity.
- A validated operational session establishes locally observed cooperative
  session ownership and current project authorization. It does not establish
  client, actor, operating-system-user, or human identity.
- Internal runtime and project session IDs are private local correlation
  coordinates, not host-native identity, actor identity, or credentials.
- The immutable Connection integration-instance ID and integration generation
  are Store-owned Runtime Home lifecycle coordinates. Together with the current
  owner inputs, they derive local lifecycle and correlation revisions.
- Explicit removal and Connection migration retire only the selected
  connection/project-owned Registry integration rows in storage-owner order.
  Pending last-project migration retains that complete Registry inventory until
  host cleanup succeeds. Neither path deletes project registrations or
  project-local Agent Sessions, Guard and workflow history, evidence, or other
  authority data; retained history cannot authorize a current call without
  current Registry ownership.
- Export and release-validation output belongs in an explicit external output
  location, not maintained docs or Runtime Home trust input.

## Related Owners

- [Agent Connection](agent-connection.md)
- [Administrative CLI](admin-cli.md)
- [MCP Transport](mcp-transport.md)
- [Storage](storage.md)
- [Security](security.md)
