"""Closed evidence-purpose boundary shared by collection and durable consumers.

Integrity establishes relationships, never naturalistic authorship or approval.
The rehearsal uses actual Product outputs and explicitly authored input transports.
"""
NATURALISTIC = "naturalistic"
REHEARSAL = "dogfood_rehearsal"
PURPOSES = {NATURALISTIC, REHEARSAL}


def validate(value):
    if value not in PURPOSES:
        raise ValueError("missing or unsupported evidence purpose")
    return value


def capture_purpose(capture):
    metadata = capture.observed_metadata["session_meta"].get("source_metadata")
    authored = capture.observed_metadata["session_meta"].get("source") == "self_authored_test_support"
    if authored or isinstance(metadata, dict) and metadata.get("authorship") == "self_authored_support_not_host_observed":
        if not isinstance(metadata, dict) or metadata.get("evidence_purpose") != REHEARSAL:
            raise ValueError("authored support capture cannot be relabeled as measured evidence")
    if isinstance(metadata, dict) and "evidence_purpose" in metadata:
        return validate(metadata["evidence_purpose"])
    return NATURALISTIC


def require_same(*values):
    purposes = {validate(value.get("evidence_purpose")) for value in values}
    if len(purposes) != 1:
        raise ValueError("evidence purpose disagreement")
    return purposes.pop()


def require_measured(value):
    if validate(value.get("evidence_purpose")) != NATURALISTIC:
        raise ValueError("rehearsal support cannot supply measured evidence or operator approval")
