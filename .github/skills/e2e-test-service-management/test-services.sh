#!/bin/bash

# ============================================================================
# EdgeQuake E2E Testing Service Manager
# ============================================================================
#
# Utility script to manage services for E2E testing workflows.
# Complements Makefile with additional debugging and monitoring features.
#
# Usage:
#   ./test-services.sh start         # Start all services
#   ./test-services.sh stop          # Stop all services
#   ./test-services.sh status        # Show service status
#   ./test-services.sh restart       # Restart all services
#   ./test-services.sh logs          # Monitor all logs
#   ./test-services.sh wait-ready    # Wait for all services to be ready
#   ./test-services.sh health-check  # Run detailed health checks
#
# ============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
FRONTEND_PORT=3000
BACKEND_PORT=8080
DB_PORT=5432
DB_NAME=edgequake
DB_USER=edgequake

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

log_error() {
    echo -e "${RED}✗ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Check if port is in use
port_in_use() {
    lsof -Pi :$1 -sTCP:LISTEN -t > /dev/null 2>&1
}

# Wait for service to be ready with timeout
wait_for_service() {
    local name=$1
    local url=$2
    local timeout=${3:-30}
    local elapsed=0
    
    echo -n "⏳ Waiting for $name..."
    
    while [ $elapsed -lt $timeout ]; do
        if curl -s "$url" > /dev/null 2>&1; then
            echo -e "\r${GREEN}✓ $name ready${NC}"
            return 0
        fi
        echo -n "."
        sleep 1
        ((elapsed++))
    done
    
    echo -e "\r${RED}✗ $name timeout after ${timeout}s${NC}"
    return 1
}

# Main commands

start_services() {
    log_info "Starting all services..."
    
    # Check if already running
    if port_in_use $FRONTEND_PORT; then
        log_warning "Frontend already running on port $FRONTEND_PORT"
    fi
    if port_in_use $BACKEND_PORT; then
        log_warning "Backend already running on port $BACKEND_PORT"
    fi
    if port_in_use $DB_PORT; then
        log_warning "Database already running on port $DB_PORT"
    fi
    
    # Start using Make
    log_info "Executing: make dev"
    make dev
    
    log_success "Services started"
}

stop_services() {
    log_info "Stopping all services..."
    make stop
    log_success "Services stopped"
}

restart_services() {
    log_info "Restarting all services..."
    stop_services
    sleep 2
    start_services
}

check_status() {
    log_info "Checking service status..."
    
    local frontend_status="${RED}✗ DOWN${NC}"
    local backend_status="${RED}✗ DOWN${NC}"
    local db_status="${RED}✗ DOWN${NC}"
    
    if curl -s http://localhost:$FRONTEND_PORT > /dev/null 2>&1; then
        frontend_status="${GREEN}✓ UP${NC}"
    fi
    
    if curl -s http://localhost:$BACKEND_PORT/api/v1/health > /dev/null 2>&1; then
        backend_status="${GREEN}✓ UP${NC}"
    fi
    
    if psql -h localhost -U $DB_USER -d $DB_NAME -c "SELECT 1" > /dev/null 2>&1; then
        db_status="${GREEN}✓ UP${NC}"
    fi
    
    echo ""
    echo -e "Frontend (port $FRONTEND_PORT):  $frontend_status"
    echo -e "Backend (port $BACKEND_PORT):   $backend_status"
    echo -e "Database (port $DB_PORT):       $db_status"
    echo ""
}

wait_ready() {
    log_info "Waiting for all services to be ready..."
    
    local all_ready=true
    
    wait_for_service "Frontend" "http://localhost:$FRONTEND_PORT" 60 || all_ready=false
    wait_for_service "Backend" "http://localhost:$BACKEND_PORT/api/v1/health" 60 || all_ready=false
    
    # Database wait
    local elapsed=0
    while [ $elapsed -lt 60 ]; do
        if psql -h localhost -U $DB_USER -d $DB_NAME -c "SELECT 1" > /dev/null 2>&1; then
            log_success "Database ready"
            break
        fi
        echo -n "."
        sleep 1
        ((elapsed++))
    done
    
    if [ "$all_ready" = true ]; then
        log_success "All services ready for testing"
        return 0
    else
        log_error "Some services failed to start"
        return 1
    fi
}

health_check() {
    log_info "Running detailed health checks..."
    
    echo ""
    
    # Frontend health
    echo -n "Frontend (http://localhost:$FRONTEND_PORT): "
    if response=$(curl -s -w "\n%{http_code}" http://localhost:$FRONTEND_PORT); then
        http_code=$(echo "$response" | tail -n1)
        if [ "$http_code" -eq 200 ]; then
            log_success "HTTP $http_code"
        else
            log_warning "HTTP $http_code"
        fi
    else
        log_error "No response"
    fi
    
    # Backend health
    echo -n "Backend (http://localhost:$BACKEND_PORT/api/v1/health): "
    if response=$(curl -s http://localhost:$BACKEND_PORT/api/v1/health); then
        if echo "$response" | grep -q "healthy\|ok"; then
            log_success "$response"
        else
            log_warning "Response: $response"
        fi
    else
        log_error "No response"
    fi
    
    # Database health
    echo -n "Database (localhost:$DB_PORT): "
    if version=$(psql -h localhost -U $DB_USER -d $DB_NAME -c "SELECT version()" 2>/dev/null); then
        log_success "PostgreSQL responsive"
        # Check extensions
        echo "  - Checking pgvector..."
        if psql -h localhost -U $DB_USER -d $DB_NAME -c "CREATE EXTENSION IF NOT EXISTS vector" 2>/dev/null; then
            log_success "    pgvector available"
        else
            log_warning "    pgvector not available"
        fi
    else
        log_error "No response"
    fi
    
    echo ""
}

monitor_logs() {
    log_info "Monitoring logs (Ctrl+C to stop)..."
    log_warning "This opens multiple processes. Use separate terminals instead."
    echo ""
    echo "Suggested separate terminals:"
    echo "  Terminal 1: make backend-logs"
    echo "  Terminal 2: make db-logs"
    echo "  Terminal 3: make frontend-logs"
    echo ""
    
    # Alternative: show last logs
    log_info "Showing recent logs..."
    
    if [ -f edgequake/target/debug/logs ]; then
        echo "Backend logs:"
        tail -10 edgequake/target/debug/logs
        echo ""
    fi
    
    echo "Docker logs:"
    docker logs --tail=10 edgequake-postgres 2>/dev/null || echo "  (database not running)"
}

# Main script logic
case "${1:-help}" in
    start)
        start_services
        ;;
    stop)
        stop_services
        ;;
    restart)
        restart_services
        ;;
    status)
        check_status
        ;;
    wait-ready)
        wait_ready
        ;;
    health-check)
        health_check
        ;;
    logs)
        monitor_logs
        ;;
    *)
        cat << EOF
${BLUE}EdgeQuake E2E Testing Service Manager${NC}

Usage: $0 <command>

Commands:
  ${GREEN}start${NC}           Start all services (frontend, backend, database)
  ${GREEN}stop${NC}            Stop all services
  ${GREEN}restart${NC}         Stop and restart all services
  ${GREEN}status${NC}          Show service status
  ${GREEN}wait-ready${NC}      Wait for all services to be ready
  ${GREEN}health-check${NC}    Run detailed health checks
  ${GREEN}logs${NC}            Monitor service logs

Examples:
  $0 start              # Start all services
  $0 status             # Check if services are running
  $0 wait-ready         # Wait for ready state before tests
  $0 health-check       # Verify all services are healthy

Environment variables:
  FRONTEND_PORT         Frontend port (default: 3000)
  BACKEND_PORT          Backend port (default: 8080)
  DB_PORT               Database port (default: 5432)
  DB_NAME               Database name (default: edgequake)
  DB_USER               Database user (default: edgequake)

EOF
        ;;
esac
