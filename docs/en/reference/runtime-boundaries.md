# Runtime boundaries reference

This document owns the component and location boundaries among `Harness Server`, `Product Repository`, `Harness Runtime Home`, and external MCP host configuration. It defines local access assumptions for those locations and routes storage and security details to their owners.

`Harness Server` is the server implementation set maintained by this repository. It is not Harness as a whole, not Core, not one running process, and not the local authority record for Harness state.

## Owns / does not own

| This document owns | This document does not own |
|---|---|
| The distinction between Harness as the product/system and `Harness Server` as the repository-maintained server implementation set. | Public API behavior, public schema shapes, or method-specific effects. |
| The distinction among Harness Server source repository, Harness Server installation, and running executable roles. | Release packaging policy or a mandatory installation-root layout. |
| The definition of `Product Repository` and Product Repository API path normalization. | Storage record layout, locks, migrations, versioning, or artifact lifecycle details. |
| The definition of `Harness Runtime Home`. | API method behavior or public schema shapes. |
| The separation between Harness Server files, product files, runtime data, and external MCP host configuration, including the exact Runtime Home/Product Repository path relationship contract. | Detailed security guarantee meanings or security non-guarantees. |
| Local access and location non-authority rules. | Projection authority, template bodies, or rendered display freshness. |
| The rule that runtime location does not by itself prove Harness authority, security authority, or isolation. | Product scope, close readiness, evidence sufficiency, or user-owned judgment meaning. |

## Component and artifact model

Harness keeps product, server implementation, executable-role, and authority-record concepts distinct.

| Term | Definition | Must not infer |
|---|---|---|
| Harness | The broader local work-authority product/system for AI-assisted product work. | It is not Core, not a source repository, and not a single executable process. |
| Core | The local authority record for Harness state. | It is not the whole Harness product/system and not an adapter or CLI executable. |
| `Harness Server` | The server implementation set maintained by this repository. At source level, it includes implementation crates, the `harness` administrative CLI, the `harness-mcp` local MCP adapter, tests, documentation, validation tooling, and repository configuration. | It is not every possible Harness product surface, not Core by itself, not `Harness Runtime Home`, not the `Product Repository`, and not one daemon or network service. |
| Harness Server source repository | The checked-out source artifact for this repository. | It is not the same thing as a deployed installation, running process, Runtime Home, Product Repository, or MCP host configuration. |
| Harness Server installation | The deployed subset of Harness Server executables and required runtime resources. | It does not imply that documentation, tests, source files, or repository metadata are present in every installation. |
| `harness` administrative process | The administrative CLI executable/process within Harness Server. | It is not a synonym for Harness or for all of Harness Server. |
| `harness-mcp` MCP adapter process | The local stdio MCP adapter executable/process within Harness Server. | It is not separate from Harness Server and not the whole Harness Server by itself. |

When a behavior is performed by one executable role, name that role. Bare `Harness Server` should be reserved for the implementation set or for statements that apply to the set as a whole.

## Filesystem-location model

Harness keeps server files, product files, runtime data, and external host configuration distinct. There is no single mandatory filesystem root for the whole Harness Server implementation set.

| Location role | Definition | Must not infer |
|---|---|---|
| Harness Server source or installation files | A source checkout, or deployed executable files and required runtime resources for Harness Server. | This is not automatically `Harness Runtime Home`, not the `Product Repository`, not MCP host configuration, not proof of Harness authority, and not inherently a network listener. |
| `Product Repository` | The user's product-file boundary: project source, product documentation, tests, configuration, and other project files. | It is not Harness runtime state, not `Harness Runtime Home`, and not proof of Harness authority. |
| `Harness Runtime Home` | The runtime storage location for Harness-owned records, local runtime metadata, and artifact data as storage/runtime owners define them. | It is not the `Product Repository`, not server installation storage by default, not automatically a security boundary, and not isolation by default. |
| External MCP host configuration | Configuration owned by the external MCP host that may name a `harness-mcp` command, process environment, or host-specific binding. | It is not Harness runtime state, not `Harness Runtime Home`, not the `Product Repository`, and not Harness Server source or installation files by definition. |

