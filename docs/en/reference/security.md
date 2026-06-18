# Security reference

This document owns Harness security guarantee wording, local-access assumptions, sensitive-action approval boundaries, and explicit security non-guarantees.

## Owns / does not own

| This document owns | This document does not own |
|---|---|
| Supported guarantee semantics for `cooperative` and capability-gated `detective` wording. | API method request/response schemas or method-specific behavior. |
| The boundary that no baseline preventive guarantee is supported. | Storage record layout, artifact lifecycle detail, locks, hashes, or migrations. |
| Local-access assumptions and access-boundary non-claims. | Connector implementation or surface-specific operating recipes. |
| Sensitive-action approval as a security-adjacent user-owned judgment boundary. | OS permissions, deployment controls, arbitrary-tool sandboxing, or host policy. |
| Non-authority rules for local files, generated displays, copied identifiers, chat text, and agent memory. | Runtime location definitions; see [Runtime Boundaries](runtime-boundaries.md). |

## Supported security guarantees

<a id="honest-guarantee-display"></a>
Harness may describe a guarantee only when [Scope](scope.md) and this security owner both support the guarantee level. If the claim depends on a surface capability, the relevant surface capability check must also pass for the named surface and observed scope.

The supported guarantee display labels are `cooperative` and `detective`; the value names are owned by [API Value Sets](api/schema-value-sets.md).

### `cooperative`

`cooperative` is the default baseline security guarantee.

Conditions:
- The caller, agent, surface, or connector follows the documented Harness paths.
- The claim stays inside documented Core, API, storage, runtime, and user-judgment boundaries.

May claim:
- Harness records, write compatibility, evidence summaries, user-owned judgments, and close-readiness results are governed by their owner contracts.
- Harness may reject, block, or require a focused user-owned judgment when the relevant owner contract says the current state is not compatible.

Must not claim:
- `cooperative` blocks arbitrary tool behavior, host commands, network access, secret access, or product-file edits outside Harness-owned paths.
- `cooperative` provides OS permission enforcement, sandboxing, tamper-proof isolation, or full security isolation.

### Capability-gated `detective`

`detective` is supported only as a limited, capability-gated claim.

Conditions:
- The claim names the surface.
- The relevant capability check has passed.
- The observed scope is documented.
- Changed-path wording is used only when the surface reports changed paths for the relevant operation.

May claim:
- The checked surface and checked capability support limited observation or mismatch reporting inside the documented observed scope.
- Observed changed paths support a limited changed-path detection claim when the reporting condition is met.
- Missing or insufficient capability routes to documented error behavior, such as `CAPABILITY_INSUFFICIENT`.

Must not claim:
- A copied `surface_id`, `access_class`, connector description, `Projection`, generated display, chat message, or agent memory proves capability.
- `detective` wording becomes prevention, sandboxing, OS permission enforcement, full monitoring, or tamper-proof storage.

### Preventive guarantees

The baseline contract does not define a supported preventive guarantee.

Must not claim:
- Harness prevents arbitrary tool execution.
- Harness provides universal pre-tool blocking.
- Harness observes or blocks command, network, or secret access by default.
- Harness provides OS sandboxing, host permission enforcement, or stronger isolation.

## Sensitive-action approval boundary

Sensitive-action approval is a user-owned judgment for a named sensitive step inside a bounded `SensitiveActionScope`.

May claim:
- Sensitive-action approval can be required before write compatibility, Run recording, or close when the relevant owner defines that requirement.
- The approved sensitive step remains scoped to the prompt, `SensitiveActionScope`, affected object, and visible consequence that the user was asked to judge.

Must not claim:
- Sensitive-action approval is `Write Authorization`, `AuthorizedAttemptScope`, OS permission, shell permission, command approval, deployment approval, final acceptance, residual-risk acceptance, or product correctness.
- Sensitive-action approval authorizes product-file writes, commands, hosts, networks, secrets, destructive operations, or unbounded activity.
- Broad approval substitutes for a required sensitive-action approval, final acceptance, residual-risk acceptance, scope decision, or `Write Authorization`.

Owner links:
- [Core Model](core-model.md) owns user-owned judgment and non-substitution rules.
- [API Judgment Schemas](api/schema-judgment.md) owns `SensitiveActionScope` shape.
- [Prepare-write method](api/method-prepare-write.md) owns `harness.prepare_write` behavior.

## Local access assumptions

Harness security claims assume local actors use the documented Harness contracts for Harness state, records, artifacts, write compatibility, and user-owned judgments.

May claim:
- Local product files can be inputs to Harness checks or user-owned judgments.
- Local runtime data location can be defined by storage/runtime owners.
- Local surfaces can provide verified capability context when [Agent Integration](agent-integration.md) and this security owner allow the claim.
- The baseline local access grant for a registered surface instance is the grant stored in `surfaces.local_access_json`.

