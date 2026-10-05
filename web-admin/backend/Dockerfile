FROM rust:slim-bookworm AS credenciais
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tools ./tools
RUN cargo build --release --locked -p pw-crypto --bin pw-validar-credenciais

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
COPY web-admin/backend/main.py .
COPY --from=credenciais /build/target/release/pw-validar-credenciais /usr/local/bin/

ENV PORT=8000
EXPOSE 8000

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
