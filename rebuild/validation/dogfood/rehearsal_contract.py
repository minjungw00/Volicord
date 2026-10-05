"""Bounded rehearsal evidence contract; no Product execution or archive dependency."""
import hashlib
import copy
import json
from pathlib import Path
import re

def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()

def require(value, message):
    if not value:
        raise ValueError(message)

CONTRACT = "product-backed-dogfood-evidence-rehearsal-2"
FIXTURE = Path(__file__).with_name("fixtures") / "evidence-rehearsal.json"
PRODUCER_FILES = ("rehearsal.py", "rehearsal_support.py", "launch_boundary_support.py", "retention_support.py", "resource_boundary_controls.py", "resource_observer_self_test.py", "resource_coverage_self_test.py", "../linux-codex-integration/launch_readiness.py", "../../crates/volicord-viewer/tests/work_explanation.rs", "../../crates/volicord-operations/tests/support/reading_fixture.rs", "rehearsal_contract.py", "evidence_purpose.py", "campaign.py", "collection_runs.py", "codex_events.py",
    "answer_observations.py", "explanation_evidence.py", "document_realization.py",
    "review_operations.py", "review_captures.py", "review_explanations.py",
    "qualification_policy.py", "result_lineage.py", "resource_observer.py", "harness.py",
    "evaluation.json", "evaluation_runs.py", "machine_findings.py", "machine-policy.json",
    "answer_projection.py", "support_evidence.py", "authority_obligations.py", "cli_observations.py",
    "qualitative_review.py", "workload_intents.py", "identity_provenance.py", "repository_state.py",
    "interaction_diagnostics.py", "../shared/recorded_action_evidence.py", "../../install.sh", "../../scripts/dogfood-campaign",
    "collection_support.py", "collection_runs_self_test.py", "campaign_self_test.py", "resume_self_test.py", "capture_self_test.py",
    "answer_observations_self_test.py", "latest_work_self_test.py", "review_operations_self_test.py", "qualification_self_test.py",
    "review_meaning_self_test.py", "fixtures/typed-host-page.json")

COLLECTION_CONTROLS = {
    'latest_work_malformed_checkpoint_completion_is_uncertainty_not_failed_write': 'latest_work_self_test.LatestWorkTests.test_malformed_checkpoint_completion_is_uncertainty_not_failed_write',
    'latest_work_repeated_checkpoint_receipt_cannot_supersede_newer_publication': 'latest_work_self_test.LatestWorkTests.test_repeated_checkpoint_receipt_cannot_supersede_newer_publication',
    'latest_work_continuation_is_not_newest_goal_creation': 'latest_work_self_test.LatestWorkTests.test_continuation_is_not_newest_goal_creation',
    'latest_work_checkpoint_then_new_goal_preserves_checkpoint_work': 'latest_work_self_test.LatestWorkTests.test_checkpoint_then_new_goal_preserves_checkpoint_work',
    'latest_work_new_work_checkpoint_advances_selection': 'latest_work_self_test.LatestWorkTests.test_new_work_checkpoint_advances_selection',
    'latest_work_goal_fallback_requires_project_checkpoint_absence': 'latest_work_self_test.LatestWorkTests.test_goal_fallback_requires_project_checkpoint_absence',
    'latest_work_ordered_checkpoints_and_unrelated_later_goal': 'latest_work_self_test.LatestWorkTests.test_ordered_checkpoints_and_unrelated_later_goal',
    'latest_work_missing_or_overlapping_checkpoint_chronology_is_indeterminate': 'latest_work_self_test.LatestWorkTests.test_missing_or_overlapping_checkpoint_chronology_is_indeterminate',
    'latest_work_known_checkpoint_existence_survives_unknown_latest_order': 'latest_work_self_test.LatestWorkTests.test_known_checkpoint_existence_survives_unknown_latest_order',
    'latest_work_fresh_goal_without_empty_project_witness_cannot_prove_no_prior_checkpoint': 'latest_work_self_test.LatestWorkTests.test_fresh_goal_without_empty_project_witness_cannot_prove_no_prior_checkpoint',
    'latest_work_wrong_work_and_checkpoint_absence_are_hard': 'latest_work_self_test.LatestWorkTests.test_wrong_work_and_checkpoint_absence_are_hard',
    'latest_work_correct_work_wrong_checkpoint_revision_is_hard': 'latest_work_self_test.LatestWorkTests.test_correct_work_wrong_checkpoint_revision_is_hard',
    'latest_work_checkpoint_removal_and_work_swap_change_actual_basis': 'latest_work_self_test.LatestWorkTests.test_checkpoint_removal_and_work_swap_change_actual_basis',
    'latest_work_unknown_selection_does_not_hide_project_contradiction': 'latest_work_self_test.LatestWorkTests.test_unknown_selection_does_not_hide_project_contradiction',
    'latest_work_later_checkpoint_cannot_change_earlier_recall': 'latest_work_self_test.LatestWorkTests.test_later_checkpoint_cannot_change_earlier_recall',
    'latest_work_unknown_selection_keeps_goal_and_revision_contradictions': 'latest_work_self_test.LatestWorkTests.test_unknown_selection_keeps_goal_and_revision_contradictions',
    'latest_work_raw_transition_reaches_campaign_and_machine_consumers': 'latest_work_self_test.LatestWorkTests.test_raw_transition_reaches_campaign_and_machine_consumers',
    'latest_work_resume_identity_conflict_is_not_masked_by_selector': 'latest_work_self_test.LatestWorkTests.test_resume_identity_conflict_is_not_masked_by_selector',
    'initial_project_recall_without_work': 'answer_observations_self_test.AnswerTests.test_initial_empty_recall_and_later_creation_do_not_require_future_work',
    'resumed_work_identity_conflict': 'answer_observations_self_test.AnswerTests.test_resume_absence_and_present_identity_conflicts_remain_hard',
    'scoped_generated_subject_omission': 'answer_observations_self_test.AnswerTests.test_generated_subject_parent_omission_retains_exact_reason_and_scope',
    'present_generated_subject_contradiction': 'answer_observations_self_test.AnswerTests.test_supported_subject_omission_cannot_mask_present_contradictions',
    'typed_null_page_metadata': 'review_operations_self_test.ProjectionTests.test_typed_null_page_is_bounded_host_exclusion',
    'user_markup_remains_semantic': 'review_operations_self_test.ProjectionTests.test_page_markup_without_host_typing_never_excludes_user_text',
    'copied_host_and_answer_lineage': 'qualification_self_test.FileBoundaryTests.test_host_metadata_user_prose_and_history_replay_from_copied_inputs',
    'request_scope_keeps_return_index_stable': 'answer_observations_self_test.AnswerTests.test_request_project_integrity_input_does_not_expand_return_index',
    'recorded_collection_dependencies': 'collection_runs_self_test.CollectionTests.test_recorded_producer_inventory_survives_current_dependency_addition',
    'project_not_found_before_initialization': 'resume_self_test.ResumeTests.test_project_not_found_before_initialization_is_not_an_identity',
    'project_not_found_identity_conflict': 'resume_self_test.ResumeTests.test_project_not_found_cannot_hide_identity_conflicts',
    'repeated_same_work_recall': 'resume_self_test.ResumeTests.test_repeated_same_work_recalls_preserve_identity',
    'later_recall_identity_conflict': 'resume_self_test.ResumeTests.test_later_recall_conflict_or_malformed_identity_is_hard',
    'bounded_direct_shell_wrapper': 'capture_self_test.CurrentExecutionTests.test_direct_calls_and_ordered_results_keep_distinct_execution_identities',
    'unsupported_execution_reaches_review': 'capture_self_test.CurrentExecutionTests.test_unsupported_execution_reaches_final_consumers_without_private_bodies',
    'unsupported_execution_cannot_pass': 'capture_self_test.CurrentExecutionTests.test_unsupported_later_wrapper_cannot_certify_validation_success',
    'old_candidate_new_collector_publication': 'collection_runs_self_test.CollectionTests.test_old_candidate_new_collector_immutable_publication_and_copy',
    'source_hash_rejection': 'collection_runs_self_test.CollectionTests.test_wrong_source_hash_fails_before_product_reads',
    'candidate_binary_prerequisite': 'collection_runs_self_test.CollectionTests.test_wrong_candidate_binary_retains_normalization_stops_dependents',
    'source_mutation_rejection': 'collection_runs_self_test.CollectionTests.test_source_mutation_during_product_read_rejects_publication',
    'candidate_rebinding_rejection': 'collection_runs_self_test.CollectionTests.test_candidate_rebinding_rejected',
    'collector_identity_rejection': 'collection_runs_self_test.CollectionTests.test_collector_tampering_rejected',
    'historical_rejection_preservation': 'collection_runs_self_test.CollectionTests.test_historical_rejection_omission_rejected',
    'raw_mutation_rejection': 'collection_runs_self_test.CollectionTests.test_raw_mutation_rejected',
}


