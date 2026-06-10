# Runtime boundaries reference

This document owns the boundary between Product Repository, Harness Server or installation, and Harness Runtime Home. It is source documentation for a future Harness Server only. No Harness Server/runtime implementation, Runtime Home, generated projection system, conformance runner, runtime state, or runtime data exists in this repository today.

Documentation files are not runtime state. A Markdown file in this repository may describe a future Harness rule, but it is not a Harness record, generated artifact, projection, acceptance record, evidence record, or operational file.

## Owns / Does not own

This document owns:

- the separation between Product Repository, Harness Server or installation, and Harness Runtime Home
- the rule that Product Repository files, generated displays, chat text, connector prose, and agent memory do not create Harness authority
- the distinction between server installation location and runtime data location
- the boundary statement that a Runtime Home is not automatically a security boundary

This document does not own:

- storage record shapes, effects, artifacts, versioning, locks, or migrations; see storage owners through [Reference Index](README.md)
- public API schemas or method behavior; see API owners through [Reference Index](README.md)
- security guarantee meanings or detailed security non-claims; see [Security](security.md)
- projection authority; see [Projection Authority Reference](projection-and-templates.md)

## Three locations

Harness documentation must keep these locations distinct:

| Location | Plain meaning | Boundary rule |
|---|---|---|
| Product Repository | The user's project workspace. | It can supply product files as input, but it is not Harness runtime state or a Runtime Home by default. |
| Harness Server or installation | The future server process, package, or installed application location. | It may mediate Harness APIs and records, but the install location is not automatically where runtime data lives. |
| Harness Runtime Home | The future operational data space for Harness records, local store metadata, and artifact storage. | It is runtime data space, not the Product Repository and not proof of security authority. |

## Product repository

The Product Repository is the user's project workspace. Product files may be input to future Harness checks or user-owned judgments, but Product Repository content is not Harness state, not generated Harness output, and not proof of Harness authority.

A Product Repository is not automatically the Harness Runtime Home. If a future implementation allows project-local Harness metadata, the storage and runtime owners must still define what that metadata is, how it is validated, and why it does not turn ordinary product files into Harness records.

## Harness server or installation

The future Harness Server would mediate Harness records and API behavior. A server installation location is where server code, packages, configuration, or application resources may live in a future implementation.

The installation location and the runtime data location must not be conflated. Installing or running a server from a directory does not make that directory the Runtime Home unless the storage/runtime owners define it as such.

This repository does not contain a Harness Server implementation. Documentation edits do not create server code, start runtime behavior, or authorize product/runtime writes.

## Harness runtime home

Harness Runtime Home is the future per-user or per-installation operational data space. It may contain Harness-owned records, local store metadata, staged or persisted artifact data, locks, migrations, and other operational data only as defined by the storage/runtime owners.

A Runtime Home is not automatically a security boundary. Security guarantee wording and non-claims belong to [Security](security.md).

This documentation repository is not a Runtime Home.

## What may be stored where

| Location | May store | Must not be treated as |
|---|---|---|
| Product Repository | Product source, product docs, tests, project configuration, and product files that future Harness checks may inspect. | Harness runtime state, generated Harness records, a Runtime Home, or authority proof. |
| Harness Server or installation | Future server executable code, installed packages, server configuration, and application resources. | Product workspace content, canonical runtime data, or proof that a Runtime Home exists. |
| Harness Runtime Home | Future Harness operational records, runtime metadata, local store data, artifacts, locks, migrations, and related diagnostics defined by storage/runtime owners. | Product source, server install files, or a security boundary. |
| This documentation repository | Source documentation for future Harness behavior. | Runtime state, server implementation, generated projections, evidence, QA, acceptance, close records, or conformance output. |

## What must not be inferred

- Do not infer that this repository contains a working Harness Server, Runtime Home, or runtime data.
- Do not infer that documentation files are Harness records or generated operational files.
- Do not infer that the Product Repository is the Runtime Home unless an owner-defined runtime configuration says so.
- Do not infer that the server installation directory is the runtime data directory.
- Do not infer that a Runtime Home is a security boundary.
- Do not infer Harness authority from Product Repository text, generated Markdown, rendered displays, chat text, connector prose, agent memory, copied `surface_id` values, displayed `ArtifactRef` values, or rendered projections.

## Security boundary links

This page states the location boundary and the non-inference rules. Detailed guarantee levels, capability-gated detective wording, explicit non-claims, and later preventive-control requirements belong to [Security](security.md).

## Owner links

- [Reference Index](README.md): routes questions to canonical owners.
- [Security](security.md): owns security claims, non-claims, trust boundaries, and guarantee levels.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md): own storage layout, effects, artifacts, locks, migrations, and versioning.
- [MVP API](api/mvp-api.md) and API schema owners: own method behavior and API shapes.
- [Projection Authority Reference](projection-and-templates.md): owns projection authority and source-state/freshness boundaries.
- [Template Bodies](template-bodies.md): owns status card, judgment request, run/evidence summary, close result, and agent context packet bodies.