Must not claim:
- Local filesystem access proves Harness authority.
- A local path, directory name, copied identifier, displayed identifier, or rendered text is a security token.
- Direct local modification outside those documented Harness contracts creates valid Harness records, evidence, acceptance, residual-risk acceptance, `Write Authorization`, or artifact authority.
- `Harness Runtime Home` is automatically an OS security boundary, sandbox, or isolation layer.
- A caller-supplied `verified` flag, requested `access_class`, `capability_profile`, or copied `verification_basis` grants local access.

## Authority boundaries

### Harness records

Harness records carry authority only through the owner contracts that create, validate, or update them.

Must not claim:
- Local file contents are tamper-proof because they describe or store Harness data.
- Product text, generated text, or copied record-looking text directly mutates Harness records.

### `Product Repository` files

[Runtime Boundaries](runtime-boundaries.md) defines `Product Repository` as the product-file boundary. This section owns only security claims and non-claims about that boundary.

May claim:
- Product files can be inspected as inputs.
- Compatible product-file writes can be governed by current scope, current Change Unit compatibility, user-owned judgments, and `Write Authorization` when the write owner requires them.

Must not claim:
- Product files are Harness state.
- Product files prove Harness authority.
- Product files become Harness records because Harness metadata is nearby.

### `Harness Runtime Home`

For security wording, treat `Harness Runtime Home` as the runtime/storage-owned operational data location.

Runtime location is defined by [Runtime Boundaries](runtime-boundaries.md). This section owns only the security non-claims for that location.

May claim:
- Storage/runtime owners define which Harness operational data belongs there and how it is validated.

Must not claim:
- `Harness Runtime Home` is the `Product Repository`.
- `Harness Runtime Home` is automatically a security boundary.
- Placing data under `Harness Runtime Home` proves security authority or isolation.

### Surfaces and capability context

Surface identity and capability context limit what may be claimed.

May claim:
- `VerifiedSurfaceContext`, `surface_id`, `surface_instance_id`, `access_class`, and capability checks can be used according to the API, agent-integration, and security owners after the current invocation access is verified against the registered local access grant.

Must not claim:
- `surface_id` alone is an authority token.
- A copied surface identifier proves capability.
- An `access_class` is OS permission or broad authority.
- `capability_profile` grants an access class.
- `verification_basis` is a caller authority token.

### Generated displays and text

Generated displays, rendered templates, chat text, connector prose, and agent memory can help readers understand source records.

Must not claim:
- A rendered display, `Projection`, status card, template output, chat message, connector description, or agent memory is a new authority source.
- Displayed `ArtifactRef`, `UserJudgment`, `Write Authorization`, or `surface_id` text creates the authority named by those identifiers.

## Explicit non-guarantees

### Operating system and isolation

Harness does not guarantee:

- OS-level sandboxing.
- OS permission enforcement.
- Tamper-proof isolation.
- Full security isolation.
- Isolation between local users, processes, tools, or hosts.

### Monitoring and prevention

Harness does not guarantee:

- Full filesystem monitoring.
- Command monitoring by default.
- Network monitoring by default.
- Secret-access monitoring by default.
- Universal pre-tool blocking.
- Prevention of malicious agent behavior outside Harness-owned paths.

### Storage and artifact authority

Harness does not guarantee:

- Tamper-proof storage.
- Native artifact capture from surfaces as a baseline guarantee.
- Artifact authority from displayed identifiers alone.
- Validation or acceptance from copied artifact, run, evidence, or judgment text.

### Broad authority inference

Harness does not allow readers or agents to infer authority from:

- Broad approval.
- Local path names.
- Copied `surface_id` values.
- Displayed `ArtifactRef` values.
- Rendered `Projection` output.
- `Product Repository` text.
- Connector prose.
- Chat text or agent memory.

## Related owners

- [Scope](scope.md): baseline inclusion, exclusions, and supported guarantee boundary.
- [Runtime Boundaries](runtime-boundaries.md): `Product Repository`, `Harness Server` or other runtime process resources, and `Harness Runtime Home` boundaries.
- [Agent Integration](agent-integration.md): surface registration, capability profiles, and verified surface context.
- [API Value Sets](api/schema-value-sets.md): `GuaranteeDisplay.level`, `access_class`, and other value names.
- [API error routing](api/error-routing.md): public error routing such as `CAPABILITY_INSUFFICIENT`.
- [Core Model](core-model.md): user-owned judgment, `Write Authorization`, acceptance, residual risk, and non-substitution rules.
- [API Judgment Schemas](api/schema-judgment.md): `SensitiveActionScope` and user-owned judgment schema shapes.
- [Storage Effects](storage-effects.md), [Storage Records](storage-records.md), and [Artifact Storage](storage-artifacts.md): storage effects, record layout, and artifact authority details.
