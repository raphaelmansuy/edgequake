# EdgeQuake → Portainer Stack 배포 가이드

Portainer에서 EdgeQuake를 배포할 때는 `edgequake/docker/portainer-compose.yml`을 기준으로 잡는 것이 가장 안전합니다.

핵심 원칙은 세 가지입니다.

- `container_name`은 쓰지 않습니다. Portainer가 stack 이름을 자동으로 붙이므로, 고정 이름은 충돌을 만들기 쉽습니다.
- `NEXT_PUBLIC_API_URL`은 런타임 env가 아니라 `build.args`로 넘깁니다. Next.js public env는 빌드 시점에 번들에 박히기 때문입니다.
- PostgreSQL은 stack 안에 포함합니다. 외부 DB에 의존하지 않도록 `postgres` 서비스와 volume을 함께 둡니다.

---

## 추천 파일

- `edgequake/docker/portainer-compose.yml`
- `edgequake/docker/docker-compose.yml` 1차 로컬 개발용
- `edgequake/docker/docker-compose-cpu-build.yml` CPU 빌드 변형

Portainer 가이드는 `portainer-compose.yml` 기준으로 읽으면 됩니다.

---

## 환경 변수

`edgequake/docker/.env.portainer.example`을 복사해서 사용합니다.

```env
POSTGRES_USER=edgequake
POSTGRES_PASSWORD=CHANGE_ME
POSTGRES_DB=edgequake

NEXT_PUBLIC_API_URL=http://localhost:11432

OPENAI_API_KEY=
OPENAI_COMPATIBLE_API_KEY=sk-your-compatible-key
OPENAI_BASE_URL=http://host.docker.internal:4000/v1

EDGEQUAKE_LLM_PROVIDER=openai
EDGEQUAKE_LLM_MODEL=qwen3.5-35b-a3b
EDGEQUAKE_EMBEDDING_PROVIDER=openai
EDGEQUAKE_EMBEDDING_MODEL=qwen3-embedding-0.6b
EDGEQUAKE_EMBEDDING_DIMENSION=1024

EDGEQUAKE_DEFAULT_LLM_PROVIDER=litellm-local
EDGEQUAKE_DEFAULT_LLM_MODEL=qwen3.5-35b-a3b
EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=litellm-local
EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=qwen3-embedding-0.6b
EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1024

EDGEQUAKE_VISION_PROVIDER=openai
EDGEQUAKE_VISION_MODEL=qwen3.5-35b-a3b
EDGEQUAKE_MODELS_CONFIG=/app/models.toml

EDGEQUAKE_VISION_TIMEOUT_SECS=600
PDFIUM_AUTO_CACHE_DIR=/tmp/edgequake-pdfium-cache

RUST_LOG=debug,edgequake=debug
RUST_BACKTRACE=1
```

`NEXT_PUBLIC_API_URL`은 브라우저가 실제로 접근할 수 있는 backend URL이어야 합니다.

- 같은 Docker host에서 접속한다면 `http://localhost:11432`
- 다른 머신에서 접속한다면 해당 host의 IP나 도메인으로 바꾸세요

`POSTGRES_PASSWORD`는 예시값이므로 실제 배포 전에 반드시 바꾸세요.

---

## Portainer Compose

`edgequake/docker/portainer-compose.yml`의 권장 형태는 아래와 같습니다.