<a id="runtime-location-product-repository"></a>
### `Product Repository`

`Product Repository` is the user's project workspace and product-file boundary.

May claim:
- Product files can be inspected as inputs to owner-defined Harness checks or user-owned judgments.
- Compatible product-file writes can be governed by the current scope, current Change Unit, required judgments, and `Write Authorization` owners.

Must not claim:
- `Product Repository` content is Harness state.
- `Product Repository` content is generated Harness output.
- `Product Repository` content proves Harness authority.
- A `Product Repository` is automatically `Harness Runtime Home`.

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
- Method-specific authorization decisions stay with API method owners.

<a id="runtime-location-server-installation"></a>
### Harness Server source, installation, and processes

`Harness Server` names the server implementation set maintained by this repository. Use `Harness Server source repository` for the checkout that contains code, documentation, tests, validation tooling, and repository configuration. Use `Harness Server installation` for deployed executables and required runtime resources.

May claim:
- `harness` is the administrative CLI/process within Harness Server.
- `harness-mcp` is the local stdio MCP adapter process within Harness Server.
- A Harness Server installation can be separate from the source repository, `Harness Runtime Home`, `Product Repository`, and MCP host configuration.
- A Harness Server installation does not need to include every source-repository file.
- In the baseline local Rust implementation, an MCP host starts `harness-mcp` as a child process and communicates through stdio.

Must not claim:
- `Harness Server` is the Harness product/system as a whole.
- `Harness Server` is Core or the local authority record for Harness state.
- `Harness Server` is only `harness`, only `harness-mcp`, one long-running daemon, or one network service.
- `harness-mcp` is separate from Harness Server rather than an executable role within it.
- Installing or running Harness from a directory makes that directory `Harness Runtime Home`.
- The installation location proves that runtime data exists there.
- The installation path grants Harness authority, security authority, or product-file write authority.
- The term `Harness Server` by itself means a TCP, HTTP, socket, or other network listener.

### Baseline local MCP process

The current local Rust MCP adapter is the `harness-mcp` stdio process, an executable role within Harness Server. An MCP host starts it as a child process, passes configuration through process environment, and exchanges line-delimited JSON-RPC through stdin/stdout. The baseline process opens no TCP, HTTP, Unix-domain socket, or other network listener.

Exact executable behavior, environment variables, framing, startup validation or preflight behavior, response wrapping, shutdown, and reconnection rules belong to [MCP Transport](mcp-transport.md). This runtime-boundaries owner only keeps the process, location, and non-inference boundaries distinct.

### External MCP host configuration

MCP host configuration belongs to the external MCP host. Harness setup may render host-neutral configuration text or files when an administrative command owner defines that behavior, but this document only owns the location boundary.

May claim:
- Host configuration can name a `harness-mcp` executable and environment values needed by that host.
- Host configuration can live outside the source repository, installation files, `Harness Runtime Home`, and `Product Repository`.

Must not claim:
- MCP host configuration is Harness runtime state by definition.
- MCP host configuration is the local authority record, a Product Repository file, or proof of Harness authority.
- A host configuration directory is automatically `Harness Runtime Home`.

<a id="runtime-location-runtime-home"></a>
### `Harness Runtime Home`

`Harness Runtime Home` is the runtime storage location for Harness runtime data.

May claim:
- Storage/runtime owners define what operational data belongs in `Harness Runtime Home`.
- Storage/runtime owners define validation, storage effects, record layout, artifact storage, versioning, and recovery behavior for that data.

Must not claim:
- `Harness Runtime Home` is the `Product Repository`.
- `Harness Runtime Home` is server installation storage by default.
- `Harness Runtime Home` is automatically a security boundary.
- `Harness Runtime Home` provides isolation by default.

