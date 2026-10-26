FROM rust:slim-bookworm AS credenciais
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tools ./tools
RUN cargo build --release --locked -p pw-crypto --bin pw-validar-credenciais

# B366: o Estúdio (Svelte + TypeScript + Vite) é empacotado aqui; o resultado são só arquivos estáticos em
# static/estudio/, servidos pelo FastAPI na mesma origem (CSP `self`). A rede só é usada no `npm ci` da build;
# em execução o painel não busca nada fora.
FROM node:22-bookworm-slim AS estudio
WORKDIR /build/web-admin/estudio
COPY web-admin/estudio/package.json web-admin/estudio/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web-admin/estudio/ ./
# Os layouts do elements das duas versões: o teste dos grupos de Itens confere cada tabela neles (lib/itens.test.ts).
COPY specs/elements_layouts/v156.json specs/elements_layouts/v7.json /build/specs/elements_layouts/
RUN npm test && npm run build

FROM python:3.11-slim-bookworm

WORKDIR /app

# Instalar dependências básicas
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY web-admin/backend/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY web-admin/backend/painel ./painel
COPY web-admin/backend/static ./static
COPY --from=estudio /build/web-admin/backend/static/estudio ./static/estudio
COPY web-admin/backend/main.py .
COPY --from=credenciais /build/target/release/pw-validar-credenciais /usr/local/bin/

ENV PORT=8000
EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
