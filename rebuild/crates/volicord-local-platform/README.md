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
