-- Migration: Fix white_list_entry schema mismatch
-- Rename columns from white_* to entry_* for consistency
-- Add missing config_id column to link whitelist entries to specific configurations
-- Reason: Rust models expect entry_* naming and config_id is required for multi-config support

BEGIN;

-- Step 1: Add config_id column (initially nullable for migration, will be made NOT NULL later)
ALTER TABLE white_list_entry
    ADD COLUMN IF NOT EXISTS config_id UUID;

-- Step 2: Rename columns to match Rust model naming
-- Note: These renames are metadata-only operations, no data is changed
ALTER TABLE white_list_entry
    RENAME COLUMN white_entry_id TO entry_id;

ALTER TABLE white_list_entry
    RENAME COLUMN white_domain_name TO entry_domain_name;

ALTER TABLE white_list_entry
    RENAME COLUMN white_reason TO entry_added_reason;

ALTER TABLE white_list_entry
    RENAME COLUMN white_added_at TO entry_created_at;

ALTER TABLE white_list_entry
    RENAME COLUMN is_white_active TO is_entry_active;

-- Step 3: Update primary key constraint name for consistency
-- Note: The constraint structure remains the same, only the name changes
ALTER TABLE white_list_entry
    DROP CONSTRAINT white_list_entry_pkey,
    ADD CONSTRAINT white_list_entry_pkey PRIMARY KEY (entry_id, tenant_id);

-- Step 4: Update unique constraint
ALTER TABLE white_list_entry
    DROP CONSTRAINT uq_white_domain_per_tenant,
    ADD CONSTRAINT uq_entry_domain_per_tenant UNIQUE (tenant_id, entry_domain_name);

-- Step 5: Update indexes to use new column names
DROP INDEX IF EXISTS idx_white_list_entry_tenant;
DROP INDEX IF EXISTS idx_white_list_entry_domain;

CREATE INDEX idx_white_list_entry_tenant ON white_list_entry (tenant_id);
CREATE INDEX idx_white_list_entry_domain ON white_list_entry (tenant_id, entry_domain_name)
    WHERE is_entry_active = TRUE;

-- Step 6: For existing data, set config_id to the tenant's primary config
-- This assumes each tenant has one config. Adjust if multiple configs exist.
UPDATE white_list_entry wle
SET config_id = (
    SELECT config_id
    FROM tenant_config tc
    WHERE tc.tenant_id = wle.tenant_id
      AND tc.is_config_active = TRUE
    ORDER BY tc.config_created_at ASC
    LIMIT 1
)
WHERE config_id IS NULL;

-- Step 7: Make config_id NOT NULL and add foreign key constraint
-- Only after all existing rows have config_id values
ALTER TABLE white_list_entry
    ALTER COLUMN config_id SET NOT NULL;

ALTER TABLE white_list_entry
    ADD CONSTRAINT fk_white_list_entry_config
        FOREIGN KEY (config_id) REFERENCES tenant_config(config_id) ON DELETE CASCADE;

-- Step 8: Create index for config_id lookups
CREATE INDEX idx_white_list_entry_config ON white_list_entry (config_id);

-- Step 9: Add comments explaining the schema
COMMENT ON COLUMN white_list_entry.entry_id IS
    'Unique identifier for whitelist entry (renamed from white_entry_id for consistency)';

COMMENT ON COLUMN white_list_entry.config_id IS
    'Links whitelist entry to specific tenant configuration (added for multi-config support)';

COMMENT ON COLUMN white_list_entry.entry_domain_name IS
    'Whitelisted domain name (renamed from white_domain_name)';

COMMENT ON COLUMN white_list_entry.entry_added_reason IS
    'Reason for whitelisting (renamed from white_reason)';

COMMENT ON COLUMN white_list_entry.entry_created_at IS
    'Timestamp when entry was created (renamed from white_added_at)';

COMMENT ON COLUMN white_list_entry.is_entry_active IS
    'Whether entry is active (renamed from is_white_active)';

COMMIT;
