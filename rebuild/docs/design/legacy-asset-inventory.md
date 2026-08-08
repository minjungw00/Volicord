# 기존 Volicord 자산 분류

- 상태: 초기 분류
- 기준선: 재구축 branch를 만든 시점의 기존 저장소 내용
- 목적: 기존 계약을 보존하지 않으면서 독립적인 기술 자산과 실패 교훈을 선별
- 중요: `reuse-candidate`는 코드 재사용 승인이 아니라 추가 검토 대상이라는 뜻임

## 1. 분류 값

| 분류 | 의미 |
|---|---|
| `reuse-candidate` | 새 책임과 테스트로 분리 가능한 primitive가 있을 수 있음 |
| `reference-only` | 설계 원칙, 실패 사례, 설치 경험 또는 테스트 아이디어만 참고 |
| `remove-at-cutover` | replacement gate 통과 후 active 제품에서 제거 |

## 2. 재사용 원칙

기존 crate 전체를 새 workspace dependency로 가져오지 않는다. 재사용이 필요하면 다음 순서를 따른다.

1. 새 제품에서 필요한 책임을 독립적으로 정의한다.
2. 기존 구현과 테스트가 그 책임을 실제로 제공하는지 검토한다.
3. workflow type, Runtime Home schema 또는 legacy adapter 결합을 제거한다.
4. 필요한 최소 코드를 새 workspace로 이동하거나 다시 구현한다.
5. 새 책임에 맞는 테스트와 failure semantics를 작성한다.
6. legacy crate dependency가 없음을 Cargo metadata로 확인한다.

Git history가 이동을 추적할 수 있으므로 호환 wrapper를 만들 필요가 없다.

## 3. 루트와 문서 자산

| 경로 | 분류 | 근거와 처리 |
|---|---|---|
| `AGENTS.md` | reference-only / replaced now | 기존 owner-routing은 재구축을 통제하지 않으므로 branch 지침으로 교체 |
| `Cargo.toml`, `Cargo.lock` | reference-only | 기존 workspace는 기준선 빌드용으로 유지하고 `rebuild`를 명시적으로 제외; cutover에서 교체 |
| `README.md`, `README.ko.md` | remove-at-cutover | 현재 authority workflow 제품 설명이므로 새 제품 문서로 전면 교체 |
| `docs/en/`, `docs/ko/` | reference-only then remove/replace | provenance, failure와 guarantee 표현은 참고하지만 active contract로 승계하지 않음 |
| `docs/doc-index.yaml`, `docs/terminology-map.yaml` | reference-only | 기존 bilingual owner system; 최종 문서 구조가 확정되면 새 정책을 별도로 설계 |
| `.github/workflows/` | reference-only | CI/release 요구를 참고하고 새 workspace 검증으로 교체 |
| `scripts/install.sh`, `scripts/install.ps1` | reference-only | host installation과 경로 처리 경험을 검토; 새 binary·Runtime Home에 맞게 다시 작성 |
| release packaging scripts, Docker files | reference-only | 실제 distribution 요구를 보존하되 현재 binary와 workflow 가정은 제거 |
| `xtask/` | reference-only then remove/replace | legacy docs/schema validation에 결합; 필요한 build task만 새 책임으로 재작성 |

## 4. crate별 초기 분류

| 경로 | 분류 | 추출 후보 또는 제거 이유 |
|---|---|---|
| `crates/volicord-platform-process` | reuse-candidate | child-process containment, termination, pipe readiness; domain dependency가 없어 우선 검토 가치가 높음 |
| `crates/volicord-test-process` | reuse-candidate | bounded process test harness와 cleanup; 새 장기 분석·명령 테스트에 적용 가능 |
| `crates/volicord-platform-fs` | mixed | path normalization, Git layout/observation, atomic publication은 후보; mutation admission과 `volicord-types` 결합은 제거 |
| `crates/volicord-store` | reference-only | SQLite transaction, atomic commit, crash recovery와 invariant test 패턴은 참고; schema와 authority semantics는 재사용하지 않음 |
| `crates/volicord-command-model` | reference-only | execution-free command declaration 아이디어는 참고; 새 CLI가 확정되기 전 코드 재사용하지 않음 |
| `crates/volicord-mcp-protocol` | reference-only | host-independent MCP capability 표현 경험을 참고; 새 public surface 이후 재평가 |
| `crates/volicord-host-contract` | remove-at-cutover | Guard correlation과 effect classification이 기존 authority boundary에 결합 |
| `crates/volicord-types` | remove-at-cutover | Task, checkpoint/application lineage, stale authority와 legacy public schemas 중심 |
| `crates/volicord-core` | remove-at-cutover | shaping authority, phase transition, reauthorization, finalization과 close coordination 중심 |
| `crates/volicord-user-action-service` | remove-at-cutover | 별도 User Channel lifecycle와 decision application이 새 단일 응답 모델과 충돌 |
| `crates/volicord-user-action-presentation` | remove-at-cutover | CLI resolution과 chat non-authority presentation 중심 |
| `crates/volicord-mcp-wire` | remove-at-cutover | legacy method request/result schema와 finalization 계약 중심 |
| `crates/volicord-mcp` | reference-only then remove | lifecycle, supervision, descriptor validation 경험은 참고; adapter는 새 domain surface로 재작성 |
| `crates/volicord-cli` | mixed | Codex setup, process supervision, rendering과 packaging 경험은 참고; UserAction·Guard·legacy Core 결합 제거 |
| `crates/volicord-test-support` | remove-at-cutover | legacy Runtime Home, Store, Core request fixture에 결합 |