EXPECTED_INNER = "unresolved"
TEMPORAL_CONTROLS = {
    'historical_explanation_regeneration': ('explanation_readiness', 'realization_binding', 'confirmed_pass', 'advisory'),
    'temporal_recall_correction': ('naturalistic_observation', 'shared_answer_integrity', 'confirmed_pass', 'advisory'),
    'mismatched_final_current_plan': ('explanation_readiness', 'realization_binding', 'confirmed_violation', 'hard_blocking'),
    'tampered_historical_explanation': ('explanation_readiness', 'campaign_inventory', 'confirmed_violation', 'hard_blocking'),
    'incomplete_attempt_fallback': ('explanation_readiness', 'realization_binding', 'confirmed_violation', 'hard_blocking'),
    'explanation_scope_boundary': ('explanation_lifecycle', 'realization_binding', 'confirmed_violation', 'hard_blocking'),
    'post_correction_old_revision': ('naturalistic_observation', 'shared_answer_integrity', 'confirmed_violation', 'hard_blocking'),
    'future_correction_scope': ('naturalistic_observation', 'shared_answer_integrity', 'confirmed_violation', 'hard_blocking'),
    'missing_temporal_evidence': ('naturalistic_observation', 'shared_answer_integrity', 'indeterminate', 'qualitative_review_required'),
    'missing_temporal_evidence_with_violation': ('naturalistic_observation', 'shared_answer_integrity', 'confirmed_violation', 'hard_blocking'),
}
BOUNDARY_CONTROLS = ('retention_metadata_round_trip', 'retention_oversize_atomic', 'candidate_shell_route', 'resource_expectation_lifecycle', 'partial_process_sampling')
CONTROLS = ('duplicate_titles', 'contradictory_shared_answer', 'missing_explanation_plan', 'mismatched_explanation_response', 'review_projection_omission', 'changed_artifact', 'memory_binary_mismatch', 'absent_human_observations', 'rehearsal_not_measured', 'copied_semantic_rehash', *TEMPORAL_CONTROLS, *BOUNDARY_CONTROLS, *COLLECTION_CONTROLS)
TOPOLOGY = {'repository_journeys': 3, 'work_items': 5, 'resume_pairs': 3, 'fresh_sessions': 8, 'work_distribution': {'volicord': 3, 'small-python': 1, 'polyglot-medium': 1}, 'resume_repository_classes': ['polyglot-medium', 'small-python', 'volicord']}

def identities():
    return {"contract": CONTRACT, "fixture_sha256": sha256(FIXTURE),
        "producer_sha256": {name: sha256(Path(__file__).parent / name)
            for name in PRODUCER_FILES}}


