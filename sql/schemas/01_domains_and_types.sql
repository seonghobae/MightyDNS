-- Custom domains for validation and reusability
CREATE DOMAIN email_address AS VARCHAR(255)
    CHECK (VALUE ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$');

CREATE DOMAIN dns_domain_name AS VARCHAR(253)
    CHECK (VALUE ~* '^([a-z0-9-]+\.)*[a-z0-9-]+$' OR VALUE = '*' OR VALUE ~* '^\*\.');

CREATE DOMAIN uuid_identifier AS UUID
    DEFAULT gen_random_uuid();

-- Enums for type safety
CREATE TYPE subscription_tier_enum AS ENUM ('free', 'pro', 'family', 'business');

CREATE TYPE subscription_status_enum AS ENUM ('active', 'expired', 'cancelled', 'suspended');

CREATE TYPE credential_type_enum AS ENUM ('fido2', 'totp', 'email_otp');

CREATE TYPE dns_result_type_enum AS ENUM ('allowed', 'blocked', 'whitelisted', 'error');

CREATE TYPE dns_query_type_enum AS ENUM ('A', 'AAAA', 'CNAME', 'MX', 'TXT', 'NS', 'SOA', 'PTR', 'SRV');

CREATE TYPE block_category_enum AS ENUM (
    'advertising',
    'malware_phishing',
    'adult_content',
    'gambling_betting',
    'social_media',
    'streaming_media',
    'tracking_telemetry',
    'cryptomining',
    'piracy_torrents',
    'custom_user'
);
