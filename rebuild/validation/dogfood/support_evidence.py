"""Current Work activation and journey-final projection integrity predicates."""


def runtime_activation_valid(runtime, activation, kind, work):
    import harness as h
    if not isinstance(runtime, dict) or not isinstance(activation, dict):
        return False
    roles = h.session_roles(kind, work)
    inventory = runtime.get("managed_file_inventory")
    return bool(
        runtime.get("kind") == "phase8_bounded_runtime_summary"
        and runtime.get("content_included") is False
        and all(type(runtime.get(k)) is int and runtime[k] >= 0
                for k in ("runtime_home_bytes", "derived_analysis_bytes"))
        and runtime["derived_analysis_bytes"] <= runtime["runtime_home_bytes"]
        and isinstance(inventory, list)
        and all(isinstance(item, dict) and set(item) == {"logical_name", "bytes"}
                and h.nonempty_string(item["logical_name"])
                and "/" not in item["logical_name"] and "\\" not in item["logical_name"]
                and type(item["bytes"]) is int and item["bytes"] >= 0 for item in inventory)
        and len({item["logical_name"] for item in inventory}) == len(inventory)
        # Runtime summary is journey-final and shared; activation is Work-specific.
        and runtime.get("start_session_start_activation_observed") is True
        and runtime.get("resume_session_start_activation_observed") is True
        and activation.get("kind") == "phase8_dogfood_activation_summary"
        and activation.get("repository_class") == kind
        and activation.get("journey_id") == h.journey_id(kind)
        and activation.get("work_slot_id") == h.work_slot_id(kind, work)
        and activation.get("start_session_start_activation_observed") is True
        and (activation.get("resume_session_start_activation_observed") is True
             if "resume" in roles else
             activation.get("resume_session_start_activation_observed") == "not_applicable")
        and all(value.get(field) is True for value in (runtime, activation)
                for field in ("repository_config_present", "repository_ownership_manifest_present"))
    )


def journey_projection_valid(context, evidence, bundle, project, candidate, kind, work, directory):
    import harness as h
    if not isinstance(context, dict) or bundle is None:
        return False
    finals = context.get("final_entries")
    entries = context.get("work_entries")
    journey = context.get("journey")
    if (not isinstance(finals, list) or len(finals) != 3
        or not all(isinstance(item, dict) for item in finals)
        or {item.get("journey_id") for item in finals if isinstance(item, dict)}
            != {h.journey_id(k) for k in h.CLASSES}
        or not isinstance(entries, list) or not all(isinstance(item, dict) for item in entries)
        or not isinstance(journey, dict)):
        return False
    identity = h.journey_id(kind)
    slots = [h.work_slot_id(kind, label) for label in h.work_slots(kind)]
    final = next(item for item in finals if item.get("journey_id") == identity)
    related = [item for item in entries if item.get("work_slot_id") in slots]
    by_slot = {item["work_slot_id"]: item for item in related}
    if len(related) != len(slots) or set(by_slot) != set(slots):
        return False
    ordered_ids = [by_slot[slot].get("work_item_id") for slot in slots]
    inventory = final.get("artifact_inventory")
    if not isinstance(inventory, dict):
        return False
    canonical = inventory.get("canonical_bundle")
    if not isinstance(canonical, dict):
        return False
    return bool(
        context.get("candidate_head") == candidate
        and journey.get("project_id") == final.get("project_id") == project == bundle.project_id
        and final.get("represented_work_slot_ids") == slots
        and h.work_slot_id(kind, work) in final["represented_work_slot_ids"]
        and final.get("projection_source_work_slot_id") == h.work_slot_id(kind, "A")
        and final.get("ordered_work_item_ids") == ordered_ids
        and all(h.nonempty_string(value) for value in ordered_ids)
        and len(set(ordered_ids)) == len(slots)
        and all(item.get("journey_id") == identity and item.get("repository_class") == kind
                and item.get("project_id") == project
                and item.get("work_label") == label
                for label, item in zip(h.work_slots(kind), (by_slot[slot] for slot in slots)))
        and all(evidence.get(key) == inventory.get(key) and
                h.verified_evidence_path(inventory.get(key), directory) is not None
                for key in ("canonical_bundle", "generated_documents", "viewer_snapshot"))
        and canonical.get("sha256") == bundle.source_sha256
        and all(item.get("canonical_evidence", {}).get("journey_final_shared_projection") is True
                and all(item["canonical_evidence"].get(key) == canonical.get(key)
                        for key in ("file", "sha256")) for item in related)
        and all(bundle.one("context_items", id=work_id, project_id=project, role="goal") is not None
                and any(row.get("work_item_id") == work_id and row.get("project_id") == project
                        for row in bundle.rows("checkpoints")) for work_id in ordered_ids)
    )