def validate_result(value, candidate, *, expected_identities=None):
    """Same predicate for gate consumption and independent bounded verification."""
    expected = expected_identities or identities()
    require(set(value) == {"kind", "candidate_head", "evidence_purpose", "contract", "fixture_sha256",
        "producer_sha256", "status", "teardown", "external_transmission", "operator_approval",
        "executables", "pipeline", "processes", "result_id", "gate_binding"}, "unexpected retained rehearsal content")
    require(set(value["executables"]) == {"volicord", "volicord-mcp", "volicord-viewer"}
        and all(re.fullmatch(r"[0-9a-f]{64}", v) for v in value["executables"].values()), "invalid executable identities")
    require(value.get("kind") == "dogfood_evidence_rehearsal"
        and value.get("candidate_head") == candidate and value.get("evidence_purpose") == "dogfood_rehearsal"
        and all(value.get(k) == v for k, v in expected.items())
        and value.get("status") == "passed" and value.get("teardown") == "completed"
        and value.get("external_transmission") == "none" and value.get("operator_approval") == "not_provided",
        "rehearsal identity/status boundary failed")
    require(value.get("result_id") == digest({k: v for k, v in value.items() if k != "result_id"}),
        "rehearsal result hash changed")
    validate_binding(value["gate_binding"])
    pipeline = value["pipeline"]
    require(set(pipeline) == {"evidence_set_sha256", "evaluation_run_id", "qualification_run_id",
        "expected_inner_verdict", "technical_evidence", "human_observations", "unresolved_criteria_count",
        "hard_findings", "copied_lineage_id", "copied_verification", "resource_sample_count", "topology",
        "measured_evidence_eligible", "controls", "executables", "temporal_evidence", "boundary_evidence", "collection_support"}, "unexpected pipeline content")
    for key in ("evidence_set_sha256", "evaluation_run_id", "qualification_run_id", "copied_lineage_id"):
        require(re.fullmatch(r"[0-9a-f]{64}", pipeline[key]), "invalid pipeline identity")
    require(pipeline["expected_inner_verdict"] == EXPECTED_INNER
        and pipeline["executables"] == value["executables"]
        and pipeline["hard_findings"] == []
        and type(pipeline["unresolved_criteria_count"]) is int
        and type(pipeline["resource_sample_count"]) is int
        and pipeline["technical_evidence"] == "not_provided" and pipeline["human_observations"] == "not_provided"
        and pipeline["unresolved_criteria_count"] > 0 and pipeline["copied_verification"] == "verified"
        and pipeline["resource_sample_count"] > 0 and pipeline["measured_evidence_eligible"] is False
        and pipeline["topology"] == TOPOLOGY
        and pipeline["controls"] == dict.fromkeys(CONTROLS, "passed"),
        "rehearsal pipeline outcome inconsistent")
    records = value.get("processes")
    require(isinstance(records, list) and len(records) >= 16 and len(records) <= 200
        and len({r["identity"] for r in records}) == len(records)
        and all(type(r["exit_code"]) is int and r["exit_code"] == 0 and r["termination"] == "exited" and type(r["duration_ns"]) is int and r["duration_ns"] >= 0
            for r in records), "missing/failed rehearsal process evidence")
    for record in records:
        require(set(record) == {"identity", "exit_code", "termination", "duration_ns", "stdout", "stderr"}
            and re.fullmatch(r"[a-z0-9-]{1,80}", record["identity"]), "unexpected process content")
        for stream in ("stdout", "stderr"):
            require(set(record[stream]) == {"bytes", "sha256"}
                and type(record[stream]["bytes"]) is int and record[stream]["bytes"] >= 0
                and re.fullmatch(r"[0-9a-f]{64}", record[stream]["sha256"]), "invalid process stream evidence")
    validate_temporal(pipeline['temporal_evidence'], records)
    validate_boundaries(pipeline['boundary_evidence'], records, value['executables'])
    validate_collection_support(pipeline['collection_support'], records)
    return value


def validate_collection_support(value, processes):
    require(isinstance(value, dict) and set(value) == {'process_identity', 'result'}, 'missing collection support execution')
    result = value['result']
    require(isinstance(result, dict) and set(result) == {'kind', 'purpose', 'checks', 'result_id'}
        and result['kind'] == 'dogfood_collection_support' and result['purpose'] == 'self_authored_support'
        and result['result_id'] == digest({k: v for k, v in result.items() if k != 'result_id'}),
        'invalid collection support receipt')
    expected = [{'id': key, 'test_name': name, 'tests_run': 1, 'status': 'passed', 'failures': 0, 'errors': 0}
        for key, name in COLLECTION_CONTROLS.items()]
    require(result['checks'] == expected, 'required collection support was omitted or not executed')
    matches = [p for p in processes if p['identity'] == value['process_identity']]
    body = (json.dumps(result, indent=2, sort_keys=True) + '\n').encode()
    require(len(matches) == 1 and matches[0]['exit_code'] == 0 and matches[0]['termination'] == 'exited'
        and matches[0]['stdout'] == {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()},
        'collection support does not bind actual process output')


