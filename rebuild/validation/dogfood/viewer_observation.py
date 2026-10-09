"""Closed browser-display context, separate from HTTP and human judgments."""
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import urlparse, parse_qs
import resource_observer

CONTEXT_KEYS = {'kind','schema_version','render_id','mode','process','runtime_binding','project_id',
    'locale','language','view','selected_work','selected_decision','canonical_read_fingerprint',
    'source_state_sha256','source_count','analysis','explanations'}
CAPTURE_KEYS = {'kind','schema_version','evidence_class','candidate_head','url','context','dom_sha256','screenshot','browser'}


def require(condition, message):
    if not condition: raise ValueError(message)


def hex_value(value, length=64):
    return isinstance(value,str) and re.fullmatch('[0-9a-f]{'+str(length)+'}',value) is not None


def local_url(value):
    parsed=urlparse(value)
    require(parsed.scheme=='http' and parsed.hostname in {'127.0.0.1','localhost','::1'}
        and parsed.port is not None and parsed.username is None and parsed.password is None,
        'explicit local browser authority required')
    return parsed


def validate_subjects(subjects):
    require(isinstance(subjects, dict) and set(subjects) == {
        'project_id', 'work_ids', 'decision_ids', 'canonical_bundle_sha256'}
        and hex_value(subjects['project_id'], 32) and hex_value(subjects['canonical_bundle_sha256']),
        'invalid campaign subject inventory')
    for key in ('work_ids', 'decision_ids'):
        ids = subjects[key]
        require(isinstance(ids, list) and all(hex_value(v, 32) for v in ids)
            and ids == sorted(set(ids)), 'invalid campaign subject identities')
    require(subjects['work_ids'], 'campaign subject Work inventory is empty')
    return subjects


def subjects_from_bundle(project, work_ids, bundle):
    """Identity membership only: fresh render state/fingerprints remain observable."""
    work_ids = sorted(work_ids)
    require(bundle.project_id == project and len(set(work_ids)) == len(work_ids)
        and all(bundle.one('context_items', id=work, project_id=project, role='goal') is not None
            and any(row.get('work_item_id') == work and row.get('project_id') == project
                for row in bundle.rows('checkpoints')) for work in work_ids),
        'campaign subject Work lacks canonical identity/history')
    decision_ids = sorted({row['id'] for row in bundle.rows('decisions')
        if row.get('project_id') == project and (row.get('work_scope') == 'project'
            or row.get('work_scope') == 'work_item' and row.get('work_item_id') in work_ids)})
    return validate_subjects({'project_id': project, 'work_ids': work_ids,
        'decision_ids': decision_ids, 'canonical_bundle_sha256': bundle.source_sha256})


def load_subjects(root, manifest):
    import review_operations as operations
    final = next(f for f in manifest['journey_final_evidence'] if f['journey_id'] == 'journey-volicord')
    name = final['artifact_inventory']['canonical_bundle']['file']
    bundle = operations.campaign_api().harness.load_canonical_bundle(operations.safe_path(root, name))
    require(manifest['artifacts'][name]['sha256'] == bundle.source_sha256,
        'campaign subject canonical bytes changed')
    works = [w for w in manifest['work_evidence'] if w['journey_id'] == 'journey-volicord']
    project = manifest['journeys']['journey-volicord']['project_id']
    require(works and all(w['project_id'] == project for w in works), 'campaign subject Project mismatch')
    return subjects_from_bundle(project, [w['work_item_id'] for w in works], bundle)


def require_subject_context(context, subjects):
    validate_subjects(subjects)
    require(context['project_id'] == subjects['project_id'], 'campaign subject Project mismatch')
    for key, inventory in (('selected_work', 'work_ids'), ('selected_decision', 'decision_ids')):
        require(context[key] is None or context[key] in subjects[inventory],
            'displayed ' + key + ' is outside campaign subjects')


