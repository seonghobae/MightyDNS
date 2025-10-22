-- Enable required PostgreSQL extensions
-- Run this first as superuser

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm"; -- For fuzzy text matching
CREATE EXTENSION IF NOT EXISTS "timescaledb"; -- For time-series data
CREATE EXTENSION IF NOT EXISTS "pg_cron"; -- For scheduled jobs

-- Grant permissions for pg_cron
GRANT USAGE ON SCHEMA cron TO mightydns_user;