def validate_boundaries(value, processes, executables):
    require(isinstance(value, dict) and set(value)=={'retention','launch','resource'}, 'missing boundary rehearsal evidence')
    def process_ref(name, output=None):
        found=next((p for p in processes if p['identity']==name), None)
        require(found is not None and found['exit_code']==0 and found['termination']=='exited', 'boundary process absent/failed')
        if output is not None:
            require(output==found['stdout'] and output['bytes']>0, 'boundary output lacks actual process binding')
    for name in ('retention','resource'):
        wrapper=value[name]
        require(set(wrapper)==({'process','output','result'} if name=='retention' else {'process','output','result','identity_controls'}), 'unexpected boundary wrapper')
        process_ref(wrapper['process'],wrapper['output'])
    retention=value['retention']['result']
    require(set(retention)=={'kind','candidate_cli_sha256','lifecycles','collection_index','oversize','evidence_role'}
        and retention['kind']=='retention_boundary_support' and retention['candidate_cli_sha256']==executables['volicord']
        and retention['evidence_role']=='authored_real_product_not_host_semantic_or_naturalistic', 'retention identity changed')
    artifact(retention['collection_index'])
    lives=retention['lifecycles']
    require(len(lives)==4 and {(v['subject_kind'],v['language']) for v in lives}=={(k,l) for k in ('work','decision') for l in ('en','ko')}
        and len({v['identity'] for v in lives})==4, 'missing many-Source language/subject cases')
    for life in lives:
        require(set(life)=={'subject_kind','language','source_count','response_bytes','retained_bytes','after_state','identity','stages','copied_review'}
            and type(life['source_count']) is int and life['source_count']>=32 and life['response_bytes']==3181
            and type(life['retained_bytes']) is int and 16384<life['retained_bytes']<=147456
            and life['after_state']=='current', 'retention boundary was not exercised')
        identity(life['identity']); artifact(life['copied_review'])
        require(set(life['stages'])=={'attempt','preparation','response','record','after','receipt'}, 'missing retained Product/campaign stages')
        for stage in life['stages'].values(): artifact(stage)
    negative=retention['oversize']
    require(set(negative)=={'exit_code','termination','duration_ns','prior_record_preserved','actionable_byte_error','stdout','stderr','readback'}
        and type(negative['exit_code']) is int and negative['exit_code']>0 and negative['termination']=='exited'
        and type(negative['duration_ns']) is int and negative['duration_ns']>=0
        and negative['prior_record_preserved'] is True and negative['actionable_byte_error'] is True, 'oversize atomic failure absent')
    artifact(negative['stderr']); artifact(negative['readback'])
    require(set(negative['stdout'])=={'bytes','sha256'} and type(negative['stdout']['bytes']) is int and negative['stdout']['bytes']>=0, 'missing oversize stdout')
    hash_value(negative['stdout']['sha256'])
    launch=value['launch']
    require(set(launch)=={'kind','execution_channel','actual_host_proof','project_id','mcp_process','routes','negative_controls'}
        and launch['kind']=='launch_boundary_support' and launch['execution_channel']=='local_subprocess_support'
        and launch['actual_host_proof']=='not_supplied_by_local_rehearsal'
        and launch['negative_controls']=={'wrong_executable':'rejected','wrong_runtime':'rejected'}, 'local launch support promoted to host proof')
    identity(launch['project_id']);process_ref(launch['mcp_process'])
    routes=launch['routes']
    require(len(routes)==2 and {r['shell'] for r in routes}=={'login','non_login'}, 'missing login/non-login route')
    for r in routes:
        require(set(r)=={'shell','bare_matches_candidate','status','cli_sha256','mcp_sha256','runtime_binding','repository_binding','process','output'}
            and r['status']=='ready' and r['bare_matches_candidate'] is (r['shell']=='non_login')
            and r['cli_sha256']==executables['volicord'] and r['mcp_sha256']==executables['volicord-mcp'], 'shadow route identity changed')
        hash_value(r['runtime_binding']);hash_value(r['repository_binding']);process_ref(r['process'],r['output'])
    require(routes[0]['runtime_binding']==routes[1]['runtime_binding'] and routes[0]['repository_binding']==routes[1]['repository_binding'], 'shell binding drift')
    resource=value['resource']['result']
    require(set(resource)=={'kind','candidate_mcp_sha256','results','processes','evidence_role'}
        and resource['kind']=='resource_boundary_support' and resource['candidate_mcp_sha256']==executables['volicord-mcp']
        and resource['evidence_role']=='real_sibling_support_not_host_or_naturalistic', 'resource identity changed')
    expected={'active_runtime':('measured',None), 'concurrent_runtime':('measured',None), 'active_plus_waiting':('measured',None),
        'active_plus_unknown':('partial','unknown_coverage'), 'missing_expected':('partial','missing_registration'),
        'stop_during_active':('partial','active_at_detach'), 'observer_interruption':('partial','observer_interrupted'),
        'sequential_eof':('measured',None), 'abrupt_exit':('partial','process_gone'),
        'pre_attachment_exit':('not_observed','unsampled_instance'), 'zero_expected_samples':('not_observed','missing_registration')}
    require(set(resource['results'])==set(expected)|{'forged_completion','partial_process_sampling'}, 'missing resource boundary cases')
    for name,(status,error) in expected.items():
        result=resource['results'][name]
        require(set(result)=={'status','sample_count','measurement_errors','termination','lifecycles','artifact_sha256'}
            and result['status']==status and type(result['sample_count']) is int
            and (result['sample_count']>0 if status in {'measured','partial'} else result['sample_count']==0)
            and (not result['measurement_errors'] if error is None else error in result['measurement_errors']), 'resource semantic outcome changed')
        require(result['termination']==('stop_requested' if name=='stop_during_active' else 'interrupted' if name=='observer_interruption' else 'duration_elapsed'), 'resource stop/interruption fact changed')
        hash_value(result['artifact_sha256'])
    require(resource['results']['sequential_eof']['lifecycles']==['stopped','stopped','stopped']
        and resource['results']['abrupt_exit']['lifecycles']==['gone'], 'normal/uncertain termination conflated')
    validate_partial_sampling(resource['results']['partial_process_sampling'], resource['results']['concurrent_runtime'])
    forged=resource['results']['forged_completion']
    require(set(forged)=={'status','mutations','basis_sha256'} and forged['status']=='rejected' and forged['mutations']==5
        and forged['basis_sha256']==resource['results']['missing_expected']['artifact_sha256'], 'forged completion control absent')
    require(resource['processes'] and any(p['exit_code']==-9 and p['termination']=='sigkill' for p in resource['processes']), 'abrupt exit lacks real process result')
    for p in resource['processes']:
        require(set(p)=={'label','exit_code','termination','duration_ns','stdout_sha256','stderr_sha256'}
            and p['exit_code']==(-9 if p['termination']=='sigkill' else 0) and p['termination'] in {'sigkill','stdin_eof'}
            and type(p['duration_ns']) is int and p['duration_ns']>=0, 'resource process outcome missing')
        hash_value(p['stdout_sha256']);hash_value(p['stderr_sha256'])
    checks=value['resource']['identity_controls']
    require(set(checks)=={'pid_reuse','executable_mismatch','inaccessible','disappearance','pid_reuse_simulated','observer_failure','sample_gap','registration_failure'}
        and checks['pid_reuse']==checks['executable_mismatch']=='rejected' and checks['observer_failure']=='failed'
        and checks['sample_gap']=='partial', 'missing resource identity/error controls')
    for name in ('inaccessible','disappearance','pid_reuse_simulated','registration_failure'): hash_value(checks[name])


