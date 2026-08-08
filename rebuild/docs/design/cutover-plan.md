# 기존 구현 교체 계획

- 상태: 초기 계획
- 목표: 새 제품이 실제 사용 gate를 통과한 뒤 기존 구현, 문서와 workflow를 제거하고 하나의 `volicord` 제품으로 정리
- 명명: 최종 산출물에 제품 세대 분기나 영구적인 임시 재구축 명칭을 남기지 않음

## 1. 기본 원칙

- 기존 구현은 교체 전까지 inspectable reference baseline으로 보존한다.
- 새 구현은 `rebuild/`의 독립 workspace와 별도 runtime schema에서 진행한다.
- 새 구현을 legacy API wrapper로 만들지 않는다.
- 교체는 파일 이동만이 아니라 product contract, installer, tests, docs와 Runtime Home 경계를 모두 바꾸는 작업이다.
- “구현 완료”가 아니라 acceptance와 dogfood gate 통과를 제거 조건으로 사용한다.
- 교체 전 archive tag 또는 동등한 복원 지점을 유지한다.

## 2. 단계

### Phase 0 — Reconstruction boundary

산출물:

- branch-wide `AGENTS.md`
- root Cargo workspace의 `rebuild` 제외
- 독립 `rebuild/Cargo.toml`
- 제품 헌장, 결정 기록, acceptance, 자산 분류와 cutover 계획
- 분리된 ignored runtime/build path

통과 조건:

- 새 workspace package가 모두 `rebuild/` 아래 있음
- legacy crate dependency가 없음
- 기존 Runtime Home을 읽거나 수정하지 않음
- legacy implementation은 reference-only로 명시됨

### Phase 1 — Product decisions and risk spikes

완료할 결정:

- Inquiry activation/termination
- 첫 Repository Intelligence 지원 범위
- LLM/privacy boundary
- 사용자 interface와 문서 출력
- portable conflict와 memory correction
- final cutover/data policy

필수 spike:

1. Volicord Rust repository 구조 분석과 coverage
2. 최소 Canonical Context의 export/import와 restart
3. Question dependency frontier와 session resume
4. source-grounded architecture document generation

통과 조건:

- 각 spike가 기술 선택, known limit와 acceptance 수정안을 남김
- disposable spike code가 production contract로 조용히 승격되지 않음

### Phase 2 — Canonical Context Kernel

순서:

1. Project와 Source
2. Question
3. Decision
4. Context Item
5. Checkpoint
6. revision, supersession, contradiction와 forget
7. portable bundle
8. deterministic Recall

통과 조건:

- LLM과 repository analyzer 없이 기록, restart, export/import, 수정과 삭제가 작동
- derived state가 없어도 canonical records를 읽을 수 있음
- user, agent, source와 generated annotation provenance가 구분됨

### Phase 3 — Repository Intelligence

순서:

1. repository snapshot과 discovery
2. supported language structural analysis
3. code entity, relation와 coverage
4. fingerprint와 incremental update
5. source-grounded search
6. semantic annotation provider
7. Decision/Context/Checkpoint linkage

통과 조건:

- Volicord 자체 저장소에서 repository-wide 설명과 source navigation 작동
- unsupported/excluded/failed coverage 표시
- semantic provider가 없어도 structural mode 사용 가능
- stale analysis가 current fact로 제시되지 않음

### Phase 4 — Inquiry and Decision experience

기능:

- material Question candidate
- fact research before asking
- dependency frontier
- option, recommendation, trade-off와 uncertainty
- current host user response linkage
- delegation, research, prototype와 deferment
- pause/resume와 recommendation batch choice

통과 조건:

- 긴 설계 세션을 새 대화에서 이어감
- 같은 판단을 다른 interface에서 다시 입력하지 않음
- 이미 답한 Question을 반복하지 않음
- 사용자 Decision과 agent recommendation이 분리됨

### Phase 5 — Recall, viewer and documents

기능:

- user/agent Resume Brief
- Decision–Context–Code Map
- local viewer의 inspect/correct/supersede/forget
- architecture, Decision, design, implementation, impact와 handoff 문서
- source snapshot, coverage와 known gaps metadata

통과 조건:

- raw JSON이나 database 없이 현재 목표, 코드 구조, 결정과 다음 단계 이해 가능
- generated document가 자동 canonical truth가 되지 않음
- stale/unavailable source가 명확함

### Phase 6 — Host integration and risk policy

기능:

- Codex와 지원 MCP host 연결
- small agent-facing surface
- CLI init, health, analyze, export/import와 repair
- high-risk effect confirmation
- bounded long-running process reporting

통과 조건:

- ordinary edit가 Volicord procedure에 의해 차단되지 않음
- 외부 배포, 비용, secret, 개인정보와 파괴적 effect는 명시적 confirmation
- MCP 장애가 repository를 사용할 수 없게 만들지 않음

### Phase 7 — Dogfood and replacement gate

반복할 전체 여정:

```text
clean install
→ project connect
→ repository analysis
→ understanding
→ staged inquiry and decision
→ actual work
→ checkpoint
→ new-session recall
→ another-clone import
→ document output
→ memory correction
→ failure recovery
```

