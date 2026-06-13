# Storage

This page is a route into the storage reference family. It points to storage owners and does not define record layout, branch effects, artifact lifecycle, versioning, API shape, security guarantees, or runtime locations.

## Storage owner routes

| Need | Owner |
|---|---|
| Persistent record layout and storage-owned values | [Storage Records](storage-records.md) |
| Method branch storage effects and API-shape-versus-effect distinctions | [Storage Effects](storage-effects.md) |
| Artifact staging, promotion, linking, body reads, retention, and integrity | [Artifact Storage](storage-artifacts.md) |
| `project_state.state_version`, idempotency, replay, events, locks, and migrations | [Storage Versioning](storage-versioning.md) |
| Product Repository, Harness Server, and Runtime Home locations | [Runtime Boundaries](runtime-boundaries.md) |

For non-storage contracts, route to [API Methods](api/methods.md), API schema owners, [Core Model](core-model.md), [API Errors](api/errors.md), or [Security](security.md) as needed.