def validate_partial_sampling(value, positive):
    """Independent portable oracle over retained current-contract primitive fields.

    This does not call the observer validator or trust a control's passed label.
    No executable/Runtime paths, process bodies or registration secrets are copied.
    """
    require(isinstance(value, dict) and set(value) == {'status','tick_index','omitted_instance_id',
        'basis_sha256','mutated_sha256','restored_sha256','original','mutated','rejection'},
        'missing partial process sampling evidence')
    require(value['status'] == 'rejected' and value['rejection'] == 'running instance lacks tick sample'
        and value['basis_sha256'] == value['restored_sha256'] == positive['artifact_sha256'],
        'partial sampling lacks actual rejection/restoration')
    hash_value(value['mutated_sha256'])
    require(value['mutated_sha256'] != value['basis_sha256'], 'partial sampling was not mutated')
    original = value['original']
    require(isinstance(original, dict) and set(original) == {'ticks','instances','duration_ns','interval_ns','measurement','status'}
        and original['status'] == 'measured' and type(original['duration_ns']) is int
        and type(original['interval_ns']) is int and 50_000_000 <= original['interval_ns'] <= 60_000_000_000
        and isinstance(original['ticks'], list) and 2 <= len(original['ticks']) <= 20
        and isinstance(original['instances'], list) and len(original['instances']) == 2,
        'missing complete concurrent primitive basis')
    instances = {}
    runtime = None
    for item in original['instances']:
        require(set(item) == {'identity','samples'} and set(item['identity']) == {'instance_id','runtime_binding'},
            'unexpected partial sampling identity content')
        ident = item['identity']['instance_id']; identity(ident); hash_value(item['identity']['runtime_binding'])
        require(ident not in instances and isinstance(item['samples'], list), 'duplicate/invalid process primitive')
        if runtime is None: runtime = item['identity']['runtime_binding']
        require(item['identity']['runtime_binding'] == runtime, 'foreign Runtime sample')
        instances[ident] = item
    last = -1
    for index, tick in enumerate(original['ticks']):
        require(set(tick) == {'elapsed_ns','errors','runtimes'} and type(tick['elapsed_ns']) is int
            and last < tick['elapsed_ns'] <= original['duration_ns'] and tick['errors'] == []
            and isinstance(tick['runtimes'], list) and len(tick['runtimes']) == 1,
            'invalid concurrent tick basis')
        if last >= 0: require(tick['elapsed_ns'] - last <= original['interval_ns'] * 2, 'concurrent basis has gap')
        last = tick['elapsed_ns']
        rt = tick['runtimes'][0]
        require(set(rt) == {'runtime_binding','expectation','authority','registered','sampled'}
            and rt['runtime_binding'] == runtime and rt['expectation'] == 'unknown' and rt['authority'] == 'none'
            and isinstance(rt['registered'], list) and len(rt['registered']) == 2
            and all(set(r) == {'instance_id','state'} and r['state'] == 'running' for r in rt['registered'])
            and {r['instance_id'] for r in rt['registered']} == set(instances)
            and isinstance(rt['sampled'], list) and len(rt['sampled']) == 2 and set(rt['sampled']) == set(instances),
            'concurrent basis lacks per-instance sampling')
        end = original['ticks'][index+1]['elapsed_ns'] if index+1 < len(original['ticks']) else original['duration_ns']
        for item in instances.values():
            require(len(item['samples']) == len(original['ticks']), 'basis sample/tick count mismatch')
            sample = item['samples'][index]
            require(set(sample) == {'elapsed_ns','rss_bytes'} and type(sample['elapsed_ns']) is int
                and tick['elapsed_ns'] <= sample['elapsed_ns']
                and (sample['elapsed_ns'] < end if index+1 < len(original['ticks']) else sample['elapsed_ns'] <= end)
                and type(sample['rss_bytes']) is int and sample['rss_bytes'] >= 0,
                'basis sample identity/tick mismatch')
    def derived(payload):
        numbers = [s['rss_bytes'] for i in payload['instances'] for s in i['samples']]
        return {'sample_count':len(numbers), 'peak_rss_bytes':max(numbers)}
    require(original['measurement'] == derived(original)
        and original['measurement']['sample_count'] == positive['sample_count'], 'basis derived measurements disagree')
    index = value['tick_index']; ident = value['omitted_instance_id']
    require(type(index) is int and 0 <= index < len(original['ticks']) and ident in instances,
        'missing omitted tick/instance identity')
    expected = copy.deepcopy(original)
    next(i for i in expected['instances'] if i['identity']['instance_id'] == ident)['samples'].pop(index)
    expected['ticks'][index]['runtimes'][0]['sampled'].remove(ident)
    expected['measurement'] = derived(expected)
    require(value['mutated'] == expected, 'partial sampling mutation does not match retained instance/tick')
    # The exact transformation keeps both running facts but only the other
    # instance's sample. Rehashed passed claims cannot erase that contradiction.
    require(len(expected['ticks'][index]['runtimes'][0]['sampled']) == 1,
        'partial sampling did not preserve the other process sample')


def hash_value(value):
    require(isinstance(value, str) and re.fullmatch(r'[0-9a-f]{64}', value), 'missing temporal digest')


def identity(value):
    require(isinstance(value, str) and re.fullmatch(r'[0-9a-f]{32}', value), 'missing temporal identity')


def artifact(value):
    require(isinstance(value, dict) and set(value) == {'bytes', 'sha256'}
        and type(value['bytes']) is int and value['bytes'] > 0, 'missing temporal artifact evidence')
    hash_value(value['sha256'])


