# 🐉 PW-Universal-Server (Modern Perfect World Server Platform)

[![Rust](https://img.shields.io/badge/Language-Rust%202021-orange.svg)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/Database-PostgreSQL%2016-blue.svg)](https://www.postgresql.org/)
[![DragonflyDB](https://img.shields.io/badge/Cache-DragonflyDB-red.svg)](https://www.dragonflydb.io/)
[![Docker](https://img.shields.io/badge/Orchestration-Docker%20Compose-2496ED.svg)](https://www.docker.com/)
[![FastAPI](https://img.shields.io/badge/API-FastAPI%20Python-009688.svg)](https://fastapi.tiangolo.com/)

Uma reescrita completa, moderna, modular e de altíssimo desempenho do servidor de **Perfect World**, desenvolvida em **Rust (Tokio assíncrono)**, **PostgreSQL 16**, **DragonflyDB**, **FastAPI** e **Next.js/Tailwind**.

A plataforma suporta nativamente **qualquer versão do jogo (v1.2.6 até v1.5.3+)** através de codecs de protocolo dinâmicos e permite rodar **múltiplos servidores concorrentes (Multi-Realm)** compartilhando a mesma base de contas global e saldo de Gold/CUBI.

---

## 🔑 Credenciais Padrão (Banco de Dados & Administrador)

Ao subir os contêineres com `docker compose up -d`, o PostgreSQL cria automaticamente as tabelas e insere as seguintes contas padrão:

### 1. Banco de Dados PostgreSQL 16
| Parâmetro | Valor Padrão | Onde Configurar / Alterar |
| :--- | :--- | :--- |
| **Host** | `localhost` (ou `pw-postgres` no Docker) | `.env` / `docker-compose.yml` |
| **Porta** | `5432` | `POSTGRES_PORT` |
| **Database** | `pw_database` | `POSTGRES_DB` |
| **Usuário** | `pw_admin` | `POSTGRES_USER` |
| **Senha** | `pw_secure_password_2026` | `POSTGRES_PASSWORD` |

### 2. Contas de Jogo & Administrador Master (Iniciais)
| Usuário | Senha | Nível de GM | Saldo Gold | Finalidade |
| :--- | :--- | :---: | :---: | :--- |
| **`admin`** | **`admin`** | **Nível 32 (God/Master)** | **1.000.000 Gold** | Conta Master com privilégios totais de GM in-game e acesso ao Painel Web. |
| **`testuser`** | **`123456`** | Nível 0 (Normal) | 50.000 Gold | Conta de jogador comum para testes de login e jogabilidade. |

---

## 🏛️ Arquitetura do Sistema

```
+-----------------------------------------------------------------------------------------------------------------------+
|                                                  CLIENTES DE JOGO (WIN32)                                             |
|                                                                                                                       |
|         +----------------------------------+                          +----------------------------------+            |
|         |   ElementClient.exe (v1.2.6)     |                          |   ElementClient.exe (v1.5.3)     |            |
|         +----------------------------------+                          +----------------------------------+            |
|                           │ (TCP Port 29000)                                            │ (TCP Port 29001)            |
+───────────────────────────┼─────────────────────────────────────────────────────────────┼─────────────────────────────+
                            ▼                                                             ▼
+-----------------------------------------------------------------------------------------------------------------------+
|                                             DOCKER MULTI-REALM EM RUST (TOKIO)                                        |
|                                                                                                                       |
|  [ REALM 1: Classic v1.2.6 ]                                 [ REALM 2: Eclipse v1.5.3 ]                              |
|  ├── pw-link (Porta 29000 pública)                           ├── pw-link (Porta 29001 pública)                         |
|  ├── pw-delivery (Roteador de Chat/Grupos)                   ├── pw-delivery (Roteador de Chat/Grupos)                 |
|  └── pw-gs (World Server 3D - 50ms Tick Loop)                └── pw-gs (World Server 3D - 50ms Tick Loop)              |
+───────────────────────────┼─────────────────────────────────────────────────────────────┼─────────────────────────────+
                            ▼                                                             ▼
+-----------------------------------------------------------------------------------------------------------------------+
|                                    CAMADA COMPARTILHADA (BANCO DE DADOS & GESTÃO)                                     |
|                                                                                                                       |
|  • pw-auth (Autenticação Global / Argon2id / MD5 / Tickets)                                                           |
|  • pw-uniquename (Unicidade de Nomes por Realm)                                                                       |
|  • pw-admin-web (Dashboard Web FastAPI + Tailwind substituindo o pwAdmin)                                             |
|  • DragonflyDB (Cache em RAM Sub-milissegundo para Sessões e Pub/Sub de Chat)                                         |
|  • PostgreSQL 16 (Tabelas Normalizadas com Índices: accounts, characters, character_items, skills, quests)            |
+-----------------------------------------------------------------------------------------------------------------------+
```

---

## 🚀 Como Rodar o Servidor (Guia Rápido)

### 1. Pré-requisitos
- [Docker](https://www.docker.com/) e Docker Compose instalados.
- Python 3.10+ (para executar testes locais e utilitários).

---

### 2. Copiar os Arquivos de Configuração (`.data`)

Cada Realm possui sua pasta dedicada dentro de `data/`. Copie os arquivos da versão correspondente:

#### Para o Realm 1.2.6 Classic:
```bash
# Copia os dados para data/realm_126/config/
cp ../files1.2.6/pwserver/gamed/config/elements.data ./data/realm_126/config/
cp ../files1.2.6/pwserver/gamed/config/gshop.data ./data/realm_126/config/
cp ../files1.2.6/pwserver/gamed/config/tasks.data ./data/realm_126/config/
cp ../files1.2.6/pwserver/gamed/config/aipolicy.data ./data/realm_126/config/
cp -r ../files1.2.6/pwserver/gamed/config/world ./data/realm_126/config/
```

#### Para o Realm 1.5.3 Eclipse:
```bash
# Copia os dados para data/realm_153/config/
cp ../pwclient_153v145/element/data/elements.data ./data/realm_153/config/
cp ../pwclient_153v145/element/data/gshop.data ./data/realm_153/config/
cp ../pwclient_153v145/element/data/tasks.data ./data/realm_153/config/
cp ../pwclient_153v145/element/data/aipolicy.data ./data/realm_153/config/
```

---

### 3. Iniciar os Serviços

#### Cenário A: Subir os 2 Servidores SIMULTÂNEOS (Multi-Realm: 1.2.6 + 1.5.3)
```bash
cd pw-universal-server/docker
docker compose up -d
```

#### Cenário B: Subir Apenas o Servidor 1.2.6 Classic
```bash
docker compose up -d pw-postgres pw-dragonfly pw-auth pw-admin-api pw-realm-126
```

---

### 4. Portas de Rede e Conexão dos Clientes

| Serviço | Porta Pública | Configuração no Cliente / Acesso |
| :--- | :---: | :--- |
| **Realm 1.2.6 Classic** | `29000` | Configurar `127.0.0.1 29000` no `serverlist.txt` do cliente 1.2.6. |
| **Realm 1.5.3 Eclipse** | `29001` | Configurar `127.0.0.1 29001` no `serverlist.txt` do cliente 1.5.3. |
| **Painel Web (pwAdmin)** | `8000` | Abra `web-admin/frontend/index.html` no navegador ou acesse `http://localhost:8000`. |
| **PostgreSQL 16** | `5432` | Usuário: `pw_admin` \| Senha: `pw_secure_password_2026` \| Banco: `pw_database` |
| **DragonflyDB (Cache)** | `6379` | Cache de sessões em RAM sub-milissegundo e canais de Chat Pub/Sub. |

---

## 📚 Índice de Documentação e Manuais

Todos os manuais técnicos e especificações detalhadas estão disponíveis no repositório:

### 📖 Manuais Técnicos (`docs/`)
- [📘 Guia de Operação: Como Rodar Servidor Único ou Multi-Realm](docs/HOW_TO_RUN_SINGLE_OR_MULTI_REALM.md)
- [💻 Manual do Usuário: Painel Web de Administração (pw-admin-web)](docs/WEB_ADMIN_USER_GUIDE.md)
- [🎨 Guia de Integração e Decodificação de Ícones (Surfaces & Iconset)](docs/SURFACES_ICONSET_GUIDE.md)
- [📦 Guia do Gerador de Atualizações CDN (pw-patch-tool)](docs/PW_PATCH_TOOL_GUIDE.md)
- [📜 Manual de Formatos Binários: elements.data, gshop, tasks e colisões](docs/FILE_FORMATS_REFERENCE.md)
- [⚡ Guia de Arquitetura do Loader e Consumo de Memória RAM](docs/LOADER_ARCHITECTURE_GUIDE.md)
- [🔍 Relatório de Auditoria e Revisão Minuciosa Final](docs/FINAL_COMPREHENSIVE_AUDIT.md)

### 📐 Especificações de Engenharia (`specs/`)
- [00. Especificação Mestre de Arquitetura](specs/00_MASTER_SPECIFICATION.md)
- [01. Schema Relacional Normalizado do PostgreSQL 16](specs/01_DATABASE_SCHEMA_POSTGRES.sql)
- [02. Arquitetura Multi-Realm & Portas de Rede](specs/02_MULTI_REALM_ARCHITECTURE.md)
- [03. Especificação do Loader de Dados Dinâmico](specs/03_DATA_LOADER_SPEC.md)
- [06. Especificação do Painel Web & Gerador de Patches](specs/06_ADMIN_PANEL_AND_CPW_SPEC.md)

---

## 🧪 Execução de Testes Automatizados

A plataforma inclui uma suíte completa de testes unitários e de integração que validam criptografia, regras de protocolo, fórmulas de combate e integridade dos arquivos `.data`:

```bash
# Executa todos os testes de ponta a ponta:
python ./tests/verify_services.py
python ./tests/test_game_mechanics_and_integrity.py
python ./tests/test_character_inventory_editing.py
python ./tests/test_account_auth_and_passwords.py
```

---

## 🛡️ Licença e Direitos
Desenvolvido para fins educacionais, de pesquisa e preservação histórica de software de emulação de Perfect World.
