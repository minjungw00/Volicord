"""Identity claims shared by realization and review; binding is not attestation."""


def unknown():
    return {"state": "unknown", "value": None}


def validate_claim(claim, *, allow_runtime_observed=False):
    if not isinstance(claim, dict) or set(claim) != {"state", "value"}:
        raise ValueError("invalid identity provenance shape")
    state, value = claim["state"], claim["value"]
    if state == "unknown" and value is None:
        return
    if (state == "self_reported" and isinstance(value, str) and value.strip()
            and len(value.encode("utf-8")) <= 4000):
        return
    if (allow_runtime_observed and state == "runtime_observed" and isinstance(value, str)
            and value.strip() == value and len(value.encode("utf-8")) <= 4000):
        return
    raise ValueError("identity must be unknown, self_reported, or bound runtime_observed; verified identity is unsupported")
