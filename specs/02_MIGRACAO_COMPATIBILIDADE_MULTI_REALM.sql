-- =============================================================================
-- MIGRAÇÃO: campos, tabelas e restrições que faltavam para o jogo funcionar
-- Projeto: PW-Universal-Server  |  PostgreSQL 16  |  aplica sobre o 01_*.sql
-- =============================================================================
--
-- REGRA DESTE ARQUIVO
--
-- Cada coluna e cada tabela aqui existe porque **alguma coisa concreta já pede
-- por ela hoje**: um campo que o protocolo carrega e que o servidor preenche com
-- zero, um valor que o repositório devolve chumbado no código porque não tem de
-- onde ler, ou uma funcionalidade que responde "ok" ao cliente e joga o dado
-- fora. A evidência está escrita em cada bloco, com arquivo e linha.
--
-- O que **não** está aqui: tudo o que seria palpite. `docs/BANCO_DE_DADOS.md`
-- lista o que ficou de fora e por quê.
--
-- É idempotente: pode rodar duas vezes.
-- =============================================================================


-- -----------------------------------------------------------------------------
-- 1. `characters` — os campos que o `RoleInfo` carrega e nós zeramos
-- -----------------------------------------------------------------------------
--
-- O `RoleInfo` (`CElementClient/Network/rpcdata/roleinfo`) tem 23 campos e vai em
-- três protocolos: `RoleList_Re`, `CreateRole_Re` e `CreateRole`. Destes,
-- `crates/pw-protocol/src/packets/s2c.rs::write_role_info` escreve **sete** com
-- constante, porque não há coluna:
--
--   level2, delete_time, create_time, lastlogin_time,
--   custom_status, charactermode, reincarnation_data, realm_data
--
-- Enquanto forem constantes, o cliente mostra a data de criação errada, não
-- mostra ícone de estado nenhum, e o `UndoDeleteRole` (protocolo que já
-- respondemos) não tem como dizer até quando dá para desfazer.

ALTER TABLE characters
    -- 2º nível do `RoleInfo` (`level2`). Nas versões com renascimento é o nível
    -- da segunda vida; no 1.2.6 fica zero e não atrapalha.
    ADD COLUMN IF NOT EXISTS level2 INT DEFAULT 0 NOT NULL,

    -- `lastlogin_time` do `RoleInfo`. Distinto de `accounts.last_login_at`: uma
    -- conta tem vários personagens e o cliente mostra o de cada um.
    ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMP WITH TIME ZONE,

    -- `delete_time` do `RoleInfo`: o instante em que a exclusão se consuma. O
    -- `is_deleted`/`deleted_at` de hoje diz "foi excluído"; isto diz "vai ser
    -- excluído em", que é o que o `UndoDeleteRole` e a tela de personagens
    -- precisam saber.
    ADD COLUMN IF NOT EXISTS delete_scheduled_at TIMESTAMP WITH TIME ZONE,

    -- Os dois `Octets` de estado visual do `RoleInfo`. Opacos para nós: são
    -- gravados como o cliente mandou e devolvidos como vieram.
    ADD COLUMN IF NOT EXISTS custom_status BYTEA,
    ADD COLUMN IF NOT EXISTS charactermode BYTEA,

    -- Os dois `Octets` que só existem a partir do 1.4.8 (o `write_role_info`
    -- corta os quatro últimos campos no 1.2.6). Ficam nulos num realm 1.2.6.
    ADD COLUMN IF NOT EXISTS reincarnation_data BYTEA,
    ADD COLUMN IF NOT EXISTS realm_data BYTEA;

-- -----------------------------------------------------------------------------
-- 2. `characters` — o que o repositório devolve chumbado por falta de coluna
-- -----------------------------------------------------------------------------
--
-- `crates/pw-storage/src/repositories/character.rs:419-423` monta o
-- `CharacterDetails` com:
--
--     reputation: 0,
--     inventory_size: 64,
--     storehouse_size: 32,
--
-- Os três campos existem no tipo de domínio (`pw-core`), ou seja: o código já
-- sabe que precisa deles. Sem coluna, expandir a bolsa não sobrevive ao logout,
-- e a reputação — que o `TaskReward` do `pw-data-loader` já sabe conceder — é
-- sempre zero.

ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS reputation INT DEFAULT 0 NOT NULL,
    ADD COLUMN IF NOT EXISTS inventory_size SMALLINT DEFAULT 64 NOT NULL,
    ADD COLUMN IF NOT EXISTS storehouse_size SMALLINT DEFAULT 32 NOT NULL,
    -- A gaiola de pets é um terceiro contêiner com expansão própria (passo 44 do
    -- roteiro de 45 passos do 1.2.6: "expandir cage de pet").
    ADD COLUMN IF NOT EXISTS petbag_size SMALLINT DEFAULT 5 NOT NULL,
    -- Dinheiro do banco. Não é o mesmo `money` do personagem: o banco tem saldo
    -- próprio, e depositar/sacar mexe nos dois.
    ADD COLUMN IF NOT EXISTS storehouse_money BIGINT DEFAULT 0 NOT NULL;

