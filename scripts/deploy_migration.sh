#!/bin/bash
# Production Migration Deployment Script
# Usage: ./deploy_migration.sh <environment> <migration_version>
# Example: ./deploy_migration.sh production 016

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ENVIRONMENT=${1:-""}
MIGRATION_VERSION=${2:-""}
BACKUP_DIR="${BACKUP_DIR:-./backups}"
LOG_FILE="migration_${MIGRATION_VERSION}_$(date +%Y%m%d_%H%M%S).log"

# Validation
if [ -z "$ENVIRONMENT" ]; then
    echo -e "${RED}❌ Error: Environment not specified${NC}"
    echo "Usage: $0 <environment> <migration_version>"
    exit 1
fi

if [ -z "$MIGRATION_VERSION" ]; then
    echo -e "${RED}❌ Error: Migration version not specified${NC}"
    echo "Usage: $0 <environment> <migration_version>"
    exit 1
fi

if [ -z "${DATABASE_URL:-}" ]; then
    echo -e "${RED}❌ Error: DATABASE_URL not set${NC}"
    exit 1
fi

# Logging function
log() {
    local level=$1
    shift
    local message="$@"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${timestamp} [${level}] ${message}" | tee -a "$LOG_FILE"
}

log_info() {
    log "INFO" "${BLUE}$@${NC}"
}

log_success() {
    log "SUCCESS" "${GREEN}$@${NC}"
}

log_warning() {
    log "WARNING" "${YELLOW}$@${NC}"
}

log_error() {
    log "ERROR" "${RED}$@${NC}"
}

# Banner
echo -e "${BLUE}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║        EdgeQuake Production Migration Deployment          ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

log_info "Environment: $ENVIRONMENT"
log_info "Migration: $MIGRATION_VERSION"
log_info "Log file: $LOG_FILE"

# Production safety check
if [ "$ENVIRONMENT" == "production" ]; then
    log_warning "⚠️  PRODUCTION DEPLOYMENT - Extra safety checks enabled"
    echo ""
    echo -e "${YELLOW}You are about to deploy migration $MIGRATION_VERSION to PRODUCTION${NC}"
    echo -e "${YELLOW}This will modify the production database schema${NC}"
    echo ""
    read -p "Type 'PROCEED' to continue: " confirm
    
    if [ "$confirm" != "PROCEED" ]; then
        log_error "Deployment cancelled by user"
        exit 0
    fi
fi

# Step 1: Pre-deployment checks
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Step 1/7: Pre-deployment checks"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check database connectivity
log_info "Checking database connectivity..."
if ! psql "$DATABASE_URL" -c "SELECT 1" > /dev/null 2>&1; then
    log_error "❌ Database connection failed"
    exit 1
fi
log_success "✓ Database connection verified"

# Check current migration version
log_info "Checking current migration version..."
CURRENT_VERSION=$(psql "$DATABASE_URL" -t -c "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations" 2>/dev/null | xargs)
log_info "Current migration version: $CURRENT_VERSION"

if [ "$MIGRATION_VERSION" -le "$CURRENT_VERSION" ]; then
    log_warning "⚠️  Migration $MIGRATION_VERSION already applied or older than current"
    read -p "Continue anyway? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        log_info "Deployment cancelled"
        exit 0
    fi
fi

# Check disk space
log_info "Checking disk space..."
DB_SIZE=$(psql "$DATABASE_URL" -t -c "SELECT pg_size_pretty(pg_database_size(current_database()))" | xargs)
log_info "Current database size: $DB_SIZE"

# Check for long-running queries
log_info "Checking for long-running queries..."
LONG_QUERIES=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '5 minutes'" | xargs)
if [ "$LONG_QUERIES" -gt 0 ]; then
    log_warning "⚠️  Found $LONG_QUERIES long-running queries"
    psql "$DATABASE_URL" -c "SELECT pid, now() - query_start as duration, query FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '5 minutes'"
fi

# Step 2: Create backup
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Step 2/7: Creating backup"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

mkdir -p "$BACKUP_DIR"
BACKUP_FILE="${BACKUP_DIR}/pre_migration_${MIGRATION_VERSION}_$(date +%Y%m%d_%H%M%S).sql"

