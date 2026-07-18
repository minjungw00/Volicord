# Failure model

This document owns the product-wide failure categories used across Core,
Store, adapters, transports, administrative commands, and diagnostics. It
defines the exact category identifiers, their semantic boundaries, persisted
state handling, and the prohibition on turning failure into a synthetic or
default success value.

It does not define a particular API response envelope, transport error shape,
method storage effect, domain-specific reason catalog, or repair command.
Those details remain with the focused owner for the affected surface.

<a id="surface-stability"></a>
## Surface Stability

The five category meanings, their machine-readable identifiers, persisted
authority and policy fail-closed rules, and no-default-conflation rules are
stable contracts. Human display text and domain-specific diagnostic details
are diagnostic unless their focused owner says otherwise.

## Canonical failure categories

Every machine-readable failure classification uses one of these exact primary
category identifiers:

| Category | Identifier | Meaning |
|---|---|---|
| `Rejected` | `rejected` | The request shape or required context is structurally invalid, so the requested operation does not proceed to policy evaluation or its successful operation branch. |
| `NotAllowed` | `not_allowed` | A structurally valid request and complete required context reached policy evaluation, and policy did not allow the requested operation. |
| `Unavailable` | `unavailable` | The operation, auxiliary capability, or required read cannot currently be performed, while the available data does not establish persisted contract corruption. |
| `Degraded` | `degraded` | The core operation can continue, but an explicitly identified verification, diagnostic, or auxiliary-information component is incomplete. |
| `Corrupt` | `corrupt` | Persisted or trusted owner data violates its declared schema, type, canonical encoding, or cross-field contract. |

A machine-readable result or diagnostic must carry the exact category
identifier and, when the domain distinguishes causes within that category, a
domain-owner-defined reason identifier. Human text alone is not a category or
reason. A reason identifier must not silently change the category meaning.

## Category selection boundaries

`Rejected` and `NotAllowed` are separated by policy evaluation. Missing or
invalid required context is `Rejected`; it must not be represented as a policy
denial. `NotAllowed` requires a structurally valid request, resolved required
context, and an actual policy decision. A method owner may define a committed
non-allow result, but the category by itself does not authorize a commit.

`Unavailable` and `Degraded` are separated by whether the core operation can
truthfully continue. If the required operation or read cannot be performed, the
category is `Unavailable`. If the core operation remains valid while a named
auxiliary verification or information source is incomplete, the category is
`Degraded`; the missing part and its effect must remain visible.

`Corrupt` applies when data that claims to follow a supported persisted or
trusted contract violates that contract. A malformed or unknown untrusted
boundary input that has not become persisted owner state is `Rejected`, not
`Corrupt`, and is not guessed into a supported shape.

An installed Codex executable or version is not an external contract and has
no certified or uncertified state. Connection verification runs behavioral
probes and reports their actual `passed`, `pending`, or `failed` results.
Failure to find or run the executable is `Unavailable` at a general
failure-category boundary and a failed `host_executable` connection check.

The administrative connection command report uses `failed` only for a typed
operational result with at least one failed required check. Pending host
observation is `action_required`, not `Degraded`, a stale/broken public status,
or an unexpected runtime error. Usage and unexpected runtime or serialization
failures remain on the CLI error channel rather than being fabricated into a
successful or action-required report.

No category implies another. In particular:

- `Unavailable` is not an empty successful result.
- `Degraded` is not full verification or an unqualified success state.
- `Corrupt` is not a missing optional value.
- `NotAllowed` is not structural rejection.

## Persisted authority and policy data

Persisted authority or policy data that cannot be decoded and validated under
its declared contract is `Corrupt` and fails closed. Any operation that depends
on that data must stop before it derives authority, makes a policy decision,
records a successful effect, or mutates dependent owner state.

Typed persisted JSON must be decoded into its complete declared type. Syntax
failure, a wrong top-level shape, an unknown closed variant, a missing required
field, an extra field where the owner rejects extras, or a violated cross-field
invariant is `Corrupt`. None of those conditions becomes an empty array, empty
object, absent value, or host-specific default.

Missing display-only or auxiliary data is not automatically corruption. Its
focused owner must classify it as `Unavailable` or `Degraded` and state whether
the core operation can continue. A valid empty array or object remains a valid
empty value only when the declared schema explicitly permits that exact value.

Recoverable state may be regenerated only through an explicit owner-defined
verify or repair flow. Reads and ordinary execution must not repair, migrate,
guess, or silently replace the data while classifying the failure.

## No synthetic or default conflation

Failure must not be converted into an ordinary value by using:

- an empty string, array, object, or zero value that the contract did not
  actually contain
- a synthetic identifier, placeholder record, fabricated timestamp, or
  fabricated capability
- a default enum variant selected after decoding or lookup failed
- a fallback host, adapter, decoder, external contract, or storage shape
- a historical alias or deprecated shape treated as the current contract
- a successful response whose only indication of failure is human text

Implementation convenience such as `unwrap_or_default()` does not change this
contract. A default is valid only when it was established before persistence by
the owner-defined typed construction path and the resulting stored value itself
passes the complete declared contract.

## Effects and response routing

The failure category does not by itself define state mutation, retryability,
HTTP or JSON-RPC status, CLI exit code, API response branch, or display text.
The affected method, adapter, transport, CLI, and storage-effect owners define
those projections while preserving this category meaning.

Adjacent owners:

- Public API branch routing and public codes: [API Errors](api/errors.md).
- Persisted record contracts: [Storage Records](storage-records.md).
- Method-to-storage effects and no-effect branches:
  [Storage Effects](storage-effects.md).
- Administrative verification and repair commands:
  [Administrative CLI](admin-cli.md).
