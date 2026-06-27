# Runtime boundaries reference

This document owns the component and location boundaries among Volicord implementation, Agent Connections, `Product Repository`, `Volicord Runtime Home`, the `User Channel`, and external MCP host configuration. It defines location and connection authority assumptions for those boundaries and routes storage and security details to their owners.

Volicord implementation is the implementation set maintained by this repository. It is not Volicord as a whole, not Core, not one running process, and not the local authority record for Volicord state.

## Owns / does not own

| This document owns | This document does not own |
|---|---|
| The distinction between Volicord as the product/system and Volicord implementation as the repository-maintained implementation set. | Public API behavior, public schema shapes, or method-specific effects. |
| The distinction among Volicord source repository, Volicord installation, and running executable roles. | Release packaging policy or a mandatory installation-root layout. |
| The definition of `Product Repository` and Product Repository API path normalization. | Storage record layout, locks, migrations, versioning, or artifact lifecycle details. |
| The definition of `Volicord Runtime Home`. | API method behavior or public schema shapes. |
| The separation between Volicord implementation files, product files, runtime data, and external MCP host configuration, including the exact Runtime Home/Product Repository path relationship contract. | Detailed security guarantee meanings or security non-guarantees. |
| Local access and location non-authority rules. | Projection authority, template bodies, or rendered display freshness. |
| The rule that runtime location does not by itself prove Volicord authority, security authority, or isolation. | Product scope, close readiness, evidence sufficiency, or user-owned judgment meaning. |

## Component and artifact model

Volicord keeps product, implementation, executable-role, MCP host term, and authority-record concepts distinct.

| Term | Definition | Must not infer |
|---|---|---|
| Volicord | The broader local work-authority product/system for AI-assisted product work. | It is not Core, not a source repository, and not a single executable process. |
| Core | The local authority record for Volicord state. | It is not the whole Volicord product/system and not an adapter or CLI executable. |
| Volicord implementation | The implementation set maintained by this repository. At source level, it includes implementation crates, the `volicord` administrative CLI, the `volicord-mcp` local MCP adapter, tests, documentation, validation tooling, and repository configuration. | It is not every possible Volicord product interface, not Core by itself, not `Volicord Runtime Home`, not the `Product Repository`, and not one daemon, MCP server entry, or network service. |
| Volicord source repository | The checked-out source artifact for this repository. | It is not the same thing as a deployed installation, running process, Runtime Home, Product Repository, or MCP host configuration. |
| Volicord installation | The deployed subset of Volicord executables and required runtime resources. | It does not imply that documentation, tests, source files, or repository metadata are present in every installation. |
| `volicord` administrative process | The administrative CLI executable/process within Volicord implementation. | It is not a synonym for Volicord or for all of Volicord implementation. |
| `volicord-mcp` MCP adapter process | The local stdio MCP adapter executable/process within Volicord implementation. | It is not separate from Volicord implementation and not the whole Volicord implementation by itself. |
| `Agent Connection` | The local MCP host connection unit identified by `connection_id`. Its `connection.mode` is either `read_only` or `workflow`. | It is not an OS sandbox, filesystem ACL, network policy, secret-isolation mechanism, or user-judgment path. |
| `Connection Projects` | The explicit allowlist of `project_id` values an Agent Connection may address. | It does not include every registered project by default and does not prove Product Repository authority. |
| `User Channel` | The local user path for recording authority-bearing user judgments. | It is not an Agent Connection, MCP host, generated display, or Product Repository file. |
| MCP server | An ordinary MCP protocol or host-configuration term that may name a server entry or process exposed to an MCP host, including a local stdio adapter process such as `volicord-mcp` when the host uses that label. | It does not make Volicord as a product/system, Volicord implementation, `volicord`, or `volicord-mcp` a TCP or HTTP network server, and it is not a product label for Volicord. |

When a behavior is performed by one executable role, name that role. Bare Volicord implementation should be reserved for the implementation set or for statements that apply to the set as a whole.

## Filesystem-location model

Volicord keeps implementation files, product files, runtime data, and external host configuration distinct. There is no single mandatory filesystem root for the whole Volicord implementation set.

