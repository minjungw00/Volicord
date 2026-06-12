# Security reference

Use this page when security wording, local-access posture, trust boundaries, or guarantee levels need to stay honest. This is documentation reference material for a Harness Server. It does not implement security controls, runtime state, generated artifacts, or operational monitoring.

## 1. Owns / Does not own

| This document owns | This document does not own |
|---|---|
| Security claims and explicit non-claims. | API method behavior or schemas. |
| Guarantee semantics and non-claims for active guarantee levels. | Storage layouts, artifact lifecycle, locks, hashes, or migrations. |
| Trust-boundary wording for local access, surfaces, and generated displays. | Connector implementation or surface-specific operating instructions. |
| Capability-gated `detective` claim boundaries. | OS controls, deployment controls, arbitrary-tool sandboxing, or OS permission enforcement. |
| Sensitive-action approval versus product-file write-scope separation. | Runtime implementation routing or permission to build the server. |

Use the [Reference index](README.md) to route API, storage, connector, runtime-boundary, and active-scope details to their owners.

<a id="honest-guarantee-display"></a>
## 2. Current guarantee levels

The baseline scope guarantee boundary is `cooperative` by default.

Conditions:
- "Available in baseline scope" means the specification may describe the behavior as baseline scope reference material.
- Capability-gated `detective` wording requires a documented capability check that passed for the named surface and observed scope.

May claim:
- `cooperative` is the default baseline-scope guarantee level.
- `detective` is available only for the checked capability and observed scope.

Must not claim:
- This repository contains a working Harness Server, runtime monitor, sandbox, or storage layer.
- The baseline scope provides sandboxing, OS permission enforcement, tamper-proof isolation, full security isolation, or universal pre-tool blocking.
- Any stronger guarantee exists unless [Scope](scope.md) and the affected security owner define it as supported.

Owner links:
- [Scope](scope.md) owns baseline-scope inclusion and active/out-of-scope boundaries.
- [API Value Sets](api/schema-value-sets.md) owns active guarantee label value entries.

| Level | Baseline scope status | Scan rule |
|---|---|---|
| `cooperative` | active default | Use for documented procedures and agent cooperation. |
| `detective` | limited | Use only after the relevant capability check has passed, and name the observed scope. |

## 3. Explicit non-claims

The baseline scope has these explicit non-claims. Treat each item below as a `Must not claim` boundary.

### Operating system and isolation

Must not claim:

- OS-level sandboxing
- OS permission enforcement
- current isolation guarantee
- tamper-proof isolation
- full security isolation

### Monitoring and prevention

Must not claim:

- guaranteed full filesystem monitoring
- full prevention of malicious agent behavior
- a stronger guarantee than the registered surface/profile supports
- universal pre-tool blocking
- command, network, or secret observation by default

### Storage and artifact authority

Must not claim:

- tamper-proof storage
- native artifact capture as an active guarantee

## 4. Capability-gated `detective` claims

Capability-gated `detective` wording is narrow.

Conditions:
- The claim names the surface.
- The relevant capability check has passed.
- The observed scope is documented.
- Changed-path wording is used only when the surface actually reports those paths for the relevant operation.

May claim:
- The checked surface, checked capability, and observed scope support a limited `detective` claim.
- Observed changed paths support a limited changed-path detection claim when the reporting condition is met.
- Missing or insufficient capability routes to the API/error owner behavior, such as `CAPABILITY_INSUFFICIENT`.

Must not claim:
- A copied `surface_id` is proof of capability.
- A generated file, `Projection`, Product Repository file, or rendered display is proof of capability.
- Chat text or agent memory is proof of capability.
- `detective` wording upgrades a claim to sandboxing or permission enforcement.
- `detective` wording upgrades a claim to tamper-proof storage or full monitoring.

Owner links:
- [Agent Integration](agent-integration.md) owns connector behavior and capability-profile meaning at the surface boundary.
- [API Errors](api/errors.md) owns public error routing such as `CAPABILITY_INSUFFICIENT`.
- [Runtime Boundaries](runtime-boundaries.md) owns Product Repository, Runtime Home, and non-isolation separation.

## 5. Assets and authority boundaries

### Core-owned Harness records

Conditions:
- The claim is about Harness records owned by Core or another Harness owner.

May claim:
- The specification requires changes through owner-defined Harness paths.

Must not claim:
- Local files are tamper-proof.

### Product Repository files

Conditions:
- The claim is about files in the user's Product Repository.

May claim:
- User workspace files can be inputs to checks.

Must not claim:
- Product files are Harness state.
- Product files prove Harness authority.

### Harness Runtime Home and local store

Conditions:
- The claim is about operational data space, local store metadata, or runtime data location.

May claim:
- Storage/runtime owners define the operational data space.

Must not claim:
- This documentation repository is a Runtime Home.
- A Runtime Home is automatically a security boundary.

