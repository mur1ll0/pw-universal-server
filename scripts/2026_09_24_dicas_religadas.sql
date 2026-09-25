-- B107 — religa as dicas (tela de ajuda do jogador novo) de quem gravou o estado "nenhum tipo
-- ativo" herdado do padrão errado do `gateway.rs`.
--
-- Sem estado gravado, o `GetHelpStates_Re` levava 32 bytes zerados; o cliente os lia como
-- versão 0, 0 scripts e palavra de tipos 0 (`CECScriptOption::BufferToOption`,
-- `ECScriptOption.cpp:121-146`): todos os tipos de dica desligados. Depois ele gravava esse
-- estado de volta (`SetHelpStates`), e as dicas nunca mais abriam. O original manda vazio ao
-- personagem novo (`gamedbmanager.cpp:378`, `gethelpstates.hpp:27-31`), e o cliente usa o
-- padrão: palavra de tipos 0x7fff, todos ativos, abertura automática ligada (`:89-101`).
--
-- Aqui só a última palavra (tipos + bit 15 "ajuda desligada") passa de 0x0000 para 0x7fff; a
-- lista de dicas já vistas fica como está. Só toca quem tem a palavra zerada.

BEGIN;

UPDATE character_client_config
   SET help_states = overlay(help_states PLACING '\xff7f'::bytea FROM length(help_states) - 1 FOR 2),
       updated_at = CURRENT_TIMESTAMP
 WHERE help_states IS NOT NULL
   AND length(help_states) >= 6
   AND substring(help_states FROM length(help_states) - 1 FOR 2) = '\x0000'::bytea;

COMMIT;
