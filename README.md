# PW-Universal-Server: Plataforma Moderna de Servidor Universal Multi-Realm

Plataforma de servidor universal de alto desempenho para **Perfect World**, desenvolvida sob o paradigma **Spec-Driven Development (SDD)**.

---

## Destaques da Arquitetura

1. **Núcleo Agnóstico Orientado a Dados (Rust Core)**:
   - Suporte simultâneo a múltiplas versões do jogo (v1.2.6 Classic, v1.3.6, v1.4.6, v1.5.3 Eclipse) a partir de um único código-fonte base.
   - Codecs de rede dinâmicos por versão (serialização/deserialização automática de pacotes C2S/S2C).
   - Leitor dinâmico universal de `elements.data` (v7 até v153), `tasks.data` e mapas 3D.

2. **Abordagem Multi-Realm com Banco Unificado**:
   - **PostgreSQL 16**: Armazenamento relacional de contas, permissões, saldos de Gold/Cash e dados de personagens em `JSONB`.
   - **DragonflyDB / Redis**: Cache em memória com latência sub-milissegundo para estado de jogadores online, instâncias e filas de chat.
   - Instâncias concorrentes de servidores para versões distintas (ex: Realm 1.2.6 na porta `29000` e Realm 1.5.3 na porta `29001`) compartilhando a mesma infraestrutura de banco de dados.

3. **Painel Web Moderno de Administração (`pw-admin-web`)**:
   - Interface web moderna (Next.js + Tailwind + FastAPI) em substituição ao antigo `pwAdmin`.
   - Controle total de contas, injeção de Gold, teletransporte, inspeção e edição de inventário ao vivo, controle de multiplicadores de servidor (Double EXP/SP/Drop) e gerenciamento dinâmico de mapas e dungeons.

4. **Modernização do Patcher & Gerador CPW (`pw-patch-tool`)**:
   - Geração de pacotes diferenciais `.cup` com compressão otimizada.
   - Servidor de atualizações com suporte a downloads resumíveis via HTTP/CDN e validação por hash SHA-256.

---

## Estrutura do Repositório

```
pw-universal-server/
├── Cargo.toml                       # Workspace Cargo (Rust)
├── specs/                           # Especificações Técnicas (SDD)
│   ├── 00_MASTER_SPECIFICATION.md   # Especificação mestre do ecossistema
│   ├── 01_DATABASE_SCHEMA_POSTGRES.sql # Schema completo PostgreSQL 16 com JSONB
│   ├── 02_MULTI_REALM_ARCHITECTURE.md  # Arquitetura de múltiplos realms concorrentes
│   └── 06_ADMIN_PANEL_AND_CPW_SPEC.md  # Especificação do Painel Web e Gerador CPW
├── crates/                          # Módulos em Rust
│   ├── pw-core/                     # Tipos compartilhados, matemática 3D e modelos
│   ├── pw-crypto/                   # Criptografia RC4, MD5 e MPPC
│   ├── pw-protocol/                 # Codecs de rede para pacotes C2S/S2C multi-versão
│   ├── pw-data-loader/              # Parser dinâmico de elements.data, tasks.data e mapas
│   ├── pw-storage/                  # Camada de persistência PostgreSQL + DragonflyDB
│   ├── pw-auth/                     # Autenticação global e Billing
│   ├── pw-uniquename/               # Unicidade de nomes de personagens e facções
│   ├── pw-link/                     # Gateway de rede TCP assíncrono
│   ├── pw-delivery/                 # Roteador central de mensagens, chat e instâncias
│   └── pw-gs/                       # Motor de Simulação de Mundo 3D (World Server)
├── docker/                          # Infraestrutura Docker Multi-Realm
│   └── docker-compose.yml           # Orquestração de 2 Realms simultâneos + DB
├── tools/                           # Ferramentas auxiliares
│   └── pw-patch-tool/               # Gerador de updates .cup e manifests CDN
└── web-admin/                       # Painel Web de Administração (Next.js + FastAPI)
```
