-- B363 — o PK do personagem (`player_invade::Save`, `gs/playerinvade.h:88-97`): o estado (0 branco, 1 rosa,
-- 2 vermelho), o tempo de agressor, o tempo de pária, o grau do vermelho e as mortes.
ALTER TABLE characters
    ADD COLUMN IF NOT EXISTS pk_estado SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_tempo_agressor INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_tempo_paria INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_grau SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_mortes INTEGER NOT NULL DEFAULT 0;

ALTER TABLE test.characters
    ADD COLUMN IF NOT EXISTS pk_estado SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_tempo_agressor INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_tempo_paria INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_grau SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pk_mortes INTEGER NOT NULL DEFAULT 0;
