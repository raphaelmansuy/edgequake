# EdgeQuake → Portainer Stack 배포 가이드

**목표**: Docker Compe 파일을 Portainer 의 Stack 으로 완벽하게 변환하여 관리하는 방법

---

## 1️⃣ 현재 구조 분석

최적의 Portainer Stack 배포를 위해 `docker/docker-compose.yml` 을 검토합니다.

### 주요 서비스 구성
| Service | 포트 | 용도 | 비고 |
|---------|------|------|------|
| `edgequake` | 8080 | Backend API (Rust/Axum) | PostgreSQL 의존성 있음 |
| `frontend` | 3000 | Frontend UI (Next.js/React 19) | Backend 의존성 있음 |
| `postgres` | 5432 | Database (PostgreSQL + pgvector + AGE) | 볼륨 persists 데이터 |

### 환경 변수 구성
- **Required**: `DATABASE_URL`, POSTGRES 설정
- **Optional**: `OPENAI_API_KEY`, `OLLAMA_HOST` 등 LLM Provider 관련

---

## 2️⃣ Portainer Stack 변환 전략

### 접근 방식 1: 파일 업로드 (가장 간단)
Portainer 웹 UI 에서 compose 파일을 직접 업로드합니다.


**장점**:
- 빠른 배포 (5 분 이내)
- 로컬/테스트 환경에 최적
- Docker Compose 와 동일한 구조 유지

**단점**:
- 수동 업데이트 필요
- 버전 관리 어려움

### 접근 방식 2: Git Repository 연동 (권장 - 프로덕션)
Git repo URL 을 Portainer 에 등록하여 자동 동기화합니다.


**파일 위치**: `edgequake/docker/portainer-compose.yml` 으로 별도 분리 추천

---

## 3️⃣ 변환된 Compose 파일

기존 `docker-compose.yml` 과의 주요 차이점:

### A. Container Name 제거
Portainer 가 자동으로 이름에 Stack 이름을 접두사로 붙이므로 중괄호로 사용합니다.

```yaml
services:
  edgequake:
    # container_name: edgequake (제거하거나 주석 처리)
    container_name: ${STACK_NAME:-edgequake}-backend  # 동적 네임스페이스
```

### B. Build vs Image 모드 선택

**Option 1 - Push to Registry (권장)**  
1. `docker build` 로 이미지 빌드 & 태그
2. Docker Hub / GitHub Container Registry 에 push
3. Portainer 에서 `image:` 참조


**옵션 코드**:

```yaml
edgequake:
  image: ghcr.io/raphaelmansuy/edgequake:${TAG:-latest}
  # build 제거 (이미지 풀)
```


**Option 2 - Local Build (개발용)**  
Portainer 가 Docker Host 에서 직접 빌드합니다. `build:` 섹션 유지합니다.

### C. Frontend API URL 수정

Docker 내부 네트워크에서 Backend 를 접근하므로 localhost 대신 서비스 이름을 사용합니다:

```yaml
frontend:
  build:
    args:
      NEXT_PUBLIC_API_URL: http://edgequake:8080  # 변경점!
  environment:
    - NEXT_PUBLIC_API_URL=http://edgequake:8080  # Docker 내부에서 사용
```

**Why?**  
- Portainer Stack 내부에서는 `localhost` 가 아닌 서비스 이름 (`edgequake`) 으로 접근해야 합니다.
- HTTP 요청이 외부 호스트 (`host.docker.internal`) 로 향하면 Connection refused 에러가 발생합니다.


---

## 4️⃣ 배포 단계별 가이드

### 단계 1: Portainer 설정 (초기화)

**Prerequisites**:
```bash
# 1. Docker Host 가 Portainer 과 통신 가능한지 확인
docker context ls

# 2. Portainer Stack 이 이미 설치되어 있는지 확인
# (만약 없음, https://www.portainer.io/install/docker-engine 참조)
docker run -d -p 8000:8000 -p 9000:9000 --name portainer \
  --restart=always \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v portainer_data:/data \
  portainer/portainer-ce:latest
```

### 단계 2: 환경 변수 설정

**`.env` 파일 생성** (`docker/` 디렉토리 하위):


```bash
# Required
POSTGRES_DB=edgequake
POSTGRES_USER=edgequake
POSTGRES_PASSWORD=SuperSecret123!

# Optional - LLM Provider 선택
OPENAI_API_KEY=sk-proj-your-openai-key  # OpenAI 모드
# EDGEQUAKE_LLM_PROVIDER=openai         # 또는 ollama

# Optional - Networking
STACK_NAME=edgequake-app  # Portainer Stack 네임스페이스
```

### 단계 3: Compose 파일 수정 (최종본)

`docker/portainer-compose.yml` 생성:


```yaml
version: '3.8'

services:
  edgequake:
    image: ghcr.io/raphaelmansuy/edgequake:${TAG:-latest}
    # 만약 빌드를直接使用하려면 `build:` 섹션 유지 (dev mode)
    
    restart: unless-stopped
    environment:
      - EDGEQUAKE_HOST=0.0.0.0
      - EDGEQUAKE_PORT=8080
      - DATABASE_URL=postgres://edgequake:${POSTGRES_PASSWORD}@postgres:5432/edgequake
      - OPENAI_API_KEY=${OPENAI_API_KEY:-}
      - OLLAMA_HOST=http://${OLLAMA_HOSTNAME:-host.docker.internal}:11434
      - EDGEQUAKE_LLM_PROVIDER=${EDGEQUAKE_LLM_PROVIDER:-ollama}
      - RUST_LOG=info,edgequake=debug
    
    depends_on:
      postgres:
        condition: service_healthy
    
    networks:
      - edgequake-network
    
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s

  frontend:
    image: ghcr.io/raphaelmansuy/edgequake-webui:${TAG:-latest}
    
    restart: unless-stopped
    environment:
      - NEXT_PUBLIC_API_URL=http://edgequake:8080
      - NODE_ENV=production
    
    depends_on:
      - edgequake
    
    networks:
      - edgequake-network

  postgres:
    image: portainer/edgequake-postgres:${TAG:-latest}  
    # 또는 공식 Postgres + Custom init 스크립트 사용
    
    restart: unless-stopped
    environment:
      - POSTGRES_USER=edgequake
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
      - POSTGRES_DB=edgequake
    
    volumes:
      - postgres-data:/var/lib/postgresql/data
      # ./init-extensions.sql 볼륨 마운트 필요 (pgvector + AGE)
    
    networks:
      - edgequake-network
    
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake -d edgequake"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres-data:
    driver: local

networks:
  edgequake-network:
    driver: bridge
```

### 단계 4: Portainer Web UI 에서 Stack 생성

1. **Portainer 로 접속**: `http://<portainer-host>:9000`
2. **Left Sidebar → Stacks** 클릭
3. **Add stack** 버튼 클릭
4. **Name**: `edgequake-app` (또는 원하는 이름)
5. **Navigation method 선택**:
   - **File browser**: 로컬에서 파일을 업로드
     - `docker/portainer-compose.yml` 드래그&드롭
   - **Web repository **(추천): Git URL 입력
     - Repository URL: `https://github.com/raphaelmansuy/edgequake.git`
     - Branch: `main` (또고 사용하려는 브랜치)
     - Path in repository: `docker/portainer-compose.yml`
6. **Environment Variables 설정** (필요한 경우):
   - Git 기반일 때 `.env` 파일도 함께 업로드되거나, 
   - UI 에서 직접 입력 가능 (예: `POSTGRES_PASSWORD=...`)
7. **Create the stack** 클릭

### 단계 5: 이미지 퍼블리시 선택 (프로덕션 모드)


**Option A - Multi-Arch Image Build**:

```bash
# amd64 + arm64 지원 빌드
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/raphaelmansuy/edgequake:${VERSION} \
  -t ghcr.io/raphaelmansuy/edgequake:latest \
  --push \
  ./edgequake
```


**Option B - Portainer 의 Build 기능 활용**:

Pro Plan 사용 시 Portainer 에서 자동으로 빌드 및 캐싱됩니다.

---

## 5️⃣ 검증 체크리스트

### ✅ Post-Deployment Verification

1. **Service Status 확인**:
   ```bash
   # Portainer 웹 UI → Stacks → edgequake-app → Containers
   
   # 또는 CLI 로:
   docker ps --filter "name=edgequake-app"
   ```

2. **Health Check 검증**:
   ```bash
   # Backend Health
   curl http://<host>:8080/health
   
   # Frontend UI
   curl -I http://<host>:3000
   
   # Database 연결 확인 (Postgres 컨테이너 내부)
   docker exec edgequake-app-postgres pg_isready -U edgequake -d edgequake
   ```

3. **네트워크 검증**:
   ```bash
   # Portainer 내 네트워크 디버깅
   docker network ls
   
   # 특정 서비스 간 통신 테스트
   docker exec edgequake-app-frontend wget -O /dev/null http://edgequake:8080/health
   ```

---

## 6️⃣常见问题 해결

### Q1: "Container exited immediately" 오류


**원인**: PostgreSQL init 스크립트 또는 환경 변수 누락.

**해결**:
```bash
# 로그 확인
docker logs edgequake-app-postgres

# Portainer Stack → Containers → postgres → Logs
```

### Q2: Backend 가 Frontend 로부터 접속 불가


**증상**: 3000 번 포트에서 `502 Bad Gateway` 또는 `Connection refused`.

**원인**: `NEXT_PUBLIC_API_URL` 이 `localhost:8080` 으로 설정되어 내부 네트워크가 아닌 외부 호스트로 요청.

