-- Tenant configuration table
CREATE TABLE tenant_config (
    config_id uuid_identifier PRIMARY KEY,
    tenant_id UUID NOT NULL,
    config_name VARCHAR(100) NOT NULL,
    config_description TEXT,
    is_logging_enabled BOOLEAN DEFAULT TRUE NOT NULL,
    is_dnssec_enabled BOOLEAN DEFAULT TRUE NOT NULL,
    blocked_response_ip INET DEFAULT '0.0.0.0'::inet,
    config_created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    config_updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    is_config_active BOOLEAN DEFAULT TRUE NOT NULL,

    CONSTRAINT fk_tenant_config_tenant
        FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE,
    CONSTRAINT uq_tenant_config_name
        UNIQUE (tenant_id, config_name)
) PARTITION BY HASH (tenant_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE tenant_config_%s PARTITION OF tenant_config FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

-- Indexes
CREATE INDEX idx_tenant_config_tenant ON tenant_config (tenant_id);
CREATE INDEX idx_tenant_config_active ON tenant_config (tenant_id, is_config_active);

-- Config block category table (3NF normalization)
CREATE TABLE config_block_category (
    config_category_id uuid_identifier PRIMARY KEY,
    config_id UUID NOT NULL,
    block_category block_category_enum NOT NULL,
    is_category_enabled BOOLEAN DEFAULT TRUE NOT NULL,
    category_updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,

    CONSTRAINT fk_config_block_category_config
        FOREIGN KEY (config_id) REFERENCES tenant_config(config_id) ON DELETE CASCADE,
    CONSTRAINT uq_config_category
        UNIQUE (config_id, block_category)
) PARTITION BY HASH (config_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE config_block_category_%s PARTITION OF config_block_category FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

CREATE INDEX idx_config_block_category_config ON config_block_category (config_id);
CREATE INDEX idx_config_block_category_enabled ON config_block_category (config_id, is_category_enabled);

-- Config custom domain table (3NF normalization)
CREATE TABLE config_custom_domain (
    custom_domain_id uuid_identifier PRIMARY KEY,
    config_id UUID NOT NULL,
    custom_domain_name dns_domain_name NOT NULL,
    custom_domain_action VARCHAR(20) DEFAULT 'block' CHECK (custom_domain_action IN ('block', 'allow')),
    custom_domain_added_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    is_custom_active BOOLEAN DEFAULT TRUE NOT NULL,

    CONSTRAINT fk_config_custom_domain_config
        FOREIGN KEY (config_id) REFERENCES tenant_config(config_id) ON DELETE CASCADE,
    CONSTRAINT uq_config_custom_domain
        UNIQUE (config_id, custom_domain_name)
) PARTITION BY HASH (config_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE config_custom_domain_%s PARTITION OF config_custom_domain FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

CREATE INDEX idx_config_custom_domain_config ON config_custom_domain (config_id);
CREATE INDEX idx_config_custom_domain_name ON config_custom_domain (custom_domain_name);
