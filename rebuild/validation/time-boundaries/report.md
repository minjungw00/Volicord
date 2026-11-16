# Runtime and validation time boundaries

## 판단

로컬 Work command의 일률적인 300초 상한은 제거한다. 호출자가 실행 시간과
명령을 명시적으로 선택하는 경로에서 긴 빌드·검사를 실행하기도 전에 거부할
근거가 없었다. 양수이고 monotonic clock에 표현 가능한 유한 timeout을 계속
요구하며, 실제 초과·종료·cleanup·출력 보존은 기존 process primitive가 수행한다.
Retention, stream/body 크기, Work/snapshot 검사와 local-only authority는 유지한다.

나머지 아래 기준은 적용 단위를 유지한다. 고정 검증 입력의 성능 기준을 실제
Project 전체의 수명·작업 수 제한으로 적용하거나 실패 측정값에 맞춰 자동으로
올리지 않는다. 작업 이력 증가에 따른 읽기 비용이 일정하다는 보장도 아니다.

## 적용 단위

| 기준 | 적용 대상 | 이력 증가와의 관계 / 판단 |
| --- | --- | --- |
| V11 전체 900초, MCP call 90초 | `three-target-installed-journey-volicord-three-work-1`: 3 target / 5 Work / 55 leaf의 Linux regression | 모든 저장소의 SLA가 아니다. Target revision·규모와 실제 시간을 기록한다. 여정·입력 규모가 달라지면 별도 근거와 workload 검토가 필요하다. 현재 숫자는 유지한다. |
| 리허설 응답 대기 60초, 전체 1,800초, 지원 process 300초 | 새로 준비한 유한한 구조 검증 여정의 watchdog | 실제 장기 사용 Project에 실행 제한으로 설치되지 않는다. 불완전한 실행은 통과가 아니며, 부하·진행 상황과 환경 원인을 별도로 확인한다. 유지한다. |
| 수집 CLI 120초 / 설명 관측 CLI 60초 | 정해진 캠페인의 검토·수명주기 자료 수집 | Product CLI 자체의 보편적인 수명 제한이 아니다. 이력을 무한히 늘린 입력의 일정한 latency를 보장하지 않는다. 유지한다. |
| Viewer 65ms / snapshot 180ms 등 | opt-in `viewer-read-budgets.json`의 특정 debug 환경과 고정 입력 | 일반 Viewer 응답 SLA가 아니다. 고정 입력의 count invariant와 timing 진단을 구분한다. 유지한다. |
| Viewer socket read/write 5초 | 개별 transport I/O 대기 | 페이지 계산 전체의 5초 제한이 아니다. 이력 수와 직접 결합된 deadline이 아니다. 유지한다. |
| 그림 엔진 초기화·실행 3초 | 입력·출력·heap이 제한된 개별 local layout | 모든 이력의 전체 그래프를 반드시 그리는 약속이 아니다. 실패한 geometry를 거부하고 원문은 보존한다. 유지한다. |
| Git status 30초 / code-origin Git 관측 10초 | 개별 로컬 저장소·파일 관측 process | Canonical Work 이력 수에 대한 상한이 아니다. 큰 working tree/blob·저장장치·부하에 따라 실패할 수 있으며 실패를 기록한다. 임의의 대형 저장소에 대한 성공 보장은 아니다. 현재 유지한다. |
| Provider 기본 180초, 인증 15초, cleanup 5초 | 제한된 provider 요청 및 process 수명 | 요청별 기본값이며 library configuration의 요청 timeout은 별도로 지정 가능하다. 저장소 규모 증가를 단일 무제한 요청으로 해석하지 않는다. 유지한다. |
| 관측 대조 fixture의 초기 응답 10초 / 샘플 확인 30초 / 관측 최대 60초 / worker join 16초 | 실제 child 3개의 시작·관측·정리를 검증하는 지원 코드 | Project 이력 한계가 아니다. Sampler 안에서 child 시작을 기다리지 않으며 누락·gap·조기 종료를 계속 거부한다. |
| Provider 분류 fixture의 30ms | 실제 timeout 분기를 검증하는 음성 입력 | timeout 입력의 30ms는 유지한다. 즉시 실패 입력은 실행 기회를 주는 2초로 분리하고, timeout/취소 입력은 관측 전에 정상 종료하지 않도록 60초 sleeper를 사용한다. 이는 fixture 수명이며 Product provider 기본값·cleanup 제한을 늘리지 않는다. |
| 로컬 Work command 300초 최대값 | 사용자가 직접 지정한 로컬 실행 요청 | 긴 정상 명령을 거부하는 제품 제한이었다. CLI와 Operations의 중복 상한을 제거하고, 필수 양수·유한 deadline 및 clock 표현 가능성 검사를 유지한다. |

