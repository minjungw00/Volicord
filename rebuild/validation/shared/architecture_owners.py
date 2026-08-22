"""Authoritative maintained identity of current Phase 3 architecture owners."""

ACTIVE_ARCHITECTURE_OWNER_FILES = (
    "architecture.md",
    "domain-model.md",
    "repository-intelligence.md",
    "privacy-and-provider-boundary.md",
    "inquiry-and-decision.md",
    "projections-and-documents.md",
    "portable-context.md",
    "versioning-policy.md",
    "failure-and-recovery.md",
)

ACTIVE_ARCHITECTURE_OWNER_PATHS = tuple(
    f"rebuild/docs/design/{name}" for name in ACTIVE_ARCHITECTURE_OWNER_FILES
)
