-- B168: executar no search_path escolhido, só em implantação autorizada.
-- A busca/autenticação usa LOWER(username); duplicatas existentes abortam o
-- índice e toda a migração. Não renomear/remover contas nem redefinir senhas.
BEGIN;
CREATE UNIQUE INDEX IF NOT EXISTS uq_accounts_usuario_lower ON accounts(LOWER(username));
ALTER TABLE comandos_administrativos ALTER COLUMN conta_id DROP NOT NULL;
COMMIT;
