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

-- Índices de Alta Performance para Autenticação e Busca
CREATE INDEX IF NOT EXISTS idx_accounts_username_lower ON accounts(LOWER(username));
CREATE INDEX IF NOT EXISTS idx_accounts_email ON accounts(email);

-- SEEDS: Contas Padrão Iniciais (Admin Master e Jogador Teste)
-- admin / admin (GM 32) -> Hash MD5("adminadmin") = 21232f297a57a5a743894a0e4a801fc3
-- testuser / 123456 (Player) -> Hash MD5("testuser123456") = 9cf0ea4cb360b37651a24d86b71f9cf7
INSERT INTO accounts (username, password_hash, email, gold_balance, gm_privileges)
VALUES 
('admin', '21232f297a57a5a743894a0e4a801fc3', 'admin@pwserver.local', 1000000, 32),
('testuser', '9cf0ea4cb360b37651a24d86b71f9cf7', 'test@pwserver.local', 50000, 0)
ON CONFLICT (username) DO NOTHING;

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
('realm_148', 'Perfect World Tides / Genesis (1.4.8)', '1.4.8', '127.0.0.1', 29002, 3000, '{"enabled_classes": [0,1,2,3,4,5,6,7,8,9], "max_level": 105, "meridians": true, "reincarnation": true}'::jsonb),
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
    cultivation INT DEFAULT 0 NOT NULL, -- Nível de Cultivo Espiritual (0 a 32)
    exp BIGINT DEFAULT 0 NOT NULL,
    sp BIGINT DEFAULT 0 NOT NULL,
    hp INT DEFAULT 100 NOT NULL,
    mp INT DEFAULT 100 NOT NULL,
    money BIGINT DEFAULT 0 NOT NULL,
    
    -- Localização 3D no Mundo
    world_id INT DEFAULT 1 NOT NULL,    -- 1 = Mapa Principal (world)
    pos_x REAL DEFAULT 550.0 NOT NULL,  -- Coordenada padrão de spawn (Cidade do Dragão)
    pos_y REAL DEFAULT 200.0 NOT NULL,
    pos_z REAL DEFAULT 650.0 NOT NULL,
    
    -- Atributos Básicos de Distribuição
    strength INT DEFAULT 10 NOT NULL,
    agility INT DEFAULT 10 NOT NULL,
    vitality INT DEFAULT 10 NOT NULL,
    energy INT DEFAULT 10 NOT NULL,
    potential_points INT DEFAULT 0 NOT NULL,
    
    -- Dados Visuais e Customização Facial/Corporal (Octets de Aparência)
    custom_data BYTEA,
    
    -- Controle de Exclusão / Ban
    is_deleted BOOLEAN DEFAULT FALSE NOT NULL,
    deleted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    
    -- Garante unicidade de nome dentro do mesmo Realm (mas permite em Realms diferentes)
    CONSTRAINT uq_character_name_per_realm UNIQUE(realm_id, name)
);

-- Índices de Alta Performance para Personagens
CREATE INDEX IF NOT EXISTS idx_characters_account ON characters(account_id);
CREATE INDEX IF NOT EXISTS idx_characters_realm ON characters(realm_id);
CREATE INDEX IF NOT EXISTS idx_characters_realm_name ON characters(realm_id, LOWER(name));
CREATE INDEX IF NOT EXISTS idx_characters_world ON characters(world_id);

-- -----------------------------------------------------------------------------
-- 4. TABELA SEPARADA DE ITENS (Inventário, Equipamento, Banco / Armazém)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS character_items (
    id BIGSERIAL PRIMARY KEY,
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    container_type SMALLINT NOT NULL, -- 0: Inventário, 1: Equipados, 2: Banco (Storehouse), 3: Caixa de Moda
    slot SMALLINT NOT NULL,           -- Índice da posição no contêiner (0..127)
    
    item_id INT NOT NULL,             -- ID do item no elements.data
    count INT DEFAULT 1 NOT NULL,     -- Quantidade empilhada do item
    durability INT DEFAULT 100 NOT NULL,
    max_durability INT DEFAULT 100 NOT NULL,
    
    -- Atributos de Forja, Refino e Pedras Espirituais
    refine_level SMALLINT DEFAULT 0 NOT NULL, -- Nível de Refino (+0 a +12)
    sockets_count SMALLINT DEFAULT 0 NOT NULL, -- Quantidade de Slots de Pedras (0 a 4)
    socket_stones INT[] DEFAULT '{}' NOT NULL, -- Array de IDs das pedras inseridas
    
    -- Dados de Fabricação, Vínculo e Expiração
    creator_name VARCHAR(32),
    bind_status INT DEFAULT 0 NOT NULL,
    expire_time TIMESTAMP WITH TIME ZONE,
    
    -- Payload Binário de Propriedades Customizadas Adicionais (Octets do Item)
    extra_data BYTEA,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    
    -- Restrição de Unicidade: Garante que nunca existam dois itens no mesmo slot do mesmo contêiner
    CONSTRAINT uq_item_slot_per_container UNIQUE(character_id, container_type, slot)
);

-- Índices de Alta Performance para Itens
CREATE INDEX IF NOT EXISTS idx_items_char_container ON character_items(character_id, container_type);
CREATE INDEX IF NOT EXISTS idx_items_template_id ON character_items(item_id);
CREATE INDEX IF NOT EXISTS idx_items_char_slot ON character_items(character_id, container_type, slot);

-- -----------------------------------------------------------------------------
-- 5. TABELA SEPARADA DE HABILIDADES (Skills Aprendidas)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS character_skills (
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    skill_id INT NOT NULL,
    level SMALLINT DEFAULT 1 NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (character_id, skill_id)
);

-- Índices de Busca para Skills
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

-- Índices de Busca para Quests
CREATE INDEX IF NOT EXISTS idx_quests_char_status ON character_quests(character_id, status);
CREATE INDEX IF NOT EXISTS idx_quests_quest_id ON character_quests(quest_id);

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

-- Índices de Busca para Facções
CREATE INDEX IF NOT EXISTS idx_factions_realm ON factions(realm_id);
CREATE INDEX IF NOT EXISTS idx_factions_master ON factions(master_character_id);

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

-- Índices de Busca para Correio
CREATE INDEX IF NOT EXISTS idx_mails_receiver ON mails(receiver_id);
CREATE INDEX IF NOT EXISTS idx_mails_sender ON mails(sender_id);
CREATE INDEX IF NOT EXISTS idx_mails_unread ON mails(receiver_id, is_read);

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

-- Índices de Busca para Auditoria
CREATE INDEX IF NOT EXISTS idx_admin_logs_action ON admin_audit_logs(action_type);
CREATE INDEX IF NOT EXISTS idx_admin_logs_created_at ON admin_audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_logs_target_acc ON admin_audit_logs(target_account_id);
