-- pg_cron scheduled jobs

-- 1. Refresh materialized view for tenant config lookup (every minute)
SELECT cron.schedule(
    'refresh-tenant-config-mv',
    '* * * * *',
    'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_tenant_config_lookup;'
);

-- 2. Cleanup expired sessions (every 15 minutes)
SELECT cron.schedule(
    'cleanup-expired-sessions',
    '*/15 * * * *',
    $$
    DELETE FROM auth_session
    WHERE session_expires_at < NOW()
       OR (is_session_active = FALSE AND session_last_activity_at < NOW() - INTERVAL '7 days');
    $$
);

-- 3. Cleanup expired IP bindings (daily at 2 AM)
SELECT cron.schedule(
    'cleanup-expired-ip-bindings',
    '0 2 * * *',
    $$
    UPDATE tenant_ip_binding
    SET is_binding_active = FALSE
    WHERE binding_expires_at IS NOT NULL
      AND binding_expires_at < NOW()
      AND is_binding_active = TRUE;
    $$
);

-- 4. Update blocklist from external sources (every 6 hours)
-- Note: This requires external script integration (placeholder)
SELECT cron.schedule(
    'update-blocklists',
    '0 */6 * * *',
    $$
    -- This will be implemented via external worker service
    -- For now, just update the next_update_at timestamp
    UPDATE block_list_source
    SET source_next_update_at = NOW() + (source_update_frequency_hours || ' hours')::INTERVAL
    WHERE is_source_active = TRUE
      AND (source_next_update_at IS NULL OR source_next_update_at <= NOW());
    $$
);

-- 5. Cleanup old audit logs (weekly on Sunday at 3 AM)
-- Note: Audit log table will be added in future phase
-- SELECT cron.schedule(
--     'cleanup-old-audit-logs',
--     '0 3 * * 0',
--     $$
--     DELETE FROM audit_log_entry
--     WHERE log_timestamp < NOW() - INTERVAL '90 days';
--     $$
-- );

-- View scheduled jobs
SELECT * FROM cron.job;
