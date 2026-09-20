-- =============================================================================
-- ESPECIFICAÇÃO DE BANCO DE DADOS: PostgreSQL Schema test
-- Projeto: PW-Universal-Server
-- Isolamento de testes de integração e unitários do schema public.
-- =============================================================================

CREATE SCHEMA IF NOT EXISTS test;

SET search_path = test, public;

-- -----------------------------------------------------------------------------
-- 1. TABELA DE CONTAS (Isolada para testes)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.accounts (
    id SERIAL PRIMARY KEY,
    username VARCHAR(64) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    email VARCHAR(128),
    gold_balance BIGINT DEFAULT 0 NOT NULL,
    silver_balance BIGINT DEFAULT 0 NOT NULL,
    gm_privileges INT DEFAULT 0 NOT NULL,
    is_banned BOOLEAN DEFAULT FALSE NOT NULL,
    ban_reason VARCHAR(255),
    ban_expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_login_at TIMESTAMP WITH TIME ZONE,
    last_login_ip VARCHAR(45)
);

CREATE INDEX IF NOT EXISTS idx_test_accounts_username_lower ON test.accounts(LOWER(username));
CREATE INDEX IF NOT EXISTS idx_test_accounts_email ON test.accounts(email);

-- -----------------------------------------------------------------------------
-- 2. TABELA DE REALMS
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.realms (
    id VARCHAR(32) PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    version VARCHAR(16) NOT NULL,
    host VARCHAR(128) NOT NULL,
    port INT NOT NULL,
    is_online BOOLEAN DEFAULT TRUE NOT NULL,
    max_players INT DEFAULT 3000 NOT NULL,
    double_exp_multiplier NUMERIC(3,1) DEFAULT 1.0 NOT NULL,
    double_sp_multiplier NUMERIC(3,1) DEFAULT 1.0 NOT NULL,
    double_drop_multiplier NUMERIC(3,1) DEFAULT 1.0 NOT NULL,
    double_gold_multiplier NUMERIC(3,1) DEFAULT 1.0 NOT NULL,
    config JSONB DEFAULT '{}'::jsonb NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- -----------------------------------------------------------------------------
-- 3. TABELA DE PERSONAGENS
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.characters (
    id SERIAL PRIMARY KEY,
    account_id INT NOT NULL REFERENCES test.accounts(id) ON DELETE CASCADE,
    realm_id VARCHAR(32) NOT NULL REFERENCES test.realms(id) ON DELETE RESTRICT,
    name VARCHAR(32) NOT NULL,
    race INT NOT NULL,
    cls INT NOT NULL,
    gender SMALLINT NOT NULL,
    level INT DEFAULT 1 NOT NULL,
    cultivation INT DEFAULT 0 NOT NULL,
    -- Pontos de teleporte descobertos: u16 LE em sequência (`_waypoint_list` do gs)
    waypoints BYTEA DEFAULT ''::bytea NOT NULL,
    exp BIGINT DEFAULT 0 NOT NULL,
    sp BIGINT DEFAULT 0 NOT NULL,
    hp INT DEFAULT 100 NOT NULL,
    mp INT DEFAULT 100 NOT NULL,
    money BIGINT DEFAULT 0 NOT NULL,
    world_id INT DEFAULT 1 NOT NULL,
    pos_x REAL DEFAULT 550.0 NOT NULL,
    pos_y REAL DEFAULT 200.0 NOT NULL,
    pos_z REAL DEFAULT 650.0 NOT NULL,
    strength INT DEFAULT 10 NOT NULL,
    agility INT DEFAULT 10 NOT NULL,
    vitality INT DEFAULT 10 NOT NULL,
    energy INT DEFAULT 10 NOT NULL,
    potential_points INT DEFAULT 0 NOT NULL,
    custom_data BYTEA,
    is_deleted BOOLEAN DEFAULT FALSE NOT NULL,
    deleted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_login_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT uq_test_character_name_per_realm UNIQUE(realm_id, name)
);

CREATE INDEX IF NOT EXISTS idx_test_characters_account ON test.characters(account_id);
CREATE INDEX IF NOT EXISTS idx_test_characters_realm ON test.characters(realm_id);
CREATE INDEX IF NOT EXISTS idx_test_characters_realm_name ON test.characters(realm_id, LOWER(name));
CREATE INDEX IF NOT EXISTS idx_test_characters_world ON test.characters(world_id);

-- -----------------------------------------------------------------------------
-- 4. ITENS DE PERSONAGEM
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.character_items (
    id BIGSERIAL PRIMARY KEY,
    character_id INT NOT NULL REFERENCES test.characters(id) ON DELETE CASCADE,
    container_type SMALLINT NOT NULL,
    slot SMALLINT NOT NULL,
    item_id INT NOT NULL,
    count INT DEFAULT 1 NOT NULL,
    durability INT DEFAULT 100 NOT NULL,
    max_durability INT DEFAULT 100 NOT NULL,
    refine_level SMALLINT DEFAULT 0 NOT NULL,
    sockets_count SMALLINT DEFAULT 0 NOT NULL,
    socket_stones INT[] DEFAULT '{}' NOT NULL,
    creator_name VARCHAR(32),
    bind_status INT DEFAULT 0 NOT NULL,
    expire_time TIMESTAMP WITH TIME ZONE,
    extra_data BYTEA,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT uq_test_item_slot_per_container UNIQUE(character_id, container_type, slot)
);

CREATE INDEX IF NOT EXISTS idx_test_items_char_container ON test.character_items(character_id, container_type);
CREATE INDEX IF NOT EXISTS idx_test_items_template_id ON test.character_items(item_id);
CREATE INDEX IF NOT EXISTS idx_test_items_char_slot ON test.character_items(character_id, container_type, slot);

-- -----------------------------------------------------------------------------
-- 5. HABILIDADES
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.character_skills (
    character_id INT NOT NULL REFERENCES test.characters(id) ON DELETE CASCADE,
    skill_id INT NOT NULL,
    level SMALLINT DEFAULT 1 NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (character_id, skill_id)
);

CREATE INDEX IF NOT EXISTS idx_test_skills_character ON test.character_skills(character_id);
CREATE INDEX IF NOT EXISTS idx_test_skills_skill_id ON test.character_skills(skill_id);

-- -----------------------------------------------------------------------------
-- 6. MISSÕES
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.character_quests (
    character_id INT NOT NULL REFERENCES test.characters(id) ON DELETE CASCADE,
    quest_id INT NOT NULL,
    status SMALLINT DEFAULT 0 NOT NULL,
    progress JSONB DEFAULT '{}'::jsonb NOT NULL,
    expire_time TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (character_id, quest_id)
);

CREATE INDEX IF NOT EXISTS idx_test_quests_char_status ON test.character_quests(character_id, status);
CREATE INDEX IF NOT EXISTS idx_test_quests_quest_id ON test.character_quests(quest_id);

-- -----------------------------------------------------------------------------
-- 7. LISTAS DE MISSÕES (Task Lists)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.character_task_lists (
    character_id INT PRIMARY KEY REFERENCES test.characters(id) ON DELETE CASCADE,
    active BYTEA NOT NULL,
    finished BYTEA NOT NULL,
    finish_time BYTEA NOT NULL,
    finish_count BYTEA NOT NULL,
    storage BYTEA NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- -----------------------------------------------------------------------------
-- 8. CONFIGURAÇÕES DE INTERFACE DO CLIENTE
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.character_client_config (
    character_id INT PRIMARY KEY REFERENCES test.characters(id) ON DELETE CASCADE,
    ui_config BYTEA,
    help_states BYTEA,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- -----------------------------------------------------------------------------
-- 9. MOLDES DE CLASSE (class_templates)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.class_templates (
    id SERIAL PRIMARY KEY,
    realm_id VARCHAR(32) NOT NULL REFERENCES test.realms(id) ON DELETE CASCADE,
    cls INT NOT NULL,
    name VARCHAR(32) NOT NULL,
    initial_level INT DEFAULT 1 NOT NULL,
    initial_cultivation INT DEFAULT 0 NOT NULL,
    initial_money BIGINT DEFAULT 0 NOT NULL,
    initial_sp BIGINT DEFAULT 0 NOT NULL,
    strength INT NOT NULL,
    agility INT NOT NULL,
    vitality INT NOT NULL,
    energy INT NOT NULL,
    spawn_world_id INT NOT NULL,
    spawn_x REAL NOT NULL,
    spawn_y REAL NOT NULL,
    spawn_z REAL NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    ui_config BYTEA,
    CONSTRAINT uq_test_class_template_per_realm UNIQUE (realm_id, cls)
);

CREATE INDEX IF NOT EXISTS idx_test_class_templates_realm ON test.class_templates(realm_id);

-- -----------------------------------------------------------------------------
-- 10. ITENS INICIAIS DO MOLDE
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.class_template_items (
    id SERIAL PRIMARY KEY,
    template_id INT NOT NULL REFERENCES test.class_templates(id) ON DELETE CASCADE,
    container_type SMALLINT NOT NULL,
    slot SMALLINT NOT NULL,
    item_id INT NOT NULL,
    count INT DEFAULT 1 NOT NULL,
    durability INT DEFAULT 0 NOT NULL,
    max_durability INT DEFAULT 0 NOT NULL,
    refine_level SMALLINT DEFAULT 0 NOT NULL,
    sockets_count SMALLINT DEFAULT 0 NOT NULL,
    socket_stones INT[] DEFAULT '{}' NOT NULL,
    CONSTRAINT uq_test_class_template_item_slot UNIQUE (template_id, container_type, slot)
);

CREATE INDEX IF NOT EXISTS idx_test_template_items_tpl ON test.class_template_items(template_id);

-- -----------------------------------------------------------------------------
-- 11. HABILIDADES INICIAIS DO MOLDE
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.class_template_skills (
    template_id INT NOT NULL REFERENCES test.class_templates(id) ON DELETE CASCADE,
    skill_id INT NOT NULL,
    level SMALLINT DEFAULT 1 NOT NULL,
    PRIMARY KEY (template_id, skill_id)
);

CREATE INDEX IF NOT EXISTS idx_test_template_skills_tpl ON test.class_template_skills(template_id);

-- -----------------------------------------------------------------------------
-- 12. FACÇÕES
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.factions (
    id SERIAL PRIMARY KEY,
    realm_id VARCHAR(32) NOT NULL REFERENCES test.realms(id) ON DELETE RESTRICT,
    name VARCHAR(32) NOT NULL,
    level SMALLINT DEFAULT 1 NOT NULL,
    master_character_id INT NOT NULL REFERENCES test.characters(id) ON DELETE RESTRICT,
    announcement VARCHAR(255),
    members JSONB DEFAULT '[]'::jsonb NOT NULL,
    fortress JSONB DEFAULT '{}'::jsonb NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT uq_test_faction_name_per_realm UNIQUE(realm_id, name)
);

CREATE INDEX IF NOT EXISTS idx_test_factions_realm ON test.factions(realm_id);
CREATE INDEX IF NOT EXISTS idx_test_factions_master ON test.factions(master_character_id);

-- -----------------------------------------------------------------------------
-- 13. CORREIO (Mails)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.mails (
    id BIGSERIAL PRIMARY KEY,
    realm_id VARCHAR(32) NOT NULL REFERENCES test.realms(id) ON DELETE CASCADE,
    sender_id INT REFERENCES test.characters(id) ON DELETE SET NULL,
    receiver_id INT NOT NULL REFERENCES test.characters(id) ON DELETE CASCADE,
    title VARCHAR(64) NOT NULL,
    message TEXT,
    attached_money BIGINT DEFAULT 0 NOT NULL,
    attached_item JSONB,
    is_read BOOLEAN DEFAULT FALSE NOT NULL,
    is_collected BOOLEAN DEFAULT FALSE NOT NULL,
    sent_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_test_mails_receiver ON test.mails(receiver_id);
CREATE INDEX IF NOT EXISTS idx_test_mails_sender ON test.mails(sender_id);
CREATE INDEX IF NOT EXISTS idx_test_mails_unread ON test.mails(receiver_id, is_read);

-- -----------------------------------------------------------------------------
-- 14. AUDITORIA ADMIN
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS test.admin_audit_logs (
    id SERIAL PRIMARY KEY,
    admin_account_id INT REFERENCES test.accounts(id) ON DELETE SET NULL,
    action_type VARCHAR(64) NOT NULL,
    target_account_id INT REFERENCES test.accounts(id) ON DELETE SET NULL,
    target_character_id INT REFERENCES test.characters(id) ON DELETE SET NULL,
    realm_id VARCHAR(32),
    details JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- -----------------------------------------------------------------------------
-- SEMENTES BÁSICAS (Copiadas a partir do schema public existente)
-- -----------------------------------------------------------------------------
INSERT INTO test.accounts SELECT * FROM public.accounts ON CONFLICT (username) DO NOTHING;
INSERT INTO test.realms SELECT * FROM public.realms ON CONFLICT (id) DO NOTHING;
INSERT INTO test.class_templates SELECT * FROM public.class_templates ON CONFLICT (realm_id, cls) DO NOTHING;
INSERT INTO test.class_template_items SELECT * FROM public.class_template_items ON CONFLICT (template_id, container_type, slot) DO NOTHING;
INSERT INTO test.class_template_skills SELECT * FROM public.class_template_skills ON CONFLICT (template_id, skill_id) DO NOTHING;

-- Sincronização das sequências
SELECT setval('test.accounts_id_seq', COALESCE((SELECT MAX(id) FROM test.accounts), 1));
SELECT setval('test.characters_id_seq', COALESCE((SELECT MAX(id) FROM test.characters), 1));
SELECT setval('test.class_templates_id_seq', COALESCE((SELECT MAX(id) FROM test.class_templates), 1));
SELECT setval('test.class_template_items_id_seq', COALESCE((SELECT MAX(id) FROM test.class_template_items), 1));