def validate_temporal(value, processes):
    """Closed portable facts, tied to actual lifecycles, observation and process outcomes."""
    require(isinstance(value, dict) and set(value) == {'regeneration', 'recall', 'outcomes', 'copied'},
        'missing temporal rehearsal evidence')
    regeneration = value['regeneration']
    require(set(regeneration) == {'work_slot_id', 'project_id', 'work_item_id', 'authorization_source_id',
        'correction_process', 'correction_receipt', 'lifecycles'}
        and regeneration['work_slot_id'] == 'journey-polyglot-medium-work-a', 'invalid regeneration binding')
    for key in ('project_id', 'work_item_id', 'authorization_source_id'):
        identity(regeneration[key])
    require(any(p['identity'] == regeneration['correction_process'] and p['stdout']['bytes'] > 0
        for p in processes), 'correction has no retained Product process')
    artifact(regeneration['correction_receipt'])
    lives = regeneration['lifecycles']
    require(isinstance(lives, list) and len(lives) == 4 and len({v['identity'] for v in lives}) == 4,
        'both completed historical/final observations in en/ko are required')
    for life in lives:
        require(set(life) == {'identity', 'publication_role', 'selected_identity', 'language', 'goal_revision',
            'goal_sources', 'before_state', 'after_state', 'plan_fingerprint', 'stages'}, 'unexpected lifecycle summary')
        identity(life['identity']); identity(life['selected_identity'])
        require(re.fullmatch(r'sha256:[0-9a-f]{64}', life['plan_fingerprint'])
            and life['after_state'] == 'current' and life['before_state'] in {'unavailable', 'stale', 'current'}
            and life['goal_sources'] and regeneration['authorization_source_id'] not in life['goal_sources'],
            'lifecycle lacks current-at-recording or distinct Source basis')
        for source in life['goal_sources']: identity(source)
        require(set(life['stages']) == {'attempt', 'preparation', 'response', 'record', 'after', 'receipt'},
            'incomplete retained lifecycle')
        for item in life['stages'].values(): artifact(item)
    for language in ('en', 'ko'):
        pair = [v for v in lives if v['language'] == language]
        require(len(pair) == 2, 'cross-locale lifecycle substitution')
        historical = next((v for v in pair if v['publication_role'] == 'historical'), None)
        final = next((v for v in pair if v['publication_role'] == 'final'), None)
        require(historical is not None and final is not None and historical['goal_revision'] == 1
            and final['goal_revision'] == 2 and type(historical['goal_revision']) is int and type(final['goal_revision']) is int
            and historical['selected_identity'] == final['identity'] == final['selected_identity']
            and historical['goal_sources'] == final['goal_sources']
            and historical['plan_fingerprint'] != final['plan_fingerprint'] and final['before_state'] == 'stale',
            'historical/final selection or actual correction missing')
    recall = value['recall']
    require(set(recall) == {'work_slot_id', 'supporting_sources', 'authorization_source_id', 'observations', 'correction'}
        and recall['work_slot_id'] == 'journey-small-python-work-a' and recall['supporting_sources'], 'missing temporal Recall')
    identity(recall['authorization_source_id'])
    require(recall['authorization_source_id'] not in recall['supporting_sources'], 'authorization replaced original Goal Source')
    for source in recall['supporting_sources']: identity(source)
    observations = recall['observations']
    require(len(observations) == 3 and [o['goal_revision'] for o in observations] == [1, 2, 2], 'missing pre/post/resume observations')
    for observation in observations:
        require(set(observation) == {'transport', 'call_id', 'sequence', 'completion_sequence', 'invocation_sequence',
            'raw_capture_sha256', 'status', 'goal_revision', 'basis_sha256'} and observation['transport'] == 'mcp'
            and observation['status'] == 'confirmed_pass' and isinstance(observation['call_id'], str)
            and re.fullmatch(r'[a-zA-Z0-9_-]{1,128}', observation['call_id'])
            and type(observation['goal_revision']) is int
            and all(type(observation[k]) is int and observation[k] >= 0 for k in ('sequence', 'completion_sequence', 'invocation_sequence'))
            and observation['invocation_sequence'] < observation['completion_sequence'], 'missing actual Recall order/status')
        hash_value(observation['raw_capture_sha256']); hash_value(observation['basis_sha256'])
    correction = recall['correction']
    require(set(correction) == {'transport', 'call_id', 'sequence', 'completion_sequence', 'invocation_sequence', 'kind',
        'raw_capture_sha256', 'session_id', 'expected_revision', 'actual_revision', 'authorization_source_id',
        'supporting_sources_changed', 'transition'} and correction['transition'] == 'successful_correction'
        and correction['kind'] == 'correct_context' and correction['transport'] == 'mcp'
        and correction['expected_revision'] == 1 and correction['actual_revision'] == 2
        and correction['authorization_source_id'] == recall['authorization_source_id']
        and correction['supporting_sources_changed'] is False
        and isinstance(correction['session_id'], str) and re.fullmatch(r'[a-zA-Z0-9_-]{1,128}', correction['session_id'])
        and isinstance(correction['call_id'], str) and re.fullmatch(r'[a-zA-Z0-9_-]{1,128}', correction['call_id'])
        and observations[0]['raw_capture_sha256'] == correction['raw_capture_sha256'] == observations[1]['raw_capture_sha256']
        and observations[0]['completion_sequence'] < correction['invocation_sequence']
            < correction['completion_sequence'] < observations[1]['invocation_sequence'], 'correction is not between actual Recalls')
    outcomes = value['outcomes']
    require(isinstance(outcomes, dict) and set(outcomes) == set(TEMPORAL_CONTROLS), 'missing required temporal control outcomes')
    for name, expected in TEMPORAL_CONTROLS.items():
        outcome = outcomes[name]
        require(set(outcome) == {'consumer', 'check', 'status', 'disposition', 'basis_sha256', 'finding', 'artifacts',
            'observation_statuses', 'goal_revisions', 'error_classes', 'rejection_sha256'}
            and tuple(outcome[k] for k in ('consumer', 'check', 'status', 'disposition')) == expected,
            'control outcome differs from maintained consumer/policy')
        hash_value(outcome['basis_sha256']); artifact(outcome['finding'])
        require(isinstance(outcome['error_classes'], list) and len(outcome['error_classes']) <= 2
            and set(outcome['error_classes']) <= {'generated goal revision basis', 'top-level recorded next action'},
            'unbounded/private control error content')
        require(isinstance(outcome['artifacts'], list) and outcome['artifacts'], 'pass label has no control artifacts')
        for item in outcome['artifacts']: artifact(item)
        if outcome['consumer'] == 'naturalistic_observation':
            require(outcome['rejection_sha256'] is None, 'Recall requires actual observations')
            if name == 'future_correction_scope':
                require(outcome['observation_statuses'] == ['confirmed_violation', 'confirmed_pass', 'confirmed_pass']
                    and outcome['goal_revisions'] == [1, 2, 2], 'future revision was not rejected at the earlier observation')
            else:
                statuses = ['confirmed_pass', expected[2], expected[2]] if name.startswith('missing_temporal') else (
                    ['confirmed_pass', 'confirmed_violation', 'confirmed_pass'] if name == 'post_correction_old_revision' else ['confirmed_pass'] * 3)
                # Missing receipt uncertainty crosses the proven resume relation; independent errors are scoped to the changed read.
                if name == 'missing_temporal_evidence_with_violation': statuses[-1] = 'indeterminate'
                require(outcome['observation_statuses'] == statuses
                    and outcome['goal_revisions'] == ([1, None, None] if name.startswith('missing_temporal') else [1, 2, 2]),
                    'missing scoped temporal observation outcomes')
            if name in {'post_correction_old_revision', 'future_correction_scope'}:
                require('generated goal revision basis' in outcome['error_classes'], 'old-current claim was not detected')
            elif name == 'missing_temporal_evidence_with_violation':
                require(outcome['error_classes'], 'independent hard error was hidden by missing evidence')
            else: require(outcome['error_classes'] == [], 'positive/gap contains a hard violation')
        else:
            require(outcome['observation_statuses'] == [] and outcome['goal_revisions'] == []
                and outcome['error_classes'] == [], 'unexpected explanation observation summary')
            if name != 'historical_explanation_regeneration': hash_value(outcome['rejection_sha256'])
    require(outcomes['historical_explanation_regeneration']['basis_sha256'] == digest({'lifecycles': lives})
        and outcomes['historical_explanation_regeneration']['artifacts'] == [v['stages']['receipt'] for v in lives],
        'regeneration success has no retained lifecycle evidence')
    copied = value['copied']
    require(set(copied) == {'lifecycles', 'recall_capture', 'evaluation'} and set(copied['lifecycles']) == {v['identity'] for v in lives},
        'copied lineage omitted historical/final observations')
    for item in copied['lifecycles'].values(): artifact(item)
    artifact(copied['recall_capture'])
    artifact(copied['evaluation'])

