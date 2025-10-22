-- Tenant subscription table
CREATE TABLE tenant_subscription (
    subscription_id uuid_identifier PRIMARY KEY,
    tenant_id UUID NOT NULL,
    lemon_squeezy_subscription_id VARCHAR(100) UNIQUE,
    lemon_squeezy_customer_id VARCHAR(100),
    subscription_tier subscription_tier_enum NOT NULL,
    subscription_status subscription_status_enum DEFAULT 'active' NOT NULL,
    subscription_started_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    subscription_expires_at TIMESTAMPTZ,
    subscription_cancelled_at TIMESTAMPTZ,
    monthly_query_limit BIGINT,
    config_limit INTEGER DEFAULT 1,
    subscription_metadata JSONB,
    is_subscription_active BOOLEAN DEFAULT TRUE NOT NULL,

    CONSTRAINT fk_tenant_subscription_tenant
        FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE,
    CONSTRAINT chk_subscription_dates
        CHECK (subscription_expires_at IS NULL OR subscription_expires_at > subscription_started_at)
) PARTITION BY HASH (tenant_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE tenant_subscription_%s PARTITION OF tenant_subscription FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

-- Indexes
CREATE INDEX idx_tenant_subscription_tenant ON tenant_subscription (tenant_id);
CREATE INDEX idx_tenant_subscription_lemon ON tenant_subscription (lemon_squeezy_subscription_id);
CREATE INDEX idx_tenant_subscription_status ON tenant_subscription (subscription_status, subscription_expires_at)
    WHERE is_subscription_active = TRUE;
