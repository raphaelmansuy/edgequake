#!/bin/bash
# EdgeQuake Docker Setup Verification Script
# This script verifies that the docker-up command is properly configured

set -e

RESET='\033[0m'
BOLD='\033[1m'
BLUE='\033[34m'
GREEN='\033[32m'
YELLOW='\033[33m'
RED='\033[31m'

echo ""
echo -e "${BOLD}${BLUE}EdgeQuake Docker Setup Verification${RESET}"
echo "======================================"
echo ""

# Check 1: Docker is installed
echo -e "${YELLOW}✓ Checking Docker installation...${RESET}"
if command -v docker &> /dev/null; then
    DOCKER_VERSION=$(docker --version)
    echo -e "${GREEN}✓ Docker found: $DOCKER_VERSION${RESET}"
else
    echo -e "${RED}✗ Docker not found. Please install Docker.${RESET}"
    exit 1
fi

# Check 2: Docker Compose is available
echo -e "${YELLOW}✓ Checking Docker Compose...${RESET}"
if docker compose version &> /dev/null; then
    echo -e "${GREEN}✓ Docker Compose available${RESET}"
else
    echo -e "${RED}✗ Docker Compose not available${RESET}"
    exit 1
fi

# Check 3: Docker daemon is running
echo -e "${YELLOW}✓ Checking Docker daemon...${RESET}"
if docker ps &>/dev/null; then
    echo -e "${GREEN}✓ Docker daemon is running${RESET}"
else
    echo -e "${RED}✗ Docker daemon is not running. Please start Docker.${RESET}"
    exit 1
fi

# Check 4: Dockerfiles exist
echo -e "${YELLOW}✓ Checking Dockerfiles...${RESET}"
BACKEND_DOCKERFILE="edgequake/docker/Dockerfile"
FRONTEND_DOCKERFILE="edgequake_webui/Dockerfile"
POSTGRES_DOCKERFILE="edgequake/docker/Dockerfile.postgres.pg18"
POSTGRES_DOCKERFILE_PG16="edgequake/docker/Dockerfile.postgres"
POSTGRES_DOCKERFILE_PG17="edgequake/docker/Dockerfile.postgres.pg17"

if [ -f "$BACKEND_DOCKERFILE" ]; then
    echo -e "${GREEN}✓ Backend Dockerfile found${RESET}"
else
    echo -e "${RED}✗ Backend Dockerfile not found at $BACKEND_DOCKERFILE${RESET}"
    exit 1
fi

if [ -f "$FRONTEND_DOCKERFILE" ]; then
    echo -e "${GREEN}✓ Frontend Dockerfile found${RESET}"
else
    echo -e "${RED}✗ Frontend Dockerfile not found at $FRONTEND_DOCKERFILE${RESET}"
    exit 1
fi

if [ -f "$POSTGRES_DOCKERFILE" ]; then
    echo -e "${GREEN}✓ PostgreSQL Dockerfile (PG18 default) found${RESET}"
else
    echo -e "${RED}✗ PostgreSQL Dockerfile not found at $POSTGRES_DOCKERFILE${RESET}"
    exit 1
fi

for _tier in "$POSTGRES_DOCKERFILE_PG16" "$POSTGRES_DOCKERFILE_PG17"; do
    if [ -f "$_tier" ]; then
        echo -e "${GREEN}✓ PostgreSQL tier Dockerfile found: $_tier${RESET}"
    else
        echo -e "${RED}✗ Missing $_tier${RESET}"
        exit 1
    fi
done

# Check 5: docker-compose.yml is valid
echo -e "${YELLOW}✓ Checking docker-compose configuration...${RESET}"
COMPOSE_FILE="edgequake/docker/docker-compose.yml"
if docker compose -f "$COMPOSE_FILE" config --quiet &>/dev/null; then
    echo -e "${GREEN}✓ docker-compose.yml is valid${RESET}"
else
    echo -e "${RED}✗ docker-compose.yml has errors${RESET}"
    docker compose -f "$COMPOSE_FILE" config 2>&1 | head -20
    exit 1
fi

# Check 6: Frontend package.json and lock file
echo -e "${YELLOW}✓ Checking frontend dependencies...${RESET}"
if [ -f "edgequake_webui/package.json" ]; then
    echo -e "${GREEN}✓ package.json found${RESET}"
else
    echo -e "${RED}✗ package.json not found${RESET}"
    exit 1
fi

if [ -f "edgequake_webui/pnpm-lock.yaml" ]; then
    echo -e "${GREEN}✓ pnpm-lock.yaml found${RESET}"
else
    echo -e "${RED}✗ pnpm-lock.yaml not found${RESET}"
    exit 1
fi

# Check 7: Required ports are available
echo -e "${YELLOW}✓ Checking port availability...${RESET}"
for port in 3000 8080 5432; do
    if lsof -i ":$port" &>/dev/null; then
        echo -e "${YELLOW}⚠ Port $port is in use. This will conflict with docker-up.${RESET}"
    else
        echo -e "${GREEN}✓ Port $port is available${RESET}"
    fi
done

# Check 8: Makefile has docker-up target
echo -e "${YELLOW}✓ Checking Makefile...${RESET}"
if grep -q "^docker-up:" Makefile; then
    echo -e "${GREEN}✓ docker-up target found in Makefile${RESET}"
else
    echo -e "${RED}✗ docker-up target not found in Makefile${RESET}"
    exit 1
fi

# Check 9: docker-compose.yml has all required services
echo -e "${YELLOW}✓ Checking docker-compose services...${RESET}"
SERVICES=("edgequake" "frontend" "postgres")
for service in "${SERVICES[@]}"; do
    if docker compose -f "$COMPOSE_FILE" config | grep -q "^services:" -A 100 | grep -q "$service"; then
        echo -e "${GREEN}✓ Service '$service' found in docker-compose.yml${RESET}"
    fi
done

echo ""
echo -e "${BOLD}${GREEN}✅ All checks passed!${RESET}"
echo ""
echo "You can now run:"
echo -e "  ${BOLD}make docker-up${RESET}"
echo ""
echo "This will:"
echo "  1. Build Docker images (first time only)"
echo "  2. Start PostgreSQL database on port 5432"
echo "  3. Start Backend API on port 8080"
echo "  4. Start Frontend on port 3000"
echo ""
echo "Access points:"
echo -e "  Frontend: ${BOLD}http://localhost:3000${RESET}"
echo -e "  Backend:  ${BOLD}http://localhost:8080${RESET}"
echo -e "  Swagger:  ${BOLD}http://localhost:8080/swagger-ui${RESET}"
echo ""