Source retention·cache 수명·확인 시각은 성능 시간 제한과 별개의 의미다. 이
검토로 Source 만료나 저장 기간을 늘리지 않는다. 다른 host가 적용하는 외부
MCP timeout은 이 저장소의 코드로 검증하거나 보장한 값이 아니다.

## 관측과 검증

- 기존 `f7aa15989` release 실행 파일의 동일 분석 대상은 별도의 격리 Runtime
  복사본에서 분석과 정상 EOF 정리를 포함한 probe를 15,461.846ms에 완료했다.
  순수 분석 계산 시간이나 실제 브라우저 latency 측정이 아니다.
  이전 두 리허설에서는 60초에 응답하지
  못했고 그 시점의 기록된 host load average 첫 값은 약 70.47 / 289.97이었다.
  이 차이는 시간 초과만으로 제품의 알고리즘 문제나 한계가 과도하다고 단정할
  수 없다는 근거다. Cache·부하·경로 효과를 완전히 분리한 비교는 아니다.
- 원래 실패, 60초 deadline, 정리 중 강제 종료와 원문 증거를 보존한다. 성공한
  별도 실행으로 이전 실패를 바꾸지 않는다. Watched workspace 밖의 동일 실행
  파일·동일 candidate 대조는 별도의 진단이며 gate qualification을 대체하지 않는다.
- 동일 release 실행 파일과 `f7aa15989`의 깨끗한 Source를 watched workspace 밖에서
  실행한 리허설 대조는 Volicord A/B/C 분석 응답을 각각 약 15,879.144ms /
  12,541.661ms / 12,691.475ms에 받았다. 60초 기준을 바꾸지 않았다. 이 관측은
  고정 구조 검증 입력에 한정되며, 임의의 큰 저장소·전체 이력 benchmark가 아니다.
- `source_observations`의 실제 CLI 회귀 검사는 86,400초를 명시한 짧은 명령의
  실행·파일 효과·근거 저장을 확인한다. 1일 동안 실행했다는 성능 측정이 아니다.
- 같은 검사에서 0 / clock에 표현 불가능한 timeout은 명령 실행이나 Source
  생성 전에 거부한다. 실제 30ms deadline을 넘기는 별도 명령은 시간 초과로
  종료되고 `timeout_detected`를 보존하며 뒤의 파일 쓰기를 수행하지 않는다.
- Provider 분류 fixture의 기존 2초 sleeper는 관측기를 명시적으로 일시 중지한
  대조에서 먼저 정상 종료했다. 재개된 Product는 관측된 종료와 누락된 응답을
  `Failed`로 정확히 보고했지만 fixture는 `TimedOut`만 요구해 실패했다.
  이 대조는 가능한 잘못된 전제를 재현한 것이며, 원본 gate의 실제 반환값과
  scheduling 상태가 보존되지 않아 원래 실패의 원인으로 확정하지 않는다.
  실패·timeout·취소 기대 결과는 완화하지 않고 fixture의 의도된 수명을 분리한다.
  수정 후에는 관측기를 약 2.3초 중지해도 fixture가 살아 있었고, 재개 후
  `TimedOut`으로 분류되어 검사가 통과했다. 실제 중지 시간을 정확히 30ms로
  보장한다는 결과나 일반적인 공급자 latency 측정은 아니다.

## 한계와 후속 기준

현재 검토는 임의 크기·모든 하드웨어·무제한 이력의 성능을 증명하지 않는다.
실제 Project의 전체 이력을 읽고 내보내는 비용은 증가할 수 있다. 그 경우에는
규모, 요청 범위, cold/warm 상태와 자원 부하를 함께 기록해 paging·재사용·비용을
검토한다. 자동 삭제, 의미 누락 또는 과거 측정의 상한 재작성으로 해결하지 않는다.

Validation workload의 operation 수·target 크기가 변하면 새 입력과 비교 조건을
먼저 선언한다. 동일 입력의 설명되지 않은 지연은 regression으로 남긴다. 개별
지정 timeout을 넘긴 실행은 계속 timeout이며, 사용자 승인 없는 재실행이나 더
긴 암묵적 실행 권한을 주지 않는다. Provider 전송·모델·Source 범위와 인간 판정은
이 검토로 확대되거나 생성되지 않는다.
