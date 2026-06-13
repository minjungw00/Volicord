# Checks

Use these read-only documentation checks after documentation edits. This index routes to focused maintenance procedures only. It does not define API, storage, schema, security, runtime, projection, evidence, QA, acceptance, close-readiness, or residual-risk contracts.

Run the checks that match the edit. For most documentation batches, start with [Structure checks](checks/structure.md) and [Links and indexes checks](checks/links-and-indexes.md), then add the focused pages that match the changed content.

## Check pages

| Page | Use for |
|---|---|
| [Structure checks](checks/structure.md) | owner boundaries, route-page shape, active/out-of-scope wording, implementation wording, reference-claim placement, readability, and final reports |
| [Language parity checks](checks/language-parity.md) | English/Korean semantic parity, identifier preservation, Korean structure, and Korean technical style |
| [Terminology checks](checks/terminology.md) | terminology-map usage, mixed-language Korean, `complete` ambiguity, close-readiness wording, and access/security wording terms |
| [API examples checks](checks/api-examples.md) | durable scenarios, field-name consistency, response snapshot consistency, refs, timestamps, and API owner routing in examples |
| [Links and indexes checks](checks/links-and-indexes.md) | relative links, anchors, `README` routes, `doc-index.yaml`, owner gaps, and LLM retrieval routes |

## Result labels

Use `PASS`, `WARN`, `FAIL`, or `SKIP` only as documentation-maintenance labels. A passing documentation check is not runtime conformance, implementation acceptance, QA completion, close readiness, or residual-risk acceptance.

Tie findings to file paths, owner documents, and suggested documentation fixes. If a check is skipped, say why.

## Report shape

Use a compact report shape after meaningful documentation edits:

- Scope:
- Changed files:
- Checks run:
- Findings:
- Skipped checks:
- Residual documentation risks:

The report should say results are documentation-maintenance findings only when that distinction could be unclear.
