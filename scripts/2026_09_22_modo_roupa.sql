-- B83 — o modo roupa (moda) tem de sobreviver ao logout.
--
-- No original o estado é um bit do `object_state` (`STATE_FASHION_MODE`, `gs/object.h:160`),
-- mas o que vai para o banco é um **blob de pares de int**: `GetPlayerCharMode` escreve
-- `(PLAYER_CHAR_MODE_FASHION=1, 1)` quando o bit está ligado, e `SetPlayerCharMode` o relê no
-- login (`gs/player.cpp:12585-12612`, `gs/userlogin.cpp:161`, `:738`). O campo do registro do
-- personagem chama-se `charactermode`.
--
-- Esse mesmo blob viaja no `RoleInfo` da **lista de personagens**, e é de lá que a tela de
-- seleção descobre como desenhar o avatar: `CECLoginPlayer::Load` lê pares de 8 bytes e a
-- chave 1 vira `m_bFashionMode` (`EC_LoginPlayer.cpp:172-189`). Sem ele, o personagem aparece
-- de armadura na seleção mesmo tendo saído de roupa — o relato do Murillo com o RT.
--
-- Guardamos o blob como o original o guarda, em vez de um booleano: é o que o protocolo
-- manda cru, e outras chaves de modo entram sem nova migração.

ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS character_mode BYTEA NOT NULL DEFAULT ''::bytea;

COMMENT ON COLUMN characters.character_mode IS
    'charactermode do original: pares (chave, valor) de int32 LE; chave 1 = modo roupa';

-- O mesmo no schema `test`, onde a suíte isola tudo (`search_path = test,public`,
-- `crates/pw-storage/src/postgres.rs:19-22`): sem isto, todo teste que grava o modo roupa
-- falha no `UPDATE`.
ALTER TABLE test.characters
    ADD COLUMN IF NOT EXISTS character_mode BYTEA NOT NULL DEFAULT ''::bytea;
