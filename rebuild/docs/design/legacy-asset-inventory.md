# 기존 Volicord 자산 분류

- 상태: 제품 결정 반영 분류
- 기준선: 재구축 branch를 만든 시점의 기존 저장소 내용
- 목적: 기존 계약을 보존하지 않으면서 독립적인 기술 primitive와 실패 교훈을 선별
- 중요: `reuse-candidate`는 코드 재사용 승인이 아니라 기술 검증 대상이라는 뜻임
- 데이터 정책: legacy Runtime Home, schema, record와 migration은 재사용·호환 대상이 아님

## 1. 분류 값

| 분류 | 의미 |
|---|---|
| `reuse-candidate` | 새 책임과 테스트로 분리 가능한 primitive가 있을 수 있음 |
| `reference-only` | 설계 원칙, 실패 사례, 설치 경험 또는 테스트 아이디어만 참고 |
| `remove-at-cutover` | replacement gate 통과 후 active 제품에서 제거 |
| `reject` | 새 제품에 가져오지 않으며 별도 검증 없이 제거 |

## 2. 재사용 원칙

기존 crate 전체를 새 workspace dependency로 가져오지 않는다. 재사용이 필요하면 다음 순서를 따른다.

1. 새 제품에서 필요한 책임을 독립적으로 정의한다.
2. 기존 구현과 테스트가 그 책임을 실제로 제공하는지 검토한다.
3. workflow type, Runtime Home schema 또는 legacy adapter 결합을 제거한다.
4. 필요한 최소 코드를 새 workspace로 이동하거나 다시 구현한다.
5. 새 책임에 맞는 test, failure와 degraded semantics를 작성한다.
6. Linux에서 동작과 dependency를 검증한다.
7. legacy crate dependency가 없음을 Cargo metadata로 확인한다.

Git history가 이동을 추적할 수 있으므로 compatibility wrapper를 만들 필요가 없다. 기술 검증 결과는 `validation-plan.md` V10 형식으로 기록한다.

## 3. 금지된 재사용

다음은 implementation convenience로도 새 제품에 가져오지 않는다.

- legacy Runtime Home path, database schema와 record identity
- migration, importer, historical export와 detection logic
- existing Task, UserAction, Change Unit, Write Ticket, Run, Evidence와 close semantics
- legacy API와 CLI command alias
- dual-read, dual-write와 two-runtime support
- existing MCP request/result를 새 이름으로 감싼 wrapper
- ordinary repository write admission
- user chat answer를 거부하고 CLI에서 같은 Decision을 재입력하는 contract
- legacy conformance test를 replacement acceptance로 사용하는 방식

## 4. 루트와 문서 자산

| 경로 | 분류 | 근거와 처리 |
|---|---|---|
| `AGENTS.md` | reference-only / replaced now | 기존 owner-routing은 재구축을 통제하지 않으므로 branch 지침으로 교체 |
| `Cargo.toml`, `Cargo.lock` | reference-only | 기존 workspace는 기준선 빌드용으로 유지하고 `rebuild`를 제외; cutover에서 교체 |
| `README.md`, `README.ko.md` | remove-at-cutover | 현재 authority workflow 제품 설명이므로 새 제품 문서로 전면 교체 |
| `docs/en/`, `docs/ko/` | reference-only then remove/replace | provenance, failure와 guarantee 표현은 참고하지만 active contract로 승계하지 않음 |
| `docs/doc-index.yaml`, `docs/terminology-map.yaml` | reference-only | 기존 bilingual owner system; final documentation policy를 새로 설계 |
| `.github/workflows/` | reference-only | CI/release 요구를 참고하고 Linux, polyglot fixture와 새 workspace 검증으로 교체 |
| `scripts/install.sh` | reference-only | Linux install, PATH와 host setup 경험만 검토; 새 binary와 Runtime Home에 맞게 다시 작성 |
| `scripts/install.ps1` | reference-only then remove | Windows는 첫 공식 OS가 아니며 현재 installer contract를 승계하지 않음 |
| release packaging scripts, Docker files | reference-only | distribution 요구를 참고하되 현재 binary, workflow와 Runtime Home 가정 제거 |
| `xtask/` | reference-only then remove/replace | legacy docs/schema validation에 결합; 필요한 build task만 새 책임으로 재작성 |

## 5. crate별 분류

