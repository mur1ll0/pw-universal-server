-- =============================================================================
-- ESPECIFICAÇÃO DE BANCO DE DADOS: PostgreSQL 16 Multi-Realm (Normalizado)
-- Projeto: PW-Universal-Server
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- -----------------------------------------------------------------------------
-- 1. TABELA DE CONTAS GLOBAIS (Compartilhada entre todos os Realms)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS accounts (
    id SERIAL PRIMARY KEY,
    username VARCHAR(64) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    email VARCHAR(128),
    gold_balance BIGINT DEFAULT 0 NOT NULL,
    silver_balance BIGINT DEFAULT 0 NOT NULL,
    gm_privileges INT DEFAULT 0 NOT NULL, -- 0: Normal, 1..32: Níveis de GM
    is_banned BOOLEAN DEFAULT FALSE NOT NULL,
    ban_reason VARCHAR(255),
    ban_expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_login_at TIMESTAMP WITH TIME ZONE,
    last_login_ip VARCHAR(45)
);

CREATE INDEX IF NOT EXISTS idx_accounts_username_lower ON accounts(LOWER(username));
CREATE INDEX IF NOT EXISTS idx_accounts_email ON accounts(email);

-- -----------------------------------------------------------------------------
-- 2. TABELA DE REALMS / SERVIDORES CONCORRENTES
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS realms (
    id VARCHAR(32) PRIMARY KEY, -- Ex: 'realm_126', 'realm_153'
    name VARCHAR(64) NOT NULL,
    version VARCHAR(16) NOT NULL, -- Ex: '1.2.6', '1.5.3'
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

INSERT INTO realms (id, name, version, host, port, max_players, config)
VALUES 
('realm_126', 'Perfect World Classic (1.2.6)', '1.2.6', '127.0.0.1', 29000, 3000, '{"enabled_classes": [0,1,2,3,4,5], "max_level": 105}'::jsonb),
('realm_153', 'Perfect World Eclipse (1.5.3)', '1.5.3', '127.0.0.1', 29001, 3000, '{"enabled_classes": [0,1,2,3,4,5,6,7,8,9,10,11], "max_level": 105, "meridians": true, "reincarnation": true}'::jsonb)
ON CONFLICT (id) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 3. TABELA DE PERSONAGENS (Metadados e Estado Core)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS characters (
    id SERIAL PRIMARY KEY,
    account_id INT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    realm_id VARCHAR(32) NOT NULL REFERENCES realms(id) ON DELETE RESTRICT,
    name VARCHAR(32) NOT NULL,
    race INT NOT NULL,     -- 0: Humano, 1: Alado, 2: Selvagem, 3: Abissal, 4: Guardião, 5: Sombrio
    cls INT NOT NULL,      -- 0..11 (Guerreiro, Mago, Bárbaro, Feiticeira, Arqueiro, Sacerdote, etc.)
    gender SMALLINT NOT NULL, -- 0: Masculino, 1: Feminino
    level INT DEFAULT 1 NOT NULL,
    cultivation INT DEFAULT 0 NOT NULL, -- Nível de Cultivo
    exp BIGINT DEFAULT 0 NOT NULL,
    sp BIGINT DEFAULT 0 NOT NULL,
    hp INT DEFAULT 100 NOT NULL,
    mp INT DEFAULT 100 NOT NULL,
    money BIGINT DEFAULT 0 NOT NULL,
    reputation INT DEFAULT 0 NOT NULL,
    world_id INT DEFAULT 1 NOT NULL,   -- ID do mapa (1 = World)
    pos_x NUMERIC(10,3) DEFAULT 550.0 NOT NULL,
    pos_y NUMERIC(10,3) DEFAULT 200.0 NOT NULL,
    pos_z NUMERIC(10,3) DEFAULT 650.0 NOT NULL,
    
    -- Capacidade de slots
    inventory_size SMALLINT DEFAULT 32 NOT NULL,
    storehouse_size SMALLINT DEFAULT 16 NOT NULL,
    
    is_deleted BOOLEAN DEFAULT FALSE NOT NULL,
    delete_time TIMESTAMP WITH TIME ZONE,
    
    custom_appearance JSONB DEFAULT '{}'::jsonb NOT NULL, -- Customização facial e estética
    version_data JSONB DEFAULT '{}'::jsonb NOT NULL,      -- Dados extras por versão (Meridianos, Rebirth)
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_login_at TIMESTAMP WITH TIME ZONE,
    
    CONSTRAINT uq_character_name_per_realm UNIQUE(realm_id, name)
);

CREATE INDEX IF NOT EXISTS idx_characters_account ON characters(account_id);
CREATE INDEX IF NOT EXISTS idx_characters_realm ON characters(realm_id);
CREATE INDEX IF NOT EXISTS idx_characters_name_lower ON characters(realm_id, LOWER(name));
CREATE INDEX IF NOT EXISTS idx_characters_level ON characters(realm_id, level DESC);
CREATE INDEX IF NOT EXISTS idx_characters_money ON characters(realm_id, money DESC);

-- -----------------------------------------------------------------------------
-- 4. TABELA SEPARADA DE ITENS (Inventário, Equipamento, Armazém, Fashion, etc.)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS character_items (
    id BIGSERIAL PRIMARY KEY,
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    container_type VARCHAR(16) NOT NULL, -- 'INVENTORY', 'EQUIPMENT', 'STOREHOUSE', 'FASHION', 'PET_CORRAL'
    slot INT NOT NULL,                   -- Índice do slot no container (0..63)
    item_id INT NOT NULL,                -- ID do item no elements.data
    count INT DEFAULT 1 NOT NULL,
    max_count INT DEFAULT 100 NOT NULL,
    refine_level SMALLINT DEFAULT 0 NOT NULL, -- Refino (+0 a +12)
    sockets_count SMALLINT DEFAULT 0 NOT NULL,-- Quantidade de furos/slots (0..4)
    sockets INT[] DEFAULT '{}' NOT NULL,      -- IDs das pedras espirituais incrustadas
    durability INT DEFAULT 1000 NOT NULL,
    max_durability INT DEFAULT 1000 NOT NULL,
    bind_status SMALLINT DEFAULT 0 NOT NULL,  -- 0: Solto, 1: Preso à alma
    custom_attributes JSONB DEFAULT '{}'::jsonb NOT NULL, -- Propriedades adicionais, nome do criador
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    
    CONSTRAINT uq_character_container_slot UNIQUE(character_id, container_type, slot)
);

-- Índices otimizados para busca e manipulação de itens
CREATE INDEX IF NOT EXISTS idx_items_char_container ON character_items(character_id, container_type);
CREATE INDEX IF NOT EXISTS idx_items_lookup ON character_items(character_id, container_type, slot);
CREATE INDEX IF NOT EXISTS idx_items_item_id ON character_items(item_id);
CREATE INDEX IF NOT EXISTS idx_items_refine ON character_items(refine_level) WHERE refine_level > 0;
CREATE INDEX IF NOT EXISTS idx_items_custom_attrs_gin ON character_items USING gin(custom_attributes);

-- -----------------------------------------------------------------------------
-- 5. TABELA SEPARADA DE HABILIDADES (Skills Aprendidas)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS character_skills (
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    skill_id INT NOT NULL,
    level SMALLINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (character_id, skill_id)
);

CREATE INDEX IF NOT EXISTS idx_skills_character ON character_skills(character_id);
CREATE INDEX IF NOT EXISTS idx_skills_skill_id ON character_skills(skill_id);

-- -----------------------------------------------------------------------------
-- 6. TABELA SEPARADA DE MISSÕES (Quests Ativas e Concluídas)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS character_quests (
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    quest_id INT NOT NULL,
    status VARCHAR(16) NOT NULL, -- 'ACTIVE', 'COMPLETED'
    progress INT[] DEFAULT '{}' NOT NULL, -- Contadores de monstros/itens da missão
    expire_time TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (character_id, quest_id)
);

CREATE INDEX IF NOT EXISTS idx_quests_char_status ON character_quests(character_id, status);

-- -----------------------------------------------------------------------------
-- 7. TABELA DE FACÇÕES / CLÃS (Por Realm)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS factions (
    id SERIAL PRIMARY KEY,
    realm_id VARCHAR(32) NOT NULL REFERENCES realms(id) ON DELETE RESTRICT,
    name VARCHAR(32) NOT NULL,
    level INT DEFAULT 1 NOT NULL,
    master_character_id INT NOT NULL REFERENCES characters(id) ON DELETE RESTRICT,
    announcement TEXT,
    members JSONB DEFAULT '[]'::jsonb NOT NULL,  -- Lista de membros e patentes
    fortress JSONB DEFAULT '{}'::jsonb NOT NULL, -- Base de Clã
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    
    CONSTRAINT uq_faction_name_per_realm UNIQUE(realm_id, name)
);

CREATE INDEX IF NOT EXISTS idx_factions_realm ON factions(realm_id);

-- -----------------------------------------------------------------------------
-- 8. TABELA DE CORREIO IN-GAME (In-Game Mailbox)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mails (
    id SERIAL PRIMARY KEY,
    realm_id VARCHAR(32) NOT NULL REFERENCES realms(id) ON DELETE CASCADE,
    sender_id INT REFERENCES characters(id) ON DELETE SET NULL,
    receiver_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    title VARCHAR(64) NOT NULL,
    message TEXT NOT NULL,
    attached_money BIGINT DEFAULT 0 NOT NULL,
    attached_item JSONB DEFAULT NULL,
    is_read BOOLEAN DEFAULT FALSE NOT NULL,
    is_collected BOOLEAN DEFAULT FALSE NOT NULL,
    sent_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE DEFAULT (CURRENT_TIMESTAMP + INTERVAL '30 days') NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mails_receiver ON mails(receiver_id);

-- -----------------------------------------------------------------------------
-- 9. TABELA DE AUDITORIA E LOGS ADMINISTRATIVOS (pwAdmin / Dashboard)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS admin_audit_logs (
    id SERIAL PRIMARY KEY,
    admin_account_id INT REFERENCES accounts(id) ON DELETE SET NULL,
    action_type VARCHAR(64) NOT NULL, -- 'GRANT_GOLD', 'BAN_USER', 'TOGGLE_MAP', 'EDIT_ITEM'
    target_account_id INT REFERENCES accounts(id) ON DELETE SET NULL,
    target_character_id INT REFERENCES characters(id) ON DELETE SET NULL,
    realm_id VARCHAR(32),
    details JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_admin_logs_action ON admin_audit_logs(action_type);
CREATE INDEX IF NOT EXISTS idx_admin_logs_created_at ON admin_audit_logs(created_at DESC);
