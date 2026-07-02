# Volicord Documentation Directory

This directory contains the maintained English and Korean Volicord documentation plus the metadata used by maintainers for owner routing and terminology control.

Choose a language first:

- English: [en/README.md](en/README.md)
- Korean: [ko/README.md](ko/README.md)

Comprehensive product and first-setup overview:

- English: [../README.md](../README.md)
- Korean: [../README.ko.md](../README.ko.md)

Fast reader routes:

- Product orientation: [English](en/getting-started/overview.md) / [Korean](ko/getting-started/overview.md)
- Environment applicability: [English](en/reference/system-requirements.md) / [Korean](ko/reference/system-requirements.md)
- Install and verify executables: [English](en/getting-started/installation.md) / [Korean](ko/getting-started/installation.md)
- Choose the Codex or Claude Code setup path: [English](en/getting-started/quickstart.md) / [Korean](ko/getting-started/quickstart.md)
- Complete host setup and recovery: [English setup](en/guides/agent-host-setup.md) / [Korean setup](ko/guides/agent-host-setup.md) / [English troubleshooting](en/guides/agent-host-troubleshooting.md) / [Korean troubleshooting](ko/guides/agent-host-troubleshooting.md)
- Multi-repository operation: [English](en/guides/multi-repository-agent-setup.md) / [Korean](ko/guides/multi-repository-agent-setup.md)
- Exact CLI and API contracts: [English CLI](en/reference/admin-cli.md) / [Korean CLI](ko/reference/admin-cli.md) / [English API](en/reference/api/methods.md) / [Korean API](ko/reference/api/methods.md)
- Reference navigation: [English](en/reference/README.md) / [Korean](ko/reference/README.md)
- Product and maintenance charter: [English](en/maintain/product-maintenance-charter.md) / [Korean](ko/maintain/product-maintenance-charter.md)
- Brand presentation and claim guidance: [English](en/maintain/brand-guidelines.md) / [Korean](ko/maintain/brand-guidelines.md)
- Diagram creation and review policy: [English](en/maintain/diagram-policy.md) / [Korean](ko/maintain/diagram-policy.md)

Shared metadata:

- [doc-index.yaml](doc-index.yaml) is the canonical machine-readable route for documentation owners and paired paths.
- [terminology-map.yaml](terminology-map.yaml) is the terminology and identifier-preservation source of truth.

Those metadata files support maintainers, translators, and agents. Ordinary readers should start from the language entry pages, Getting Started, Guides, Development, or Reference pages.

Maintainers and implementation agents should read [../AGENTS.md](../AGENTS.md), then the relevant documentation, translation, brand, and validation policies under `docs/*/maintain/`.

This README is an entry route. It does not define API behavior, storage effects, security guarantees, schemas, Core authority semantics, or detailed owner maps.
