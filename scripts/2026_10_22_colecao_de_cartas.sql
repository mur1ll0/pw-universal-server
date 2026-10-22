-- B357 — a coleção das Cartas de General (`gplayer_imp::_generalcard_collection`, `abase::bitmap<512>`,
-- `gs/player_imp.h:1798`, `GENERALCARD_MAX_COLLECTION` = 512, `config.h:218`): um bit por `show_order` de carta
-- obtida (`FirstAcquireItem`, `player.cpp:15348-15364`), gravado com o personagem (`player.cpp:10300`).
ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS generalcard_collection BYTEA NOT NULL DEFAULT ''::bytea;

ALTER TABLE test.characters
    ADD COLUMN IF NOT EXISTS generalcard_collection BYTEA NOT NULL DEFAULT ''::bytea;
