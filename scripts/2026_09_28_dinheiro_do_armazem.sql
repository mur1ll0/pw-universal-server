-- B147 — o dinheiro do armazém (`player_trashbox::_money`, até `TRASHBOX_MONEY_CAPACITY` 2·10⁹,
-- `gs/config.h:78`), movido pelo `EXG_TRASHBOX_MONEY` (C2S 61; `PlayerExchangeTrashMoney`,
-- `gs/player.cpp:7727-7779`). Os itens do armazém já moram em `character_items` com
-- `container_type` 2.
ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS storehouse_money BIGINT NOT NULL DEFAULT 0;

ALTER TABLE test.characters
    ADD COLUMN IF NOT EXISTS storehouse_money BIGINT NOT NULL DEFAULT 0;
