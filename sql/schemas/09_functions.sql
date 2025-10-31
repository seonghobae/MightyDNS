-- Function: Check domain against blocklist and whitelist
CREATE OR REPLACE FUNCTION check_domain_blocklist(
    p_domain dns_domain_name,
    p_tenant_id UUID,
    p_config_id UUID
)
RETURNS TABLE (
    is_blocked BOOLEAN,
    is_whitelisted BOOLEAN,
    block_category block_category_enum,
    block_reason VARCHAR
) AS $$
DECLARE
    v_is_whitelisted BOOLEAN;
    v_is_blocked BOOLEAN;
    v_block_category block_category_enum;
    v_block_reason VARCHAR;
BEGIN
    -- Check whitelist first (highest priority)
    SELECT EXISTS (
        SELECT 1 FROM white_list_entry
        WHERE tenant_id = p_tenant_id
          AND white_domain_name = p_domain
          AND is_white_active = TRUE
    ) INTO v_is_whitelisted;

    IF v_is_whitelisted THEN
        RETURN QUERY SELECT FALSE, TRUE, NULL::block_category_enum, 'Whitelisted by user'::VARCHAR;
        RETURN;
    END IF;

    -- Check global blocklist
    SELECT
        TRUE,
        ble.block_category,
        ble.block_reason
    INTO v_is_blocked, v_block_category, v_block_reason
    FROM block_list_entry ble
    WHERE ble.block_domain_name = p_domain
      AND ble.is_block_active = TRUE
    LIMIT 1;

    IF v_is_blocked THEN
        -- Check if this category is enabled for the tenant config
        IF EXISTS (
            SELECT 1 FROM config_block_category
            WHERE config_id = p_config_id
              AND block_category = v_block_category
              AND is_category_enabled = TRUE
        ) THEN
            RETURN QUERY SELECT TRUE, FALSE, v_block_category, v_block_reason;
            RETURN;
        END IF;
    END IF;

    -- Check custom block domains
    IF EXISTS (
        SELECT 1 FROM config_custom_domain
        WHERE config_id = p_config_id
          AND custom_domain_name = p_domain
          AND custom_domain_action = 'block'
          AND is_custom_active = TRUE
    ) THEN
        RETURN QUERY SELECT TRUE, FALSE, 'custom_user'::block_category_enum, 'Custom user block'::VARCHAR;
        RETURN;
    END IF;

    -- Check wildcard blocking (e.g., *.ads.example.com)
    IF EXISTS (
        SELECT 1 FROM block_list_entry ble
        WHERE p_domain LIKE '%' || REPLACE(ble.block_domain_name, '*', '')
          AND ble.block_domain_name LIKE '*.%'
          AND ble.is_block_active = TRUE
        LIMIT 1
    ) THEN
        RETURN QUERY SELECT TRUE, FALSE, 'advertising'::block_category_enum, 'Wildcard block'::VARCHAR;
        RETURN;
    END IF;

    -- Not blocked
    RETURN QUERY SELECT FALSE, FALSE, NULL::block_category_enum, NULL::VARCHAR;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function: Generate random tenant identifier
CREATE OR REPLACE FUNCTION generate_tenant_identifier()
RETURNS VARCHAR(32) AS $$
DECLARE
    chars TEXT := 'abcdefghijklmnopqrstuvwxyz0123456789';
    result TEXT := '';
    i INTEGER;
BEGIN
    FOR i IN 1..16 LOOP
        result := result || substr(chars, floor(random() * length(chars) + 1)::INTEGER, 1);
    END LOOP;
    RETURN result;
END;
$$ LANGUAGE plpgsql VOLATILE;
