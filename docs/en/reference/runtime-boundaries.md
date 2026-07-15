# Runtime Boundaries

This document owns the component and location boundaries among Volicord implementation, Agent Connections, `Product Repository`, `Volicord Runtime Home`, the `User Channel`, and external MCP host configuration. It defines location and connection authority assumptions for those boundaries and routes storage and security details to their owners.

Volicord implementation is the implementation set maintained by this repository. It is not Volicord as a whole, not Core, not one running process, and not the local authority record for Volicord state.

## Owns / does not own

| This document owns | This document does not own |
|---|---|
| The distinction between Volicord as the product/system and Volicord implementation as the repository-maintained implementation set. | Public API behavior, public schema shapes, or method-specific effects. |
| The distinction among Volicord source repository, Volicord installation, and running executable roles. | Release packaging policy or a mandatory installation-root layout. |
| The definition of `Product Repository` and Product Repository API path normalization. | Storage record layout, locks, schema initialization, versioning, or artifact lifecycle details. |
| The definition of `Volicord Runtime Home`. | API method behavior or public schema shapes. |
| The separation between Volicord implementation files, product files, runtime data, and external MCP host configuration, including the exact Runtime Home/Product Repository path relationship contract. | Detailed security guarantee meanings or security non-guarantees. |
| Local access and location non-authority rules. | Projection authority, template bodies, or rendered display freshness. |
| The rule that runtime location does not by itself prove Volicord authority, security authority, or isolation. | Product scope, close readiness, evidence sufficiency, or user-owned judgment meaning. |

## Component model

Volicord keeps product, implementation, executable-role, MCP host term, and authority-record concepts distinct.

| Term | Definition | Must not infer |
|---|---|---|
| Volicord | The local work authority record for AI-assisted product work. | It is not Core, not a source repository, and not a single executable process. |
| Core | The local authority record for Volicord state. | It is not the whole Volicord product/system and not an adapter or CLI executable. |
| Volicord implementation | The implementation set maintained by this repository. At source level, it includes implementation crates, the `volicord` administrative CLI, the `volicord mcp --stdio` local MCP adapter, tests, documentation, validation tooling, and repository configuration. | It is not every possible Volicord product interface, not Core by itself, not `Volicord Runtime Home`, not the `Product Repository`, and not one daemon, MCP server entry, or network service. |
| Volicord source repository | The checked-out source artifact for this repository. | It is not the same thing as a deployed installation, running process, Runtime Home, Product Repository, or MCP host configuration. |
| Volicord installation | The deployed subset of Volicord executables and required runtime resources. | It does not imply that documentation, tests, source files, or repository metadata are present in every installation. |
| `volicord` administrative process | The administrative CLI executable/process within Volicord implementation. | It is not a synonym for Volicord or for all of Volicord implementation. |
| `volicord mcp --stdio` MCP adapter process | The local stdio MCP adapter executable/process within Volicord implementation. | It is not separate from Volicord implementation and not the whole Volicord implementation by itself. |
| `Agent Connection` | The local MCP host connection unit stored with `connection_internal_id`, a connection intent, host scope, and `connection.mode` of `workflow` or `read_only`. | It is not an OS sandbox, filesystem ACL, network policy, secret-isolation mechanism, user-facing identifier requirement, or user-action resolution path. |
| `Connection Projects` | The explicit allowlist of `project_internal_id` values an Agent Connection may address after user-facing repository-root selection. | It does not include every registered project by default and does not prove Product Repository authority. |
| `User Channel` | The local user path for recording authority-bearing user actions, including judgments and evidence observations. | It is not an Agent Connection, MCP host, generated display, or Product Repository file. |
| MCP server | An ordinary MCP protocol or host-configuration term that may name a server entry or process exposed to an MCP host, including a local stdio adapter process such as `volicord mcp --stdio` when the host uses that label. | It does not make Volicord as a product/system, Volicord implementation, `volicord`, or `volicord mcp --stdio` a TCP or HTTP network server, and it is not a product label for Volicord. |

When a behavior is performed by one executable role, name that role. Bare Volicord implementation should be reserved for the implementation set or for statements that apply to the set as a whole.

## Filesystem-location model

