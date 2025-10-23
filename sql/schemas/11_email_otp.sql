-- Email OTP table for passwordless authentication
-- Stores temporary OTP codes sent via email

CREATE TABLE IF NOT EXISTS email_otp (
    tenant_id UUID NOT NULL REFERENCES tenant_account(tenant_id) ON DELETE CASCADE,
    otp_code VARCHAR(6) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    PRIMARY KEY (tenant_id)
);

-- Index for cleanup of expired OTPs
CREATE INDEX IF NOT EXISTS idx_email_otp_expires_at
    ON email_otp (expires_at);

-- Comment
COMMENT ON TABLE email_otp IS 'Stores temporary OTP codes for email-based passwordless authentication';
COMMENT ON COLUMN email_otp.tenant_id IS 'Tenant who requested the OTP';
COMMENT ON COLUMN email_otp.otp_code IS '6-digit OTP code';
COMMENT ON COLUMN email_otp.expires_at IS 'When this OTP expires (typically 10 minutes)';
