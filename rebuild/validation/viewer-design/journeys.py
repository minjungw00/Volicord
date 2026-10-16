"""Bounded source-first journey inputs, separate from editorial UI expectations.

Prepare uses an existing authored Goal-only fixture and disposable Runtime copy.
No provider, explanation generation, historical import or human verdict is used.
"""
import argparse
import hashlib
from html import escape
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    return json.loads(Path(path).read_text())


def missing_admit(root):
    spec = load(root / 'case.json')
    for name, digest in spec['files'].items():
        if sha((root / name).read_bytes()) != digest:
            raise ValueError('missing-case input changed: ' + name)
    status = load(root / 'status.json')
    work, = status['work_history']
    canonical = load(root / 'canonical.json')
    if (status['project_id'] != spec['project'] or work['work_item_id'] != spec['work']
            or canonical['id'] != spec['work'] or canonical['statement'] != spec['goal']
            or work['evidence']['goal']['original_text'] != spec['goal']):
        raise ValueError('missing-case canonical association')
    if (work['answers']['explanation_state'] != 'missing' or work['checkpoint_ids']
            or work['answers']['source_excerpts'] or work['state'] != 'open'
            or work['answers']['provenance'] is not None):
        raise ValueError('missing-case must preserve absent explanation and Checkpoint')
    if 'return 5' not in (root / 'source.py').read_text():
        raise ValueError('independent fixture Source changed')
    return spec, work


def prepare(args):
    out = args.output.resolve()
    if out.exists() or not out.is_relative_to(REPO / 'rebuild/.local'):
        raise ValueError('choose a new ignored output')
    fixture = load(args.fixture / 'fixture.json')
    out.mkdir(parents=True)
    shutil.copytree(args.fixture / 'runtime', out / 'runtime')
    # Inspect canonical meaning independently of Viewer output. Never edit originals.
    with sqlite3.connect(f'file:{args.fixture / "runtime/canonical.sqlite3"}?mode=ro', uri=True) as db:
        db.row_factory = sqlite3.Row
        goal = dict(db.execute('select id, project_id, revision, role, statement from context_items where id=?', (bytes.fromhex(fixture['work']),)).fetchone())
        goal = {k: v.hex() if isinstance(v, bytes) else v for k, v in goal.items()}
        assert db.execute('select count(*) from checkpoints').fetchone()[0] == 0
    assert goal['statement'] == fixture['goal'] and goal['role'] == 'goal'
    (out / 'canonical.json').write_text(json.dumps(goal, ensure_ascii=False, indent=2))
    shutil.copyfile(args.fixture / 'repo/main.py', out / 'source.py')
    base = [str(args.binary.resolve()), '--runtime', str(out / 'runtime'), '--project', fixture['project'], '--language', 'ko']
    commands = [[base[0], 'status', *base[1:], '--json'],
                [base[0], 'viewer', 'export', *base[1:], '--output', str(out / 'baseline.html')]]
    receipts = []
    for i, command in enumerate(commands):
        p = subprocess.run(command, capture_output=True)
        (out / f'command-{i}.stdout').write_bytes(p.stdout)
        (out / f'command-{i}.stderr').write_bytes(p.stderr)
        receipts.append(dict(argv=command, exit=p.returncode))
        if p.returncode:
            raise ValueError('Product read failed; inspect retained command streams')
    shutil.copyfile(out / 'command-0.stdout', out / 'status.json')
    spec = dict(project=fixture['project'], work=fixture['work'], goal=fixture['goal'],
                classification='existing authored Goal-only fixture, not a historical campaign or generated explanation',
                binary_sha256=sha(args.binary.read_bytes()), commands=receipts,
                files={name: sha((out / name).read_bytes()) for name in ['canonical.json','source.py','status.json','baseline.html']})
    (out / 'case.json').write_text(json.dumps(spec, ensure_ascii=False, indent=2))
    missing_admit(out)
    print(json.dumps(dict(status='prepared', output=str(out), commands=receipts)))


# These checks were fixed from original code/command/report input, without reading
# candidate/prototype prose as authority. They establish bounded facts, not relevance
# or independent semantic approval of every sentence in an authored explanation.
CHECKS = {
    'b': [('next', ['"absent" =>', '"unavailable" =>', '"failed" | "freshness_unknown"', '"stale" =>']),
          ('details', ['limited <= 3']), ('bound', ['truncate(32)', 'omitted_coverage_count']),
          ('report', ['aggregate exited 101', 'subsequent final recheck', 'nondeterminism remains unfixed', 'not_requested'])],
    'click': [('open', ['if self._f is not None:', 'return self._f', 'self.name, self.mode', 'self._f = rv']),
              ('reproduction', ['read lazy=True absolute=False: B', 'read lazy=False absolute=False: A', 'read lazy=True absolute=True: A', 'All 16 scratch cases passed']),
              ('report', ['No repository changed paths', '14 passed, 100 deselected', 'exit 127', 'Windows', 'policy remains unconfirmed'])]
}


def source_check(works):
    evidence = []
    for w in works:
        for ref, needles in CHECKS[w['authored']['key']]:
            source = w['sources'][ref]
            for needle in needles:
                if needle not in source['text']:
                    raise ValueError('source-first outcome missing: ' + needle)
            evidence.append(dict(work=w['selected']['work_item_id'], ref=ref,
                                 citation=source['citation'], source_sha256=sha(source['text'].encode()), checks=needles))
    return evidence