Volicord keeps implementation files, product files, runtime data, and external host configuration distinct. There is no single mandatory filesystem root for the whole Volicord implementation set.

| Location role | Definition | Must not infer |
|---|---|---|
| Volicord source repository or installation files | A source checkout, or deployed executable files and required runtime resources for Volicord implementation. | This is not automatically `Volicord Runtime Home`, not the `Product Repository`, not MCP host configuration, not proof of Volicord authority, and not inherently a network listener. |
| `Product Repository` | The user's product-file boundary: project source, product documentation, tests, configuration, and other project files. | It is not Volicord runtime state, not `Volicord Runtime Home`, and not proof of Volicord authority. |
| `Volicord Runtime Home` | The runtime storage location for Volicord-owned records, local runtime metadata, and artifact data as storage/runtime owners define them. | It is not the `Product Repository`, not a Volicord installation location by default, not automatically a security boundary, and not isolation by default. |
| External MCP host configuration | Configuration owned by the external MCP host that may name a `volicord mcp --stdio` command, process environment, or host-specific binding. | It is not Volicord runtime state, not `Volicord Runtime Home`, not the `Product Repository`, and not Volicord source repository or installation files by definition. |

### Runtime and host responsibilities

The following summary covers the baseline local Rust implementation. Detailed record placement belongs to [Storage Records](storage-records.md), artifact lifecycle to [Artifact Storage](storage-artifacts.md), administrative command behavior to [Administrative CLI](admin-cli.md), and MCP process behavior to [MCP Transport](mcp-transport.md).

**`Volicord Runtime Home`**

- **Contains:** `registry.sqlite`; the lazily created non-authority `diagnostics.sqlite`; per-project `projects/{project_internal_id}/state.sqlite`; and project artifact storage such as `projects/{project_internal_id}/artifacts/` when artifact storage is used. The registry stores Runtime Home identity and paths, installation profiles, repository-root-based project registrations, project aliases, Agent Connections, Connection Projects membership, host-hook installations, and `managed host configuration state`. Project state can store tasks, change units, write tickets, evidence metadata, User Channel user-action resolutions, artifacts, and session-watch records. The separate diagnostics database stores only bounded local operability aggregates.
- **Used by:** `volicord init`, project, connection, inbox, changes, doctor, diagnostics, and hidden internal hook commands through their owner-defined paths. `volicord doctor --privacy-footprint` reports storage categories and counts without printing row bodies. `volicord diagnostics session` reads only the bounded diagnostics store after normal setup checks. `volicord mcp --stdio`, Core, and Store use Runtime Home state for startup, project routing, Core state, artifacts, and best-effort operability aggregation.
- **Boundary:** It is not a Product Repository, external host configuration, or installation directory. It does not provide or prove OS sandboxing, network isolation, scanning, host trust, actor attribution, write prevention, tamper-proof audit, full filesystem monitoring, correctness, test sufficiency, review completion, final acceptance, or residual-risk acceptance.

**`Product Repository`**

- **Contains:** User product files and only explicitly requested integration files, such as project-scoped host configuration, detective host-hook policy, or managed guidance.
- **Used by:** User or host tools for ordinary product-file edits. Volicord may inspect product paths as inputs and may write explicit integration files only through owner-defined administrative paths.
- **Boundary:** It is not Runtime Home state, Core storage, or default artifact storage. Its contents do not prove Volicord authority.

**`managed host configuration state` in the Runtime Home registry**

- **Contains:** `connection_internal_id`, host kind, connection intent, host scope, optional `project_internal_id`, internal server name, configuration target, mode, enabled state, managed fingerprint, verification summary status, verification report JSON, user actions JSON, host-hook installation status, and metadata.
- **Used by:** `volicord init`, the `volicord connection` commands, and hidden internal hook flows to create, update, list, verify, or remove registry rows, host-hook installation rows, and Connection Projects membership.
- **Boundary:** It is not the external host configuration object. It does not prove that the host trusted, approved, loaded, initialized, or exposed `volicord mcp --stdio`, or ran detective host hooks.

**External MCP host configuration**