## 5. 테스트 자산

| 경로 | 분류 | 처리 |
|---|---|---|
| `tests/conformance` | reference-only then remove | legacy cross-method 계약을 새 acceptance로 승계하지 않음; failure cases만 추출 |
| `tests/agent-evaluation` | reuse-candidate at scenario level | agent output quality, compact projection과 misuse 사례를 새 recall/inquiry 평가로 재작성 |
| `tests/integration` | mixed | install, process, filesystem와 crash 사례를 재사용할 수 있으나 legacy workflow expectation은 제거 |
| `tests/release-integrity` | reference-only | package completeness와 generated artifact 검증 요구를 새 layout에 맞게 재작성 |
| `tests/release-smoke` | reuse-candidate at harness level | clean install과 실행 smoke pattern을 새 product journey로 교체 |

## 6. 보존할 설계 교훈

다음은 기존 record나 workflow가 아니라 재구축에서 유지할 원칙이다.

- 사용자 판단과 에이전트 해석을 구분한다.
- source와 provenance를 잃지 않는다.
- 성공한 domain mutation과 실패한 response projection을 구분한다.
- aggregate operation은 부분 실패와 누락을 정직하게 보고한다.
- command execution은 stdout, stderr, exit state와 termination을 잃지 않는다.
- cooperative 기록을 OS sandbox나 보안 enforcement로 과장하지 않는다.
- generated/runtime state를 maintained source와 문서에 섞지 않는다.
- stale, unavailable, superseded와 current를 구분한다.
- 테스트나 fixture가 유일한 product contract가 되지 않게 한다.

## 7. 제거할 제품 의미

다음 의미는 재구축의 기본 제품에 포함하지 않는다.

- 모든 repository 변경을 Task에 먼저 intake
- shaping과 implementation phase progression
- Change Unit을 통한 작업 경계 승인
- 일반 쓰기에 Write Ticket 필요
- 채팅 답변을 거부하고 CLI User Channel에서 같은 판단을 재입력
- Run과 Evidence의 의무적 분리
- final acceptance가 없는 모든 작업의 close 차단
- `check_close`와 `close_task` ceremony
- adapter가 exact state version과 authority coordinate를 운반하는 workflow
- ordinary-write Guard admission

고위험 effect confirmation이나 규제 요구가 나중에 필요하면 작은 Canonical Context Core 위의 별도 정책으로 새로 설계한다.

## 8. 승인 전 확인이 필요한 재사용 후보

각 후보는 별도 spike에서 다음을 확인한다.

### Process primitives

- 강제 종료와 child tree cleanup
- stdout/stderr bounded capture
- timeout과 exit-status 보존
- Windows/Linux 차이
- legacy type dependency 유무

### Filesystem/Git primitives

- path normalization과 symlink 정책
- repository identity와 worktree/clone 처리
- source fingerprint
- atomic publication
- dirty change 관찰의 정확성
- mutation admission 코드와의 분리 가능성

### SQLite patterns

- transaction boundary
- crash/fault injection
- schema versioning과 repair
- portable bundle과 local database 역할 분리
- legacy DDL 또는 record identity 의존 제거

## 9. 현재 확인 한계

이 문서는 구조와 명시된 책임을 기준으로 한 초기 분류다. 개별 function 단위 재사용 가능성, platform coverage와 라이선스·dependency 영향은 아직 검증하지 않았다. 실제 코드 이동은 spike, 새 contract와 테스트 승인 후 수행한다.
