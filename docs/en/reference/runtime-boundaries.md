# Runtime Boundaries

This document owns filesystem-location and process boundaries among the
`Product Repository`, `Volicord Runtime Home`, installed Volicord executables,
managed Codex configuration, and the stdio MCP child process.

## Component Model

| Component | Boundary |
|---|---|
| Product Repository | The canonical Git work tree containing user product files and explicitly managed project configuration. |
| Volicord Runtime Home | Local registry, project state, authoritative operational sessions, and runtime-owned artifacts. |
| Volicord installation | The selected `volicord` executable and its build identity. It is not the Runtime Home. |
| Managed Codex configuration | User- or project-owned configuration that starts the exact managed stdio process. It is not Core authority. |
| `volicord mcp --stdio` | One local child process bound to one current Agent Connection. It is not a network service. |

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

A personal connection writes user-owned managed configuration. A shared
connection writes project-owned configuration and forwards `VOLICORD_HOME`
without embedding a machine-local Runtime Home path. The generated command,
arguments, and managed launch markers select the registered Connection and its
optional project at startup. They are cooperative launch context, not identity
credentials.

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
ownership of unrelated repository content and is not host-capability or runtime
certification.

Operational connection verification discovers the actual `codex` command on
`PATH`, canonicalizes the observed executable path under the platform topology
rules, and runs its version command. It records only path and version
diagnostics. It does not resolve a package-native artifact, hash executable
bytes, derive a platform executable identity, or require the version to appear in an
exact-host allowlist.

## Volicord Runtime Home

The Runtime Home contains only Volicord-owned runtime state: registry storage,
per-project storage, authoritative operational sessions, and runtime-managed
artifact bytes. It is selected explicitly or through the platform rule owned by
[Administrative CLI](admin-cli.md#runtime-home-selection).

The Runtime Home must not be placed inside a Product Repository. Product files,
maintained docs, release working output, test results, screenshots,
credentials, and transcripts are not Runtime Home records.

Inside WSL2, validation checks the Runtime Home or its nearest existing
ancestor against the exact distribution ext4 boundary before initialization.
Project homes and runtime-managed artifacts remain within that same boundary;
Linux-looking `/mnt/*` or other non-ext4 locations are unsupported.

## Baseline MCP Process

The supported process is `volicord mcp --stdio`. It reads JSON-RPC from stdin
and writes responses to stdout. It opens no TCP, HTTP, Unix-domain socket, or
other network transport listener. Exact startup, binding, and protocol behavior
belongs to [MCP Transport](mcp-transport.md).

One process is bound to one enabled Agent Connection and receives a new
Volicord-generated Registry runtime-session ID at process start. Its project set is the
stored allowlist or an explicitly selected member. It does not discover
authority from arbitrary filesystem proximity.

The Registry owns process lifecycle milestones and cross-project runtime/host
session reservations. Each project database owns its project Agent Session and
host session/thread/turn correlation. MCP retains those native coordinates
until an actual project is selected; the Store then derives the project
session coordinate from the Connection, current project integration revision,
and native session. Because SQLite cannot enforce a foreign key between those
separate database files, a valid Guard observation may first create an unbound
project session. The first actual managed MCP tool call for the same host
identity first validates the current managed runtime without mutation, then
establishes or validates the exact unbound project anchor. Only after project
ownership validation commits does the Registry revalidate the current owner
facts and reserve cross-project uniqueness with the exact project revision. A
final project transaction attaches that runtime to the anchor. Project
ownership conflicts leave no Registry reservation. An unbound project anchor
and a Registry reservation without project attachment are independently
non-authoritative. Exact replay under unchanged owner state repairs an
interrupted final attach. A process row is not a lease or liveness signal, so a
crashed apparently open row and concurrent processes never select or block
Guard correlation. `diagnostics.sqlite` is a separate best-effort carrier and
is never an operational authority source.

## Location And Authority Boundaries

- Product Repository writes still require the applicable Core authority.
- Runtime Home write access is not Product Repository write permission.
- Managed configuration and its launch markers are not a user decision, Write
  Ticket, host attestation, client identity, or human identity.
- A validated operational session proves only locally observed cooperative
  session ownership and current project authorization. It does not prove a
  binary, host, client, actor, or human identity.
- Internal runtime and project session IDs are private local correlation
  coordinates, not host-native identity, actor identity, or credentials.
- The immutable Connection integration-instance ID and integration generation
  are Runtime Home lifecycle coordinates. They are not host or actor identity,
  release certification, security credentials, or caller-selected values.
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
