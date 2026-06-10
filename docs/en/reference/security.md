# Security reference

Use this page when security wording, local-access posture, trust boundaries, or guarantee levels need to stay honest. This is documentation source material for a future Harness Server. It does not implement security controls, runtime state, generated artifacts, or operational monitoring.

## 1. Owns / Does not own

| This document owns | This document does not own |
|---|---|
| Security claims and explicit non-claims. | API method behavior or schemas. |
| Guarantee semantics and non-claims for `cooperative`, `detective`, `preventive`, and `isolated`. | Storage layouts, artifact lifecycle, locks, hashes, or migrations. |
| Trust-boundary wording for local access, surfaces, and generated displays. | Connector implementation or surface-specific operating instructions. |
| Capability-gated `detective` claim boundaries. | OS hardening, deployment hardening, arbitrary-tool sandboxing, or OS permission enforcement. |
| Sensitive-action approval versus product-file write-scope separation. | Runtime implementation status or permission to build the server. |

Use the [Reference index](README.md) to route API, storage, connector, runtime-boundary, and active-scope details to their owners.

<a id="honest-guarantee-display"></a>
## 2. Current guarantee levels

The current MVP guarantee boundary is `cooperative` by default. `detective` wording is allowed only for a documented, observed scope after the relevant capability check has passed. `preventive` and `isolated` are not active current-MVP guarantees.

`preventive` and `isolated` are related but distinct. `preventive` means prevention-oriented controls that may be future/later. `isolated` is a stronger isolation-oriented label that is also reserved for later/profile-gated use. In the current MVP, `isolated` does not support claims of OS sandboxing, OS permission enforcement, tamper-proof isolation, or full security isolation.

In this documentation-only repository, "available in current MVP" means the specification may describe the behavior as current MVP source material. It does not mean this repository contains a working Harness Server, runtime monitor, sandbox, or storage layer.

| Level | Available in current MVP? | Conditions | May claim | Must not claim |
|---|---|---|---|---|
| `cooperative` | yes | default documented behavior | recorded procedure and agent cooperation | OS-level enforcement |
| `detective` | limited | capability check passed and observed scope only | limited detection for observed changed paths | complete monitoring |
| `preventive` | no | future/later prevention-oriented mechanism | nothing in current MVP | sandboxing, OS permission enforcement, isolation, or active blocking without a promoted mechanism |
| `isolated` | no | reserved later/profile-gated value only | no current isolation guarantee | OS sandboxing, OS permission enforcement, tamper-proof isolation, full security isolation |

## 3. Explicit non-claims

The current MVP does not claim:

- OS-level sandboxing
- OS permission enforcement
- current isolation guarantee
- tamper-proof storage
- tamper-proof isolation
- full security isolation
- guaranteed full filesystem monitoring
- complete prevention of malicious agent behavior
- a stronger guarantee than the registered surface/profile supports
- universal pre-tool blocking
- command, network, or secret observation by default
- native artifact capture as an active guarantee

## 4. Capability-gated `detective` claims

Capability-gated `detective` wording is narrow:

- A capability check can support a `detective` claim only for the named surface, the checked capability, and the observed scope.
- A copied `surface_id`, generated file, `Projection`, chat text, Product Repository file, rendered display, or agent memory is not proof of capability.
- Observed changed paths can support a limited changed-path detection claim only when the surface actually reports those paths for the relevant operation.
- A missing or insufficient capability should route to the API/error owner behavior, such as `CAPABILITY_INSUFFICIENT`, rather than inventing authority.
- `detective` wording never upgrades a claim to sandboxing, permission enforcement, tamper-proof storage, or complete monitoring.

## 5. Assets

| Asset | Current MVP security posture |
|---|---|
| Core-owned Harness records | Changed only through owner-defined Harness paths in the specification. This is not a claim that local files are tamper-proof. |
| Product Repository files | User workspace files. They can be inputs to checks, but they are not Harness state and are not proof of Harness authority. |
| Harness Runtime Home and local store | Future operational data space owned by storage/runtime owners. This documentation repository is not a Runtime Home. |
| Artifacts and staged handles | `ArtifactRef`, `ArtifactInput`, and `StagedArtifactHandle` require API/storage validation. Displayed identifiers do not create artifact authority. |
| Surface identity and capability profile | Registered surface context and capability checks limit what may be claimed. `surface_id` alone is not an authority token. |
| User-owned judgments | Sensitive-action approval, final acceptance, waiver, and residual-risk acceptance remain distinct. None of them grants OS permission. |

## 6. Trust boundaries