-- -----------------------------------------------------------------------------
-- 3. `character_items` — os cinco campos do `GRoleInventory` que vão zerados
-- -----------------------------------------------------------------------------
--
-- O item no fio (`rpcdata/groleinventory`) é
-- `{id, pos, count, max_count, data, proctype, expire_date, guid1, guid2, mask}`.
-- O `write_role_info` escreve `proctype`, `expire_date`, `guid1`, `guid2` e
-- `mask` como zero, e o `ItemRepository` devolve `max_count: 100` chumbado
-- (`repositories/item.rs:37`) — inclusive para equipamento, que empilha 1.
--
-- O par `guid1`/`guid2` é o identificador único da instância do item. É o que
-- permite rastrear um item específico em troca, correio e leilão — e é o que
-- torna possível provar uma duplicação em vez de discutir sobre ela.

ALTER TABLE character_items
    ADD COLUMN IF NOT EXISTS max_count INT DEFAULT 1 NOT NULL,
    ADD COLUMN IF NOT EXISTS proctype INT DEFAULT 0 NOT NULL,
    ADD COLUMN IF NOT EXISTS expire_date INT DEFAULT 0 NOT NULL,
    ADD COLUMN IF NOT EXISTS guid1 INT DEFAULT 0 NOT NULL,
    ADD COLUMN IF NOT EXISTS guid2 INT DEFAULT 0 NOT NULL,
    ADD COLUMN IF NOT EXISTS mask INT DEFAULT 0 NOT NULL;

-- O `guid` de 64 bits é `(guid1, guid2)`. Uma sequência única no banco inteiro
-- garante que dois realms nunca gerem o mesmo — o que importa no dia em que
-- houver transferência entre realms, e não custa nada hoje.
CREATE SEQUENCE IF NOT EXISTS item_guid_seq AS BIGINT START 1;

-- Índice para achar uma instância pelo `guid`. Parcial: os itens antigos, com
-- `guid` zero, ficam de fora e não inflam o índice.
CREATE INDEX IF NOT EXISTS idx_items_guid
    ON character_items(guid1, guid2)
    WHERE guid1 <> 0 OR guid2 <> 0;

-- -----------------------------------------------------------------------------
-- 4. As três coisas que o servidor responde "ok" e joga fora
-- -----------------------------------------------------------------------------
--
-- `crates/pw-link/src/gateway.rs:979-1034`: `SetUIConfig`, `SetCustomData` e
-- `SetHelpStates` respondem `result: 0` e descartam o conteúdo; os `Get*`
-- correspondentes devolvem vazio. O jogador arruma a interface, reloga, e está
-- tudo no lugar padrão de novo — sem nenhum erro em lugar nenhum.
--
-- São três blobs opacos por personagem. Tabelas separadas (e não colunas em
-- `characters`) porque são gravados a cada mudança de interface e lidos uma vez
-- no login: misturá-los com a linha do personagem faria cada `UPDATE` de posição
-- reescrever quilobytes.

