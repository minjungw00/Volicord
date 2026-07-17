# External contracts

This document owns Volicord's external-contract descriptor, exact boundary
adapter selection, canonical internal-model boundary, compatibility window, and
shared Git object-ID validation contract.

It does not define the payload schema of a particular storage, host, transport,
or release artifact. Those shapes remain with their focused owners. It also
does not define API response envelopes or storage effects. Cross-surface
failure-category meaning belongs to the [Failure Model](failure-model.md).

<a id="surface-stability"></a>
## Surface Stability

The `ExternalContractDescriptor` shape, exact registry key, canonical boundary
flow, `unsupported_external_contract` reason, compatibility window, and Git
object-ID validation rules are stable contracts. Adapter source layout,
registry implementation, and decoder helper types are internal.

## `ExternalContractDescriptor`

Every Volicord-owned external format that crosses a boundary into the
canonical internal model is identified by this exact descriptor:

```yaml
ExternalContractDescriptor:
  contract_id: string
  schema_digest: string
  capabilities: string[]
```

Field meanings:

| Field | Contract |
|---|---|
| `contract_id` | The exact semantic kind of contract. It identifies what the format represents, not a numeric schema revision. |
| `schema_digest` | The exact digest of the format's structure and canonical encoding. It distinguishes structurally different contracts with the same semantic kind. |
| `capabilities` | The complete set of capabilities the described format provides. An omitted capability is not inferred or supplied by default. |

The descriptor is structural boundary input. `contract_id` and
`schema_digest` must both be present, non-empty, and compared exactly. A
Volicord-owned `contract_id` must not use a numeric revision suffix such as
`-v1` or `-v2` as compatibility identity. Structural change is identified by
`schema_digest`; capability availability is represented by `capabilities`.

The owner of a particular external format defines that format's canonical
encoding, digest construction, capability vocabulary, and payload bounds. This
document owns how Volicord selects a boundary adapter from those facts; it does
not create a second digest algorithm or capability catalog.

## Exact adapter registry selection

The boundary adapter registry key is the exact pair:

```text
contract_id + schema_digest
```

Both values are matched exactly as produced by the format owner. Registry
selection does not trim, case-fold, numerically compare, partially match, or
normalize either value after descriptor validation. Capability checks happen
only after the exact registry entry has been selected, and the selected entry
must satisfy every capability required by the receiving boundary.

A boundary must not:

- compare numeric versions or order contract identifiers by version
- try multiple decoders in sequence
- infer a format from field presence, field absence, empty values, or payload
  content
- retry with another decoder after parsing fails
- fill missing descriptor or payload fields with defaults
- accept an unregistered `contract_id` or `schema_digest`
- keep external-format branches in Core or Store

A well-formed descriptor whose exact pair is absent from the registry, or whose
registered capabilities cannot satisfy the receiving boundary, is
`UnsupportedContract` with machine-readable reason
`unsupported_external_contract`. A malformed descriptor is `Rejected`; it is
not searched against other adapters. The owning adapter or transport projects
that category and reason into its own response shape.

## Canonical boundary model

External input follows one direction:

```text
external format
-> exact descriptor validation and registry selection
-> one strict boundary decoder or adapter
-> one canonical internal type
-> Core
-> Store
```

The selected decoder must either produce the complete canonical internal type
or fail. Core and Store receive only canonical internal types and must not
branch on external contract identifiers, schema digests, decoder generations,
or external field layouts.

Output follows the reverse ownership boundary but emits only the current
canonical external contract. A producer must not emit a historical descriptor,
compatibility alias, partially populated shape, or format selected from caller
payload characteristics.

## Compatibility window

Before Volicord 1.0, only the current descriptor for a Volicord-owned external
contract is supported. The registry contains no historical adapter, placeholder
adapter, or fallback decoder for that contract.

Starting with Volicord 1.0, a boundary registry supports exactly:

- the current publicly released descriptor; and
- the immediately previous publicly released descriptor.

Each entry is still selected only by exact `contract_id + schema_digest`.
Support for the immediately previous descriptor does not permit numeric version
dispatch, decoder probing, or support for an older descriptor. The canonical
Core and Store model remains singular.

Every non-current adapter entry must carry sunset metadata that records both a
specific removal condition and the last Volicord release that supports the
entry. When either boundary is reached, the entry is removed rather than
silently retained. This policy defines a future compatibility window; it does
not require a previous-descriptor adapter before 1.0 and does not authorize one
in the current release.

<a id="shared-git-object-id-contract"></a>
## Shared Git object-ID contract

Every Volicord boundary that accepts a Git object ID uses the same validation
and canonicalization rules:

- Input is exactly 40 or exactly 64 ASCII hexadecimal characters.
- The only accepted bytes are `0-9`, `a-f`, and `A-F`.
- Uppercase and lowercase hexadecimal input are both accepted.
- The canonical representation is lowercase ASCII hexadecimal.
- Every other length is rejected, including 39, 41 through 63, and 65
  characters.
- Leading or trailing whitespace, a prefix such as `0x`, non-ASCII digits,
  Unicode lookalikes, separators, and an empty value are rejected rather than
  trimmed or normalized.

Storage, comparison, digest input, receipt binding, and adapter output use the
canonical lowercase representation. A caller's accepted uppercase spelling is
not a second identity. This contract does not infer a Git object type or
repository from the identifier alone.

## Adjacent owners

- Product-wide failure categories and no-default-conflation rules:
  [Failure Model](failure-model.md).
- Storage-specific manifest and database-open compatibility:
  [Storage Versioning](storage-versioning.md).
- Managed host and connection semantics: [Agent Connection](agent-connection.md).
- Exact release-artifact evidence: [Host Release Evidence](host-release-evidence.md).
- Public API error branches and codes: [API Errors](api/errors.md).
