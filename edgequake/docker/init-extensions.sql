-- EdgeQuake PostgreSQL Extensions Initialization
-- This script only creates required extensions, NOT tables.
-- Tables are created by SQLx migrations in the Rust application.

-- CRITICAL: Set the default search_path for the edgequake user to public ONLY
-- This prevents SQLx from creating _sqlx_migrations in a user-specific schema
ALTER USER edgequake SET search_path TO public;

-- UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Vector similarity search (pgvector)
CREATE EXTENSION IF NOT EXISTS "vector";

-- Apache AGE for graph database (optional)
DO $$ 
BEGIN
    CREATE EXTENSION IF NOT EXISTS "age" CASCADE;
    RAISE NOTICE 'Apache AGE extension enabled successfully';
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Apache AGE extension not available: %. Graph features will use fallback storage.', SQLERRM;
END $$;

-- Log completion
DO $$
BEGIN
    RAISE NOTICE 'EdgeQuake extensions initialized. Tables will be created by SQLx migrations.';
END $$;
