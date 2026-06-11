# 이후 후보 색인

이 색인은 비활성 이후 후보를 카테고리 문서로 안내합니다. 경로 목록일 뿐이며 계약 담당 문서가 아닙니다.

현재 범위는 [현재 MVP 범위](../reference/active-mvp-scope.md)를 봅니다. 이 페이지는 현재 MVP 동작, 구현 작업, 런타임 권한, 현재 API 계약, 현재 저장 효과, 현재 보안 보장, 런타임/서버 코딩 시작 허가를 정의하지 않습니다.

이 색인이나 카테고리 파일의 언급은 승격이 아닙니다. 후보는 현재 범위와 관련된 현재 담당 문서 또는 승격 과정에서 만든 새 담당 문서가 같은 문서 전용 작업 묶음에서 갱신된 뒤에만 현재 MVP로 들어갈 수 있습니다.

승격 시점의 담당 문서 갱신은 필요한 담당 문서 작업이 승격 시점에 일어난다는 뜻입니다. 승격 전의 현재 요구사항을 만들지 않으며, 후보 항목 자체도 현재 계약 담당 문서가 아닙니다.

프로필 조건부 값과 이후 후보는 다릅니다. 어떤 값이 프로필 조건부 값이 되려면 현재 범위와 담당 문서가 그 프로필과 지원 값 집합을 함께 이름 붙여야 합니다. 이 페이지의 값은 그런 승격 전까지 이후 후보입니다.

## 후보 경로

### 아티팩트와 증거