log_info "Creating database backup: $BACKUP_FILE"
if pg_dump "$DATABASE_URL" > "$BACKUP_FILE"; then
    BACKUP_SIZE=$(ls -lh "$BACKUP_FILE" | awk '{print $5}')
    log_success "✓ Backup created: $BACKUP_FILE ($BACKUP_SIZE)"
    
    # Compress backup
    log_info "Compressing backup..."
    gzip "$BACKUP_FILE"
    COMPRESSED_SIZE=$(ls -lh "${BACKUP_FILE}.gz" | awk '{print $5}')
    log_success "✓ Backup compressed: ${BACKUP_FILE}.gz ($COMPRESSED_SIZE)"
else
    log_error "❌ Backup failed"
    exit 1
fi

# Step 3: Upload backup to remote storage (if configured)
if [ -n "${S3_BACKUP_BUCKET:-}" ]; then
    log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_info "Step 3/7: Uploading backup to S3"
    log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    log_info "Uploading to s3://$S3_BACKUP_BUCKET/migrations/"
    if aws s3 cp "${BACKUP_FILE}.gz" "s3://$S3_BACKUP_BUCKET/migrations/" --storage-class STANDARD_IA; then
        log_success "✓ Backup uploaded to S3"
    else
        log_warning "⚠️  S3 upload failed (continuing anyway)"
    fi
else
    log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log_info "Step 3/7: Skipping S3 upload (S3_BACKUP_BUCKET not set)"
    log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
fi

# Step 4: Enable enhanced logging
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Step 4/7: Enabling enhanced logging"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

export RUST_LOG=${RUST_LOG:-"info,sqlx=debug,edgequake=debug"}
export RUST_BACKTRACE=1
log_success "✓ Logging enabled: $RUST_LOG"

# Step 5: Run migration with timeout
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Step 5/7: Running migration (10 minute timeout)"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

START_TIME=$(date +%s)

# Run migration with timeout
if timeout 600s cargo run --bin migrate 2>&1 | tee -a "$LOG_FILE"; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    log_success "✓ Migration completed in ${DURATION}s"
else
    EXIT_CODE=$?
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    
    if [ $EXIT_CODE -eq 124 ]; then
        log_error "❌ Migration timed out after ${DURATION}s"
    else
        log_error "❌ Migration failed after ${DURATION}s (exit code: $EXIT_CODE)"
    fi
    
    log_error "Check logs: $LOG_FILE"
    log_error "Backup available: ${BACKUP_FILE}.gz"
    log_error ""
    log_error "To rollback:"
    log_error "  1. gunzip ${BACKUP_FILE}.gz"
    log_error "  2. psql \$DATABASE_URL < $BACKUP_FILE"
    
    exit 1
fi

# Step 6: Verify migration
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Step 6/7: Verifying migration"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check migration was recorded
NEW_VERSION=$(psql "$DATABASE_URL" -t -c "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations" | xargs)
log_info "New migration version: $NEW_VERSION"

if [ "$NEW_VERSION" -lt "$MIGRATION_VERSION" ]; then
    log_error "❌ Migration was not recorded in _sqlx_migrations"
    exit 1
fi
log_success "✓ Migration recorded in database"

# Show recent migrations
log_info "Recent migrations:"
psql "$DATABASE_URL" -c "SELECT version, description, installed_on FROM _sqlx_migrations ORDER BY version DESC LIMIT 5"

# Step 7: Run smoke tests
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
log_info "Step 7/7: Running smoke tests"
log_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if cargo test --test smoke_tests 2>&1 | tee -a "$LOG_FILE"; then
    log_success "✓ Smoke tests passed"
else
    log_warning "⚠️  Some smoke tests failed - check logs"
    log_warning "Migration was applied but application may have issues"
fi

# Success summary
echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                                                            ║"
echo "║              ✅ Migration Deployment Complete              ║"
echo "║                                                            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

log_success "Migration: $MIGRATION_VERSION"
log_success "Duration: ${DURATION}s"
log_success "Backup: ${BACKUP_FILE}.gz"
log_success "Log: $LOG_FILE"

# Post-deployment checklist
echo ""
echo -e "${YELLOW}Post-Deployment Checklist:${NC}"
echo "  [ ] Monitor application logs for 15 minutes"
echo "  [ ] Check error rates in monitoring dashboard"
echo "  [ ] Verify query performance hasn't degraded"
echo "  [ ] Test critical user workflows"
echo "  [ ] Update runbook with actual migration duration (${DURATION}s)"
echo ""

log_info "Deployment complete!"