```yaml
services:
  edgequake:
    build:
      context: ..
      dockerfile: docker/Dockerfile
    image: docker-edgequake:latest
    ports:
      - "11432:8080"
    environment:
      - HOST=0.0.0.0
      - PORT=8080
      - DATABASE_URL=postgresql://${POSTGRES_USER:-edgequake}:${POSTGRES_PASSWORD:-CHANGE_ME}@postgres:5432/${POSTGRES_DB:-edgequake}
      - EDGEQUAKE_MODELS_CONFIG=/app/models.toml
      - OPENAI_API_KEY=${OPENAI_API_KEY:-}
      - OPENAI_COMPATIBLE_API_KEY=${OPENAI_COMPATIBLE_API_KEY:-sk-0000}
      - OPENAI_BASE_URL=${OPENAI_BASE_URL:-http://host.docker.internal:4000/v1}
      - EDGEQUAKE_LLM_PROVIDER=${EDGEQUAKE_LLM_PROVIDER:-openai}
      - EDGEQUAKE_LLM_MODEL=${EDGEQUAKE_LLM_MODEL:-qwen3.5-35b-a3b}
      - EDGEQUAKE_EMBEDDING_PROVIDER=${EDGEQUAKE_EMBEDDING_PROVIDER:-openai}
      - EDGEQUAKE_EMBEDDING_MODEL=${EDGEQUAKE_EMBEDDING_MODEL:-qwen3-embedding-0.6b}
      - EDGEQUAKE_EMBEDDING_DIMENSION=${EDGEQUAKE_EMBEDDING_DIMENSION:-1024}
      - EDGEQUAKE_DEFAULT_LLM_PROVIDER=litellm-local
      - EDGEQUAKE_DEFAULT_LLM_MODEL=qwen3.5-35b-a3b
      - EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=litellm-local
      - EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=qwen3-embedding-0.6b
      - EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1024
      - EDGEQUAKE_VISION_PROVIDER=${EDGEQUAKE_VISION_PROVIDER:-openai}
      - EDGEQUAKE_VISION_MODEL=${EDGEQUAKE_VISION_MODEL:-qwen3.5-35b-a3b}
      - EDGEQUAKE_VISION_TIMEOUT_SECS=${EDGEQUAKE_VISION_TIMEOUT_SECS:-600}
      - PDFIUM_AUTO_CACHE_DIR=${PDFIUM_AUTO_CACHE_DIR:-/tmp/edgequake-pdfium-cache}
      - WORKER_THREADS=2
      - RUST_LOG=debug,edgequake=debug,tower_http=warn,axum=warn
      - RUST_BACKTRACE=1
    depends_on:
      postgres:
        condition: service_healthy
    extra_hosts:
      - "host.docker.internal:host-gateway"
    volumes:
      - type: bind
        source: /root/edgequake/config/models.toml
        target: /app/models.toml
        read_only: true

  frontend:
    build:
      context: ../../
      dockerfile: edgequake_webui/Dockerfile
      args:
        NEXT_PUBLIC_API_URL: ${NEXT_PUBLIC_API_URL:-http://localhost:11432}
    image: docker-frontend:latest
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
    depends_on:
      - edgequake

  postgres:
    build:
      context: .
      dockerfile: Dockerfile.postgres
    image: docker-postgres:latest
    ports:
      - "${POSTGRES_PORT:-5432}:5432"
    environment:
      - POSTGRES_USER=${POSTGRES_USER:-edgequake}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-CHANGE_ME}
      - POSTGRES_DB=${POSTGRES_DB:-edgequake}
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./init-extensions.sql:/docker-entrypoint-initdb.d/init.sql:ro

volumes:
  postgres-data:
    driver: local
```

포인트는 다음입니다.

- frontend는 `build.args.NEXT_PUBLIC_API_URL`로 빌드합니다.
- `NEXT_PUBLIC_API_URL`은 `environment`가 아니라 `build.args`에서 관리합니다.
- postgres는 stack 내부에 두고 `DATABASE_URL`을 그 서비스명(`postgres`)으로 연결합니다.
- `container_name`은 쓰지 않습니다.

---

## Portainer에서 배포하기

### 1. Stack 생성

1. Portainer에 로그인합니다.
2. `Stacks` → `Add stack`을 엽니다.
3. Git repository 또는 file upload 방식을 선택합니다.
4. Compose file path를 `edgequake/docker/portainer-compose.yml`로 지정합니다.
5. `.env.portainer.example`의 값을 Portainer environment variables에 넣습니다.
6. `Create the stack`을 클릭합니다.

### 2. 주의할 점

- backend는 `11432:8080`, frontend는 `3000:3000`으로 공개됩니다.
- 브라우저가 frontend를 열 때 API 호출은 `NEXT_PUBLIC_API_URL`로 빌드된 주소를 사용합니다.
- 다른 머신에서 접속한다면 `NEXT_PUBLIC_API_URL`을 host/IP 기준으로 바꾼 뒤 frontend 이미지를 다시 빌드해야 합니다.

---

## 검증

배포 후 아래를 확인합니다.

```bash
curl http://<host>:11432/health
curl -I http://<host>:3000
```

PostgreSQL은 Portainer UI에서 `postgres` 서비스 로그를 확인하거나, stack의 Console 기능에서 `pg_isready -U edgequake -d edgequake`를 실행합니다.

---

## 자주 하는 실수

### 1. frontend가 잘못된 backend로 붙는 경우

원인:
- `NEXT_PUBLIC_API_URL`을 runtime env로만 넣음
- 또는 host-reachable URL이 아닌 내부 서비스 이름을 넣음

해결:
- `build.args.NEXT_PUBLIC_API_URL`을 수정하고 frontend 이미지를 다시 빌드합니다.

### 2. PostgreSQL이 바로 종료되는 경우

원인:
- `POSTGRES_PASSWORD` 또는 `POSTGRES_DB` 값 누락

해결:
- `.env.portainer.example`의 값을 확인하고, Portainer stack의 env와 compose가 같은 값을 쓰는지 확인합니다.

### 3. Ollama 또는 OpenAI-compatible proxy 연결 실패

원인:
- `OPENAI_BASE_URL` 또는 `OLLAMA_HOST`가 Docker host에서 접근 불가능

해결:
- `host.docker.internal` 경로와 Portainer의 `extra_hosts` 설정을 확인합니다.

---

## 선택 사항: 이미지 기반 배포

build 대신 published image를 쓰고 싶다면, backend/frontend/postgres 이미지를 각각 GHCR에 올린 뒤 compose의 `build:`를 `image:`로 바꾸면 됩니다.

그 경우에도 `NEXT_PUBLIC_API_URL`은 frontend 이미지를 만들 때 반드시 빌드 인자로 넣어야 합니다.
