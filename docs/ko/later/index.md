# 이후 후보 색인

이 색인은 비활성 이후 후보를 카테고리 문서로 안내합니다. 요약 표일 뿐이며 계약 담당 문서가 아닙니다.

현재 범위는 [현재 MVP 범위](../reference/active-mvp-scope.md)를 봅니다.

이 페이지는 현재 MVP 동작, 구현 작업, 런타임 권한, 현재 API 계약, 현재 저장 효과, 현재 보안 보장, 런타임/서버 코딩 시작 허가를 정의하지 않습니다.

이 색인이나 카테고리 파일의 언급은 승격이 아닙니다. 후보는 현재 범위와 관련 담당 문서가 같은 문서 전용 작업 묶음에서 갱신된 뒤에만 현재 MVP로 들어갈 수 있습니다.

프로필 조건부 값과 이후 후보는 다릅니다. 어떤 값이 프로필 조건부 값이 되려면 현재 범위와 담당 문서가 그 프로필과 지원 값 집합을 함께 이름 붙여야 합니다. 이 페이지의 값은 그런 승격 전까지 이후 후보입니다.

## 후보 요약

| 후보 | 카테고리 | 요약 | 상세 링크 |
|---|---|---|---|
| 보증 강화 | 보안과 보증 | 현재 MVP보다 강한 증거, 검증, 닫기 준비 상태 보증 주장을 다루는 이후 후보입니다. | [상세](security-and-assurance.md#assurance-hardening) |
| 전체 `Evidence Manifest` | 아티팩트와 증거 | 매니페스트 수준 증거 기록과 렌더링 요약을 다루는 이후 후보입니다. | [상세](artifacts-and-evidence.md#full-evidence-manifest) |
| 탐색 요약, 질문 대기열, 가정 기록 | 작업 흐름과 협업 | 열린 질문, 가정, 작업 맥락을 담는 구체화 기록 이후 후보입니다. | [상세](workflow-and-collaboration.md#discovery-brief-question-queue-and-assumption-register) |
| 수동 QA 작업 흐름과 `qa_gate` | 정책과 적합성 | 수동 QA 관문 정책과 닫기 준비 상태 관계를 다루는 이후 후보입니다. | [상세](policy-and-conformance.md#manual-qa-workflow-and-qa-gate) |
| 수동 QA 면제 `qa_waiver` | 정책과 적합성 | 사용자 소유 판단을 대체하지 않는 수동 QA 정책 면제 경로 이후 후보입니다. | [상세](policy-and-conformance.md#manual-qa-waiver-qa-waiver) |
| 검증 관문 `verification_gate` | 정책과 적합성 | 검증 관문 정책과 닫기 준비 상태 관계를 다루는 이후 후보입니다. | [상세](policy-and-conformance.md#verification-gate-verification-gate) |
| 검증 위험 수락 `verification_risk_acceptance` | 작업 흐름과 협업 | 검증 위험을 사용자가 수락하는 판단 경로 이후 후보입니다. | [상세](workflow-and-collaboration.md#verification-risk-acceptance-verification-risk-acceptance) |
| `Eval`과 분리형 검증 작업 흐름 | 작업 흐름과 협업 | 평가와 분리형 검증 작업 흐름 이후 후보입니다. | [상세](workflow-and-collaboration.md#eval-and-detached-verification-workflows) |
| 전체 `Decision Packet`과 `presentation=full` | 작업 흐름과 협업 | 전체 형식 판단 표시 이후 후보입니다. | [상세](workflow-and-collaboration.md#full-decision-packet-and-presentation-full) |
| 상세 위험 검토와 잔여 위험 생명주기 | 작업 흐름과 협업 | 더 풍부한 위험 검토 기록, 잔여 위험 생명주기, 만료 동작 이후 후보입니다. | [상세](workflow-and-collaboration.md#rich-risk-review-and-residual-risk-lifecycle) |
| 설계 관문과 정책 차단 사유: `design_gate`, `design_policy` | 정책과 적합성 | 설계 관문, 정책 차단 사유 범주, 설계 품질 정책 이후 후보입니다. | [상세](policy-and-conformance.md#design-gates-and-policy-blockers) |
| 설계 정책 면제 | 정책과 적합성 | 설계 정책 차단 사유에 대한 면제 경로 이후 후보입니다. | [상세](policy-and-conformance.md#design-policy-waiver) |
| 넓은 설계 검증기와 심각도 기반 차단 | 정책과 적합성 | 검증기 ID, 심각도 의미, 차단 정책 이후 후보입니다. | [상세](policy-and-conformance.md#broad-design-validators-and-severity-based-blocking) |
| 전체 설계 품질 정책 계열 | 정책과 적합성 | `shared_design`, `domain_language`, `codebase_stewardship` 같은 설계 품질 정책 계열 이후 후보입니다. | [상세](policy-and-conformance.md#full-design-quality-policy-families) |
| 운영 강화 | 보안과 보증 | 로컬 운영 진단과 더 강한 보안 자세를 다루는 이후 후보입니다. | [상세](security-and-assurance.md#operations-hardening) |
| 향후 로컬 운영자 명령 묶음 | 커넥터와 접점 | `harness doctor`, `harness export`, `harness conformance run` 같은 로컬 명령 접점 이후 후보입니다. | [상세](connectors-and-surfaces.md#future-local-operator-command-family) |
| 내보내기 | 아티팩트와 증거 | 내보내기 동작, 내보내기 아티팩트, 가림 경계를 다루는 이후 후보입니다. | [상세](artifacts-and-evidence.md#export) |
| 릴리스 인계 | 작업 흐름과 협업 | 운영 환경 권한 없이 릴리스 인계 흐름을 다루는 이후 후보입니다. | [상세](workflow-and-collaboration.md#release-handoff) |
| 내보내기와 인계 형식 | 아티팩트와 증거 | 내보내기나 인계를 위한 파일 형식, 번들 계약, 출처 추적 요구사항 이후 후보입니다. | [상세](artifacts-and-evidence.md#export-and-handoff-formats) |
| 복구와 `reconcile` | 작업 흐름과 협업 | 복구, `reconcile`, 상태 복구 흐름 이후 후보입니다. | [상세](workflow-and-collaboration.md#recovery-and-reconcile) |
| 운영자 준비 상태와 `doctor` 접점 | 커넥터와 접점 | 로컬 준비 상태와 진단 접점 이후 후보입니다. | [상세](connectors-and-surfaces.md#operator-readiness-and-doctor-surfaces) |
| 상태 보기 새로고침과 최신성 진단 | 커넥터와 접점 | 상태 보기 자료의 새로고침과 최신성 가시성 이후 후보입니다. | [상세](connectors-and-surfaces.md#projection-refresh-and-freshness-diagnostics) |
| 영속 상태 보기 작업 | 작업 흐름과 협업 | 상태 보기 작업 생명주기와 작업 저장소 이후 후보입니다. | [상세](workflow-and-collaboration.md#persistent-projection-jobs) |
| 상태 보기 `reconcile`과 편집 가능한 상태 보기 영역 | 작업 흐름과 협업 | 상태 보기 `reconcile`, managed-block 복구, 편집 가능한 상태 보기 영역 이후 후보입니다. | [상세](workflow-and-collaboration.md#projection-reconcile-and-editable-projection-areas) |
| 더 강한 로컬 역량 프로필 | 보안과 보증 | 관찰, 캡처, 격리, 차단 역량을 나타내는 프로필 라벨 이후 후보입니다. | [상세](security-and-assurance.md#stronger-local-capability-profiles) |
| 명령, 네트워크, 비밀값 접근 관찰 | 보안과 보증 | 선택된 명령, 네트워크, 비밀값 접근 의도를 관찰하는 이후 후보입니다. | [상세](security-and-assurance.md#command-network-and-secret-access-observation) |
| 명령, 네트워크, 비밀값 도구 실행 전 차단 | 보안과 보증 | 도구 실행 전 예방형 차단 주장을 다루는 이후 후보입니다. | [상세](security-and-assurance.md#command-network-and-secret-pre-tool-blocking) |
| 향후 적합성 실행 진입점 | 정책과 적합성 | 실행 가능한 적합성 실행기, 모음, 보고 계약 이후 후보입니다. | [상세](policy-and-conformance.md#future-conformance-run-entrypoint) |
| `harness.next` | 작업 흐름과 협업 | 다음 행동 API 메서드 이후 후보입니다. | [상세](workflow-and-collaboration.md#harness-next) |
| `harness.launch_verify` | 작업 흐름과 협업 | 검증 시작 API 메서드 이후 후보입니다. | [상세](workflow-and-collaboration.md#harness-launch-verify) |
| `harness.record_eval` | 작업 흐름과 협업 | 평가 기록 API 메서드 이후 후보입니다. | [상세](workflow-and-collaboration.md#harness-record-eval) |
| `harness.record_manual_qa` | 작업 흐름과 협업 | 수동 QA 기록 API 메서드 이후 후보입니다. | [상세](workflow-and-collaboration.md#harness-record-manual-qa) |
| 이후 읽기 전용 리소스 | 커넥터와 접점 | `policy`, `evidence-manifest`, `surface`, `report`, `bundle`, `journey`, `design` 같은 읽기 전용 리소스 이후 후보입니다. | [상세](connectors-and-surfaces.md#later-read-only-resources) |
| 이후 `harness.record_run` 분기 | 작업 흐름과 협업 | 검증 입력, 피드백 루프 갱신, TDD 추적 갱신을 위한 `harness.record_run` 분기 이후 후보입니다. | [상세](workflow-and-collaboration.md#later-harness-record-run-branches) |
| 역량 조건부 `prepare_write`와 `record_run` 관찰 | 보안과 보증 | 쓰기 준비와 실행 기록 주변의 명령, 네트워크, 비밀값 접근 관찰 이후 후보입니다. | [상세](security-and-assurance.md#capability-gated-prepare-write-and-record-run-observation) |
| 이후 사용자 판단 분기 | 작업 흐름과 협업 | `qa_waiver`, `verification_risk_acceptance`, 면제, `reconcile`, 잔여 위험, 더 풍부한 수락 분기 이후 후보입니다. | [상세](workflow-and-collaboration.md#later-user-judgment-branches) |
| 이후 스키마 확장 | 정책과 적합성 | 여러 영역에 걸친 필드, enum 값, 검증기 이후 후보입니다. | [상세](policy-and-conformance.md#later-schema-extensions) |
| 역량 프로필 지원 필드 | 보안과 보증 | 관찰, 캡처, 도구 실행 전 차단, 격리 역량 지원 필드 이후 후보입니다. | [상세](security-and-assurance.md#capability-profile-support-fields) |
| 역량 조건부 권한 관찰 필드 | 보안과 보증 | `intended_commands`, `intended_network`, `network_write`, `secret_access` 같은 필드 이후 후보입니다. | [상세](security-and-assurance.md#capability-gated-authorization-observation-fields) |
| 이후 행위자, 생산자, 캡처 출처 값 | 아티팩트와 증거 | `evaluator`, `operator`, `capture_adapter` 같은 생산자, 행위자, 캡처 출처 값 이후 후보입니다. | [상세](artifacts-and-evidence.md#later-actor-producer-and-capture-source-values) |
| 네이티브 아티팩트 캡처 | 아티팩트와 증거 | 참조 스테이징을 넘어 아티팩트 데이터를 직접 캡처하는 이후 후보입니다. | [상세](artifacts-and-evidence.md#native-artifact-capture) |
| 이후 닫기와 보증 필드 | 보안과 보증 | 닫기, 관문, 검증, QA, 설계, 보증 필드 이후 후보입니다. | [상세](security-and-assurance.md#later-close-and-assurance-fields) |
| 이후 다음 행동 값 | 작업 흐름과 협업 | `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` 같은 다음 행동 값 이후 후보입니다. | [상세](workflow-and-collaboration.md#later-next-action-values) |
| 읽기 전용 플레이북과 판단 맥락 메타데이터 | 아티팩트와 증거 | 판단을 그 자체로 충족하지 않으면서 증거 검토를 도울 수 있는 읽기 전용 메타데이터 이후 후보입니다. | [상세](artifacts-and-evidence.md#read-only-playbook-and-judgment-context-metadata) |
| 이후 참조와 아티팩트 값 계열 | 아티팩트와 증거 | 번들, 매니페스트, QA 캡처, 내보내기 구성요소, 설계, `Eval`, 수동 QA, TDD, `Projection`, 관련 참조 값 계열 이후 후보입니다. | [상세](artifacts-and-evidence.md#later-reference-and-artifact-value-families) |
| `ValidatorResult` 안정 ID와 정책 계열 | 정책과 적합성 | 안정 검증기 식별, 정책 계열, 심각도, 면제 어휘 이후 후보입니다. | [상세](policy-and-conformance.md#validatorresult-stable-ids-and-policy-families) |
| 면제, `reconcile`, 잔여 위험 분기 | 작업 흐름과 협업 | 면제, `reconcile`, 잔여 위험 분기 이후 후보입니다. | [상세](workflow-and-collaboration.md#waiver-reconcile-and-residual-risk-branches) |
| 이후 템플릿 이름 | 아티팩트와 증거 | 더 풍부한 판단, 증거, 실행, 설계, 내보내기 표시를 위한 템플릿 이름 이후 후보입니다. | [상세](artifacts-and-evidence.md#later-template-names) |
| 향후 fixture 계열 | 정책과 적합성 | 실행 가능한 fixture 계열, 적합성 모음, 검증 주장, 보고 형식 이후 후보입니다. | [상세](policy-and-conformance.md#future-fixture-families) |
| 대시보드와 호스팅 작업 흐름 | 커넥터와 접점 | 대시보드, 호스팅 작업 흐름, 시각화, 카드, 아티팩트 대시보드 접점 이후 후보입니다. | [상세](connectors-and-surfaces.md#dashboard-and-hosted-workflows) |
| 검증 결과 카드와 더 풍부한 검증 작업 흐름 | 작업 흐름과 협업 | QA를 대체하지 않는 검증 카드와 더 풍부한 검증 작업 흐름 이후 후보입니다. | [상세](workflow-and-collaboration.md#verification-result-cards-and-richer-verification-workflows) |
| 브라우저 캡처 자동화 | 아티팩트와 증거 | 브라우저 스크린샷, 녹화, 캡처된 UI 상태를 증거 자료로 다루는 이후 후보입니다. | [상세](artifacts-and-evidence.md#browser-capture-automation) |
| 접점 간 검증 | 커넥터와 접점 | IDE, CLI, 채팅, MCP, 호스팅 접점 사이의 검증 가시성 이후 후보입니다. | [상세](connectors-and-surfaces.md#cross-surface-verification) |
| 더 넓은 커넥터와 호스팅 런타임 | 커넥터와 접점 | 커넥터 마켓플레이스, 호스팅 UI, 호스팅 런타임, 원격 런타임 이후 후보입니다. | [상세](connectors-and-surfaces.md#broader-connectors-and-hosted-runtime) |
| 커넥터 적합성 생태계 | 커넥터와 접점 | 커넥터 대상 호환성 주장, 마켓플레이스 신호, 보고 접점 이후 후보입니다. | [상세](connectors-and-surfaces.md#connector-conformance-ecosystem) |
| 네이티브 후크와 고급 사이드카 감시 | 보안과 보증 | 더 넓은 도구 가시성을 뒷받침할 수 있는 네이티브 후크나 사이드카 감시 주장 이후 후보입니다. | [상세](security-and-assurance.md#native-hooks-and-advanced-sidecar-watcher) |
| 맥락 색인과 파생 지표 | 작업 흐름과 협업 | 그 자체로 권한이 되지 않으면서 작업 흐름 검토를 돕는 맥락 색인과 파생 지표 이후 후보입니다. | [상세](workflow-and-collaboration.md#context-index-and-derived-metrics) |
| 팀 작업 흐름과 오케스트레이션 | 작업 흐름과 협업 | 팀 권한, 공유 역량 집합, 오케스트레이션, 병렬 흐름 동작 이후 후보입니다. | [상세](workflow-and-collaboration.md#team-workflows-and-orchestration) |
| 고급 릴리스와 배포 자동화 | 작업 흐름과 협업 | 배포, 카나리, 롤백, 병합, 운영 환경 모니터링 자동화 이후 후보입니다. | [상세](workflow-and-collaboration.md#advanced-release-and-deployment-automation) |
| 고급 검증기와 인터페이스 확인 | 정책과 적합성 | 고급 검증기, 설계 정책 검증기, 언어 확인, 인터페이스 확인 이후 후보입니다. | [상세](policy-and-conformance.md#advanced-validators-and-interface-checks) |
