# Volicord local viewer

This crate is the loopback-only local Project viewer. It renders existing
projection, privacy, health, document, and Guarded request data and delegates
all writes to `volicord-operations`. It owns no domain state or database.

Run it with an explicit Project and optional separate runtime:

```text
volicord-viewer --runtime /absolute/runtime --project PROJECT_ID
```

Fixed product text is bundled in English and Korean. `--language` records an
arbitrary requested generated-content language without an allowlist.

The loopback HTTP surface renders current Project state for every `GET /`
request. Query parameters select `level=overview|working|deep`, `locale=en|ko`,
and the unrestricted generated-content `language`. Memory correction,
supersession and forgetting, explicit document export, and exact Guarded
confirmation forms submit to Local Operations; the viewer does not persist or
reinterpret their domain state.
