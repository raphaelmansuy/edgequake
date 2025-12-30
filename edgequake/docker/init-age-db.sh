#!/bin/bash
# ============================================================================
# EdgeQuake AGE (Graph) Test Database Initialization Script
# ============================================================================
#
# This script sets up the Apache AGE test database with:
#   - AGE extension for graph database
#   - pgvector extension for vector storage
#   - Application user for RLS testing
#   - All necessary migrations
#
# ============================================================================

set -e

echo "==> Initializing EdgeQuake AGE test database..."

# Enable extensions
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Enable required extensions
    CREATE EXTENSION IF NOT EXISTS age;
    CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
    
    -- Load AGE extension
    LOAD 'age';
    
    -- Set search path to include AGE catalog
    SET search_path = ag_catalog, "\$user", public;
    
    -- Create application user (non-superuser) for RLS testing
    DO \$\$
    BEGIN
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_user') THEN
            CREATE ROLE app_user WITH LOGIN PASSWORD 'app_password_123';
        END IF;
    END \$\$;
    
    -- Grant necessary permissions to app_user
    GRANT CONNECT ON DATABASE $POSTGRES_DB TO app_user;
    GRANT USAGE ON SCHEMA public TO app_user;
    GRANT USAGE ON SCHEMA ag_catalog TO app_user;
    
    -- Enable RLS testing settings
    ALTER DATABASE $POSTGRES_DB SET app.current_tenant_id TO '';
    ALTER DATABASE $POSTGRES_DB SET app.current_workspace_id TO '';
    
    -- Create helper function for getting current tenant
    CREATE OR REPLACE FUNCTION current_tenant_id()
    RETURNS UUID AS \$func\$
    BEGIN
        RETURN NULLIF(current_setting('app.current_tenant_id', true), '')::UUID;
    EXCEPTION WHEN OTHERS THEN
        RETURN NULL;
    END;
    \$func\$ LANGUAGE plpgsql STABLE;
    
    -- Create helper function for getting current workspace
    CREATE OR REPLACE FUNCTION current_workspace_id()
    RETURNS UUID AS \$func\$
    BEGIN
        RETURN NULLIF(current_setting('app.current_workspace_id', true), '')::UUID;
    EXCEPTION WHEN OTHERS THEN
        RETURN NULL;
    END;
    \$func\$ LANGUAGE plpgsql STABLE;
    
    \echo 'AGE test database extensions and functions created successfully!'
EOSQL

# Run migration files in order
echo "==> Running migrations..."
for file in /docker-entrypoint-initdb.d/migrations/*.sql; do
    if [ -f "$file" ]; then
        echo "Applying migration: $(basename "$file")"
        psql -v ON_ERROR_STOP=0 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" -f "$file" || true
    fi
done

# Grant permissions to app_user on all tables
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    -- Grant permissions on all existing tables
    GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_user;
    GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO app_user;
    
    -- Grant AGE specific permissions
    GRANT SELECT ON ALL TABLES IN SCHEMA ag_catalog TO app_user;
    
    -- Set default privileges for future tables
    ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_user;
    ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO app_user;
    
    \echo 'Permissions granted to app_user!'
EOSQL

echo "==> AGE test database initialization complete!"
