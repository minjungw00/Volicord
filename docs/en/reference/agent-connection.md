# Agent Connection Reference

This document defines the first-release Agent Connection contract. It owns the
exact `host_kind=codex` record connection surface, the canonical managed host
binding, host verification receipts, and the boundary between the Codex adapter
and Core.

<a id="owns-and-does-not-own"></a>

## Owns / Does Not Own

This document owns:

- the accepted exact `host_kind`, integration profile, connection intents,
  transport, user-action delivery path, and platform-environment values;
- `ManagedHostBinding` fields, canonical encoding, and digest meaning;
- `HostVerificationReceipt` fields and the checks Core performs before
  consuming a receipt;
- Codex adapter discovery, installation, verification, repair, and uninstall
  responsibilities;
- exact Codex artifact eligibility at the Agent Connection boundary; and
- typed handling of persisted host-setup `UserAction` values.

This document does not own:

- stdio framing, MCP initialization, tool routing, or shutdown; see
  [MCP Transport](mcp-transport.md);
- administrative command syntax, output, or exit codes; see
  [Administrative CLI](admin-cli.md);
- exact database tables or storage effects; see
  [Storage Records](storage-records.md) and
  [Storage Effects](storage-effects.md);
- release-cell execution and artifact evidence; see
  [Host Release Evidence](host-release-evidence.md);
- operating-system topology and filesystem prerequisites; see
  [System Requirements](system-requirements.md);
- Core `UserActionRequest` and `UserActionResolution` schemas; see
  [API User Action Schemas](api/schema-user-action.md); or
- product-wide failure-category and security meanings; see
  [Failure Model](failure-model.md) and [Security](security.md).

<a id="surface-stability"></a>

## Surface Stability

