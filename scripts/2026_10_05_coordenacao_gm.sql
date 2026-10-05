-- B169: revisão global e recibos de reconciliação. Aplicar no search_path escolhido.
-- Não modifica privilégios, contas ou personagens existentes.
BEGIN;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS revisao_gm BIGINT NOT NULL DEFAULT 0;
CREATE TABLE IF NOT EXISTS coordenacao_gm_revisao (
    unico BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (unico),
    revisao BIGINT NOT NULL DEFAULT 0
);
INSERT INTO coordenacao_gm_revisao(unico) VALUES(TRUE) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS coordenacao_gm_processos (
    processo TEXT PRIMARY KEY,
    encarnacao TEXT NOT NULL,
    revisao BIGINT NOT NULL DEFAULT 0,
    atualizado_em TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
COMMIT;
