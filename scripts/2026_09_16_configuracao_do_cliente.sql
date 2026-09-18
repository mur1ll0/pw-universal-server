-- A configuração que o cliente guarda no servidor: barras de atalho, layout da interface,
-- opções do jogador — e as marcas de ajuda já vistas.
--
-- # O que é o `ui_config`
--
-- `CECGameRun::SaveConfigsToServer` (`EC_GameRun.cpp:2014-2134`) junta, num bloco só,
-- `USERCFG_VERSION` (4 bytes sem compressão) + comprimido(`[int tamanho][host]
-- [int tamanho][layout da UI][int tamanho][opções]`), onde "host" é
-- `CECHostPlayer::SaveConfigData` — **as barras de atalho** (`m_aSCSets1/2`, cada atalho
-- de comando, habilidade, item, grupo de habilidades...). Manda no `SetUIConfig` ao sair e
-- quando o conteúdo muda. Na entrada, `GetUIConfig_Re` devolve o mesmo bloco e
-- `LoadConfigsFromServer` (`:2139`) o desfaz — bloco vazio cai nas opções padrão e deixa as
-- barras vazias, que era o que acontecia.
--
-- O servidor não interpreta nada: o `gamedbd` original grava o `Octets` como veio. Aqui
-- também — é opaco e versionado pelo próprio cliente.
--
-- `help_states` é o `SetHelpStates`/`GetHelpStates_Re`: as dicas de tutorial já mostradas.

BEGIN;

CREATE TABLE IF NOT EXISTS character_client_config (
    character_id INT PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    ui_config BYTEA,
    help_states BYTEA,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

COMMIT;