| 경로 | 분류 | 추출 후보 또는 제거 이유 |
|---|---|---|
| `crates/volicord-platform-process` | reuse-candidate | child-process containment, termination, pipe readiness와 bounded result를 V10에서 검증 |
| `crates/volicord-test-process` | reuse-candidate | process test harness와 cleanup을 long-running analyzer·command test에 검토 |
| `crates/volicord-platform-fs` | mixed | path normalization, Git/worktree observation, fingerprint, atomic publication 후보; mutation admission과 legacy type 결합 제거 |
| `crates/volicord-store` | reference-only | SQLite transaction, atomic commit, crash recovery와 fault-injection pattern만 참고; DDL과 authority semantics 재사용 금지 |
| `crates/volicord-command-model` | reference-only | execution-free command declaration 아이디어만 참고; 새 CLI contract 후 재평가 |
| `crates/volicord-mcp-protocol` | reference-only | host-independent capability 표현 경험만 참고; legacy method schema 재사용 금지 |
| `crates/volicord-host-contract` | remove-at-cutover | Guard correlation과 effect classification이 legacy authority boundary에 결합 |
| `crates/volicord-types` | remove-at-cutover | Task, application lineage, stale authority와 legacy public schema 중심 |
| `crates/volicord-core` | remove-at-cutover | shaping authority, phase transition, reauthorization, finalization과 close coordination 중심 |
| `crates/volicord-user-action-service` | remove-at-cutover | 별도 User Channel lifecycle와 Decision application이 current-host single-answer model과 충돌 |
| `crates/volicord-user-action-presentation` | remove-at-cutover | CLI resolution과 chat non-authority presentation 중심 |
| `crates/volicord-mcp-wire` | remove-at-cutover | legacy method request/result schema와 finalization contract 중심 |
| `crates/volicord-mcp` | reference-only then remove | lifecycle, supervision, descriptor validation 경험은 참고; adapter는 새 high-level domain surface로 재작성 |
| `crates/volicord-cli` | mixed | Codex setup, process supervision, rendering과 packaging 경험은 참고; UserAction, Guard, migration와 legacy Core 결합 제거 |
| `crates/volicord-test-support` | remove-at-cutover | legacy Runtime Home, Store, Core request fixture에 결합 |

## 6. Repository Intelligence 관련 분류

기존 Volicord는 새 제품의 다중 언어 Repository Intelligence 계약을 구현한 기준선이 아니다.

- 기존 source search, path observation 또는 documentation helper가 있더라도 `inventory`, `structural`, `semantic`, `ecosystem` capability를 충족한다고 간주하지 않는다.
- 기존 Rust implementation을 분석하는 test helper는 polyglot analyzer 기반으로 승격하지 않는다.
- Java, Python, JavaScript, TypeScript, C, C++와 Rust support는 `validation-plan.md` V01과 V02에서 새로 검증한다.
- parser, language server, index format와 agent orchestration 선택은 legacy implementation에 구속되지 않는다.
- code graph, semantic annotation와 generated document는 Derived State로 새로 설계한다.

## 7. 테스트 자산

| 경로 | 분류 | 처리 |
|---|---|---|
| `tests/conformance` | reference-only then remove | legacy cross-method contract를 새 acceptance로 승계하지 않음; failure cases만 추출 |
| `tests/agent-evaluation` | reuse-candidate at scenario level | output quality, bounded projection와 misuse 사례를 Recall, Inquiry와 source-grounded explanation 평가로 재작성 |
| `tests/integration` | mixed | install, process, filesystem와 crash 사례를 참고하되 legacy workflow expectation과 data fixture 제거 |
| `tests/release-integrity` | reference-only | package completeness와 generated artifact 검증 요구를 새 layout에 맞게 재작성 |
| `tests/release-smoke` | reuse-candidate at harness level | Linux clean install과 Codex journey로 교체 |
| SignalBox workflow scenario | reference-only | user judgment forgery, dirty-change misattribution와 false verification 방지 교훈만 유지; Task/Write Ticket/close success flow는 제거 |

## 8. 보존할 설계 교훈

다음은 기존 record나 workflow가 아니라 재구축에서 유지할 원칙이다.

- 사용자 판단과 에이전트 해석을 구분한다.
- source와 provenance를 잃지 않는다.
- 성공한 domain mutation과 실패한 response projection을 구분한다.
- aggregate operation은 부분 실패와 누락을 정직하게 보고한다.
- command execution은 stdout, stderr, exit state와 termination을 잃지 않는다.
- cooperative 기록을 OS sandbox나 security enforcement로 과장하지 않는다.
- generated/runtime state를 maintained source와 문서에 섞지 않는다.
- stale, unavailable, superseded와 current를 구분한다.
- 테스트나 fixture가 유일한 product contract가 되지 않게 한다.
- 기존 dirty change를 current work result로 잘못 귀속하지 않는다.
- 실행하지 않은 validation을 성공으로 주장하지 않는다.

## 9. 제거할 제품 의미

다음 의미는 재구축의 기본 제품에 포함하지 않는다.

