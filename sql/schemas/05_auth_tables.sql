-- Authentication credential table
CREATE TABLE auth_credential (
    credential_id uuid_identifier PRIMARY KEY,
    tenant_id UUID NOT NULL,
    credential_type credential_type_enum NOT NULL,
    credential_public_key BYTEA,
    credential_counter BIGINT DEFAULT 0,
    totp_secret_key VARCHAR(64),
    credential_name VARCHAR(100),
    credential_created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    credential_last_used_at TIMESTAMPTZ,
    is_credential_active BOOLEAN DEFAULT TRUE NOT NULL,

    CONSTRAINT fk_auth_credential_tenant
        FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE,
    CONSTRAINT chk_fido2_required_fields
        CHECK (credential_type != 'fido2' OR (credential_public_key IS NOT NULL AND credential_counter IS NOT NULL)),
    CONSTRAINT chk_totp_required_fields
        CHECK (credential_type != 'totp' OR totp_secret_key IS NOT NULL)
) PARTITION BY HASH (tenant_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE auth_credential_%s PARTITION OF auth_credential FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

CREATE INDEX idx_auth_credential_tenant ON auth_credential (tenant_id);
CREATE INDEX idx_auth_credential_type ON auth_credential (tenant_id, credential_type)
    WHERE is_credential_active = TRUE;

-- Authentication session table
CREATE TABLE auth_session (
    session_id uuid_identifier PRIMARY KEY,
    tenant_id UUID NOT NULL,
    session_token_hash VARCHAR(64) UNIQUE NOT NULL,
    session_ip_address INET NOT NULL,
    session_user_agent TEXT,
    session_created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    session_expires_at TIMESTAMPTZ NOT NULL,
    session_last_activity_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    is_session_active BOOLEAN DEFAULT TRUE NOT NULL,

    CONSTRAINT fk_auth_session_tenant
        FOREIGN KEY (tenant_id) REFERENCES tenant_account(tenant_id) ON DELETE CASCADE,
    CONSTRAINT chk_session_expiry
        CHECK (session_expires_at > session_created_at)
) PARTITION BY HASH (tenant_id);

-- Create partitions
DO $$
DECLARE
    partition_num INTEGER;
BEGIN
    FOR partition_num IN 0..15 LOOP
        EXECUTE format(
            'CREATE TABLE auth_session_%s PARTITION OF auth_session FOR VALUES WITH (MODULUS 16, REMAINDER %s)',
            partition_num, partition_num
        );
    END LOOP;
END $$;

CREATE INDEX idx_auth_session_tenant ON auth_session (tenant_id);
CREATE INDEX idx_auth_session_token ON auth_session (session_token_hash);
CREATE INDEX idx_auth_session_expires ON auth_session (session_expires_at)
    WHERE is_session_active = TRUE;
