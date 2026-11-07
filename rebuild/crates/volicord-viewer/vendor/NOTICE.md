# Diagram layout dependencies

Volicord embeds elkjs 0.11.1 (EPL-2.0) and the QuickJS runtime supplied by
rquickjs/rquickjs-sys 0.11.0 (MIT). The installed executable contains the local
engine; Node, npm, a CDN and separate runtime engine files are not required.

ELK source, distribution provenance and build instructions:
https://github.com/kieler/elkjs/tree/0.11.1

Rust bindings and bundled native runtime source:
https://github.com/DelSkayn/rquickjs/tree/c99675e50201d74554d0f667094f800e1287001b
https://crates.io/crates/rquickjs-sys/0.11.0

The adjacent license texts and provenance manifests retain the upstream notices
and exact source/distribution identities. The installation manifest identifies
the selected engine/configuration/metrics and installed Viewer executable.
Geometry is derived presentation, not retained explanation meaning or authority.
