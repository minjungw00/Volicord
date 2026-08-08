# Reconstruction validation assets

This directory contains maintained inputs, assertions, report summaries, and
small disposable experiment implementations. It is an internal validation
surface, not a Volicord product command or production architecture.

## Commands

- `rebuild/scripts/validate self-test` checks command execution, output and
  status preservation, signal reporting, and non-fail-fast aggregation with
  fake commands.
- `rebuild/scripts/validate focused <label> -- <command> [arguments...]` runs
  the exact argument vector from the repository root and records the command,
  working directory, timestamps, duration, complete separate stdout/stderr,
  exit code, and termination details.
- `rebuild/scripts/validate final` runs the four root `AGENTS.md` aggregate
  commands in order, records every result, and fails after all commands finish
  if any failed. It is reserved for work whose validation role is `final`.

## Maintained and generated boundaries

Commit self-authored fixtures, their manifest entries, experiment source,
assertions, report templates, and reviewed report summaries. Fixture entries
record purpose, expectations, unsupported constructs, a deterministic content
hash, origin, and license.

Do not commit raw analyzer output, generated graphs, copied source repositories,
logs, measurement scratch data, caches, or local model output. The runner writes
these to ignored `rebuild/.local/validation/`. Maintained reports may cite an
artifact path and hash, but local artifacts are reproducibility evidence rather
than durable design truth.

V01, V03, and V05 share this policy and the report shape in
`report-template.md`. Each experiment owns its fixture subdirectory, assertions,
and maintained report without turning spike code into a production dependency.