- 모든 repository 변경을 Task에 먼저 intake
- shaping과 implementation phase progression
- Change Unit을 통한 작업 경계 승인
- 일반 쓰기에 Write Ticket 필요
- chat 답변을 거부하고 CLI User Channel에서 같은 판단을 재입력
- Run과 Evidence의 의무적 분리
- final acceptance가 없는 모든 작업의 close 차단
- `check_close`와 `close_task` ceremony
- adapter가 exact state version과 authority coordinate를 운반하는 workflow
- ordinary-write Guard admission
- legacy Runtime Home migration, detection와 export
- 기존 API와 command compatibility

고위험 effect confirmation이나 규제 요구가 나중에 필요하면 작은 Canonical Context Kernel 위의 별도 정책으로 새로 설계한다.

## 10. 승인 전 확인이 필요한 재사용 후보

각 후보는 `validation-plan.md` V10에서 다음을 확인한다.

### Process primitives

- 강제 종료와 child tree cleanup
- stdout/stderr bounded capture
- timeout과 exit-status 보존
- Linux behavior
- legacy type dependency 유무

### Filesystem/Git primitives

- path normalization과 symlink policy
- repository identity와 worktree/clone handling
- source fingerprint
- atomic publication
- dirty change observation accuracy
- mutation admission code와의 분리 가능성

### Storage patterns

- transaction boundary
- crash/fault injection
- schema versioning과 repair
- portable bundle과 local database 역할 분리
- legacy DDL 또는 record identity 의존 제거

### Host/process integration

- Codex MCP lifecycle
- child process supervision
- bounded health/result rendering
- host-independent adapter portion
- legacy method catalog와 UserAction dependency 제거 가능성

## 11. 재사용 판정 형식

각 검토 대상은 다음 중 하나로 종료한다.

| 판정 | 의미 |
|---|---|
| `adopt_as_new_primitive` | 새 책임과 테스트로 코드 이동 가능 |
| `reimplement_from_behavior` | 동작과 test idea만 참고하고 새로 구현 |
| `reference_only` | 설계·failure 교훈만 사용 |
| `reject` | 새 제품에 사용하지 않음 |

`adopt_as_new_primitive`에는 다음 evidence가 필요하다.

- legacy workflow type dependency 없음
- 새 workspace에서 독립 build
- 새 responsibility test
- Linux validation
- license와 dependency 검토
- new error/degraded semantics

## 12. 현재 확인 한계

V10은 `rebuild/validation/local-platform-primitives/report.md`에서 Linux process,
filesystem/Git와 storage pattern을 재현하고 다음 최종 판정을 내렸다.

| V10 대상 | 최종 판정 | Production 처리 |
|---|---|---|
| legacy platform process | `reimplement_from_behavior` | Linux process group과 pipe-drain behavior만 새 complete-artifact/result 책임으로 재구현 |
| legacy test process | `reference_only` | timeout, stream, exit와 descendant test idea만 참고; bounded capture API는 승격하지 않음 |
| path containment | `reimplement_from_behavior` | canonical root, lexical normalization과 symlink escape를 새 local path observation으로 재구현 |
| Git layout | `reimplement_from_behavior` | clone/worktree coordinate를 local-only observation으로 재구현하고 Project identity로 사용하지 않음 |
| repository observer | `reference_only` | dirty/failure fixture만 참고하고 Repository Intelligence authority를 복제하지 않음 |
| content fingerprint | `reimplement_from_behavior` | typed length-delimited fingerprint만 새 Source observation 책임으로 재구현 |
| atomic no-replace publication | `adopt_as_new_primitive` | same-parent ordinary-file validation, Linux no-replace rename, namespace effect와 parent sync를 새 좁은 module로 이동 |
| Runtime Home mutation lease | `reject` | legacy Runtime Home과 ordinary-write admission 결합 때문에 제거 대상 유지 |
| Store transaction pattern | `reference_only` | crash/transaction test idea만 참고; canonical transaction은 `volicord-context`가 계속 소유 |
| Store schema와 repair | `reject` | legacy DDL, numeric dispatch, Runtime Home과 authority meaning을 가져오지 않음 |

승격된 production boundary는 `volicord-local-platform`이며 legacy crate dependency,
Runtime Home, UserAction, Task, Write Ticket, Evidence 또는 Guard admission 의미를 갖지
않는다. Storage primitive나 두 번째 canonical engine은 추가하지 않았다.

남은 한계는 공식 지원 범위 밖 OS, cgroup/subreaper 또는 sandbox guarantee, network
filesystem과 hostile path-replacement race다. 이 한계는 accepted Linux/local-first
contract를 바꾸는 Decision revisit trigger가 아니다.

기존 구현이 복잡하다는 사실만으로 primitive가 나쁘다고 판단하지 않으며, 기존 구현이 작동한다는 사실만으로 새 제품 의미에 적합하다고 판단하지 않는다.
