-- Migration: Fix query_latency_ms overflow issue
-- Change SMALLINT (max 32,767ms) to INTEGER (max 2,147,483,647ms)
-- Reason: DNS queries can timeout beyond 32 seconds, causing overflow

BEGIN;

-- Step 1: Drop the continuous aggregate that depends on query_latency_ms
DROP MATERIALIZED VIEW IF EXISTS analytics_hourly_summary CASCADE;

-- Step 2: Alter the column type in dns_query_log
-- Note: This is safe because SMALLINT can be implicitly cast to INTEGER
ALTER TABLE dns_query_log
    ALTER COLUMN query_latency_ms TYPE INTEGER;

-- Step 3: Recreate the continuous aggregate with INTEGER type
CREATE MATERIALIZED VIEW analytics_hourly_summary
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', query_timestamp) AS hour_timestamp,
    tenant_id,
    COUNT(*) AS total_query_count,
    COUNT(*) FILTER (WHERE query_result_type = 'blocked') AS blocked_query_count,
    COUNT(*) FILTER (WHERE query_result_type = 'whitelisted') AS whitelisted_query_count,
    AVG(query_latency_ms)::INTEGER AS avg_query_latency_ms,  -- Changed from SMALLINT to INTEGER
    jsonb_agg(DISTINCT query_domain_name ORDER BY query_domain_name LIMIT 10)
        FILTER (WHERE query_result_type = 'blocked') AS top_blocked_domains,
    jsonb_agg(DISTINCT query_domain_name ORDER BY query_domain_name LIMIT 10)
        FILTER (WHERE query_result_type = 'allowed') AS top_allowed_domains
FROM dns_query_log
GROUP BY hour_timestamp, tenant_id
WITH NO DATA;

-- Step 4: Recreate the refresh policy
SELECT add_continuous_aggregate_policy('analytics_hourly_summary',
    start_offset => INTERVAL '2 hours',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes',
    if_not_exists => TRUE
);

-- Step 5: Recreate the unique index
CREATE UNIQUE INDEX idx_analytics_hourly_summary_unique
ON analytics_hourly_summary (hour_timestamp, tenant_id);

-- Step 6: Add a comment explaining the change
COMMENT ON COLUMN dns_query_log.query_latency_ms IS
    'Query latency in milliseconds. INTEGER type used to prevent overflow (SMALLINT max 32,767ms insufficient for timeout scenarios).';

COMMENT ON COLUMN analytics_hourly_summary.avg_query_latency_ms IS
    'Average query latency in milliseconds. INTEGER type used to match dns_query_log.query_latency_ms and prevent overflow.';

COMMIT;
