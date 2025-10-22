-- Tenant IP binding table for UDP/53 tenant identification
CREATE TABLE tenant_ip_binding (
    binding_id uuid_identifier PRIMARY KEY,
    tenant_id UUID NOT NULL,
    config_id UUID NOT NULL,
    ip_address INET UNIQUE NOT NULL,
    ip_description VARCHAR(255),
    binding_created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    binding_expires_at TIMESTAMPTZ,
    is_binding_active BOOLEAN DEFAULT TRUE NOT NULL,

    CONSTRAINT fk_tenant_ip_binding_tenant
        FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE,
    CONSTRAINT fk_tenant_ip_binding_config
        FOREIGN KEY (config_id) REFERENCES tenant_config(config_id) ON DELETE CASCADE
) PARTITION BY HASH (tenant_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE tenant_ip_binding_%s PARTITION OF tenant_ip_binding FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

CREATE INDEX idx_tenant_ip_binding_tenant ON tenant_ip_binding (tenant_id);
CREATE INDEX idx_tenant_ip_binding_ip ON tenant_ip_binding (ip_address) WHERE is_binding_active = TRUE;
CREATE INDEX idx_tenant_ip_binding_expires ON tenant_ip_binding (binding_expires_at)
    WHERE binding_expires_at IS NOT NULL AND is_binding_active = TRUE;