def validate_binding(value):
    if value is None:
        return
    require(isinstance(value, dict) and set(value) == {'gate_invocation', 'final_sha256', 'binding_sha256'}
        and re.fullmatch(r'[A-Za-z0-9_.-]{1,160}', str(value['gate_invocation']))
        and all(re.fullmatch(r'[0-9a-f]{64}', str(value[k])) for k in ('final_sha256', 'binding_sha256')),
        'invalid bounded gate binding')


def validate_retained_result(value, candidate, expected):
    """Failure receipts retain only closed identities/counts; messages stay private."""
    if value.get('status') == 'passed':
        return validate_result(value, candidate, expected_identities=expected)
    common = {'kind', 'candidate_head', 'evidence_purpose', 'contract', 'fixture_sha256',
        'producer_sha256', 'status', 'teardown', 'external_transmission', 'operator_approval',
        'processes', 'result_id', 'gate_binding', 'failure_kind'}
    require(common <= set(value) <= common | {'executables', 'pipeline'}, 'unexpected failed rehearsal content')
    require(value['kind'] == 'dogfood_evidence_rehearsal' and value['candidate_head'] == candidate
        and value['evidence_purpose'] == 'dogfood_rehearsal' and value['status'] == 'failed'
        and value['teardown'] in {'completed', 'failed'} and value['external_transmission'] == 'none'
        and value['operator_approval'] == 'not_provided' and all(value[k] == v for k, v in expected.items())
        and re.fullmatch(r'[A-Za-z]{1,80}', str(value['failure_kind']))
        and value['result_id'] == digest({k: v for k, v in value.items() if k != 'result_id'}),
        'invalid failed rehearsal receipt')
    validate_binding(value['gate_binding'])
    require(isinstance(value['processes'], list) and len(value['processes']) <= 200, 'invalid failed processes')
    for record in value['processes']:
        require(set(record) == {'identity', 'exit_code', 'termination', 'duration_ns', 'stdout', 'stderr'}
            and re.fullmatch(r'[a-z0-9-]{1,80}', str(record['identity']))
            and type(record['exit_code']) is int and record['termination'] in {'exited', 'signal', 'timeout', 'interrupted'}
            and type(record['duration_ns']) is int and record['duration_ns'] >= 0, 'invalid failed process')
        for name in ('stdout', 'stderr'):
            stream = record[name]
            require(set(stream) == {'bytes', 'sha256'} and type(stream['bytes']) is int and stream['bytes'] >= 0
                and re.fullmatch(r'[0-9a-f]{64}', str(stream['sha256'])), 'invalid failed stream')
    if 'executables' in value:
        require(set(value['executables']) == {'volicord', 'volicord-mcp', 'volicord-viewer'}
            and all(re.fullmatch(r'[0-9a-f]{64}', str(v)) for v in value['executables'].values()),
            'invalid failed executable identity')
    if 'pipeline' in value:
        # Only an already completed pipeline can precede a later continuity failure.
        positive = {k: v for k, v in value.items() if k != 'failure_kind'}
        positive.update(status='passed', teardown='completed')
        positive['result_id'] = digest({k: v for k, v in positive.items() if k != 'result_id'})
        validate_result(positive, candidate, expected_identities=expected)
    return value


def dependencies(root):
    base = root / 'rebuild/validation/dogfood'
    paths = {'fixture': base / 'fixtures/evidence-rehearsal.json',
        **{name: base / name for name in PRODUCER_FILES}}
    return {'contract': CONTRACT, 'inputs': {name: {'path': path.resolve().relative_to(root.resolve()).as_posix(),
        'sha256': sha256(path), 'status': 'available'} for name, path in paths.items()}}


def expected_from_dependencies(value):
    require(set(value) == {'contract', 'inputs'} and value['contract'] == CONTRACT,
        'rehearsal dependencies are missing')
    inputs = value['inputs']
    require(set(inputs) == {'fixture', *PRODUCER_FILES}, 'rehearsal dependency inventory changed')
    for name, entry in inputs.items():
        path = (Path('rebuild/validation/dogfood') / ('fixtures/evidence-rehearsal.json' if name == 'fixture' else name))
        # Resolve lexical ../shared without consulting a source checkout.
        from posixpath import normpath
        require(entry == {'path': normpath(str(path)), 'sha256': entry.get('sha256'), 'status': 'available'}
            and isinstance(entry['sha256'], str) and re.fullmatch(r'[0-9a-f]{64}', entry['sha256']),
            'rehearsal dependency identity malformed')
    return {'contract': CONTRACT, 'fixture_sha256': inputs['fixture']['sha256'],
        'producer_sha256': {name: inputs[name]['sha256'] for name in PRODUCER_FILES}}


def not_run_stage():
    return {'contract': CONTRACT, 'status': 'not_run', 'invocation_count': 0,
        'result_sha256': None, 'result': None, 'execution': None}