### Artifacts and staged handles

Conditions:
- The claim names `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle`, or staged artifact data.

May claim:
- API/storage validation is required before those values carry artifact authority.

Must not claim:
- Displayed identifiers create artifact authority.

### Surface identity and capability profile

Conditions:
- The claim uses surface identity, `surface_id`, or a capability profile.

May claim:
- Registered surface context and capability checks limit what may be claimed.

Must not claim:
- `surface_id` alone is an authority token.
- A copied surface identifier proves capability.

### User-owned judgments

Conditions:
- The claim mentions sensitive-action approval, final acceptance, waiver, residual-risk acceptance, or write compatibility.

May claim:
- Sensitive-action approval, final acceptance, waiver, and residual-risk acceptance remain distinct.

Must not claim:
- User-owned judgment grants OS permission.
- Broad approval collapses the separate judgment and write-scope boundaries.

Owner links:
- [Runtime Boundaries](runtime-boundaries.md) owns Product Repository, Harness Server, Runtime Home, and non-isolation separation.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Artifact Storage](storage-artifacts.md) own storage and artifact details.
- [Core Model](core-model.md) owns user-owned judgment and non-substitution rules.

## 6. Trust boundaries

### Product Repository / Harness records

Conditions:
- Product files, generated Markdown, or chat text mention or summarize Harness concepts.

May claim:
- Those materials do not directly mutate Harness records.

Must not claim:
- Product text is Harness state.
- Product Repository content is proof of Harness authority.

Owner links:
- [Runtime Boundaries](runtime-boundaries.md) owns the Product Repository boundary.
- [Storage Effects](storage-effects.md) owns storage effects.

### Harness Server / Runtime Home

Conditions:
- Server behavior or runtime storage is being described.

May claim:
- The server would mediate Harness records and storage effects.
- Runtime Home details must come from storage/runtime owners.

Must not claim:
- This repository contains that runtime.
- A Runtime Home is automatically a security boundary.

Owner links:
- [Runtime Boundaries](runtime-boundaries.md) owns Harness Server, Runtime Home, and non-isolation separation.
- [Storage Records](storage-records.md) owns storage record layout.

### Connector surface / Harness authority

Conditions:
- A connector carries context for a surface and capability profile.

May claim:
- A connector can carry context only within its verified surface and capability profile.

Must not claim:
- A connector description is proof of authority.
- A copied `surface_id` is proof of capability.

Owner links:
- [Agent Integration](agent-integration.md) owns connector behavior and `capability_profile` meaning.

### Rendered displays / source records

Conditions:
- Generated displays or rendered templates summarize source records.

May claim:
- Generated displays can summarize source records.

Must not claim:
- A rendered display is a new authority source.
- Rendered text replaces source-record authority.

Owner links:
- [Projection Authority Reference](projection-and-templates.md) owns projection authority and freshness boundaries.
- [Template Bodies](template-bodies.md) owns rendered template bodies.

### User judgment / product-file write scope

Conditions:
- Sensitive-action approval, product-file write compatibility, or `Write Authorization` is being described.

May claim:
- Sensitive-action approval is separate from product-file write compatibility and `Write Authorization`.

Must not claim:
- Broad approval substitutes for either boundary.
- User approval becomes sandboxing or OS permission.

Owner links:
- [Core Model](core-model.md) owns user-owned judgment and non-substitution rules.
- [Storage Effects](storage-effects.md) owns product-file and Harness-record storage effects.

## 7. Threat/control summary