| Location role | Definition | Must not infer |
|---|---|---|
| Volicord source repository or installation files | A source checkout, or deployed executable files and required runtime resources for Volicord implementation. | This is not automatically `Volicord Runtime Home`, not the `Product Repository`, not MCP host configuration, not proof of Volicord authority, and not inherently a network listener. |
| `Product Repository` | The user's product-file boundary: project source, product documentation, tests, configuration, and other project files. | It is not Volicord runtime state, not `Volicord Runtime Home`, and not proof of Volicord authority. |
| `Volicord Runtime Home` | The runtime storage location for Volicord-owned records, local runtime metadata, and artifact data as storage/runtime owners define them. | It is not the `Product Repository`, not a Volicord installation location by default, not automatically a security boundary, and not isolation by default. |
| External MCP host configuration | Configuration owned by the external MCP host that may name a `volicord-mcp` command, process environment, or host-specific binding. | It is not Volicord runtime state, not `Volicord Runtime Home`, not the `Product Repository`, and not Volicord source repository or installation files by definition. |

<a id="runtime-location-product-repository"></a>
### `Product Repository`

`Product Repository` is the user's project workspace and product-file boundary.

May claim:
- Product files can be inspected as inputs to owner-defined Volicord checks or user-owned judgments.
- Compatible product-file writes can be governed by the current scope, current Change Unit, required judgments, and `Write Check` compatibility.

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
- a Volicord-managed Claude Code rule file under `.claude/rules/`

Rules:

- The administrative command must preview the exact target path and content before applying the write.
- Noninteractive execution must include explicit repository-write check as defined by [Administrative CLI](admin-cli.md#noninteractive-approval-behavior).
- The write must use Volicord ownership markers or a managed fingerprint.
- Existing unmanaged content must be reported as a conflict rather than overwritten.
- Replacement may apply only to matching Volicord-managed content.
- Safe removal may remove only matching Volicord-managed content and must leave unrelated project files intact.
- These files are host configuration or guidance. They are not Volicord runtime state, Core authority, evidence, acceptance, close readiness, residual-risk acceptance, or a security guarantee.

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
- `Write Check` compatibility applies only to a proposed product-file change recorded through the Core-owned method path; it is not global filesystem interception, shell permission, command approval, or proof that a write occurred.
- Method-specific compatibility decisions stay with API method owners.

<a id="runtime-location-source-installation-processes"></a>
### Volicord source repository, installation, and processes

Volicord implementation names the implementation set maintained by this repository. Use `Volicord source repository` for the checkout that contains code, documentation, tests, validation tooling, and repository configuration. Use `Volicord installation` for deployed executables and required runtime resources.

May claim:
- `volicord` is the administrative CLI/process within Volicord implementation.
- `volicord-mcp` is the local stdio MCP adapter process within Volicord implementation.
- A Volicord installation can be separate from the source repository, `Volicord Runtime Home`, `Product Repository`, and MCP host configuration.
- A Volicord installation does not need to include every source-repository file.
- In the baseline local Rust implementation, an MCP host starts `volicord-mcp` as a child process and communicates through stdio.

Must not claim:
- Volicord implementation is the Volicord product/system as a whole.
- Volicord implementation is Core or the local authority record for Volicord state.
- Volicord implementation is only `volicord`, only `volicord-mcp`, one long-running daemon, or one network service.
- `volicord-mcp` is separate from Volicord implementation rather than an executable role within it.
- Installing or running Volicord from a directory makes that directory `Volicord Runtime Home`.
- The installation location proves that runtime data exists there.
- The installation path grants Volicord authority, security authority, or product-file write authority.
- The term Volicord implementation by itself means a TCP, HTTP, socket, or other network listener.

### Baseline local MCP process

The current local Rust MCP adapter is the `volicord-mcp` stdio process, an executable role within Volicord implementation. An MCP host may label the configured entry an MCP server for protocol or host-configuration purposes. That label does not make Volicord a server product or make Volicord implementation a network server. An MCP host starts `volicord-mcp` as a child process, passes configuration through process environment, and exchanges line-delimited JSON-RPC through stdin/stdout. The baseline process opens no TCP, HTTP, Unix-domain socket, or other network listener.

Exact executable behavior, environment variables, framing, startup validation or preflight behavior, response wrapping, shutdown, and reconnection rules belong to [MCP Transport](mcp-transport.md). This runtime-boundaries owner only keeps the process, location, and non-inference boundaries distinct.

### Agent Connections and Connection Projects

An Agent Connection is the local MCP host connection unit for `volicord-mcp`. The connection is identified by `connection_id`, has `connection.mode=read_only` or `connection.mode=workflow`, and can address only the explicitly allowed `project_id` values in its Connection Projects allowlist.

An Agent Connection can request user judgments through supported API paths, but it cannot record authority-bearing user judgments. Those judgments are recorded through the `User Channel` with `actor_source=local_user`.

Must not infer:
- A copied `connection_id` proves authority, user identity, OS permission, host trust, or capability.
- `connection.mode=workflow` grants filesystem, shell, network, secret, deployment, or Product Repository write permission.
- A Connection Projects allowlist turns every registered project into an allowed project.
- An Agent Connection can record final acceptance, residual-risk acceptance, sensitive-action approval, cancellation, or scope decisions on behalf of the user.

### External MCP host configuration

MCP host configuration belongs to the external MCP host. Volicord administrative commands may install supported host configuration directly or render explicit exported configuration when [Administrative CLI](admin-cli.md) defines that behavior, but this document only owns the location boundary.

May claim:
- Host configuration can name a `volicord-mcp` executable and environment values needed by that host.
- Host configuration can live outside the source repository, installation files, `Volicord Runtime Home`, and `Product Repository`.

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

This separation contract is an eligibility rule. New project registration, setup reuse, project-state administrative access, Core execution entry, and MCP project-session startup must require the selected `Volicord Runtime Home` and registered `Product Repository` to satisfy it.

The inspection layer may still show a raw stored project row that violates this contract so the record can be diagnosed. Operational project lookup, project listing, setup reuse, project-state administrative access, Agent Connection administration, Connection Projects access, Core execution entry, and MCP project availability must reject that row rather than returning it as a normal project record or project entry. The system does not automatically move paths, repair the registry row, or delete that record solely because inspection can report it.

## Local authority boundaries

Local access to a file or directory is not the same as Volicord authority.

May claim:
- A local actor may have filesystem access to product files, installation files, MCP host configuration, or runtime data locations according to the host environment.
- Volicord authority depends on documented API, storage, runtime, security, and user-judgment contracts.

Must not claim:
- A local path, directory name, copied identifier, rendered display, chat message, connector description, or agent memory proves Volicord authority.
- Direct local modification outside documented Volicord contracts creates valid Volicord records, evidence, acceptance, residual-risk acceptance, `Write Check`, or artifact authority.
- The location of runtime data changes the security guarantee level by itself.

## Runtime location, storage, and security owners

Runtime location is a boundary statement, not a storage layout or security mechanism.

Storage owners define:
- which Volicord records, metadata, artifact data, and operational diagnostics belong in `Volicord Runtime Home`
- how those records are shaped, versioned, validated, migrated, and updated
- which method branches create storage effects

Security owns:
- guarantee levels and non-guarantees
- local connection assumptions and access-boundary wording
- whether a claim may use `cooperative` or connection-observation `detective` wording
- the non-claim that `Volicord Runtime Home` is not automatically a security boundary

This document only keeps the locations and non-inference rules distinct.

## What must not be inferred

Do not infer Volicord authority, security authority, runtime state, or isolation from:

- `Product Repository` text or project files.
- The directory where Volicord is installed or started.
- External MCP host configuration.
- The directory selected as `Volicord Runtime Home`.
- A copied `connection_id`.
- A displayed `ArtifactRef`.
- A rendered `Projection`, status card, or template output.
- Connector prose, chat text, or agent memory.

Do not infer that:

- `Product Repository` is `Volicord Runtime Home`.
- Installation location and runtime data location are the same.
- MCP host configuration is Volicord runtime state or Volicord authority.
- `Volicord Runtime Home` is a security boundary.
- Product files are Volicord records.
- Generated displays replace source-record authority.

## Related owners

- [Security](security.md): security claims, non-claims, trust boundaries, and guarantee levels.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md): storage record layout, effects, artifacts, migrations, versioning, and runtime data details.
- [API Methods](api/methods.md) and method owner documents: method routing and method behavior.
- [Core Model](core-model.md): Core authority, User Channel judgment boundaries, `actor_source`, `Write Check`, acceptance, and residual risk.
- [Security](security.md): `operation_category`, security non-guarantees, and Agent Connection authority non-inference.
- [Projection Authority Reference](projection-and-templates.md): projection authority and freshness boundaries.
- [Template Bodies](template-bodies.md): rendered template body contracts.