- **Contains:** Host-owned or user-managed configuration that can name a `volicord mcp --stdio` process. Personal/local overlays may carry an internal Agent Connection binding, absolute command, and the absolute `VOLICORD_HOME` selected by init. Repository-visible shared Codex and Claude Code entries carry the typed `volicord mcp --stdio --discover-repository --host <host>` descriptor with no local IDs or literal Runtime Home path. They forward the clone-local `VOLICORD_HOME` through the exact host-specific portable form: Codex `env_vars = ["VOLICORD_HOME"]` or Claude Code `"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`.
- **Used by:** The external host for loading and trust decisions. `volicord` may write supported direct configuration only when [Administrative CLI](admin-cli.md) defines that behavior.
- **Boundary:** It is not Runtime Home registry state or Core authority, and it does not prove Volicord authority. When stored in a `Product Repository`, it is only an explicit integration file.

**`volicord` administrative CLI process**

- **Handles:** Runtime Home initialization, project registration, Agent Connection and Connection Projects management, host configuration, status, verification, mode changes, and owner-defined safe removal.
- **Started by:** A local operator or user.
- **Boundary:** It is not a public Volicord API method path, OS security enforcement layer, host trust decision, or blanket Product Repository edit authority.

**`volicord mcp --stdio` MCP adapter process**

- **Handles:** One local stdio child process bound to one Agent Connection, either by an explicit local ID or by a unique local repository-discovery result. For a Volicord-managed launch, it uses the Runtime Home bound by the init-produced configuration; a user-managed explicit-ID launch resolves Runtime Home from its process inputs under [MCP Transport](mcp-transport.md). It validates connection state, exposes tools by `connection.mode`, selects allowed projects, derives adapter-owned invocation facts, and routes public method calls through Core and Store. Repository discovery requires an explicitly forwarded, nonempty, absolute `VOLICORD_HOME`; it rejects an absent, empty, or relative value before platform-default substitution, canonicalizes the current Git worktree, and narrows the process to the one registered project selected in that Runtime Home.
- **Started by:** An MCP host, which communicates through stdin/stdout.
- **Boundary:** It does not grant arbitrary product-file edit authority or authority to record user-action resolutions. It does not enforce host trust, provide sandboxing, or open an MCP network transport listener. Unless disabled, the process may separately bind an ephemeral loopback-only HTTP listener for local User Channel consent; that listener is not the MCP transport.

<a id="runtime-location-product-repository"></a>
### `Product Repository`

`Product Repository` is the user's project workspace and product-file boundary.

May claim:
- Product files can be inspected as inputs to owner-defined Volicord checks or user-owned judgments.
- Compatible product-file writes can be governed by the current scope, current Change Unit, required judgments, and write-ticket compatibility.

Must not claim:
- `Product Repository` content is Volicord state.
- `Product Repository` content is generated Volicord output.
- `Product Repository` content proves Volicord authority.
- A `Product Repository` is automatically `Volicord Runtime Home`.

<a id="explicit-integration-files-in-product-repositories"></a>
### Explicit integration files in Product Repositories

Volicord runtime state, SQLite databases, generated records, runtime homes, logs, projections, QA results, acceptance records, close-readiness state, and residual-risk records must not be written into a `Product Repository`.

The only baseline exceptions are explicitly requested integration files:

- project-scoped host configuration, such as Codex `.codex/config.toml` or Claude Code `.mcp.json`
- a Volicord-managed block in `AGENTS.md`
- an intent-independent local policy overlay at `.volicord/policy.json`
- host hook configuration, such as Codex `.codex/hooks.json` or Claude Code
  `.claude/settings.local.json` for personal init and `.claude/settings.json`
  for shared init
- Volicord-managed hook wrapper scripts under Codex `.codex/hooks/` or Claude
  Code `.claude/hooks/`
- Volicord-managed host rule files, such as Codex `.codex/rules/*.rules` or
  Claude Code files under `.claude/rules/`
- for every Git-backed init, a Volicord-managed block in the worktree's
  effective Git `info/exclude`; this is untracked Git metadata, not a Product
  Repository file

