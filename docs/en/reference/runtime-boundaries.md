# Runtime boundaries reference

This document owns the location boundary among `Product Repository`, Harness installation or runtime process, and `Harness Runtime Home`. It defines local access assumptions for those locations and routes storage and security details to their owners.

## Owns / does not own

| This document owns | This document does not own |
|---|---|
| The definition of `Product Repository`. | Storage record layout, locks, migrations, versioning, or artifact lifecycle details. |
| The definition of `Harness Runtime Home`. | API method behavior or public schema shapes. |
| The separation between product files, installation/runtime files, and runtime data. | Detailed security guarantee meanings or security non-guarantees. |
| Local access and location non-authority rules. | Projection authority, template bodies, or rendered display freshness. |
| The rule that runtime location does not by itself prove Harness authority, security authority, or isolation. | Product scope, close readiness, evidence sufficiency, or user-owned judgment meaning. |

## Location model

Harness keeps three local location boundaries distinct.

| Boundary | Definition | Must not infer |
|---|---|---|
| `Product Repository` | The user's project workspace, including product source, product documentation, tests, configuration, and other project files. | It is not Harness runtime state, not `Harness Runtime Home`, and not proof of Harness authority. |
| Harness installation or runtime process | The process, package, application resources, and configuration used to run Harness behavior. | The installation location is not automatically the runtime data location. |
| `Harness Runtime Home` | The per-user or per-installation operational data space for Harness-owned records, local runtime metadata, and artifact data as storage/runtime owners define them. | It is not the `Product Repository`, not automatically a security boundary, and not isolation by default. |

<a id="runtime-location-product-repository"></a>
### `Product Repository`

`Product Repository` is the user's project workspace.

May claim:
- Product files can be inspected as inputs to owner-defined Harness checks or user-owned judgments.
- Compatible product-file writes can be governed by the active scope, Change Unit, required judgments, and write-authorization owners.

Must not claim:
- `Product Repository` content is Harness state.
- `Product Repository` content is generated Harness output.
- `Product Repository` content proves Harness authority.
- A `Product Repository` is automatically `Harness Runtime Home`.

<a id="runtime-location-server-installation"></a>
### Harness installation or runtime process

Harness installation or runtime process location is where Harness executable code, packages, application resources, or process configuration may live.

May claim:
- The runtime process mediates Harness API behavior and Harness records through documented owner contracts.
- Installation resources and runtime data can live in different locations.

Must not claim:
- Installing or running Harness from a directory makes that directory `Harness Runtime Home`.
- The installation location proves that runtime data exists there.
- The installation path grants Harness authority, security authority, or product-file write authority.

<a id="runtime-location-runtime-home"></a>
### `Harness Runtime Home`

`Harness Runtime Home` is the operational data space for Harness runtime data.

May claim:
- Storage/runtime owners define what operational data belongs in `Harness Runtime Home`.
- Storage/runtime owners define validation, storage effects, record layout, artifact storage, versioning, and recovery behavior for that data.

Must not claim:
- `Harness Runtime Home` is the `Product Repository`.
- `Harness Runtime Home` is server installation storage by default.
- `Harness Runtime Home` is automatically a security boundary.
- `Harness Runtime Home` provides isolation by default.

## Local access boundaries

Local access to a file or directory is not the same as Harness authority.

May claim:
- A local actor may have filesystem access to product files, installation files, or runtime data locations according to the host environment.
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
- The directory selected as `Harness Runtime Home`.
- A copied `surface_id`.
- A displayed `ArtifactRef`.
- A rendered `Projection`, status card, or template output.
- Connector prose, chat text, or agent memory.

Do not infer that:

- `Product Repository` is `Harness Runtime Home`.
- Installation location and runtime data location are the same.
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
