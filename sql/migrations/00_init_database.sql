-- MightyDNS Database Initialization Script
-- Run this as PostgreSQL superuser

-- Create database and user
CREATE DATABASE mightydns;
CREATE USER mightydns_user WITH ENCRYPTED PASSWORD 'change-me-in-production';
GRANT ALL PRIVILEGES ON DATABASE mightydns TO mightydns_user;

-- Connect to mightydns database
\c mightydns

-- Enable extensions (requires superuser)
\i ../schemas/00_extensions.sql

-- Grant schema permissions
GRANT ALL ON SCHEMA public TO mightydns_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO mightydns_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO mightydns_user;

-- Now connect as mightydns_user for remaining schema creation
-- \c mightydns mightydns_user

-- Create domains and types
\i ../schemas/01_domains_and_types.sql

-- Create tables
\i ../schemas/02_tenant_account.sql
\i ../schemas/03_tenant_subscription.sql
\i ../schemas/04_tenant_config.sql
\i ../schemas/05_auth_tables.sql
\i ../schemas/06_blocklist_whitelist.sql
\i ../schemas/07_dns_query_log.sql
\i ../schemas/08_ip_binding.sql

-- Create functions
\i ../schemas/09_functions.sql

-- Create materialized views
\i ../schemas/10_materialized_views.sql

-- Setup pg_cron jobs (run as superuser)
-- \c mightydns postgres
-- \i ../cron_jobs/setup_cron.sql