| Threat or confusion | Details |
|---|---|
| documented procedure ignored | See [Documented procedure ignored](#security-threat-procedure-ignored) |
| product write outside expected scope | See [Product write outside scope](#security-threat-product-write-outside-scope) |
| changed paths differ from expected scope | See [Changed paths differ](#security-threat-changed-paths-differ) |
| stale or copied authority appears in text | See [Stale or copied authority](#security-threat-stale-copied-authority) |
| local Harness files modified outside server | See [Local Harness files modified](#security-threat-local-files-modified) |
| sensitive-action approval treated as broad approval | See [Broad approval confusion](#security-threat-broad-approval-confusion) |

<a id="security-threat-procedure-ignored"></a>
### Documented procedure ignored

Condition:
- An agent ignores the documented procedure.

Current control:
- The specification records the expected procedure and requires owner-defined Harness paths for Harness state changes.

Guarantee level:
- `cooperative`.

Not allowed:
- It cannot prevent a malicious agent from acting outside Harness.

<a id="security-threat-product-write-outside-scope"></a>
### Product write outside scope

Condition:
- Product write is outside the expected scope.

Current control:
- `harness.prepare_write` and `Write Authorization` can express product-file write compatibility in the specification.

Guarantee level:
- `cooperative`.

Not allowed:
- They do not grant or deny OS file permission.

<a id="security-threat-changed-paths-differ"></a>
### Changed paths differ

Condition:
- Changed paths differ from the expected scope.

Current control:
- A passed capability check may support limited detection for observed changed paths.

Guarantee level:
- `detective`.

Not allowed:
- This is not full filesystem monitoring.

<a id="security-threat-stale-copied-authority"></a>
### Stale or copied authority

Condition:
- Stale or copied authority appears in text.

Current control:
- Registered surface context, staged-handle validation, and owner-defined checks must be used instead of copied identifiers.

Guarantee level:
- `cooperative`.
- `detective` only when observed.

Not allowed:
- Copied `surface_id`, `ArtifactRef`, or rendered text is not authority.

<a id="security-threat-local-files-modified"></a>
### Local Harness files modified

Condition:
- Local Harness files are modified outside the server.

Current control:
- Storage/runtime owners may define consistency checks or rejection behavior.

Guarantee level:
- `cooperative` unless a stronger mechanism is promoted.

Not allowed:
- No tamper-proof storage is claimed.

<a id="security-threat-broad-approval-confusion"></a>
### Broad approval confusion

Condition:
- Sensitive-action approval is treated as broad approval.

Current control:
- Non-substitution rules keep sensitive-action approval, final acceptance, residual-risk acceptance, and write compatibility separate.

Guarantee level:
- `cooperative`.

Not allowed:
- User approval does not become sandboxing or OS permission.

## 8. `cooperative` behavior

`cooperative` behavior is the default baseline-scope guarantee level.

Conditions:
- The connected surface follows the documented procedure.
- The specification defines what Harness should record.
- Owner-defined Harness paths carry state changes, write compatibility, evidence summaries, user-owned judgments, and close-readiness outcomes.

May claim:
- Harness records, checks, routes, rejects within its own API path, or asks for the right user-owned judgment.
- The specification requires server behavior to keep owner-defined outcomes on their documented paths.

Must not claim:
- Harness blocks arbitrary tools.
- Harness controls OS permissions.
- Harness makes files tamper-proof.
- Harness prevents malicious agent behavior.

Owner links:
- [Core Model](core-model.md) owns user-owned judgment and non-substitution rules.
- [Storage Effects](storage-effects.md) owns storage effects.
- [Runtime Boundaries](runtime-boundaries.md) owns runtime and Product Repository separation.

## 9. `detective` behavior

`detective` behavior is capability-gated and scope-limited.

Conditions:
- The relevant surface has shown that it can observe the fact being claimed.
- The capability check for that exact surface and operation has passed.
- The observed scope is named.

May claim:
- Harness can report a mismatch or observed fact within the observed scope.
- Limited changed-path reporting is available only for the checked surface and operation.

Must not claim:
- Command monitoring.
- Network monitoring.
- Secret access monitoring.
- Full filesystem monitoring.
- Pre-execution blocking.

Owner links:
- [Agent Integration](agent-integration.md) owns connector capability meaning.
- [API Errors](api/errors.md) owns capability-related error routing.

Exceptions:
- Another active owner may document and prove an exact stronger mechanism. Without that owner, keep the wording `cooperative` or capability-gated `detective`.

## 10. Stronger Guarantee Boundary

The baseline scope has no active stronger prevention or isolation guarantee.

Must not claim:
- Harness prevents arbitrary tool behavior.
- Harness provides OS sandboxing or permission enforcement.
- Harness makes files or records tamper-proof.
- A profile, surface, or generated display proves a stronger guarantee by itself.
- A stronger guarantee exists before [Scope](scope.md) and the affected security owner define it as supported.

May claim:
- Server obligations may be described as "the specification requires" inside the active owner path.

Owner links:
- [Scope](scope.md) owns active/out-of-scope status.
- [API Value Sets](api/schema-value-sets.md) owns active guarantee label value entries.
- [Scope Reference](scope.md) owns category-level exclusions for stronger capabilities, monitoring, isolation, and pre-tool controls.

## 11. Cross-owner links

- [Scope](scope.md) owns baseline scope inclusion, exclusion, and active/out-of-scope boundaries.
- [Runtime Boundaries](runtime-boundaries.md) owns Product Repository, Harness Server, Runtime Home, and non-isolation separation.
- [Agent Integration](agent-integration.md) owns connector behavior and `capability_profile` meaning at the surface boundary.
- [API Methods](api/methods.md), method owner documents, [API Value Sets](api/schema-value-sets.md), and [API Errors](api/errors.md) own method routing, method behavior, value sets, and public error routing.
- [Core Model](core-model.md) owns user-owned judgment and non-substitution rules.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Artifact Storage](storage-artifacts.md) own storage and artifact details.
- [Scope Reference](scope.md) owns category-level exclusions for stronger capabilities, monitoring, isolation, and pre-tool controls.