CREATE TABLE IF NOT EXISTS character_ui_config (
    character_id INT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    ui_config BYTEA NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS character_help_states (
    character_id INT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    help_states BYTEA NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- O `SetCustomData` é um blob à parte do `custom_data` de aparência que já existe
-- em `characters` — aquele é a face/cabelo do `RoleInfo`, este é o que o cliente
-- guarda por conta própria. Nomes parecidos, conteúdos diferentes.
CREATE TABLE IF NOT EXISTS character_custom_data (
    character_id INT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    data BYTEA NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- -----------------------------------------------------------------------------
-- 5. Amigos — o `GetFriendList` devolve três listas vazias
-- -----------------------------------------------------------------------------
--
-- `gateway.rs:1003`: `groups`, `friends` e `status` saem vazios com o comentário
-- "a lista de amigos ainda não vem do armazenamento". Os passos 38 a 41 do
-- roteiro do 1.2.6 são exatamente isto: convidar, aceitar, listar, sussurrar.
--
-- Os grupos são as pastas que o jogador cria na janela de amigos; o `friends`
-- referencia um grupo.

CREATE TABLE IF NOT EXISTS character_friend_groups (
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    group_id SMALLINT NOT NULL,
    name VARCHAR(32) NOT NULL,
    PRIMARY KEY (character_id, group_id)
);

CREATE TABLE IF NOT EXISTS character_friends (
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    friend_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    group_id SMALLINT DEFAULT 0 NOT NULL,
    -- 0 = amigo, 1 = bloqueado. O bloqueio usa a mesma janela no cliente.
    relation SMALLINT DEFAULT 0 NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (character_id, friend_id),
    -- Ninguém é amigo de si mesmo; sem isto, um `role_id` repetido na lista faz
    -- o cliente desenhar o próprio jogador como amigo.
    CONSTRAINT ck_friend_nao_e_si_mesmo CHECK (character_id <> friend_id)
);

CREATE INDEX IF NOT EXISTS idx_friends_do_personagem ON character_friends(character_id);
-- Para o caminho inverso: "quem tem este personagem na lista" — é o que avisa os
-- amigos quando alguém entra ou sai.
CREATE INDEX IF NOT EXISTS idx_friends_inverso ON character_friends(friend_id);

-- -----------------------------------------------------------------------------
-- 6. Membros de facção — tirando a lista de dentro do JSONB
-- -----------------------------------------------------------------------------
--
-- `factions.members JSONB` guarda a lista inteira num campo só. Três problemas
-- concretos, não estéticos:
--
--   1. não há integridade: um personagem excluído continua na lista, e nada no
--      banco impede;
--   2. a pergunta mais comum do jogo — "de que facção é este personagem?" — vira
--      uma varredura de todas as facções do realm;
--   3. dois membros entrando ao mesmo tempo reescrevem o mesmo documento, e um
--      dos dois some sem erro.
--
-- A coluna `members` **não é removida aqui**: quem já tem dados precisa migrar
-- antes. `docs/BANCO_DE_DADOS.md` traz o `INSERT ... SELECT` de migração.

CREATE TABLE IF NOT EXISTS faction_members (
    faction_id INT NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
    character_id INT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    -- Patente dentro da facção (0 = membro comum; o topo é o mestre, que também
    -- está em `factions.master_character_id`).
    rank SMALLINT DEFAULT 0 NOT NULL,
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (faction_id, character_id),
    -- Um personagem pertence a **uma** facção. Esta é a restrição que o JSONB
    -- não tinha como ter.
    CONSTRAINT uq_uma_faccao_por_personagem UNIQUE (character_id)
);

CREATE INDEX IF NOT EXISTS idx_faction_members_personagem ON faction_members(character_id);

-- -----------------------------------------------------------------------------
-- 7. Nome de personagem: unicidade que o cliente enxerga
-- -----------------------------------------------------------------------------
--
-- `uq_character_name_per_realm UNIQUE(realm_id, name)` distingue maiúsculas: no
-- mesmo realm cabem `Murillo` e `murillo`, que na tela são duas pessoas
-- diferentes com o mesmo nome — e é assim que se personifica alguém para um
-- golpe de troca. O índice `idx_characters_realm_name` já existe sobre
-- `LOWER(name)`, mas não é único: ele acelera a busca e não impede nada.
--
-- O nome continua reservado enquanto a exclusão está pendente — é o que o
-- `UndoDeleteRole` precisa para poder desfazer.
--
-- ATENÇÃO: se o banco já tiver dois nomes que só diferem por maiúsculas, a
-- criação falha. `docs/BANCO_DE_DADOS.md` traz a consulta que os encontra.
CREATE UNIQUE INDEX IF NOT EXISTS uq_characters_realm_nome_minusculo
    ON characters(realm_id, LOWER(name));

-- -----------------------------------------------------------------------------
-- 8. Versão do realm: o mesmo cuidado que o código já tem
-- -----------------------------------------------------------------------------
--
-- `pw-link` e `pw-gs` **abortam** com um `GAME_VERSION` que não reconhecem, em
-- vez de cair no 1.2.6 em silêncio. A tabela `realms` aceitava qualquer texto:
-- um realm com `version = '1.53'` subiria e o painel mostraria uma versão que
-- não existe. A restrição alinha o banco ao código.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ck_realms_versao_conhecida'
    ) THEN
        ALTER TABLE realms
            ADD CONSTRAINT ck_realms_versao_conhecida
            CHECK (version IN ('1.2.6', '1.4.8', '1.5.3'));
    END IF;
END $$;

-- -----------------------------------------------------------------------------
-- 9. Índices que faltavam para as consultas do caminho quente
-- -----------------------------------------------------------------------------

-- A tela de personagens: "os meus deste realm, que não estão excluídos". Hoje
-- isso usa `idx_characters_account` e filtra o resto na memória.
CREATE INDEX IF NOT EXISTS idx_characters_conta_realm_vivos
    ON characters(account_id, realm_id)
    WHERE is_deleted = FALSE;

-- A varredura de exclusões a consumar. Parcial: só as pendentes entram.
CREATE INDEX IF NOT EXISTS idx_characters_exclusao_agendada
    ON characters(delete_scheduled_at)
    WHERE delete_scheduled_at IS NOT NULL AND is_deleted = FALSE;

-- Correio a expirar (o `mails.expires_at` já existe e nada o varre).
CREATE INDEX IF NOT EXISTS idx_mails_expiram
    ON mails(expires_at)
    WHERE is_collected = FALSE;
