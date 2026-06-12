# Runtime boundaries reference

This document owns the boundary between Product Repository, Harness Server or installation, and Harness Runtime Home.

Conditions:
- Use this page when a claim depends on where product files, server installation files, or Harness runtime data live.
- Treat this repository as source documentation for a Harness Server only.

May claim:
- A Markdown file in this repository may describe a Harness rule.
- Product Repository, Harness Server or installation, and Harness Runtime Home are distinct locations.

Must not claim:
- Documentation files are Harness Server/runtime implementation material, Runtime Home data, generated projection systems, conformance runners, runtime state, or runtime data.
- Documentation files are runtime state, Harness records, generated artifacts, projections, acceptance records, evidence records, or operational files.

Owner links:
- [Security](security.md) owns security guarantee meanings and non-claims.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Artifact Storage](storage-artifacts.md) own storage details.

## Owns / Does not own

This document owns:

- the separation between Product Repository, Harness Server or installation, and Harness Runtime Home
- the rule that Product Repository files do not create Harness authority
- the rule that generated displays, chat text, connector prose, and agent memory do not create Harness authority
- the distinction between server installation location and runtime data location
- the boundary statement that a Runtime Home is not automatically a security boundary

This document does not own:

- storage record shapes, effects, artifacts, versioning, locks, or migrations; see storage owners through [Reference Index](README.md)
- public API schemas or method behavior; see API owners through [Reference Index](README.md)
- security guarantee meanings or detailed security non-claims; see [Security](security.md)
- projection authority; see [Projection Authority Reference](projection-and-templates.md)

## Three locations

Harness documentation must keep these locations distinct:

| Location | Details |
|---|---|
| Product Repository | See [Product Repository location](#runtime-location-product-repository) |
| Harness Server or installation | See [Harness Server or installation location](#runtime-location-server-installation) |
| Harness Runtime Home | See [Harness Runtime Home location](#runtime-location-runtime-home) |

<a id="runtime-location-product-repository"></a>
### Product Repository location

Conditions:
- The user's project workspace.

May claim:
- It can supply product files as input.

Must not claim:
- It is Harness runtime state or a Runtime Home by default.

Owner links:
- [Storage Effects](storage-effects.md) owns product-file and Harness-record storage effects.
- [Security](security.md) owns security non-claims.

<a id="runtime-location-server-installation"></a>
### Harness Server or installation location

Conditions:
- The server process, package, or installed application location.

May claim:
- It may mediate Harness APIs and records.

Must not claim:
- The install location is automatically where runtime data lives.

Owner links:
- [API Methods](api/methods.md) routes method behavior to API owners.
- [Storage Records](storage-records.md) owns runtime data record layout.

<a id="runtime-location-runtime-home"></a>
### Harness Runtime Home location

Conditions:
- The operational data space for Harness records, local store metadata, and artifact storage.

May claim:
- Storage/runtime owners may define what operational data belongs there.

Must not claim:
- It is the Product Repository.
- It is proof of security authority.
- It provides isolation by default.

Owner links:
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Artifact Storage](storage-artifacts.md) own runtime data details.
- [Security](security.md) owns security guarantee wording.

## Product repository

The Product Repository is the user's project workspace.

Conditions:
- Product files may be input to Harness checks or user-owned judgments.
- Project-local Harness metadata is allowed only when storage/runtime owners define it.

May claim:
- Product Repository files are user workspace files.
- Owner-defined project-local Harness metadata may exist when the storage and runtime owners define it.

Must not claim:
- Product Repository content is Harness state.
- Product Repository content is generated Harness output.
- Product Repository content is proof of Harness authority.
- A Product Repository is automatically the Harness Runtime Home.
- Ordinary product files become Harness records because project-local metadata exists.

Owner links:
- [Storage Effects](storage-effects.md) owns product-file and Harness-record effects.
- [Core Model](core-model.md) owns user-owned judgment boundaries.
- [Security](security.md) owns security authority non-claims.

## Harness server or installation

The Harness Server mediates Harness records and API behavior.

Conditions:
- A server installation location is where server code, packages, configuration, or application resources may live.
- A directory is a Runtime Home only when storage/runtime owners define it as such.

May claim:
- A Harness Server mediates Harness records and API behavior.
- A server installation location can be distinct from runtime data storage.

Must not claim:
- The installation location and runtime data location are the same by default.
- Installing or running a server from a directory makes that directory the Runtime Home.
- Documentation files are executable server material.
- Documentation edits create server code, start runtime behavior, or authorize product/runtime writes.

Owner links:
- [API Methods](api/methods.md) routes API method behavior.
- [Storage Records](storage-records.md) owns runtime data record layout.
- [Storage Effects](storage-effects.md) owns product/runtime write effects.

## Harness runtime home

Harness Runtime Home is the per-user or per-installation operational data space.

Conditions:
- Storage/runtime owners define what operational data belongs in the Runtime Home.

May include:
- Harness-owned records.
- Local store metadata.
- Staged or persisted artifact data.
- Locks, migrations, and related diagnostics.

May claim:
- A Runtime Home can hold Harness operational data when storage/runtime owners define the data and validation rules.

Must not claim:
- A Runtime Home is the Product Repository.
- A Runtime Home is automatically a security boundary.
- This documentation repository is a Runtime Home.

Owner links:
- [Security](security.md) owns security guarantee wording and non-claims.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md) own storage and runtime data details.

## Storage location assumptions

Conditions:
- A claim names where product files, server installation files, Harness records, artifact data, or runtime metadata live.

May claim:
- Product Repository storage, Harness Server or installation storage, Harness Runtime Home storage, and documentation repository storage are separate assumptions.
- Storage/runtime owners may define operational data placement.

Must not claim:
- Product Repository storage, server installation storage, and Runtime Home storage are the same by default.
- A storage location proves Harness authority, security authority, or isolation.
- This documentation repository is runtime data storage.

Owner links:
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md) own storage details.
- [Security](security.md) owns security boundary and isolation non-claims.