**해결**: YAML 의 Build args 및 환경 변수를 확인합니다:
```yaml
frontend:
  build:
    args:
      NEXT_PUBLIC_API_URL: http://edgequake:8080  # ✅ Correct
  environment:
    - NEXT_PUBLIC_API_URL=http://edgequake:8080  # ✅ Correct
```

### Q3: 볼륨 데이터 소실


**증상**: Container 재시작 후 DB 데이터가 사라짐.

**원인**: `volumes` 섹션의 드라이버 설정이 local 이지만 Portainer 가 다른 Docker Volume 을 생성함.

**해결**:
- Portainer → Volumes 메뉴에서 postgres-data 볼륨을 확인합니다.
- 기존 Stack 을 삭제할 때 `-v` 플래그를 사용해야 데이터가 유지됩니다:


  ```bash
  # UI 에서 "Prune dangling volumes" 체크 해제

### Q4: Ollama Host 연결 불가


**증상**: Entity extraction 이 실패하고 로그에 `Connection refused to host.docker.internal:11434` 가 뜹니다.

**해결**:
- Docker 내부 Container 가 Ollama 를 호스트에서 실행 중으로 볼 수 없으면 외부 서비스나 OpenAI 로 변경합니다.


  ```yaml
  environmental:
    - OLLAMA_HOST=http://host.docker.internal:11434
  
  extra_hosts:  # Linux 환경에서만 필요
    - "host.docker.internal:host-gateway"
  ```

---

## 7️⃣ CI/CD 연동 (GitHub Actions 예시)


```yaml
# .github/workflows/portainer-deploy.yml
name: Deploy to Portainer

on:
  push:
    branches: [main]
    tags: ['v*']

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      # Backend 이미지 빌드 & Push
      - name: Build Docker image
        run: |
          VERSION=${{ github.ref_type == 'tag' && github.ref_name || 'latest' }}
          docker buildx build \
            --platform linux/amd64,linux/arm64 \
            -t ghcr.io/raphaelmansuy/edgequake:${VERSION} \
            -t ghcr.io/raphaelmansuy/edgequake:latest \
            --push \
            ./edgequake
      
      # Postgres 이미지 빌드 & Push (custom extensions 포함)
      - name: Build Postgres image
        run: |
          docker buildx build \
            --platform linux/amd64,linux/arm64 \
            -t ghcr.io/raphaelmansuy/edgequake-postgres:${VERSION} \
            --push \
            ./edgequake/docker
      
  deploy-portainer:
    needs: build-and-push
    runs-on: ubuntu-latest
    steps:
      # Portainer API 를 통한 배포 (선택사항)
      - name: Trigger Portainer Stack Update
        run: |
          curl -X PUT \
            -H "Authorization: Bearer ${{ secrets.PORTAINER_API_KEY }}" \
            https://<portainer-host>:9000/api/stacks/1/git/reset \
            --data 'webhook=true'
```

---

## 8️⃣ 모니터링 및 운영 가이드

### Health Check Endpoints

| Service | URL | Status 코드 |
|---------|-----|------------|
| Backend API | `http://<host>:8080/health` | 200 OK |
| Swagger UI | `http://<host>:8080/swagger-ui` | 200 OK |
| Frontend | `http://<host>:3000` | 200 OK |

### Portainer Metrics Dashboard


**추천 설정**:
1. **Left Sidebar → Metrics** 에서 Prometheus 를 활성화합니다.
2. Backend 에서 `/metrics` 엔드포인트를 지원합니다 (만약 구현 안 됨, Micrometer 또는 prometheus Rust crate 추가).

### 로그 관리

```bash
# JSON 포맷 로깅 허용: docker compose logs -f json edgequake-app
docker logs --tail 100 -f $(docker ps -q --filter "name=edgequake-app-backend")
```

---

## 9️⃣ 요약 및 Next Steps


### ✅ 완료된 작업
- [x] Portainer Stack 포맷 변환 전략 분석 (2 옵션)
- [x] 환경 변수 리팩토링 가이드 작성
- [x] Frontend/Backend 간 네트워크 통신 수정 제안
- [x] CI/CD 파이프라인 예제 제공

### 🚀 다음 단계 선택사항

1. **GitOps 방식 자동화**: Portainer + GitHub 연동 (추천)
2. **Custom Postgres 이미지** 빌드: pgvector + AGE extensions 포함
3. **Prometheus/Grafana** 통합: Observability 강화
4. **Backup Strategy**: 볼륨 백업/복구 자동화 스크립트

---


## 🔗 관련 문서

- [Docker Compose vs Portainer](https://docs.portainer.io/user/docker/compose/)
- [Best Practices for Production Deployments](./production-deployment.md)
- [Multi-Architecture Build Guide](./multi-arch-builds.md)

**Last Updated**: 2026-03-31  
**Author**: EdgeQuake Team (CEO & System Architect Role)

