-- Conferência SÓ DE LEITURA do schema public antes de publicar link/GS/painel (B172).
-- A suíte roda no schema test, já migrado: ela não enxerga o que falta no banco real.
-- Uso:
--   docker exec -i pw-postgres sh -c 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -At' < scripts/conferir_public_antes_de_publicar.sql
-- Saída vazia = pode publicar. Cada linha é um problema a resolver antes.

-- 1. Colunas e tabelas que o código atual exige (migrações scripts/2026_10_05_*.sql).
SELECT 'falta coluna public.accounts.' || c
FROM unnest(ARRAY['revisao_gm']) AS c
WHERE NOT EXISTS (SELECT 1 FROM information_schema.columns
                  WHERE table_schema='public' AND table_name='accounts' AND column_name=c)
UNION ALL
SELECT 'falta tabela public.' || t
FROM unnest(ARRAY['comandos_administrativos','coordenacao_gm_revisao','coordenacao_gm_processos']) AS t
WHERE NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name=t)
UNION ALL
SELECT 'falta indice public.uq_accounts_usuario_lower'
WHERE NOT EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname='public' AND indexname='uq_accounts_usuario_lower')
-- 2. Senha: o link confere HMAC com chave MD5(nome+senha) (gameclient.cpp:133-139).
--    Hash fora de 32 hex nunca autentica. 32 hex em outro formato (MD5 só da senha)
--    também não, e não dá para detectar sem a senha: as contas padrão são conferidas.
UNION ALL
SELECT 'conta ' || username || ': password_hash nao e MD5 hex (nunca autentica)'
FROM public.accounts WHERE password_hash !~ '^[0-9a-f]{32}$'
UNION ALL
SELECT 'conta ' || username || ': senha padrao fora de MD5(nome+senha)'
FROM public.accounts
WHERE username IN ('admin','testuser') AND password_hash <> md5(username || username)
-- 3. Nomes que colidem ignorando caixa impedem o índice único.
UNION ALL
SELECT 'nomes repetidos ignorando caixa: ' || lower(username)
FROM public.accounts GROUP BY lower(username) HAVING count(*) > 1;
