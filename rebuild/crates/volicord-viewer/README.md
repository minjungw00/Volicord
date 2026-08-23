# Volicord local viewer

This crate is the loopback-only local Project viewer. It renders existing
projection, privacy, health, document, and Guarded request data and delegates
all writes to `volicord-operations`. It owns no domain state or database.

Run it with an explicit Project and optional separate runtime:

```text
volicord-viewer --runtime /absolute/runtime --project PROJECT_ID
```

Export the current Viewer projection as one self-contained, read-only local
HTML snapshot and exit:

```text
volicord-viewer --runtime /absolute/runtime --project PROJECT_ID --snapshot /absolute/path/viewer.html
```

Snapshot publication is atomic, refuses a relative or existing destination,
opens no listener, and performs no upload or other network transmission. The
result keeps current degradation, privacy state, document previews, and a
closed Project/canonical/repository-analysis basis disclosure, but contains no
forms, authenticity token, mutation endpoint, script, or live-Viewer link. It
remains readable after the Runtime is no longer available. Sharing the file is
a separate user-controlled action outside this command.

Fixed product text is bundled in English and Korean. `--language` records an
arbitrary requested generated-content language without an allowlist. Because
the local Viewer has no active-host realizer, an arbitrary language displays a
truthful unavailable/degraded notice and never presents its fixed English body
as requested-language success.

The loopback HTTP surface renders current Project state for every `GET /`
request. Query parameters select `level=overview|working|deep`, `locale=en|ko`,
and the unrestricted generated-content `language`. Memory correction,
supersession and forgetting, explicit document export, and exact Guarded
confirmation forms submit to Local Operations; the viewer does not persist or
reinterpret their domain state.

Every level begins with the bounded `ProjectUnderstanding` read model: Goal and
why, completed/current/remaining work, next steps, Decision rationale and code
impact, material Questions, architecture, generated interpretations, evidence,
freshness, and gaps. Inline accessible SVG component/dependency and flow
diagrams are drawn only from inspectable entity/relation topology and require
no JavaScript, CDN, or external renderer. `overview` and `working` lead with
Goal, current work and verification, Decision consequence, open Questions,
next step, and material degradation.
Opaque identities, raw relations, canonical records, and detailed capability
evidence are subordinate to `deep` or closed audit disclosure; they are not
removed from the shared Project projection.