Labels follow the canonical vocabulary in
[Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

| Surface | Stability | Contract |
|---|---|---|
| First-release value sets, `PlatformEnvironment`, `ManagedHostBinding` fields and digest, and `HostVerificationReceipt` fields | `stable` | These are exact boundary contracts. |
| Codex discovery, managed installation, verification, repair, uninstall, and drift result semantics | `stable` | Implementations may change without changing the observable contract. |
| Adapter modules, filesystem helpers, encoders, and Store query helpers | `internal` | They must preserve the stable boundary but are not public surfaces. |
| Human-readable verification, degraded-state, and repair guidance | `diagnostic` | Machine-readable categories, reasons, and typed fields remain authoritative. |

<a id="first-release-surface"></a>

## First-Release Surface

The first release accepts only this Agent Connection surface:

| Dimension | Exact value |
|---|---|
| Host | `host_kind=codex` |
| Integration profile | `integration_profile=record` |
| Connection intent | `personal` or `shared` |
| Transport | Volicord-managed stdio MCP started with `volicord mcp --stdio` |
| User-owned action delivery | CLI inbox |
| Platform environment | `linux`, `macos`, `native_windows`, or `wsl2` |

The `connection_scope` field in a canonical binding carries the selected
connection intent and therefore accepts only `personal` or `shared`. A
`personal` connection installs user-owned local Codex configuration. A
`shared` connection installs the supported project-owned Codex configuration
inside the selected `Product Repository`. Both remain bound to the selected
project, connection, Runtime Home, platform environment, and exact managed
configuration.

An Agent Connection is a stored local integration record in the
`Volicord Runtime Home`. It names one connection and its allowed projects; it
does not grant operating-system permission, establish user identity, or prove
that Codex loaded the managed entry. One managed stdio MCP process is bound to
one current Agent Connection.

User-owned actions are delivered through the CLI inbox. An agent-facing MCP
connection may request an owner-defined action, but it cannot act as the local
user channel or resolve that action on the user's behalf.

<a id="external-contract-linkage"></a>

## External Contract Linkage

The canonical host-binding payload is a Volicord-owned external format. Before
it reaches the canonical Agent Connection model, its boundary adapter is
selected by the exact descriptor owned by
[External Contracts](external-contracts.md):

```yaml
ExternalContractDescriptor:
  contract_id: string
  schema_digest: string
  capabilities: string[]
```

The adapter registry key is the exact `contract_id + schema_digest` pair. The
descriptor capability set must contain every capability required by the
receiving Agent Connection boundary. Missing descriptor fields, an unknown
pair, an omitted capability, or a decode failure is not repaired by probing
another format or filling a default.

The current pre-1.0 release accepts only its current descriptor. The selected
descriptor decodes to one canonical `ManagedHostBinding` before Core or Store
is called. Core and Store do not branch on descriptor generations, host
configuration syntax, or payload characteristics.

<a id="platform-environment"></a>

## `PlatformEnvironment`

`PlatformEnvironment` is a closed value set:

| Value | Meaning |
|---|---|
| `linux` | The native Linux release cell. |
| `macos` | The native macOS release cell. |
| `native_windows` | The native Windows release cell. |
| `wsl2` | The independent WSL2 release cell. It is not inferred from `linux`. |

The binding and receipt must carry the same exact value. Verification never
substitutes one platform result for another. In particular, `wsl2` requires
explicit WSL2 detection and the topology in
[System Requirements](system-requirements.md#wsl2-topology).

Runtime platform observation also derives the exact `ReleaseTargetTriple` for
the executing Volicord binary. The closed published set and its allowed
environment cells are owned by [System Requirements](system-requirements.md#first-release-environment-matrix).
Support lookup uses this observed target directly; it never derives identity
from an operating-system name alone. WSL2 is explicitly detected and maps only
to `x86_64-unknown-linux-gnu` in the first release.

<a id="platform-release-coordinate"></a>

## `PlatformReleaseCoordinate`

Every binding and receipt carries one required closed
`platform_release_coordinate` object. Native Linux, macOS, and native Windows
use exactly:

```yaml
kind: native
```

WSL2 uses exactly:

```yaml
kind: wsl2
distribution_name: Ubuntu-24.04
distribution_id: ubuntu
distribution_version: "24.04"
environment_image: Ubuntu-24.04-LTS-WSL2
```

Unknown fields, a `native` coordinate paired with `platform_environment=wsl2`,
a WSL2 coordinate paired with a native environment, or any different WSL2
value is invalid. The three distribution values are observed in the current
WSL2 process. `environment_image` is the exact runtime support-policy image
registered for those facts. Exact support lookup compares it with the embedded
support entry, so an entry for another distribution image cannot authorize the
binding.

<a id="codex-capability"></a>

## `CodexCapability`

This document owns the capability identifiers used by `ManagedHostBinding`,
`HostVerificationReceipt`, `CodexSupportEntry`, and
`CodexReleaseEvidenceEntry`. `CodexCapability` is the closed value set below:

| Value | Required first-release behavior |
|---|---|
| `managed_stdio_mcp` | The artifact can launch and retain the managed stdio MCP boundary. |
| `record_workflow` | The artifact can complete the first-release Record-profile workflow. |
| `personal_managed_binding` | The artifact supports the exact `personal` managed-binding lifecycle. |
| `shared_managed_binding` | The artifact supports the exact `shared` managed-binding lifecycle. |

`FirstReleaseCodexCapabilities` is the set containing all four values. Every
first-release binding, receipt, support entry, and release-evidence entry
carries exactly that set, sorted by ascending UTF-8 bytes:

```text
managed_stdio_mcp
personal_managed_binding
record_workflow
shared_managed_binding
```

Unknown values, duplicate values, a different order, or a strict subset are
invalid. The capability set describes the exact artifact's verified behavior;
it is not inferred from `connection_scope`, a command name, or the selected
platform.

<a id="managed-host-binding"></a>

## `ManagedHostBinding`

The canonical binding and its nested records have these exact closed shapes.
The written field order is also the canonical record order:

```yaml
ManagedHostBinding:
  host_kind: codex
  connection_scope: personal | shared
  command: ManagedCommand
  arguments: string[]
  forwarded_environment: EnvironmentForwarding[]
  configuration_target: ConfigurationTarget
  process_binding: ProcessBinding
  required_capabilities: CodexCapability[]
  platform_environment: PlatformEnvironment
  platform_release_coordinate: PlatformReleaseCoordinate

ManagedCommand:
  resolution: path_lookup | absolute_path
  program: string

EnvironmentForwarding:
  source_name: string
  target_name: string

ConfigurationTarget:
  owner: user | project
  path: string

ProcessBinding:
  process_id: u64
  process_start_token: string
  platform_instance_token: string
  executable_path: string
  executable_digest: string
```

Every shown member is required, and unknown members are invalid. JSON decoding
also rejects duplicate keys. `host_kind` is exactly `codex`;
`connection_scope` matches the stored connection intent;
`required_capabilities` is exactly `FirstReleaseCodexCapabilities`; and
`platform_environment` is the exact detected platform.
`platform_release_coordinate` is the matching exact native or WSL2 coordinate
defined above.

`ManagedCommand.resolution=path_lookup` requires `program` to be one nonempty
basename with no path separator. `absolute_path` requires the normalized
absolute path form below. `arguments` is required and preserves every item and
its order; an empty list and an empty argument are identity-bearing values, not
missing data. Each string is valid UTF-8, contains no NUL, and is at most 4,096
bytes.

`forwarded_environment` contains declarations, not ambient values. Each name
matches `[A-Z_][A-Z0-9_]*`, entries are sorted by
`target_name` then `source_name` UTF-8 bytes, and duplicate `target_name`
values are invalid. An empty list is encoded explicitly and is valid only when
the selected managed configuration requires no forwarding.

`ConfigurationTarget.owner` is `user` for `personal` and `project` for
`shared`. Its `path` identifies the exact managed Codex configuration file.
`ProcessBinding.process_id` is nonzero. The two token fields are opaque
adapter observations of 1 through 256 UTF-8 bytes, contain no control
characters, and together distinguish PID reuse and platform-instance restart.
`executable_path` is the resolved canonical path of the currently observed
Codex executable. `executable_digest` is exactly 64 lowercase hexadecimal
characters containing the SHA-256 of those executable bytes.

Linux, macOS, and WSL2 canonical paths start with `/`, use `/` separators,
and contain no `.` or `..` segment, repeated separator, or non-root trailing
separator. Native Windows canonical paths use an uppercase drive prefix and
forward slashes, such as `C:/...`; UNC, device, relative, DrvFS, and
Windows-to-WSL converted spellings are invalid. Component spelling after the
drive prefix is preserved. Runtime containment rules remain with
[Runtime Boundaries](runtime-boundaries.md).

A managed configuration may be installed before Codex starts, but it does not
become a complete `ManagedHostBinding` until the adapter observes and validates
the live `ProcessBinding`. Missing fields and disallowed empty strings are
invalid; no field is optional or synthesized.

<a id="canonical-binding-encoding"></a>

### Canonical Encoding And Digest

The binding codec is independent of JSON, YAML, Serde map order, and host
endianness. It uses the following primitives:

```text
u32be(n)     = n as exactly four unsigned big-endian bytes
u64be(n)     = n as exactly eight unsigned big-endian bytes
blob(b)      = u32be(byte_length(b)) || b
string(s)    = blob(UTF8(s))
list(items)  = u32be(item_count) || blob(item_1_encoding) || ...
record(fields in declared order)
              = u32be(field_count)
                || string(field_1_name) || blob(field_1_encoding)
                || ...
```

An enum uses `string` with its exact literal spelling; `process_id` uses
`u64be`; strings use `string`; arrays use `list`; and nested objects use
`record` recursively. `canonical_binding_bytes` is the `record` encoding of
`ManagedHostBinding` in the order shown above. Nested records use their shown
order. `arguments` preserves its order; environment declarations and
capabilities use their required canonical order. Field names are encoded, so an
allowed empty string or list still has a present named field and cannot collide
with absence.

`PlatformReleaseCoordinate` is a nested record. The native record contains
only `kind=native`. The WSL2 record contains `kind=wsl2`,
`distribution_name`, `distribution_id`, `distribution_version`, and
`environment_image` in that order.

All counts and byte lengths must fit `u32`. Validation and path normalization
happen before encoding. The encoder performs no trimming, case folding, path
conversion, default insertion, omission, or map iteration.

```text
binding_digest = "sha256:" || lowercase_hex(sha256(
  "volicord.managed-host-binding\0"
  || canonical_binding_bytes
))
```

`binding_digest` is therefore exactly `sha256:` followed by 64 lowercase
hexadecimal characters. It identifies the exact verified binding content and
is not a format-version number. Content characteristics must never select
another codec.

<a id="codex-adapter-responsibilities"></a>

## Codex Adapter Responsibilities

The Codex adapter owns all host-specific inspection and mutation:

- discover the Codex installation referenced by the current binding, its
  configuration target, and the current platform environment;
- install only the managed entry represented by the canonical binding;
- construct `ManagedHostBinding` and its digest;
- observe and bind the exact `PlatformReleaseCoordinate`;
- calculate the digest of every generated managed artifact;
- inspect exact Codex artifact and executable identity;
- validate current process binding;
- detect missing, modified, or extra managed configuration as configuration
  drift;
- verify the complete binding and issue a typed
  `HostVerificationReceipt`;
- repair owner-defined managed state from current canonical inputs; and
- uninstall only the matching Volicord-managed state.

Discovery does not make an artifact supported. The adapter accepts only one
embedded `CodexSupportEntry` whose `codex_artifact_digest` equals
`process_binding.executable_digest`, whose `target_triple` and
`platform_environment` match the observed Volicord target and current
environment, whose `integration_profile` is `record`, and whose
`verified_capabilities` exactly equals `required_capabilities`. Its exact
`platform_release_coordinate` must also equal the binding coordinate. The
current binding, receipt, and support entry must therefore agree on platform
coordinate, executable digest, profile, and the exact canonical capability
set. External release evidence is not read during this lookup. A recognizable
command name, a reported version range, a nearby artifact, a partial capability
match, or an entry for another platform is insufficient. Any absence or mismatch is
`UnsupportedContract` with machine-readable reason
`unsupported_host_artifact`.

Repair regenerates canonical managed state and host-setup action data; it does
not overwrite unrelated Codex configuration or silently change the selected
project, connection, intent, profile, or platform environment. Uninstall
removes only content whose current identity still matches Volicord ownership.

Core does not parse Codex configuration, shell syntax, generated files, command
strings, filesystem placement rules, or process syntax. It receives only the
canonical binding identity and typed receipt.

<a id="host-verification-receipt"></a>

## `HostVerificationReceipt`

The adapter issues a receipt only after every verification check succeeds. Its
closed shape is:

```yaml
HostVerificationReceipt:
  contract_id: volicord.host-verification-receipt
  project_id: string
  connection_id: string
  host_kind: codex
  integration_profile: record
  platform_environment: PlatformEnvironment
  platform_release_coordinate: PlatformReleaseCoordinate
  required_capabilities: CodexCapability[]
  verified_capabilities: CodexCapability[]
  binding_digest: string
  generated_artifacts_digest: string
  executable_digest: string
  policy_digest: string
  verifier_build_digest: string
  observed_at: string
  expires_at: string
  result: verified
```

Every member is required, unknown members and duplicate JSON keys are invalid,
and no value is defaulted. `project_id` and `connection_id` are the exact
current Store identifiers: each is 1 through 1,024 UTF-8 bytes, contains a
non-whitespace character and no control character, and is preserved without
trimming. Both capability arrays exactly equal
`FirstReleaseCodexCapabilities`.

`platform_release_coordinate` exactly matches the coordinate in the canonical
binding and the current independently observed platform facts. A WSL2 receipt
therefore cannot be reused for another distribution or image even when its
`platform_environment` value is still `wsl2`.

`binding_digest` has the canonical `sha256:<64-lowercase-hex>` form defined
above. `executable_digest` is the raw 64-lowercase-hex SHA-256 of the observed
Codex executable and exactly equals
`process_binding.executable_digest` and the matched support entry's
`codex_artifact_digest`. `policy_digest` is the exact current canonical
`policy_fingerprint` in `sha256:<64-lowercase-hex>` form.
`verifier_build_digest` is the raw 64-lowercase-hex SHA-256 of the exact
Volicord verifier executable bytes.

`generated_artifacts_digest` uses the binding codec's `string`, `list`, and
`record` primitives. Each generated artifact entry is the two-field record
`path` then `digest`, where `path` is its normalized absolute platform path
and `digest` is the raw 64-lowercase-hex SHA-256 of its bytes. Entries are
sorted by `path` UTF-8 bytes after duplicate-path rejection:

```text
generated_artifacts_digest =
  "sha256:" || lowercase_hex(sha256(
    "volicord.generated-managed-artifacts\0"
    || list(generated_artifact_entry_records)
  ))
```

`observed_at` and `expires_at` are canonical RFC 3339 UTC timestamps and must
satisfy `observed_at < expires_at`. `result` is exactly `verified`. A failed,
unavailable, degraded, corrupt, or unsupported verification returns the
applicable Failure Model result and does not issue a receipt with another
`result` value.

A receipt is immutable after issuance. It is evidence for one exact binding,
not a bearer token, user identity, host attestation, or independent source of
Core authority.

<a id="core-receipt-validation"></a>

## Core Receipt Validation

Core consumes only a typed receipt and validates all of the following before a
receipt-dependent operation proceeds:

- `project_id` matches the resolved current project;
- `connection_id` matches the resolved current Agent Connection;
- `host_kind=codex` and `integration_profile=record` match the connection;
- `platform_environment` matches the current connection, binding,
  `process_binding`, and exact support entry;
- `platform_release_coordinate` matches the current independently observed
  coordinate, binding, receipt, and support entry exactly;
- `required_capabilities` and `verified_capabilities` exactly equal each other,
  `FirstReleaseCodexCapabilities`, and the support entry's
  `verified_capabilities`;
- `policy_digest` matches the current policy basis;
- `binding_digest` and `generated_artifacts_digest` match the current stored
  binding and managed artifact identity;
- `executable_digest` matches the current process binding and exact support-entry
  `codex_artifact_digest`;
- `verifier_build_digest` matches the currently accepted verifier build;
- `contract_id=volicord.host-verification-receipt`,
  `result=verified`, and `observed_at <= current_time < expires_at`;
- the receipt is bound to the current Store records, including the current
  project, connection, binding, policy, and capability requirements.

A Store change that invalidates any compared fact makes the receipt stale even
before `expires_at`. A platform lifecycle change that invalidates
`process_binding` also makes it stale; WSL2 restart behavior is defined by
[System Requirements](system-requirements.md#wsl2-topology). Core
does not re-inspect host files or the executable to compensate for an invalid
receipt.

<a id="persisted-host-setup-user-actions"></a>

## Persisted Host-Setup `UserAction` Values

Host setup may persist a typed array of setup actions for connection status and
repair guidance. This array is diagnostic connection state; it is not a Core
`UserActionRequest`, `UserActionResolution`, policy decision, or authority
record.

The complete closed `UserAction` type is validated both before write and after
read:

- a valid `[]` means there are no current host-setup actions;
- syntax failure, a non-array value, an unknown action variant, a missing
  required field, or an invalid payload is not converted to `[]`;
- reads never synthesize host-specific default actions;
- when a default action is required, the adapter computes and validates it
  before persistence;
- corrupt persisted data is reported with category `corrupt` and
  machine-readable reason `persisted_user_actions_corrupt`;
- when the connection's core managed binding remains usable, its status is
  machine-readably `degraded` and identifies the unavailable setup guidance;
- any operation that depends on the corrupt action data fails closed; and
- an explicit verify or repair flow may regenerate the current typed value.

Ordinary reads do not repair, migrate, guess, or replace the value. The
[Failure Model](failure-model.md) owns the distinction between `Corrupt` and a
`Degraded` connection projection.

<a id="threat-model"></a>

## Threat Model

Trusted:

- the same operating-system user account;
- the `Volicord Runtime Home` owned by that account; and
- that account's Store write access.

Untrusted:

- external host input;
- a stale receipt;
- a receipt for another project or connection;
- manually modified configuration;
- a modified executable or generated artifact; and
- a Codex artifact absent from the supported manifest.

Tampering with Runtime Home by a malicious process running with the same user
permissions is outside the first-release threat model. This contract does not
add receipt signing, an operating-system keystore, key rotation, or revocation.

<a id="adjacent-owners"></a>

## Adjacent Owners

- External descriptor selection:
  [External Contracts](external-contracts.md).
- Canonical failure categories:
  [Failure Model](failure-model.md).
- Managed stdio MCP behavior:
  [MCP Transport](mcp-transport.md).
- Install, verify, repair, and uninstall commands:
  [Administrative CLI](admin-cli.md).
- Platform cells and WSL2 topology:
  [System Requirements](system-requirements.md).
- Exact Codex release artifacts and capabilities:
  [Host Release Evidence](host-release-evidence.md).
- Runtime and repository path boundaries:
  [Runtime Boundaries](runtime-boundaries.md).
- Security guarantees and non-guarantees:
  [Security](security.md).