<a id="runtime-home-product-repository-separation"></a>
### Runtime Home/Product Repository path separation

A valid registered project must use a `Harness Runtime Home` and `Product Repository` whose resolved filesystem paths are separate and have no ancestor-descendant relationship.

Prohibited relationships:

| Relationship | Contract |
|---|---|
| Same resolved path | `Harness Runtime Home` and `Product Repository` must not resolve to the same path. |
| `Product Repository` inside `Harness Runtime Home` | A `Product Repository` must not be located within `Harness Runtime Home`. |
| `Harness Runtime Home` inside `Product Repository` | `Harness Runtime Home` must not be located within a `Product Repository`. |

Permitted relationship:
- Separate resolved paths with no ancestor-descendant relationship are permitted.
- This rule does not prohibit intentionally selecting the `Harness Server` source repository as a `Product Repository` when that source repository remains separate from `Harness Runtime Home`.

This separation contract is an eligibility rule. New project registration, setup reuse, project-state administrative access, Core execution entry, and MCP project-session startup must require the selected `Harness Runtime Home` and registered `Product Repository` to satisfy it.

Registry-level inspection may still show a stored legacy project record that violates this contract so the record can be diagnosed. Registry visibility does not make the record eligible to open the project-state database, perform surface administration, enter Core execution, or start an MCP project session. The system does not automatically move paths, repair the registry row, or delete that record solely because it remains visible.

## Local access boundaries

Local access to a file or directory is not the same as Harness authority.

May claim:
- A local actor may have filesystem access to product files, installation files, MCP host configuration, or runtime data locations according to the host environment.
- Harness authority depends on documented API, storage, runtime, security, and user-judgment contracts.

Must not claim:
- A local path, directory name, copied identifier, rendered display, chat message, connector description, or agent memory proves Harness authority.
- Direct local modification outside documented Harness contracts creates valid Harness records, evidence, acceptance, residual-risk acceptance, `Write Authorization`, or artifact authority.
- The location of runtime data changes the security guarantee level by itself.

## Runtime location, storage, and security owners

Runtime location is a boundary statement, not a storage layout or security mechanism.

Storage owners define:
- which Harness records, metadata, artifact data, and operational diagnostics belong in `Harness Runtime Home`
- how those records are shaped, versioned, validated, migrated, and updated
- which method branches create storage effects

Security owns:
- guarantee levels and non-guarantees
- local-access assumptions and access-boundary wording
- whether a claim may use `cooperative` or capability-gated `detective` wording
- the non-claim that `Harness Runtime Home` is not automatically a security boundary

This document only keeps the locations and non-inference rules distinct.

## What must not be inferred

Do not infer Harness authority, security authority, runtime state, or isolation from:

- `Product Repository` text or project files.
- The directory where Harness is installed or started.
- External MCP host configuration.
- The directory selected as `Harness Runtime Home`.
- A copied `surface_id`.
- A displayed `ArtifactRef`.
- A rendered `Projection`, status card, or template output.
- Connector prose, chat text, or agent memory.

Do not infer that:

- `Product Repository` is `Harness Runtime Home`.
- Installation location and runtime data location are the same.
- MCP host configuration is Harness runtime state or Harness authority.
- `Harness Runtime Home` is a security boundary.
- Product files are Harness records.
- Generated displays replace source-record authority.

## Related owners

- [Security](security.md): security claims, non-claims, trust boundaries, and guarantee levels.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md): storage record layout, effects, artifacts, migrations, versioning, and runtime data details.
- [API Methods](api/methods.md) and method owner documents: method routing and method behavior.
- [Core Model](core-model.md): Core authority, user-owned judgments, `Write Authorization`, acceptance, and residual risk.
- [Agent Integration](agent-integration.md): surface context and capability-profile boundaries.
- [Projection Authority Reference](projection-and-templates.md): projection authority and freshness boundaries.
- [Template Bodies](template-bodies.md): rendered template body contracts.