| Location | Details |
|---|---|
| Product Repository | See [Product Repository storage](#runtime-storage-product-repository) |
| Harness Server or installation | See [Harness Server or installation storage](#runtime-storage-server-installation) |
| Harness Runtime Home | See [Harness Runtime Home storage](#runtime-storage-runtime-home) |
| This documentation repository | See [Documentation repository storage](#runtime-storage-documentation-repository) |

<a id="runtime-storage-product-repository"></a>
### Product Repository storage

Conditions:
- The location is the user's project workspace.

May claim:
- Product source, product docs, tests, project configuration, and product files that Harness checks may inspect.

Must not claim:
- Product Repository storage is Harness runtime state.
- Product Repository storage contains generated Harness records by default.
- Product Repository storage is a Runtime Home.
- Product Repository storage proves Harness authority.

Owner links:
- [Storage Effects](storage-effects.md) owns product-file effects.
- [Security](security.md) owns authority and guarantee non-claims.

<a id="runtime-storage-server-installation"></a>
### Harness Server or installation storage

Conditions:
- The location is the server process, package, or installed application location.

May claim:
- Server executable code, installed packages, server configuration, and application resources.

Must not claim:
- Harness Server or installation storage is product workspace content.
- Harness Server or installation storage is canonical runtime data.
- Harness Server or installation storage proves that a Runtime Home exists.

Owner links:
- [API Methods](api/methods.md) routes method behavior.
- [Storage Records](storage-records.md) owns runtime data records.

<a id="runtime-storage-runtime-home"></a>
### Harness Runtime Home storage

Conditions:
- Storage/runtime owners define the operational data space.

May claim:
- Harness operational records defined by storage/runtime owners.
- Runtime metadata and local store data defined by storage/runtime owners.
- Artifacts, locks, migrations, and related diagnostics defined by storage/runtime owners.

Must not claim:
- Harness Runtime Home storage is product source.
- Harness Runtime Home storage is server install files.
- Harness Runtime Home storage is a security boundary.
- Harness Runtime Home storage provides isolation by default.

Owner links:
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md) own runtime data details.
- [Security](security.md) owns security boundary claim wording and non-claims.

<a id="runtime-storage-documentation-repository"></a>
### Documentation repository storage

Conditions:
- The location is this documentation repository.

May claim:
- Source documentation for Harness behavior.

Must not claim:
- Documentation repository storage is runtime state.
- Documentation repository storage is executable server material.
- Documentation repository storage contains generated projections.
- Documentation repository storage contains evidence, QA, acceptance, close records, or conformance output.

Owner links:
- [Implementation Guide](../build/implementation-guide.md) owns implementation routing.
- [Security](security.md) owns security non-claims.

## What must not be inferred

Conditions:
- A reader sees Harness concepts in documentation, Product Repository text, generated Markdown, rendered displays, connector prose, chat text, or agent memory.
- A reader sees a copied `surface_id`, displayed `ArtifactRef`, rendered projection, install directory, or local directory name.

Must not claim or infer:
- Documentation files are a working Harness Server, Runtime Home, or runtime data.
- Documentation files are Harness records or generated operational files.
- The Product Repository is the Runtime Home unless an owner-defined runtime configuration says so.
- The server installation directory is the runtime data directory.
- A Runtime Home is a security boundary.
- Product Repository text, generated Markdown, rendered displays, or rendered projections create Harness authority.
- Chat text, connector prose, or agent memory creates Harness authority.
- Copied `surface_id` values or displayed `ArtifactRef` values create Harness authority.

Owner links:
- [Security](security.md) owns detailed authority, guarantee, and isolation non-claims.
- [Projection Authority Reference](projection-and-templates.md) owns projection authority and freshness boundaries.
- [Agent Integration](agent-integration.md) owns connector surface boundaries.

## Security boundary links

This page states the location boundary and the non-inference rules. Detailed guarantee levels, capability-gated detective wording, explicit non-claims, and stronger-control requirements belong to [Security](security.md).

## Owner links

- [Reference Index](README.md): routes questions to canonical owners.
- [Security](security.md): owns security claims, non-claims, trust boundaries, and guarantee levels.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md): own storage layout, effects, artifacts, locks, migrations, and versioning.
- [API Methods](api/methods.md), method owner documents, and API schema owners: own method routing, method behavior, and API shapes.
- [Projection Authority Reference](projection-and-templates.md): owns projection authority and source-state/freshness boundaries.
- [Template Bodies](template-bodies.md): owns status card, judgment request, run/evidence summary, close result, and agent context packet bodies.
