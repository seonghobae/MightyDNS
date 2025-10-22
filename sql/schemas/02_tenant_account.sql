-- Tenant account table with hash partitioning
CREATE TABLE tenant_account (
    tenant_id uuid_identifier PRIMARY KEY,
    email_address email_address UNIQUE NOT NULL,
    tenant_identifier VARCHAR(32) UNIQUE NOT NULL,
    subscription_tier subscription_tier_enum DEFAULT 'free' NOT NULL,
    subscription_status subscription_status_enum DEFAULT 'active' NOT NULL,
    account_created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    account_updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    is_account_active BOOLEAN DEFAULT TRUE NOT NULL,

    CONSTRAINT chk_tenant_identifier_format CHECK (tenant_identifier ~* '^[a-z0-9]{8,32}$')
) PARTITION BY HASH (tenant_id);

-- Create 16 partitions initially (can expand dynamically)
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE tenant_account_%s PARTITION OF tenant_account FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

-- Indexes
CREATE INDEX idx_tenant_account_email ON tenant_account (email_address);
CREATE INDEX idx_tenant_account_identifier ON tenant_account (tenant_identifier);
CREATE INDEX idx_tenant_account_subscription ON tenant_account (subscription_tier, subscription_status)
    WHERE is_account_active = TRUE;

-- Trigger function to update account_updated_at
CREATE OR REPLACE FUNCTION update_timestamp_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.account_updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_tenant_account_updated
    BEFORE UPDATE ON tenant_account
    FOR EACH ROW
    EXECUTE FUNCTION update_timestamp_column();