최소 대상:

- Volicord 자체 Rust workspace
- 소규모 단일 언어 application
- 문서와 여러 언어가 섞인 중간 규모 repository

정확한 대상과 수는 `open-decisions.md` Q8에서 확정한다.

### Phase 8 — Cutover

하나의 집중된 교체 batch로 다음을 수행한다.

1. 기존 `crates/`, `tests/`, `xtask/`와 legacy workspace 제거
2. `rebuild/` 내용을 최종 root layout으로 이동
3. root Cargo manifest와 lockfile 교체
4. final binary와 package 이름을 `volicord`로 정리
5. installer, MCP setup, release packaging과 workflows 교체
6. README와 maintained documentation 전면 교체
7. 임시 `rebuild/` 경계와 work instructions 제거 또는 최종 정책으로 재작성
8. legacy terms와 workflow method가 active product에 남지 않았는지 검사
9. clean install과 full acceptance를 final root에서 다시 실행

## 3. 제거 gate

다음 항목이 모두 통과하기 전 기존 구현을 삭제하지 않는다.

- [ ] clean install과 uninstall/reinstall
- [ ] Codex 또는 첫 지원 host 연결과 health
- [ ] stable Project ID와 분리된 runtime home
- [ ] repository-wide structural analysis와 coverage
- [ ] source-grounded architecture/flow explanation
- [ ] staged Question frontier와 pause/resume
- [ ] 현재 host의 한 번의 사용자 답변으로 Decision 기록
- [ ] ordinary work와 source-linked Checkpoint
- [ ] verification, user review와 acceptance의 독립 상태
- [ ] 완전히 새로운 session의 Recall
- [ ] 다른 clone 또는 computer의 bundle import
- [ ] divergent bundle 감지와 결정된 conflict 처리
- [ ] memory correction, supersession과 deletion
- [ ] source-grounded document output
- [ ] semantic provider unavailable fallback
- [ ] partial parser failure와 coverage degradation
- [ ] derived index corruption과 rebuild
- [ ] canonical transaction crash recovery
- [ ] long-running child process termination과 exit result 보존
- [ ] Volicord dogfood와 추가 실제 repository acceptance

## 4. 기존 Runtime Home과 데이터

현재 계획은 기존 Runtime Home schema를 새 제품에서 직접 읽거나 확장하지 않는다. 자동 migration 여부는 `open-decisions.md` Q8이 확정한다.

자동 migration을 채택하지 않을 경우 최소 동작은 다음과 같다.

- legacy Runtime Home 감지
- silent overwrite 금지
- 기존 경로와 backup 안내
- 사용자가 요청한 경우 제한된 historical export
- 새 Runtime Home을 명시적으로 초기화

historical export는 legacy workflow를 새 제품에서 재현하지 않는다. 가능한 경우 목표, 사용자 resolution, continuity와 결과를 사람이 읽을 수 있는 reference 자료로 내보내는 수준으로 제한한다.

## 5. Cutover 시 제거 대상

활성 제품에서 다음을 제거한다.

- Task phase와 shaping/implementation progression
- Change Unit
- ordinary-write Write Ticket과 Guard admission
- CLI 전용 UserAction resolution과 별도 application transition
- Run/Evidence/final-acceptance/close ceremony
- legacy Core, Store, types와 MCP method schema
- legacy conformance와 SignalBox success criteria
- legacy Runtime Home installer assumptions
- old README와 bilingual contract tree

고위험 confirmation, provenance, source observation와 process reliability는 새 계약으로 다시 구현된 경우에만 남긴다.

## 6. Final naming and versioning

- 최종 root, package, binary와 command는 하나의 `volicord` 제품을 나타낸다.
- public API에 영구적인 제품 세대 namespace나 기존 구현 호환 namespace를 두지 않는다.
- database, portable bundle, analysis snapshot과 generated document에는 독립적인 schema or format version을 둔다.
- format migration은 제품 세대 명칭이 아니라 데이터 해석 계약으로 관리한다.

## 7. Rollback과 역사 보존

- 교체 직전 commit에 archive tag를 만든다.
- final cutover가 실패하면 branch를 되돌리고 legacy와 replacement를 runtime에서 동시에 활성화하지 않는다.
- legacy source와 문서는 Git history/tag로 보존하며 active tree에 archive copy를 중복 저장하지 않는다.
- cutover 후 발견한 문제를 이유로 legacy workflow compatibility layer를 자동 복원하지 않는다. 필요한 사용자 가치와 최소 수정안을 새 Core 기준으로 설계한다.

## 8. Cutover 완료 판정

다음이 모두 참일 때 교체가 완료된다.

1. root에서 새 workspace와 final `volicord` binary가 build/test/package된다.
2. active docs가 새 제품 목적과 실제 동작만 설명한다.
3. legacy workflow code와 public surface가 제거되었다.
4. clean environment와 실제 repositories에서 acceptance가 다시 통과했다.
5. old Runtime Home을 silent overwrite하지 않는다.
6. 임시 `rebuild/` 이름과 product-generation labels가 active artifact에 남지 않는다.
