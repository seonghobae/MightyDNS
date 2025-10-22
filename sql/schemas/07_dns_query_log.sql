-- DNS query log table with TimescaleDB hypertable
CREATE TABLE dns_query_log (
    query_id BIGSERIAL,
    tenant_id UUID NOT NULL,
    config_id UUID,
    query_timestamp TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    query_domain_name dns_domain_name NOT NULL,
    query_type dns_query_type_enum NOT NULL,
    query_result_type dns_result_type_enum NOT NULL,
    response_ip_address INET,
    query_latency_ms SMALLINT,
    query_source_protocol VARCHAR(10) CHECK (query_source_protocol IN ('doh', 'dot', 'udp', 'tcp')),
    query_source_ip INET,

    PRIMARY KEY (query_id, query_timestamp)
);

-- Convert to TimescaleDB hypertable (automatic time-based partitioning)
SELECT create_hypertable('dns_query_log', 'query_timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Indexes
CREATE INDEX idx_dns_query_log_tenant_time ON dns_query_log (tenant_id, query_timestamp DESC);
CREATE INDEX idx_dns_query_log_domain ON dns_query_log (query_domain_name, query_timestamp DESC);
CREATE INDEX idx_dns_query_log_result ON dns_query_log (query_result_type, query_timestamp DESC);

-- Retention policy: Delete logs older than 7 days
SELECT add_retention_policy('dns_query_log', INTERVAL '7 days', if_not_exists => TRUE);

-- Continuous aggregate for hourly analytics
CREATE MATERIALIZED VIEW analytics_hourly_summary
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', query_timestamp) AS hour_timestamp,
    tenant_id,
    COUNT(*) AS total_query_count,
    COUNT(*) FILTER (WHERE query_result_type = 'blocked') AS blocked_query_count,
    COUNT(*) FILTER (WHERE query_result_type = 'whitelisted') AS whitelisted_query_count,
    AVG(query_latency_ms)::SMALLINT AS avg_query_latency_ms,
    jsonb_agg(DISTINCT query_domain_name ORDER BY query_domain_name LIMIT 10)
        FILTER (WHERE query_result_type = 'blocked') AS top_blocked_domains,
    jsonb_agg(DISTINCT query_domain_name ORDER BY query_domain_name LIMIT 10)
        FILTER (WHERE query_result_type = 'allowed') AS top_allowed_domains
FROM dns_query_log
GROUP BY hour_timestamp, tenant_id
WITH NO DATA;

-- Refresh policy: Update every 5 minutes
SELECT add_continuous_aggregate_policy('analytics_hourly_summary',
    start_offset => INTERVAL '2 hours',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes',
    if_not_exists => TRUE
);

-- Create unique index for the continuous aggregate
CREATE UNIQUE INDEX idx_analytics_hourly_summary_unique
ON analytics_hourly_summary (hour_timestamp, tenant_id);