def bounded_execution(value):
    streams = {}
    for key in ('stdout', 'stderr'):
        path = value.get(key)
        if not path:
            streams[key] = None
        else:
            try:
                body = Path(path).read_bytes()
                streams[key] = {'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()}
            except OSError:
                streams[key] = None
    return {'exit_code': value.get('exit_code'), 'wrapper_exit_code': value.get('wrapper_exit_code'),
        'termination': value.get('termination'), 'spawn_error': value.get('spawn_error') is not None,
        'duration_ms': value.get('duration_ms'), 'started_at': value.get('started_at'),
        'ended_at': value.get('ended_at'), **streams}


def stage_evidence(result, execution, path, candidate, final_hash, invocation, dependency):
    retained = None
    try:
        if path.is_file() and json.loads(path.read_bytes()) == result:
            retained = validate_retained_result(result, candidate, expected_from_dependencies(dependency))
    except (ValueError, OSError, TypeError, KeyError, AttributeError):
        pass
    evidence = {'contract': CONTRACT, 'status': 'failed', 'invocation_count': 1,
        'result_sha256': sha256(path) if retained is not None else None,
        'result': retained, 'execution': bounded_execution(execution)}
    if execution.get('spawn_error') is not None:
        evidence['status'] = 'environment_blocked'
        return evidence
    try:
        require(type(execution.get('exit_code')) is int and execution.get('exit_code') == 0
            and type(execution.get('wrapper_exit_code')) is int and execution.get('wrapper_exit_code') == 0
            and execution.get('termination') is None, 'rehearsal process failed')
        require(path.is_file() and json.loads(path.read_bytes()) == result, 'rehearsal result is missing or changed')
        validate_result(result, candidate, expected_identities=expected_from_dependencies(dependency))
        binding = result['gate_binding']
        require(binding['gate_invocation'] == invocation and binding['final_sha256'] == final_hash,
            'rehearsal is not bound to this gate Final')
        evidence['status'] = 'passed'
        validate_stage({'dogfood_rehearsal': evidence, 'validated_candidate_head': candidate,
            'gate_invocation': invocation, 'final_summary_sha256': final_hash,
            'dependency_snapshot': {'dogfood_rehearsal': dependency},
            'final_aggregate': {'status': 'succeeded'}, 'contract_coverage_execution': {'status': 'passed'},
            'phase_8_ready': False})
    except (ValueError, OSError, TypeError, KeyError, AttributeError):
        evidence['status'] = 'failed'
    return evidence


def validate_stage(capsule):
    stage = capsule.get('dogfood_rehearsal')
    require(isinstance(stage, dict) and set(stage) == set(not_run_stage())
        and stage['contract'] == CONTRACT, 'mandatory rehearsal stage is missing or malformed')
    expected = expected_from_dependencies(capsule.get('dependency_snapshot', {}).get('dogfood_rehearsal', {}))
    require(isinstance(capsule.get('gate_invocation'), str) and re.fullmatch(r'[A-Za-z0-9_.-]{1,160}', capsule['gate_invocation']), 'gate invocation identity missing')
    status = stage['status']
    require(status in {'not_run', 'passed', 'failed', 'environment_blocked'}, 'invalid rehearsal stage status')
    later = capsule.get('live_provider_qualification', {}).get('status', 'not_run') != 'not_run' or capsule.get('official_v11', {}).get('status', 'not_run') != 'not_run'
    if status == 'not_run':
        require(stage == not_run_stage() and not later and capsule.get('phase_8_ready') is not True,
            'not-run rehearsal contains later-stage evidence')
        return False
    require(type(stage['invocation_count']) is int and stage['invocation_count'] == 1,
        'rehearsal must execute exactly once')
    execution = stage['execution']
    require(isinstance(execution, dict) and set(execution) == {'exit_code', 'wrapper_exit_code', 'termination', 'spawn_error', 'duration_ms', 'started_at', 'ended_at', 'stdout', 'stderr'},
        'rehearsal execution evidence is missing')
    require(capsule.get('final_aggregate', {}).get('status') == 'succeeded'
        and capsule.get('contract_coverage_execution', {}).get('status') == 'passed',
        'rehearsal ran before passed Final/mapped tests')
    require(type(execution['spawn_error']) is bool and isinstance(execution['duration_ms'], (int, float))
        and execution['duration_ms'] >= 0, 'invalid rehearsal execution observations')
    from datetime import datetime
    require(datetime.fromisoformat(execution['ended_at']) >= datetime.fromisoformat(execution['started_at']),
        'rehearsal execution timestamps reversed')
    for name in ('stdout', 'stderr'):
        stream = execution[name]
        require(isinstance(stream, dict) and set(stream) == {'bytes', 'sha256'}
            and type(stream['bytes']) is int and stream['bytes'] >= 0
            and isinstance(stream['sha256'], str) and re.fullmatch(r'[0-9a-f]{64}', stream['sha256']),
            'missing rehearsal execution stream identity')
    if status != 'passed':
        require((status == 'environment_blocked') is execution['spawn_error'], 'rehearsal failure classification changed')
        require(not later and capsule.get('phase_8_ready') is not True
            and capsule.get('blocking_classification') in {'dogfood_rehearsal_failed', 'dogfood_rehearsal_environment_blocked'},
            'failed rehearsal cannot permit external stages/readiness')
        require((stage['result'] is None) == (stage['result_sha256'] is None), 'failed result presence changed')
        if stage['result'] is not None:
            validate_retained_result(stage['result'], capsule['validated_candidate_head'], expected)
            require(stage['result_sha256'] == hashlib.sha256((json.dumps(stage['result'], indent=2, sort_keys=True) + '\n').encode()).hexdigest(), 'failed result hash changed')
        require(stage['result_sha256'] is None or re.fullmatch(r'[0-9a-f]{64}', str(stage['result_sha256'])),
            'invalid failed rehearsal result identity')
        return False
    require(type(execution['exit_code']) is int and execution['exit_code'] == 0
        and type(execution['wrapper_exit_code']) is int and execution['wrapper_exit_code'] == 0
        and execution['termination'] is None and execution['spawn_error'] is False,
        'passed rehearsal has unsuccessful execution')
    result = stage['result']
    validate_result(result, capsule['validated_candidate_head'], expected_identities=expected)
    require(stage['result_sha256'] == hashlib.sha256((json.dumps(result, indent=2, sort_keys=True) + '\n').encode()).hexdigest(),
        'retained rehearsal result hash changed')
    binding = result['gate_binding']
    require(isinstance(binding, dict) and set(binding) == {'gate_invocation', 'final_sha256', 'binding_sha256'}
        and isinstance(binding['gate_invocation'], str) and re.fullmatch(r'[A-Za-z0-9_.-]{1,160}', binding['gate_invocation'])
        and binding['gate_invocation'] == capsule['gate_invocation']
        and binding['final_sha256'] == capsule.get('final_summary_sha256')
        and re.fullmatch(r'[0-9a-f]{64}', str(binding['binding_sha256'])), 'rehearsal gate-parent binding missing')
    return True