def validate_capture(value, *, candidate_head, viewer_sha256, runtime_binding=None, project=None, locale=None, subjects=None):
    require(isinstance(value,dict) and set(value)==CAPTURE_KEYS
        and value['kind']=='dogfood_viewer_display_capture' and value['schema_version']==1
        and value['evidence_class']=='browser_display_capture' and value['candidate_head']==candidate_head
        and hex_value(value['dom_sha256']), 'invalid browser display capture binding')
    c=value['context']; require(isinstance(c,dict) and set(c)==CONTEXT_KEYS, 'invalid Viewer context')
    require(c['kind']=='volicord_viewer_observation_context' and c['schema_version']==1
        and c['mode']=='live' and hex_value(c['render_id'],32) and hex_value(c['project_id'],32)
        and hex_value(c['runtime_binding']) and hex_value(c['source_state_sha256'])
        and isinstance(c['canonical_read_fingerprint'],str)
        and hex_value(c['canonical_read_fingerprint'])
        and c['locale'] in {'en','ko'} and (locale is None or c['locale']==locale)
        and isinstance(c['language'],str) and 0<len(c['language'].encode())<=128
        and type(c['source_count']) is int and c['source_count']>=0
        and (runtime_binding is None or c['runtime_binding']==runtime_binding)
        and (project is None or c['project_id']==project), 'Viewer locale/Runtime/Project/basis mismatch')
    proc=c['process'];require(isinstance(proc,dict) and set(proc)=={'pid','start_ticks','boot_id','executable_path','executable_sha256'}
        and proc['executable_sha256']==viewer_sha256 and type(proc['pid']) is int and proc['pid']>0
        and type(proc['start_ticks']) is int and proc['start_ticks']>0
        and isinstance(proc['boot_id'],str) and re.fullmatch(r'[0-9a-f-]{36}',proc['boot_id'])
        and isinstance(proc['executable_path'],str) and Path(proc['executable_path']).is_absolute(),
        'Viewer process/executable binding mismatch')
    view=c['view'];require(isinstance(view,dict) and view.get('view') in {'overview','work','decisions','code','tools'}
        and set(view)<= {'view','work','decision','entity','scope','tool','page'}
        and all(isinstance(v,str) and len(v)<=1024 for v in view.values()), 'invalid Viewer view')
    parsed=local_url(value['url']);query=parse_qs(parsed.query,keep_blank_values=True)
    require(parsed.path=='/' and not parsed.fragment and all(len(v)==1 for v in query.values())
        and set(query)<=set(view)|{'locale','language'}
        and all(query.get(k,[v])==[v] for k,v in view.items())
        and query.get('locale',['en'])==[c['locale']]
        and query.get('language',['en'])==[c['language']], 'display URL/context mismatch')
    for name in ['selected_work','selected_decision']:
        require(c[name] is None or hex_value(c[name],32),'invalid displayed subject')
    if 'work' in view: require(view['work']==c['selected_work'],'displayed Work mismatch')
    if 'decision' in view: require(view['decision']==c['selected_decision'],'displayed Decision mismatch')
    if subjects is not None:
        require_subject_context(c, subjects)
    require(isinstance(c['analysis'],list) and len(c['analysis'])<=64,'invalid Analysis basis')
    for a in c['analysis']:
        require(isinstance(a,dict) and set(a)=={'analysis_snapshot','repository_snapshot','freshness'}
            and hex_value(a['analysis_snapshot']) and hex_value(a['repository_snapshot'])
            and a['freshness'] in {'current','stale','unknown','unavailable'},'invalid Analysis observation')
    require(isinstance(c['explanations'],list) and len(c['explanations'])<=4096,'invalid explanation states')
    identities=set()
    for e in c['explanations']:
        require(isinstance(e,dict) and set(e)=={'kind','identity','state','plan_fingerprint','generated_at_unix_micros','realization_sha256'}
            and e['kind'] in {'work','decision'} and hex_value(e['identity'],32)
            and e['state'] in {'current','stale','unavailable','unsupported','corrupt'}
            and (e['kind'],e['identity']) not in identities,'invalid explanation observation')
        identities.add((e['kind'],e['identity']))
        if e['state']=='current':
            require(isinstance(e['plan_fingerprint'],str) and re.fullmatch(r'sha256:[0-9a-f]{64}',e['plan_fingerprint'])
                and type(e['generated_at_unix_micros']) is int and hex_value(e['realization_sha256']), 'missing current explanation basis')
        else:
            require(all(e[k] is None for k in ['plan_fingerprint','generated_at_unix_micros','realization_sha256']),
                'non-current explanation cannot supply a current realization')
    screenshot=value['screenshot'];require(isinstance(screenshot,dict) and set(screenshot)=={'path','sha256','bytes'}
        and isinstance(screenshot['path'],str) and Path(screenshot['path']).name==screenshot['path']
        and screenshot['path'].endswith('.png') and hex_value(screenshot['sha256'])
        and type(screenshot['bytes']) is int and 0<screenshot['bytes']<=32<<20,'invalid screenshot binding')
    browser=value['browser']; require(isinstance(browser,dict) and set(browser)=={'version','geometry','zoom'}
        and isinstance(browser['version'],str) and 0<len(browser['version'])<100
        and isinstance(browser['geometry'],dict) and set(browser['geometry'])=={'scrollX','scrollY','innerWidth','innerHeight','dpr'}
        and all(type(v) in {int,float} and 0<=v<1e7 for v in browser['geometry'].values()),'invalid browser geometry')
    # Zoom observations are supplied only by the existing native-tab-zoom driver.
    zoom=browser['zoom']
    if zoom is not None:
        require(isinstance(zoom,dict) and set(zoom)=={'mechanism','before','after','factor','settings'}
            and zoom['mechanism']=='chrome.tabs.setZoom; automatic per-tab browser zoom, not CSS/device emulation'
            and zoom['factor'] in {1,2} and isinstance(zoom['settings'],dict)
            and set(zoom['settings'])<= {'mode','scope','defaultZoomFactor'}
            and zoom['settings'].get('mode')=='automatic' and zoom['settings'].get('scope')=='per-tab',
            'actual native tab zoom observation required')
        for geometry in [zoom['before'],zoom['after']]:
            require(isinstance(geometry,dict) and set(geometry)=={'innerWidth','dpr','visualScale','cssZoom'}
                and geometry['cssZoom']=='1' and geometry['visualScale']==1
                and all(type(geometry[k]) in {int,float} and 0<geometry[k]<1e7 for k in ['innerWidth','dpr']),
                'invalid native zoom geometry')
        require(zoom['after']['innerWidth']==browser['geometry']['innerWidth']
            and zoom['after']['dpr']==browser['geometry']['dpr'], 'zoom/capture geometry mismatch')
        if zoom['factor']==2:
            require(abs(zoom['after']['dpr']/zoom['before']['dpr']-2)<0.01
                and abs(zoom['after']['innerWidth']*2-zoom['before']['innerWidth'])<=1,
                '200 percent native zoom geometry mismatch')
    return value