| Boundary | Rule | Non-claim |
|---|---|---|
| Product Repository / Harness records | Product files, generated Markdown, and chat text do not directly mutate Harness records. | Product text is not Harness state. |
| Harness Server / Runtime Home | The future server would mediate Harness records and storage effects. | This repository does not contain that runtime. |
| Connector surface / Harness authority | A connector can carry context only within its verified surface and capability profile. | A connector description is not proof of authority. |
| Rendered displays / source records | Generated displays can summarize source records. | A rendered display is not a new authority source. |
| User judgment / product-file write scope | Sensitive-action approval is separate from product-file write compatibility and `Write Authorization`. | Broad approval does not substitute for either boundary. |

## 7. Threat/control summary

| Threat or confusion | Current control statement | Guarantee level | Limit |
|---|---|---|---|
| Agent ignores the documented procedure. | The specification records the expected procedure and requires owner-defined Harness paths for Harness state changes. | `cooperative` | It cannot prevent a malicious agent from acting outside Harness. |
| Product write is outside the expected scope. | `harness.prepare_write` and `Write Authorization` can express product-file write compatibility in the specification. | `cooperative` | They do not grant or deny OS file permission. |
| Changed paths differ from the expected scope. | A passed capability check may support limited detection for observed changed paths. | `detective` | It is not full filesystem monitoring. |
| Stale or copied authority appears in text. | Registered surface context, staged-handle validation, and owner-defined checks must be used instead of copied identifiers. | `cooperative` / `detective` when observed | Copied `surface_id`, `ArtifactRef`, or rendered text is not authority. |
| Local Harness files are modified outside the future server. | Storage/runtime owners may define consistency checks or rejection behavior. | `cooperative` unless a later mechanism is promoted | No tamper-proof storage is claimed. |
| Sensitive-action approval is treated as broad approval. | Non-substitution rules keep sensitive-action approval, final acceptance, residual-risk acceptance, and write compatibility separate. | `cooperative` | User approval does not become sandboxing or OS permission. |

## 8. `cooperative` behavior

`cooperative` behavior means the connected surface follows the documented procedure and the specification defines what Harness should record. The specification requires future server behavior to keep owner-defined state changes, write compatibility, evidence summaries, user-owned judgments, and close-readiness outcomes on their documented paths.

`cooperative` wording may say Harness records, checks, routes, rejects within its own API path, or asks for the right user-owned judgment. It must not say Harness blocks arbitrary tools, controls OS permissions, makes files tamper-proof, or prevents malicious agent behavior.

## 9. `detective` behavior

`detective` behavior means Harness can report a mismatch or observed fact after the relevant surface has shown that it can observe that fact. Examples include limited changed-path reporting after the capability check for that exact surface and operation has passed.

`detective` wording must include the observed scope. It must not imply command monitoring, network monitoring, secret access monitoring, full filesystem monitoring, or pre-execution blocking unless another active owner documents and proves that exact mechanism.

## 10. Later `preventive` and `isolated` boundary

`preventive` behavior means a documented mechanism stops or denies an action before it happens. `isolated` behavior requires a documented boundary strong enough to support an isolation guarantee over execution, state, permissions, or storage. The current MVP has no active `preventive` guarantee and no active `isolated` isolation guarantee.

A later or profile-gated `preventive` or `isolated` claim requires a promoted owner to document:

- the mechanism that prevents the action or provides isolation
- the exact covered operation, path, surface, profile, and isolation boundary when `isolated` is claimed
- the bypass and fallback behavior
- the proof path and user-visible error behavior
- paired English/Korean documentation and active-scope promotion

Until those conditions are met, use "the specification requires" for future server obligations and keep the guarantee level `cooperative` or capability-gated `detective`. Do not use `isolated` as a synonym for `preventive`; `isolated` is reserved for a stronger isolation-oriented guarantee.

## 11. Cross-owner links

- [Active MVP Scope](active-mvp-scope.md) owns current MVP inclusion, exclusion, and active/later boundaries.
- [Runtime Boundaries](runtime-boundaries.md) owns Product Repository, Harness Server, Runtime Home, and non-isolation separation.
- [Agent Integration](agent-integration.md) owns connector behavior and `capability_profile` meaning at the surface boundary.
- [MVP API](api/mvp-api.md), [API Value Sets](api/schema-value-sets.md), and [API Errors](api/errors.md) own method behavior, value sets, and public error routing.
- [Core Model](core-model.md) owns user-owned judgment and non-substitution rules.
- [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Artifact Storage](storage-artifacts.md) own storage and artifact details.
- [Later Candidate Index](../later/index.md) owns deferred stronger capability, monitoring, isolation, and preventive-control candidates.
