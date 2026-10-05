-- E3/E4 (B167): aplicar no search_path escolhido durante implantação autorizada.
-- Escrita da conta e resultado final compartilham a mesma transação. Não apagar IDs:
-- a retenção preserva deduplicação após reinício e entre realms. Sem senha/hash aqui.
BEGIN;
CREATE TABLE IF NOT EXISTS comandos_administrativos (
    operacao_id VARCHAR(64) PRIMARY KEY,
    administrador_id INTEGER NOT NULL,
    realm_origem VARCHAR(32) NOT NULL,
    conta_id INTEGER NOT NULL,
    impressao BYTEA NOT NULL CHECK (octet_length(impressao) = 32),
    resultado JSONB NOT NULL,
    criado_em TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
COMMIT;