def for_manifest(manifest, value, locale, subjects):
    journey=manifest['journeys']['journey-volicord']
    validate_subjects(subjects)
    final = next(f for f in manifest['journey_final_evidence'] if f['journey_id'] == 'journey-volicord')
    require(subjects['project_id'] == journey['project_id'] and subjects['work_ids'] == sorted(
        w['work_item_id'] for w in manifest['work_evidence'] if w['journey_id'] == 'journey-volicord')
        and subjects['canonical_bundle_sha256'] == final['artifact_inventory']['canonical_bundle']['sha256'],
        'campaign subject inventory differs from immutable evidence')
    return validate_capture(value,candidate_head=manifest['candidate_head'],
        viewer_sha256=manifest['candidate_artifacts']['volicord-viewer']['sha256'],
        runtime_binding=resource_observer.path_binding(Path(journey['runtime_home'])),
        project=journey['project_id'],locale=locale,subjects=subjects)


def load_contexts(paths, manifest, subjects, *, locales=("en", "ko")):
    contexts={locale:[] for locale in ('en','ko')};seen=set()
    for path in paths:
        value=json.loads((path/'display-context.json').read_bytes())
        locale=value['context']['locale'];for_manifest(manifest,value,locale,subjects)
        screenshot=path/value['screenshot']['path']
        data=screenshot.read_bytes()
        require(len(data)==value['screenshot']['bytes'] and hashlib.sha256(data).hexdigest()==value['screenshot']['sha256'],
            'displayed screenshot changed')
        identity=hashlib.sha256(json.dumps(value,sort_keys=True).encode()).hexdigest()
        require(identity not in seen,'same displayed observation cannot be reused')
        seen.add(identity);contexts[locale].append(value)
    require(all(contexts[locale] for locale in locales),'both locales require actual displayed contexts before a locale reference')
    contexts = {locale: contexts[locale] if locale in locales else [] for locale in ('en', 'ko')}
    return contexts
