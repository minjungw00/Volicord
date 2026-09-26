"""Reviewer-local discovery and post-reveal one-to-one coverage validation.

Mappings are evidence-grounded steward judgments, not a semantic oracle. A
resolution may correct facts about a fixed dimension, never add a dimension.
"""
import re


ASSESSMENT_FIELDS = {
    "dimension_id", "outcome_scope", "classification", "materiality_conclusion",
    "material_outcome_unavoidable", "operator_prompt_does_not_disclose_material_outcome",
    "basis", "provenance_reference_indices",
}
COVERAGE_FIELDS = {
    "obligation", "dimension_id", "reviewer_outcome_scope", "evaluator_outcome_scope",
    "status", "basis", "provenance_reference_indices",
}
MAX_DIMENSIONS = 32


def bounded(value, maximum):
    return isinstance(value, str) and bool(value.strip()) and len(value.encode()) <= maximum


def references_valid(indices, count):
    return (isinstance(indices, list) and bool(indices)
        and all(type(i) is int and 0 <= i < count for i in indices)
        and len(indices) == len(set(indices)))


def assessment_errors(assessments, rules, reference_count, maximum):
    if not isinstance(assessments, list) or not 1 <= len(assessments) <= MAX_DIMENSIONS:
        return ["provisional review requires bounded independently discovered assessments"]
    errors, identities = [], []
    for item in assessments:
        if not isinstance(item, dict) or set(item) != ASSESSMENT_FIELDS:
            errors.append("blind assessment requires the current dimension fields")
            continue
        identity = item.get("dimension_id")
        if not isinstance(identity, str) or re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}", identity) is None:
            errors.append("blind assessment requires reviewer-local stable dimension identity")
        else:
            identities.append(identity)
        if not bounded(item.get("outcome_scope"), maximum) or not bounded(item.get("basis"), maximum):
            errors.append("blind assessment requires bounded outcome scope and source-grounded reasoning")
        if not references_valid(item.get("provenance_reference_indices"), reference_count):
            errors.append("blind assessment must cite reviewer-visible provenance")
        classification = item.get("classification")
        rule = rules.get(classification) if isinstance(classification, str) else None
        if rule is None:
            errors.append("blind assessment classification is unsupported")
        elif any(item.get(field) != rule[field] or (field != "materiality_conclusion" and item.get(field) is not rule[field])
                 for field in ("materiality_conclusion", "material_outcome_unavoidable", "operator_prompt_does_not_disclose_material_outcome")):
            errors.append("blind assessment materiality, unavoidability or disclosure is inconsistent")
    if len(identities) != len(set(identities)):
        errors.append("blind assessment dimension identities must be unique")
    return errors


def coverage_errors(rows, provisional, obligations, reference_count, maximum):
    """Require distinct fixed dimensions; missing coverage is never resolvable."""
    if not isinstance(rows, list) or len(rows) != len(obligations):
        return ["blind_coverage_gap: every evaluator obligation requires a pre-reveal assessment mapping"]
    dimensions = {item["dimension_id"]: item for item in provisional.get("assessments", [])
                  if isinstance(item, dict) and isinstance(item.get("dimension_id"), str)} if isinstance(provisional, dict) else {}
    errors, seen_obligations, used_dimensions = [], set(), set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != COVERAGE_FIELDS:
            errors.append("blind_coverage_gap: malformed obligation assessment mapping")
            continue
        obligation = row.get("obligation")
        if not isinstance(obligation, str) or obligation not in obligations or obligation in seen_obligations:
            errors.append("blind_coverage_gap: obligation mappings must cover the exact unique evaluator obligations")
            continue
        seen_obligations.add(obligation)
        identity = row.get("dimension_id")
        dimension = dimensions.get(identity) if isinstance(identity, str) else None
        if dimension is None or identity in used_dimensions or row.get("status") == "blind_coverage_gap":
            errors.append(f"blind_coverage_gap: {obligation} has no distinct independently fixed assessment")
            continue
        used_dimensions.add(identity)
        if row.get("reviewer_outcome_scope") != dimension.get("outcome_scope"):
            errors.append("blind_coverage_gap: mapping rewrites the independently fixed outcome scope")
        if not bounded(row.get("evaluator_outcome_scope"), maximum) or not bounded(row.get("basis"), maximum):
            errors.append("blind coverage mapping requires bounded evaluator scope and equivalence reasoning")
        if not references_valid(row.get("provenance_reference_indices"), reference_count):
            errors.append("blind coverage mapping requires inspectable provenance")
        expected = "independently_assessed" if dimension.get("classification") == obligation else "resolved_from_evidence"
        if row.get("status") != expected:
            errors.append("blind coverage mapping must resolve factual disagreement about the already assessed dimension")
    if seen_obligations != set(obligations):
        errors.append("blind_coverage_gap: evaluator obligation has no independently assessed dimension")
    return errors


def classifications(provisional):
    assessments = provisional.get("assessments", []) if isinstance(provisional, dict) else []
    if not isinstance(assessments, list):
        return []
    return sorted({item["classification"] for item in assessments if isinstance(item, dict)
        and isinstance(item.get("classification"), str)})


def comparison_disagreements(rows, provisional, rules):
    """Compare paired immutable dimensions, never their representative summary."""
    assessments = provisional.get("assessments", []) if isinstance(provisional, dict) else []
    dimensions = {item["dimension_id"]: item for item in assessments if isinstance(item, dict)
        and isinstance(item.get("dimension_id"), str)} if isinstance(assessments, list) else {}
    disagreements = set()
    for row in rows if isinstance(rows, list) else []:
        if not isinstance(row, dict):
            continue
        identity, obligation = row.get("dimension_id"), row.get("obligation")
        dimension = dimensions.get(identity) if isinstance(identity, str) else None
        expected = rules.get(obligation) if isinstance(obligation, str) else None
        if dimension is None or expected is None:
            continue  # Coverage validator emits the explicit gap separately.
        if dimension.get("classification") != obligation:
            disagreements.add("classification")
        for field, label in (("materiality_conclusion", "materiality_conclusion"),
            ("material_outcome_unavoidable", "material_outcome_unavoidable"),
            ("operator_prompt_does_not_disclose_material_outcome", "operator_prompt_disclosure")):
            if dimension.get(field) != expected[field]:
                disagreements.add(label)
    return [field for field in ("classification", "materiality_conclusion", "material_outcome_unavoidable",
        "operator_prompt_disclosure") if field in disagreements]
