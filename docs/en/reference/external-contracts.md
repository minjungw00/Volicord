# External contracts

This document owns the validation and canonicalization rules shared by
Volicord boundaries that accept Git object IDs. Focused Reference owners define
the other current external formats directly; Volicord has no generic external
descriptor registry, decoder probing, or cross-format compatibility window.

<a id="surface-stability"></a>
## Surface Stability

The shared Git object-ID validation rules are stable. Adapter source layout and
parsing helpers are internal.

<a id="shared-git-object-id-contract"></a>
## Shared Git object-ID contract

Every Volicord boundary that accepts a Git object ID uses the same rules:

- Input is exactly 40 or exactly 64 ASCII hexadecimal characters.
- The only accepted bytes are `0-9`, `a-f`, and `A-F`.
- Uppercase and lowercase hexadecimal input are both accepted.
- The canonical representation is lowercase ASCII hexadecimal.
- Every other length is rejected, including 39, 41 through 63, and 65
  characters.
- Leading or trailing whitespace, a prefix such as `0x`, non-ASCII digits,
  Unicode lookalikes, separators, and an empty value are rejected rather than
  trimmed or normalized.

Storage, comparison, digest input, and adapter output use the canonical
lowercase representation. A caller's accepted uppercase spelling is not a
second identity. This contract does not infer a Git object type or repository
from the identifier alone.

Malformed or unknown untrusted boundary input is `Rejected`. Persisted data
that claims the current owner-defined shape but violates it is `Corrupt`.
Volicord does not try fallback decoders or reinterpret a current boundary from
payload characteristics.

## Adjacent owners

- Product-wide failure categories and no-default-conflation rules:
  [Failure Model](failure-model.md).
- Storage-specific manifest and database-open handling:
  [Storage Versioning](storage-versioning.md).
- Managed host and connection semantics: [Agent Connection](agent-connection.md).
- Public API error branches and codes: [API Errors](api/errors.md).
