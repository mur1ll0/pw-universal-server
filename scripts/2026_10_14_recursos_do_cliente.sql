-- P1 dos recursos do cliente (B233): registro da versão 0 e das alterações de cada versão de
-- cliente (`docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` §3). Aplicar no search_path
-- escolhido (o `test` da suíte e o `public` do realm), como as outras migrações:
--   docker exec -i pw-postgres sh -c 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1' < scripts/2026_10_14_recursos_do_cliente.sql
--   (para o test: acrescente  -c "SET search_path TO test"  antes do -f, ou rode com PGOPTIONS=--search_path=test)
--
-- O caminho de cada arquivo é guardado em BYTES GBK (`caminho`), não em texto: é com esses
-- bytes que o cliente abre o arquivo (`AFileImage.cpp:127`, `fopen` ANSI), e o nome no disco
-- depende da página de código de cada Windows (§7.7). Só de acréscimo: linha publicada não muda.
BEGIN;

-- Uma por versão de cliente ('1.5.5', '1.2.6').
CREATE TABLE IF NOT EXISTS recursos_linhas (
    id VARCHAR(16) PRIMARY KEY,
    origem TEXT NOT NULL,                       -- pasta de onde a versão 0 foi importada
    base_sha256 BYTEA NOT NULL CHECK (octet_length(base_sha256) = 32), -- do manifesto da versão 0
    importada_em TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- O cliente original inteiro (versão 0): um registro por arquivo.
CREATE TABLE IF NOT EXISTS recursos_base (
    linha VARCHAR(16) NOT NULL REFERENCES recursos_linhas(id) ON DELETE CASCADE,
    caminho BYTEA NOT NULL,                     -- bytes GBK, separador '\', relativo a element\
    origem VARCHAR(8) NOT NULL CHECK (origem IN ('pacote', 'solto', 'nosso')),
    pacote_original VARCHAR(64),                -- 'configs.pck' etc. (origem = 'pacote')
    pacote VARCHAR(64) NOT NULL,                -- pacote de instalação (.tar.zst) que o leva
    tamanho BIGINT NOT NULL CHECK (tamanho >= 0),
    sha256 BYTEA NOT NULL CHECK (octet_length(sha256) = 32),
    md5 BYTEA NOT NULL CHECK (octet_length(md5) = 16),
    PRIMARY KEY (linha, caminho)
);

CREATE TABLE IF NOT EXISTS recursos_alteracoes (
    id BIGSERIAL PRIMARY KEY,
    linha VARCHAR(16) NOT NULL REFERENCES recursos_linhas(id),
    autor INTEGER,                              -- conta GM
    criada_em TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    descricao TEXT NOT NULL DEFAULT '',
    estado VARCHAR(12) NOT NULL DEFAULT 'rascunho'
        CHECK (estado IN ('rascunho', 'publicada', 'descartada')),
    operacao_id VARCHAR(64) UNIQUE,             -- idempotência, como comandos_administrativos
    reverte BIGINT REFERENCES recursos_alteracoes(id)
);

CREATE TABLE IF NOT EXISTS recursos_alteracao_itens (
    alteracao BIGINT NOT NULL REFERENCES recursos_alteracoes(id) ON DELETE CASCADE,
    caminho BYTEA NOT NULL,
    destino VARCHAR(8) NOT NULL CHECK (destino IN ('cliente', 'servidor', 'ambos')),
    acao VARCHAR(12) NOT NULL CHECK (acao IN ('adicionar', 'substituir', 'remover')),
    sha256_antes BYTEA CHECK (sha256_antes IS NULL OR octet_length(sha256_antes) = 32),
    sha256_depois BYTEA CHECK (sha256_depois IS NULL OR octet_length(sha256_depois) = 32),
    tamanho BIGINT,
    resumo JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (alteracao, caminho)
);

-- Versões publicadas (a 0 é a importação). O manifesto e a assinatura ficam aqui também.
CREATE TABLE IF NOT EXISTS recursos_versoes (
    linha VARCHAR(16) NOT NULL REFERENCES recursos_linhas(id) ON DELETE CASCADE,
    numero INTEGER NOT NULL CHECK (numero >= 0),
    alteracoes BIGINT[] NOT NULL DEFAULT '{}',
    manifesto_sha256 BYTEA NOT NULL CHECK (octet_length(manifesto_sha256) = 32),
    assinatura BYTEA NOT NULL CHECK (octet_length(assinatura) = 64), -- Ed25519 do manifesto
    publicada_em TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    publicada_por INTEGER,                      -- NULL na versão 0 (importação)
    minima INTEGER NOT NULL DEFAULT 0,          -- versão mínima aceita pelo launcher
    PRIMARY KEY (linha, numero)
);

COMMIT;
