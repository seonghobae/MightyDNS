-- Materialized view for fast tenant config lookup
CREATE MATERIALIZED VIEW mv_tenant_config_lookup AS
SELECT
    ta.tenant_id,
    ta.tenant_identifier,
    ta.email_address,
    ta.subscription_tier,
    ta.subscription_status,
    tc.config_id,
    tc.config_name,
    tc.is_logging_enabled,
    tc.is_dnssec_enabled,
    tc.blocked_response_ip,
    array_agg(DISTINCT cbc.block_category) FILTER (WHERE cbc.is_category_enabled = TRUE) AS enabled_categories
FROM tenant_account ta
JOIN tenant_config tc ON ta.tenant_id = tc.tenant_id
LEFT JOIN config_block_category cbc ON tc.config_id = cbc.config_id
WHERE ta.is_account_active = TRUE
  AND tc.is_config_active = TRUE
GROUP BY ta.tenant_id, ta.tenant_identifier, ta.email_address, ta.subscription_tier,
         ta.subscription_status, tc.config_id, tc.config_name, tc.is_logging_enabled,
         tc.is_dnssec_enabled, tc.blocked_response_ip;

-- Unique index for fast lookups
CREATE UNIQUE INDEX idx_mv_tenant_config_lookup_identifier
    ON mv_tenant_config_lookup (tenant_identifier, config_id);

CREATE INDEX idx_mv_tenant_config_lookup_tenant
    ON mv_tenant_config_lookup (tenant_id);
