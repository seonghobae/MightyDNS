-- Block list source table
CREATE TABLE block_list_source (
    source_id uuid_identifier PRIMARY KEY,
    source_name VARCHAR(100) UNIQUE NOT NULL,
    source_url TEXT NOT NULL,
    source_format VARCHAR(20) DEFAULT 'hosts' CHECK (source_format IN ('hosts', 'domains', 'adblock', 'dnsmasq')),
    source_category block_category_enum NOT NULL,
    source_description TEXT,
    source_last_updated_at TIMESTAMPTZ,
    source_next_update_at TIMESTAMPTZ,
    source_update_frequency_hours INTEGER DEFAULT 6,
    is_source_active BOOLEAN DEFAULT TRUE NOT NULL
);

CREATE INDEX idx_block_list_source_category ON block_list_source (source_category);
CREATE INDEX idx_block_list_source_next_update ON block_list_source (source_next_update_at)
    WHERE is_source_active = TRUE;

-- Block list entry table with hash partitioning
CREATE TABLE block_list_entry (
    block_entry_id BIGSERIAL,
    block_domain_name dns_domain_name NOT NULL,
    source_id UUID,
    block_category block_category_enum NOT NULL,
    block_reason VARCHAR(255),
    block_added_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    block_updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    is_block_active BOOLEAN DEFAULT TRUE NOT NULL,

    PRIMARY KEY (block_entry_id, block_domain_name),
    CONSTRAINT fk_block_list_entry_source
        FOREIGN KEY (source_id) REFERENCES block_list_source(source_id) ON DELETE SET NULL,
    CONSTRAINT uq_block_domain_name
        UNIQUE (block_domain_name)
) PARTITION BY HASH (block_domain_name);

-- Create 64 partitions for blocklist (millions of domains)
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..63 LOOP
        EXECUTE format(
            'CREATE TABLE block_list_entry_%s PARTITION OF block_list_entry FOR VALUES WITH (MODULUS 64, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

CREATE INDEX idx_block_list_entry_domain ON block_list_entry (block_domain_name) WHERE is_block_active = TRUE;
CREATE INDEX idx_block_list_entry_category ON block_list_entry (block_category);
CREATE INDEX idx_block_list_entry_source ON block_list_entry (source_id);

-- GIN index for wildcard/fuzzy matching
CREATE INDEX idx_block_list_entry_domain_gin ON block_list_entry USING gin (block_domain_name gin_trgm_ops);

-- White list entry table (renamed columns for consistency with Rust models)
CREATE TABLE white_list_entry (
    entry_id uuid_identifier,
    tenant_id UUID NOT NULL,
    config_id UUID NOT NULL,  -- Link to specific tenant configuration
    entry_domain_name dns_domain_name NOT NULL,
    entry_added_reason VARCHAR(255),
    entry_created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    is_entry_active BOOLEAN DEFAULT TRUE NOT NULL,

    PRIMARY KEY (entry_id, tenant_id),
    CONSTRAINT fk_white_list_entry_tenant
        FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE,
    CONSTRAINT fk_white_list_entry_config
        FOREIGN KEY (config_id) REFERENCES tenant_config(config_id) ON DELETE CASCADE,
    CONSTRAINT uq_entry_domain_per_tenant
        UNIQUE (tenant_id, entry_domain_name)
) PARTITION BY HASH (tenant_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE white_list_entry_%s PARTITION OF white_list_entry FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

CREATE INDEX idx_white_list_entry_tenant ON white_list_entry (tenant_id);
CREATE INDEX idx_white_list_entry_domain ON white_list_entry (tenant_id, entry_domain_name)
    WHERE is_entry_active = TRUE;
CREATE INDEX idx_white_list_entry_config ON white_list_entry (config_id);