A requested guard-integration managed-file application may use
implementation-private sibling entries in the target directory for staging,
displacement, or rollback. Those entries are transient parts of the one
requested integration-file write; they are not additional baseline
integration-file types, Runtime Home data, managed host configuration, or
durable recovery records. A successful application or a verified rollback must
remove every sibling entry still owned by that attempt. When automatic recovery
stops to avoid overwriting concurrently changed bytes, the command may leave
only the entries it reports as present at inspection. Their names and retention
are not stable interfaces. Inspect a reported entry before any manual deletion
or replacement.

For Record-profile init, the default personal Codex connection uses the Codex
user configuration target; explicit `--shared` adds the project-scoped
`.codex/config.toml` target. Both intents still apply the repository-local
`.volicord/policy.json` and managed Volicord guidance block in `AGENTS.md`.
Claude Code personal init uses only the local CLI target for MCP registration;
shared init selects the repository `.mcp.json` project file as the primary host
target. Personal Claude Code detective hooks use
`.claude/settings.local.json`, while shared detective hooks use
`.claude/settings.json`. Both intents keep `/.volicord/` and generated hook
wrapper scripts untracked through Git `info/exclude` without changing
`.gitignore`; a standalone personal init also protects its personal hook
configuration and rule paths. `.volicord/policy.json` declares
`storage_scope=local_overlay` and records the selected `connection_intent`; it
must not be committed as a shared projection. Generated wrapper scripts are
also local because they carry process-binding paths and identifiers. Every
managed lifecycle or final-output wrapper exports the init-selected absolute
`VOLICORD_HOME` and invokes the installation profile's absolute
`volicord_command`; it does not trust an ambient host Runtime Home or a
PATH-resolved bare command.

For one Product Repository, these repository-local surfaces represent one
selected built-in host adapter, active intent, and profile. An init that changes the
selected host, intent, or profile must preserve unrelated mixed-owner content,
retire matching Volicord-owned prior-host, opposite-intent, or
no-longer-applicable projections, and keep the union of prior and requested
local-only Git excludes until migration succeeds. Safe retirement is
conditional on the planned ownership marker, projection, or managed
fingerprint; changed or unmanaged content is a conflict.

Codex `.codex/hooks.json` is exact-owned by the Volicord integration and
different existing JSON is a conflict. Claude Code
`.claude/settings.local.json` preserves unrelated settings through a managed
projection, but the host treats the whole file as local, so personal init
excludes the complete path. Shared hook configuration and rule files remain
repository-visible. Whether to commit those shared surfaces is a Product
Repository policy decision. The Volicord MCP entry in a shared
`.codex/config.toml` or `.mcp.json` is clone-portable only in the exact
host-specific repository-discovery shape: command `volicord`, arguments
`mcp --stdio --discover-repository --host codex|claude-code`, and one portable
Runtime Home forwarding directive. Codex uses
`env_vars = ["VOLICORD_HOME"]`; Claude Code uses
`"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`. Neither form embeds a Runtime
Home path. Connection/project IDs, absolute commands, literal Runtime Home
paths, and secret-like or other environment keys belong only to local overlays
and are invalid in that shared entry. Each clone must still be registered with
a unique enabled shared connection in its selected local Runtime Home, and the
launching host must provide that same nonempty, absolute `VOLICORD_HOME`.

For a linked worktree, the common `info/exclude` contains only the
intent-independent policy and wrapper paths read safely by every sibling.
Personal Detective init is rejected before applying files because its
additional personal-only paths cannot be isolated in that common metadata.
These integration surfaces are not Runtime Home data.

Rules:

- The administrative command must preview the exact target path and content before applying the write.
- Project-scoped host MCP configuration must use the explicit shared-intent
  command path. Other init-owned repository integration files use the explicit
  init command and conflict behavior defined by
  [Administrative CLI](admin-cli.md#noninteractive-approval-behavior).
- The write must use Volicord ownership markers or a managed fingerprint.
- Existing unmanaged content must be reported as a conflict rather than overwritten.
- Replacement may apply only to matching Volicord-managed content.
- Guard-integration conditional creation or replacement must reject a changed
  target or parent path instead of treating a stale plan as authority. Exact
  commit, rollback, residual-entry reporting, and metadata behavior remain with
  [Administrative CLI](admin-cli.md#agent-host-setup-and-init).
- Safe removal may remove only matching Volicord-managed content and must leave unrelated project files intact.
- These files are host configuration or guidance. They are not Volicord runtime state, Core authority, evidence, acceptance, close readiness, residual-risk acceptance, or a security guarantee.
- Cwd-independent hook commands and managed wrapper path verification are host
  configuration health checks. They do not make these files Volicord runtime
  state and do not provide OS sandboxing, command blocking, network blocking,
  secret blocking, or global filesystem interception.

Detective coverage is bounded observation. Host hooks and the session watcher
may surface unrecorded Product Repository changes after coverage starts, and
status or close-readiness results may report `CoverageSummary` fields for the
active profile, host-hook state, session-watcher state, coverage start,
snapshot status timestamp, unresolved unrecorded-change count, and coverage
non-guarantees. That summary does not provide full filesystem monitoring,
actor identity proof, write prevention, tamper-proof audit, OS enforcement, or
security isolation.

The session watcher is a bounded Product Repository snapshot scanner, not a
full monitoring service. By default it skips `.git/`, `.hg/`, `.svn/`, `.jj/`,
`.volicord/`, `target/`, `node_modules/`, `dist/`, `build/`, `coverage/`, and
`vendor/`, and the Runtime Home/Product Repository path-separation rule keeps
the selected `Volicord Runtime Home` outside the scanned repository. It does
not follow symlinks by default. User-facing status, guard, doctor, and
coverage summaries can report `files_scanned`, `files_skipped`,
`unreadable_paths_count`, degraded reason counts for file-count limit,
file-size limit, unreadable path, skipped-by-policy path, and symlink skipped,
plus a sample of skipped paths.

<a id="product-repository-api-path-normalization"></a>
### Product Repository API path normalization

These rules apply when an API, schema, or method owner identifies a field as a `Product Repository` product path.

Rules:
- API product paths are repository-relative paths inside the `Product Repository`.
- Absolute paths are invalid as `Product Repository` API paths.
- Path normalization resolves `.` segments and non-escaping `..` segments; a path that would escape the repository via `..` is invalid.
- Symlinks that resolve outside the `Product Repository` are invalid for `Product Repository` path fields.
- Internal path comparisons use normalized repo-relative paths.
- API responses record normalized relative paths only.

Does not imply:
- These path rules do not provide OS sandboxing, command blocking, network blocking, secret blocking, or baseline detective enforcement.
- Write-ticket compatibility applies only to a proposed product-file change recorded through the Core-owned method path; it is not global filesystem interception, shell permission, command approval, or proof that a write occurred.
- Method-specific compatibility decisions stay with API method owners.

<a id="runtime-location-source-installation-processes"></a>
### Volicord source repository, installation, and processes

Volicord implementation names the implementation set maintained by this repository. Use `Volicord source repository` for the checkout that contains code, documentation, tests, validation tooling, and repository configuration. Use `Volicord installation` for deployed executables and required runtime resources.

May claim:
- `volicord` is the administrative CLI/process within Volicord implementation.
- `volicord mcp --stdio` is the local stdio MCP adapter process within Volicord implementation.
- A Volicord installation can be separate from the source repository, `Volicord Runtime Home`, `Product Repository`, and MCP host configuration.
- A Volicord installation does not need to include every source-repository file.
- In the baseline local Rust implementation, an MCP host starts `volicord mcp --stdio` as a child process and communicates through stdio.

Must not claim:
- Volicord implementation is the Volicord product/system as a whole.
- Volicord implementation is Core or the local authority record for Volicord state.
- Volicord implementation is only `volicord`, only `volicord mcp --stdio`, one long-running daemon, or one network service.
- `volicord mcp --stdio` is separate from Volicord implementation rather than an executable role within it.
- Installing or running Volicord from a directory makes that directory `Volicord Runtime Home`.
- The installation location proves that runtime data exists there.
- The installation path grants Volicord authority, security authority, or product-file write authority.
- The term Volicord implementation by itself means a TCP, HTTP, socket, or other network listener.

### Baseline local MCP process

The current local Rust MCP adapter is the `volicord mcp --stdio` stdio process, an executable role within Volicord implementation. An MCP host may label the configured entry an MCP server for protocol or host-configuration purposes. That label does not make Volicord a server product or make Volicord implementation a network server. An MCP host starts `volicord mcp --stdio` as a child process, passes configuration through process environment, and exchanges line-delimited JSON-RPC through stdin/stdout. The MCP transport itself opens no TCP, HTTP, Unix-domain socket, or other network transport listener. Unless disabled by `VOLICORD_LOCAL_WEB_CONSENT`, the same process attempts to bind an ephemeral loopback-only HTTP listener for local User Channel consent. Failure to start that optional listener does not prevent stdio startup. The consent listener is not the MCP transport; its exact behavior belongs to [MCP Transport](mcp-transport.md#local-web-consent-fallback).

For a Volicord-managed launch, the process environment must bind the MCP child
to the Runtime Home selected by init. Personal/local configuration supplies the
selected absolute path directly. Clone-portable shared configuration forwards
the launching host's `VOLICORD_HOME` without embedding the path, and
repository-discovery startup requires that value to be present and nonempty.
It must also be absolute and does not fall back to a platform-default Runtime
Home.

The separate `volicord serve --transport local-http` mode is a Local HTTP
transport for local/Docker use. It is not the baseline stdio process and must
not be treated as a public network API, SaaS endpoint, multi-user server, or
security boundary. Exact listener, authentication, Origin, and HTTP wire
behavior belongs to [MCP Transport](mcp-transport.md).

Exact executable behavior, environment variables, framing, startup validation or preflight behavior, response wrapping, shutdown, and reconnection rules belong to [MCP Transport](mcp-transport.md). This runtime-boundaries owner only keeps the process, location, and non-inference boundaries distinct.

### Agent Connections and Connection Projects

An Agent Connection is the local MCP host connection unit for `volicord mcp --stdio`. The connection has `connection_internal_id`, a connection intent of `personal`, `shared`, or `global`, host scope, `connection.mode=workflow` or `connection.mode=read_only`, and can address only the explicitly allowed `project_internal_id` values in its Connection Projects allowlist. User-facing administrative commands select the connection by host, intent, and repository root rather than requiring internal identities. MCP-visible project selection uses a `project_selector` returned by Volicord.

An Agent Connection can request user actions through supported API paths, but it cannot record authority-bearing user-action resolutions. Those resolutions are recorded through the `User Channel` with `actor_source=local_user`.

Must not infer:
- A copied `connection_id` process-binding value proves authority, user identity, OS permission, host trust, or capability.
- `connection.mode=workflow` grants filesystem, shell, network, secret, deployment, or Product Repository write permission.
- A Connection Projects allowlist turns every registered project into an allowed project.
- An Agent Connection can record final acceptance, residual-risk acceptance, sensitive-action approval, cancellation, or scope decisions on behalf of the user.

### External MCP host configuration

MCP host configuration belongs to the external MCP host. Volicord administrative commands may install configuration for an accepted managed-host target directly when [Administrative CLI](admin-cli.md) defines that behavior; user-managed external host configuration remains a host-owned surface. This document only owns the location boundary.

May claim:
- Host configuration can name a `volicord mcp --stdio` executable, an internal Agent Connection binding, and environment values needed by that host.
- Host configuration can live outside the source repository, installation files, `Volicord Runtime Home`, and `Product Repository`.
- Shared repository host configuration can forward `VOLICORD_HOME` without
  embedding its local path; generated local hook wrappers can bind the selected
  absolute path and absolute `volicord_command` while remaining explicit local
  integration files.

Must not claim:
- MCP host configuration is Volicord runtime state by definition.
- MCP host configuration is the local authority record, a Product Repository file, or proof of Volicord authority.
- A host configuration directory is automatically `Volicord Runtime Home`.
- Installing host configuration means the host has trusted, approved, loaded, initialized, or exposed the MCP server.

<a id="runtime-location-runtime-home"></a>
### `Volicord Runtime Home`

`Volicord Runtime Home` is the runtime storage location for Volicord runtime data.

May claim:
- Storage/runtime owners define what operational data belongs in `Volicord Runtime Home`.
- Storage/runtime owners define validation, storage effects, record layout, artifact storage, versioning, and recovery behavior for that data.
- `diagnostics.sqlite` may live at the Runtime Home root while remaining separate from the registry and every project authority database. Its location does not give its observations Core, evidence, close-readiness, User Channel, or security authority.

Must not claim:
- `Volicord Runtime Home` is the `Product Repository`.
- `Volicord Runtime Home` is a Volicord installation location by default.
- `Volicord Runtime Home` is automatically a security boundary.
- `Volicord Runtime Home` provides isolation by default.

<a id="runtime-home-product-repository-separation"></a>
### Runtime Home/Product Repository path separation

A valid registered project must use a `Volicord Runtime Home` and `Product Repository` whose resolved filesystem paths are separate and have no ancestor-descendant relationship.

Prohibited relationships:

| Relationship | Contract |
|---|---|
| Same resolved path | `Volicord Runtime Home` and `Product Repository` must not resolve to the same path. |
| `Product Repository` inside `Volicord Runtime Home` | A `Product Repository` must not be located within `Volicord Runtime Home`. |
| `Volicord Runtime Home` inside `Product Repository` | `Volicord Runtime Home` must not be located within a `Product Repository`. |

Permitted relationship:
- Separate resolved paths with no ancestor-descendant relationship are permitted.
- This rule does not prohibit intentionally selecting the Volicord source repository as a `Product Repository` when that source repository remains separate from `Volicord Runtime Home`.

This separation contract is an eligibility rule. New project registration, profile reuse, project-state administrative access, Core execution entry, and MCP project-session startup must require the selected `Volicord Runtime Home` and registered `Product Repository` to satisfy it.

On native Windows, Runtime Home/Product Repository boundary validation accepts
local drive-letter paths and rejects UNC paths, WSL UNC paths such as
`\\wsl$\...`, and WSL mount-style paths such as `/mnt/c/...`. Windows boundary
comparison is component-aware and case-insensitive after path normalization.
Symlink or junction aliases that resolve to the same path or to an
ancestor-descendant relationship are invalid where the host filesystem exposes
that resolution to Volicord.

The inspection layer may still show a raw stored project row that violates this contract so the record can be diagnosed. Operational project lookup, project listing, profile reuse, project-state administrative access, Agent Connection administration, Connection Projects access, Core execution entry, and MCP project availability must reject that row rather than returning it as a normal project record or project entry. The system does not automatically move paths, repair the registry row, or delete that record solely because inspection can report it.

## Local authority boundaries

Local access to a file or directory is not the same as Volicord authority.

May claim:
- A local actor may have filesystem access to product files, installation files, MCP host configuration, or runtime data locations according to the host environment.
- Volicord authority depends on documented API, storage, runtime, security, and user-judgment contracts.

Must not claim:
- A local path, directory name, copied identifier, rendered display, chat message, connector description, or agent memory proves Volicord authority.
- Direct local modification outside documented Volicord contracts creates valid Volicord records, evidence, acceptance, residual-risk acceptance, write ticket, or artifact authority.
- The location of runtime data changes the security guarantee level by itself.

## Runtime location, storage, and security owners

Runtime location is a boundary statement, not a storage layout or security mechanism.

Storage owners define:
- which Volicord records, metadata, artifact data, and operational diagnostics belong in `Volicord Runtime Home`
- how those records are shaped, initialized, versioned, validated, and updated
- which method branches create storage effects

Security owns:
- guarantee levels and non-guarantees
- local connection assumptions and access-boundary wording
- whether a claim may use `cooperative` or connection-observation `detective` wording
- the non-claim that `Volicord Runtime Home` is not automatically a security boundary

This document only keeps the locations and non-inference rules distinct.

## Related owners

- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md): storage record layout, effects, artifacts, schema initialization, versioning, and runtime data details.
- [API Methods](api/methods.md) and method owner documents: method routing and method behavior.
- [Core Model](core-model.md): Core authority, User Channel user-action boundaries, `actor_source`, write ticket, acceptance, and residual risk.
- [Security](security.md): security claims, non-claims, trust boundaries, guarantee levels, `operation_category`, and Agent Connection authority non-inference.
- [Projection Authority Reference](projection-and-templates.md): projection authority and freshness boundaries.
- [Template Bodies](template-bodies.md): rendered template body contracts.