def attach(out, missing_root, works, frame):
    evidence = source_check(works)
    spec, missing = missing_admit(missing_root)
    for name in ['case.json', 'canonical.json', 'source.py', 'status.json']:
        shutil.copyfile(missing_root / name, out / 'inputs' / ('missing-' + name))
    shutil.copyfile(missing_root / 'baseline.html', out / 'baseline-missing.html')
    body = '<article class="doc"><header class="doc-head"><h1>Goal-only Work</h1><p class="unavailable" role="status">설명이 기록되지 않았습니다. 원래 목표와 기록된 사실만 읽습니다.</p></header>'
    body += '<blockquote class="goal">' + escape(spec['goal']) + '</blockquote>'
    for f in missing['answers']['prose'] + missing['answers']['facts']:
        body += f'<p class="unit-text" data-fact="{f["question"]}">{escape(f["text"])}</p>'
    body += '<p>Work에 연결된 코드 근거가 없어 목적에 맞는 코드를 선택할 수 없습니다. 저장소 파일의 존재만으로 이 Work의 코드라고 추측하지 않습니다.</p>'
    body += '<details class="record-box"><summary>전체 근거 · 누락된 설명</summary><pre class="full-audit">' + escape(json.dumps(missing, ensure_ascii=False, indent=2)) + '</pre><a href="inputs/missing-status.json">Product readback</a><a href="baseline-missing.html#code">기존 Repository Map (Work 근거와 별개)</a></details></article>'
    (out / 'missing.html').write_text(frame('Goal-only Work', body))
    cases = []
    for w in works:
        key = w['authored']['key']; selected = w['selected']
        # Discovery in this comparison uses the same exact control identity/state.
        # The historical A/B/C catalog remains separately reviewable, not an oracle.
        title = next(u['text'] for u in w['authored']['units'] if u['role'] == 'heading')
        intro = f'<h1>Work · {key.upper()}</h1><p class="source-note">같은 단일 Work 후보 집합의 탐색 비교 · 과거 A/B/C 목록과 별개</p><article class="work-card" data-work-id="{selected["work_item_id"]}"><h2>{escape(title)}</h2><p>상태: {escape(selected["state"])}</p><a class="button" href="{key}.html">작업 읽기 →</a></article>'
        (out / f'find-{key}.html').write_text(frame('Work 찾기', intro))
        cases.append(dict(key=key, work=selected['work_item_id'], baseline=f'baseline-{key}.html#overview', target=f'find-{key}.html#main', mechanism_ref='next' if key == 'b' else 'open'))
    cases.append(dict(key='missing', work=spec['work'], baseline='baseline-missing.html#overview', target='missing.html', code_task='unavailable; no Work-bound Source', explanation='missing'))
    tasks = [
        ('find', '목표에 해당하는 Work를 찾으세요. 상태와 어떤 기록을 읽는지 설명하세요.'),
        ('problem', '이 작업에서 해결하거나 조사하려던 문제는 무엇인가요?'),
        ('mechanism', '주요 동작을 설명하고 그 설명을 확인할 코드를 선택하세요.'),
        ('limits', '검증 결과를 믿고 다음 작업을 시작하기 전에 알아야 할 한계는 무엇인가요?'),
        ('comparison', 'Click의 세 조건에서 관측 결과가 어떻게 달랐나요? 무엇이 아직 확인되지 않았나요?'),
        ('return', '코드에서 원래 읽던 문단으로 돌아가세요. 위치와 문맥이 유지되나요?'),
        ('missing', 'Goal-only 사례에서 무엇을 알 수 있고 무엇은 아직 설명할 수 없나요?')]
    instructions = '<h1>Viewer 여정 비교</h1><p>사용자 검토 대기 · 서로 다른 창에서 같은 사례의 두 경로를 읽으세요. 순서를 번갈아 비교하고, 데스크톱과 좁은 화면에서 반복하세요. 시간과 오해는 실제 관찰한 경우에만 기록합니다.</p>'
    for c in cases:
        instructions += f'<section class="reading-section"><h2>{c["key"].upper()}</h2><p><a class="button" href="{c["baseline"]}">현재 Viewer</a> <a class="button" href="{c["target"]}">새 구성</a></p></section>'
    instructions += '<ol>' + ''.join('<li>' + escape(t) + '</li>' for _, t in tasks) + '</ol><p>검토 답변에는 manifest SHA-256, 화면 너비, 사례, 수행한 순서, 실제 혼동·실패를 포함하세요. 예상 답안은 이 페이지에 제공하지 않습니다. 침묵은 승인이 아닙니다.</p><a href="manifest.json">정확한 후보 identity</a> · <a href="index.html">전체 검토</a>'
    (out / 'journeys.html').write_text(frame('여정 비교', instructions, active='index'))
    receipt = dict(cases=cases, source_first=evidence, tasks=[dict(id=k, prompt=t) for k,t in tasks],
                   missing_case=spec, human_timing=None, human_errors=None, human_acceptance='pending',
                   evaluator_separation='source-first assertions consume original ranges; presentation prompts contain no editorial answer; no independent model reviewer run',
                   content_delta='B/Click exact units, facts, Goal, excerpts and audit retained in detail. Matched target discovery shows title/state only; baseline discovery also has synopsis/limits/Goal disclosures and Project Purpose. Those Work facts remain one entry away in target; no discovery-only comprehension superiority is claimed. Historical A/B/C catalog excluded from matched discovery. Baseline repository map/diagnostics/documents remain via baseline links; target does not recreate them.')
    (out / 'inputs/journeys.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2))
    return receipt


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--fixture', type=Path, required=True); p.add_argument('--binary', type=Path, required=True); p.add_argument('--output', type=Path, required=True)
    prepare(p.parse_args())
