# Local platform primitives

This crate owns small Linux process and filesystem mechanisms needed by local
operations. It does not own orchestration, canonical meaning, repository
analysis, user confirmation, or a runtime-home schema.

Process observations keep complete stdout and stderr in distinct
caller-selected artifacts and report exit, signal, timeout, cancellation, and
child-tree cleanup as separate facts. Filesystem observations distinguish
lexical paths from symlink resolution, expose local clone/worktree coordinates
without inferring Project identity, derive bounded display-name hints through
local Git origin lineage without network or source-body reads, type source
fingerprints, and publish ordinary files atomically without replacing an
existing destination.

The crate has no dependency on an excluded Volicord service crate and provides no legacy
API, Runtime Home, migration, or workflow compatibility.

Scoped Source reads pin each directory descriptor and refuse symlinks and special
files. `read_utf8_excerpt` shares that opening boundary, validates UTF-8 and hashes
the full file by streaming up to 16 MiB (plus one overflow sentinel byte), measures
absolute byte/line coordinates, and returns only a requested nonempty region of
at most 16,384 bytes. Local Operations owns snapshot/hash admission, Work authority,
canonical Source binding and retention; this primitive grants none of them.