- [전체 `Evidence Manifest`](artifacts-and-evidence.md#full-evidence-manifest): 매니페스트 수준 증거 기록과 렌더링 요약.
- [내보내기](artifacts-and-evidence.md#export): 내보내기 동작, 내보내기 아티팩트, 가림 경계.
- [내보내기와 인계 형식](artifacts-and-evidence.md#export-and-handoff-formats): 파일 형식, 번들 계약, 출처 추적 요구사항.
- [이후 행위자, 생산자, 캡처 출처 값](artifacts-and-evidence.md#later-actor-producer-and-capture-source-values): 생산자, 행위자, 캡처 출처 어휘.
- [네이티브 아티팩트 캡처](artifacts-and-evidence.md#native-artifact-capture): 참조 스테이징을 넘어서는 직접 아티팩트 캡처.
- [읽기 전용 플레이북과 판단 맥락 메타데이터](artifacts-and-evidence.md#read-only-playbook-and-judgment-context-metadata): 판단을 직접 충족하지 않는 증거 검토 메타데이터.
- [이후 참조와 아티팩트 값 계열](artifacts-and-evidence.md#later-reference-and-artifact-value-families): 번들, 매니페스트, QA, 내보내기, 설계, `Eval`, 수동 QA, TDD, `Projection`, 관련 참조 값.
- [이후 템플릿 이름](artifacts-and-evidence.md#later-template-names): 더 풍부한 판단, 증거, 실행, 설계, 내보내기 표시 이름.
- [브라우저 캡처 자동화](artifacts-and-evidence.md#browser-capture-automation): 브라우저 스크린샷, 녹화, 캡처된 UI 상태 증거 자료.

### 커넥터와 접점

- [향후 로컬 운영자 명령 묶음](connectors-and-surfaces.md#future-local-operator-command-family): `harness doctor`, `harness export`, `harness conformance run` 같은 로컬 명령 접점.
- [운영자 준비 상태와 `doctor` 접점](connectors-and-surfaces.md#operator-readiness-and-doctor-surfaces): 로컬 준비 상태와 진단 접점.
- [상태 보기 새로고침과 최신성 진단](connectors-and-surfaces.md#projection-refresh-and-freshness-diagnostics): 상태 보기 자료의 새로고침과 최신성 가시성.
- [이후 읽기 전용 리소스](connectors-and-surfaces.md#later-read-only-resources): `policy`, `evidence-manifest`, `surface`, `report`, `bundle`, `journey`, `design` 같은 읽기 전용 리소스.
- [대시보드와 호스팅 작업 흐름](connectors-and-surfaces.md#dashboard-and-hosted-workflows): 대시보드, 호스팅 작업 흐름, 시각화, 카드, 아티팩트 대시보드 접점.
- [접점 간 검증](connectors-and-surfaces.md#cross-surface-verification): IDE, CLI, 채팅, MCP, 호스팅 접점 사이의 검증 가시성.
- [더 넓은 커넥터와 호스팅 런타임](connectors-and-surfaces.md#broader-connectors-and-hosted-runtime): 커넥터 마켓플레이스, 호스팅 UI, 호스팅 런타임, 원격 런타임 후보.
- [커넥터 적합성 생태계](connectors-and-surfaces.md#connector-conformance-ecosystem): 커넥터 대상 호환성 주장, 마켓플레이스 신호, 보고 접점.

### 정책과 적합성

- [수동 QA 작업 흐름과 `qa_gate`](policy-and-conformance.md#manual-qa-workflow-and-qa-gate): 수동 QA 관문 정책과 닫기 준비 상태 관계.
- [수동 QA 면제 `qa_waiver`](policy-and-conformance.md#manual-qa-waiver-qa-waiver): 사용자 소유 판단을 대체하지 않는 수동 QA 정책 면제 경로.
- [검증 관문 `verification_gate`](policy-and-conformance.md#verification-gate-verification-gate): 검증 관문 정책과 닫기 준비 상태 관계.
- [설계 관문과 정책 차단 사유](policy-and-conformance.md#design-gates-and-policy-blockers): 설계 관문, 정책 차단 사유 범주, 설계 품질 정책.
- [설계 정책 면제](policy-and-conformance.md#design-policy-waiver): 설계 정책 차단 사유에 대한 면제 경로.
- [넓은 설계 검증기와 심각도 기반 차단](policy-and-conformance.md#broad-design-validators-and-severity-based-blocking): 검증기 ID, 심각도 의미, 차단 정책.
- [전체 설계 품질 정책 계열](policy-and-conformance.md#full-design-quality-policy-families): `shared_design`, `domain_language`, `codebase_stewardship` 같은 정책 계열.
- [향후 적합성 실행 진입점](policy-and-conformance.md#future-conformance-run-entrypoint): 실행 가능한 적합성 실행기, 모음, 보고 계약.
- [이후 스키마 확장](policy-and-conformance.md#later-schema-extensions): 여러 영역에 걸친 필드, enum 값, 검증기.
- [`ValidatorResult` 안정 ID와 정책 계열](policy-and-conformance.md#validatorresult-stable-ids-and-policy-families): 안정 검증기 식별, 정책 계열, 심각도, 면제 어휘.
- [향후 픽스처 계열](policy-and-conformance.md#future-fixture-families): 실행 가능한 픽스처 계열, 적합성 모음, 검증 주장, 보고 형식.
- [고급 검증기와 인터페이스 확인](policy-and-conformance.md#advanced-validators-and-interface-checks): 고급 검증기, 설계 정책 검증기, 언어 확인, 인터페이스 확인.

### 보안과 보증

- [보증 강화](security-and-assurance.md#assurance-hardening): 더 강한 증거, 검증, 닫기 준비 상태 보증 주장.
- [운영 강화](security-and-assurance.md#operations-hardening): 로컬 운영 진단과 더 강한 보안 자세.
- [더 강한 로컬 역량 프로필](security-and-assurance.md#stronger-local-capability-profiles): 관찰, 캡처, 격리, 차단 역량 프로필 라벨.
- [명령, 네트워크, 비밀값 접근 관찰](security-and-assurance.md#command-network-and-secret-access-observation): 선택된 명령, 네트워크, 비밀값 접근 의도 관찰.
- [명령, 네트워크, 비밀값 도구 실행 전 차단](security-and-assurance.md#command-network-and-secret-pre-tool-blocking): 도구 실행 전 예방형 차단 주장.
- [역량 조건부 `prepare_write`와 `record_run` 관찰](security-and-assurance.md#capability-gated-prepare-write-and-record-run-observation): 쓰기 준비와 실행 기록 주변의 관찰.
- [역량 프로필 지원 필드](security-and-assurance.md#capability-profile-support-fields): 관찰, 캡처, 도구 실행 전 차단, 격리 역량 지원 필드.
- [역량 조건부 권한 관찰 필드](security-and-assurance.md#capability-gated-authorization-observation-fields): `intended_commands`, `intended_network`, `network_write`, `secret_access` 같은 필드.
- [이후 닫기와 보증 필드](security-and-assurance.md#later-close-and-assurance-fields): 닫기, 관문, 검증, QA, 설계, 보증 필드.
- [네이티브 후크와 고급 사이드카 감시](security-and-assurance.md#native-hooks-and-advanced-sidecar-watcher): 더 넓은 도구 가시성을 위한 네이티브 후크나 사이드카 감시 주장.

### 작업 흐름과 협업

- [탐색 요약, 질문 대기열, 가정 기록](workflow-and-collaboration.md#discovery-brief-question-queue-and-assumption-register): 열린 질문, 가정, 작업 맥락 구체화 기록.
- [검증 위험 수락 `verification_risk_acceptance`](workflow-and-collaboration.md#verification-risk-acceptance-verification-risk-acceptance): 검증 위험을 사용자가 수락하는 판단 경로.
- [`Eval`과 분리형 검증 작업 흐름](workflow-and-collaboration.md#eval-and-detached-verification-workflows): 평가와 분리형 검증 작업 흐름.
- [전체 `Decision Packet`과 `presentation=full`](workflow-and-collaboration.md#full-decision-packet-and-presentation-full): 전체 형식 판단 표시.
- [상세 위험 검토와 잔여 위험 생명주기](workflow-and-collaboration.md#rich-risk-review-and-residual-risk-lifecycle): 더 풍부한 위험 검토 기록, 잔여 위험 생명주기, 만료 동작.
- [릴리스 인계](workflow-and-collaboration.md#release-handoff): 운영 환경 권한 없는 릴리스 인계 흐름.
- [복구와 `reconcile`](workflow-and-collaboration.md#recovery-and-reconcile): 복구, `reconcile`, 상태 복구 흐름.
- [영속 상태 보기 작업](workflow-and-collaboration.md#persistent-projection-jobs): 상태 보기 작업 생명주기와 작업 저장소.
- [상태 보기 `reconcile`과 편집 가능한 상태 보기 영역](workflow-and-collaboration.md#projection-reconcile-and-editable-projection-areas): 상태 보기 `reconcile`, managed-block 복구, 편집 가능한 상태 보기 영역.
- [`harness.next`](workflow-and-collaboration.md#harness-next): 다음 행동 API 메서드.
- [`harness.launch_verify`](workflow-and-collaboration.md#harness-launch-verify): 검증 시작 API 메서드.
- [`harness.record_eval`](workflow-and-collaboration.md#harness-record-eval): 평가 기록 API 메서드.
- [`harness.record_manual_qa`](workflow-and-collaboration.md#harness-record-manual-qa): 수동 QA 기록 API 메서드.
- [이후 `harness.record_run` 분기](workflow-and-collaboration.md#later-harness-record-run-branches): 검증 입력, 피드백 루프 갱신, TDD 추적 갱신 분기.
- [이후 사용자 판단 분기](workflow-and-collaboration.md#later-user-judgment-branches): `qa_waiver`, `verification_risk_acceptance`, 면제, `reconcile`, 잔여 위험, 더 풍부한 수락 분기.
- [이후 다음 행동 값](workflow-and-collaboration.md#later-next-action-values): `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` 같은 다음 행동 값.
- [면제, `reconcile`, 잔여 위험 분기](workflow-and-collaboration.md#waiver-reconcile-and-residual-risk-branches): 면제, `reconcile`, 잔여 위험 분기.
- [검증 결과 카드와 더 풍부한 검증 작업 흐름](workflow-and-collaboration.md#verification-result-cards-and-richer-verification-workflows): QA를 대체하지 않는 검증 카드와 더 풍부한 검증 작업 흐름.
- [맥락 색인과 파생 지표](workflow-and-collaboration.md#context-index-and-derived-metrics): 그 자체로 권한이 되지 않는 작업 흐름 검토 맥락.
- [팀 작업 흐름과 오케스트레이션](workflow-and-collaboration.md#team-workflows-and-orchestration): 팀 권한, 공유 역량 집합, 오케스트레이션, 병렬 흐름 동작.
- [고급 릴리스와 배포 자동화](workflow-and-collaboration.md#advanced-release-and-deployment-automation): 배포, 카나리, 롤백, 병합, 운영 환경 모니터링 자동화.
