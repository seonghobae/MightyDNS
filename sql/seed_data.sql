-- Seed data for development and testing

-- Insert default blocklist sources
INSERT INTO block_list_source (source_name, source_url, source_format, source_category, source_description, source_update_frequency_hours) VALUES
('OISD Big', 'https://big.oisd.nl/domainswild', 'domains', 'advertising', 'OISD comprehensive blocklist - ads, tracking, malware', 6),
('AdGuard DNS', 'https://adguardteam.github.io/AdGuardSDNSFilter/Filters/filter.txt', 'adblock', 'advertising', 'AdGuard DNS filter', 12),
('StevenBlack Unified', 'https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts', 'hosts', 'advertising', 'Unified hosts file - ads and malware', 24),
('URLhaus Malware', 'https://urlhaus.abuse.ch/downloads/hostfile/', 'hosts', 'malware_phishing', 'Malware domains from URLhaus', 6),
('Phishing Army', 'https://phishing.army/download/phishing_army_blocklist_extended.txt', 'domains', 'malware_phishing', 'Phishing domains', 12),
('Gambling Blocklist', 'https://raw.githubusercontent.com/olbat/ut1-blacklists/master/blacklists/gambling/domains', 'domains', 'gambling_betting', 'Gambling and betting sites', 168),
('Adult Content Filter', 'https://raw.githubusercontent.com/olbat/ut1-blacklists/master/blacklists/adult/domains', 'domains', 'adult_content', 'Adult content sites', 168);

-- Insert sample blocked domains (in production, these would be fetched from sources)
INSERT INTO block_list_entry (block_domain_name, source_id, block_category, block_reason, is_block_active) VALUES
-- Advertising
('ads.example.com', (SELECT source_id FROM block_list_source WHERE source_name = 'OISD Big' LIMIT 1), 'advertising', 'Ad server', TRUE),
('doubleclick.net', (SELECT source_id FROM block_list_source WHERE source_name = 'AdGuard DNS' LIMIT 1), 'advertising', 'Google ad network', TRUE),
('adservice.google.com', (SELECT source_id FROM block_list_source WHERE source_name = 'AdGuard DNS' LIMIT 1), 'advertising', 'Google ad service', TRUE),
('*.ad-provider.com', (SELECT source_id FROM block_list_source WHERE source_name = 'OISD Big' LIMIT 1), 'advertising', 'Wildcard ad provider', TRUE),

-- Tracking
('analytics.google.com', (SELECT source_id FROM block_list_source WHERE source_name = 'AdGuard DNS' LIMIT 1), 'tracking_telemetry', 'Google Analytics', TRUE),
('facebook.com', (SELECT source_id FROM block_list_source WHERE source_name = 'OISD Big' LIMIT 1), 'tracking_telemetry', 'Facebook tracking pixel', TRUE),
('track.example.com', (SELECT source_id FROM block_list_source WHERE source_name = 'OISD Big' LIMIT 1), 'tracking_telemetry', 'Tracking domain', TRUE),

-- Malware/Phishing
('malware-example.com', (SELECT source_id FROM block_list_source WHERE source_name = 'URLhaus Malware' LIMIT 1), 'malware_phishing', 'Known malware distributor', TRUE),
('phishing-site.com', (SELECT source_id FROM block_list_source WHERE source_name = 'Phishing Army' LIMIT 1), 'malware_phishing', 'Phishing attempt', TRUE),

-- Social Media (for productivity/parental control)
('facebook.com', NULL, 'social_media', 'Social media platform', TRUE),
('twitter.com', NULL, 'social_media', 'Social media platform', TRUE),
('instagram.com', NULL, 'social_media', 'Social media platform', TRUE),
('tiktok.com', NULL, 'social_media', 'Social media platform', TRUE),
('reddit.com', NULL, 'social_media', 'Social media platform', TRUE)
ON CONFLICT (block_domain_name) DO NOTHING;

-- Insert test tenant account
INSERT INTO tenant_account (tenant_id, email_address, tenant_identifier, subscription_tier, subscription_status)
VALUES
    ('00000000-0000-0000-0000-000000000001', 'test@example.com', 'test123456', 'free', 'active'),
    ('00000000-0000-0000-0000-000000000002', 'pro@example.com', 'pro123456', 'pro', 'active')
ON CONFLICT (tenant_id) DO NOTHING;

-- Insert test tenant subscription
INSERT INTO tenant_subscription (tenant_id, subscription_tier, subscription_status, monthly_query_limit, config_limit)
VALUES
    ('00000000-0000-0000-0000-000000000001', 'free', 'active', 300000, 1),
    ('00000000-0000-0000-0000-000000000002', 'pro', 'active', NULL, 10)
ON CONFLICT (subscription_id) DO NOTHING;

-- Insert test tenant config
INSERT INTO tenant_config (config_id, tenant_id, config_name, config_description, is_logging_enabled)
VALUES
    ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', 'Default Config', 'Default configuration for test user', TRUE),
    ('10000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000002', 'Default Config', 'Default configuration for pro user', TRUE),
    ('10000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000002', 'Work Config', 'Permissive config for work', FALSE)
ON CONFLICT (config_id) DO NOTHING;

-- Insert test block categories for configs
INSERT INTO config_block_category (config_id, block_category, is_category_enabled) VALUES
-- Free user: ads + malware only
('10000000-0000-0000-0000-000000000001', 'advertising', TRUE),
('10000000-0000-0000-0000-000000000001', 'malware_phishing', TRUE),

-- Pro user default: ads + malware + tracking
('10000000-0000-0000-0000-000000000002', 'advertising', TRUE),
('10000000-0000-0000-0000-000000000002', 'malware_phishing', TRUE),
('10000000-0000-0000-0000-000000000002', 'tracking_telemetry', TRUE),

-- Pro user work config: malware only
('10000000-0000-0000-0000-000000000003', 'malware_phishing', TRUE)
ON CONFLICT (config_id, block_category) DO NOTHING;

-- Insert test whitelist entry
INSERT INTO white_list_entry (tenant_id, white_domain_name, white_reason)
VALUES
    ('00000000-0000-0000-0000-000000000001', 'ads.trusted-site.com', 'Required for service to work'),
    ('00000000-0000-0000-0000-000000000002', 'analytics.work-tool.com', 'CRM analytics endpoint')
ON CONFLICT (tenant_id, white_domain_name) DO NOTHING;

-- Refresh materialized view
REFRESH MATERIALIZED VIEW mv_tenant_config_lookup;

-- Display summary
SELECT 'Seed data inserted successfully!' AS status;
SELECT COUNT(*) AS block_list_sources FROM block_list_source;
SELECT COUNT(*) AS block_list_entries FROM block_list_entry;
SELECT COUNT(*) AS tenant_accounts FROM tenant_account;
SELECT COUNT(*) AS tenant_configs FROM tenant_config;
SELECT COUNT(*) AS whitelist_entries FROM white_list_entry;
