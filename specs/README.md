# Specs do pw-universal-server

> **Para que servem:** ser a descrição **curta e atual** de como o sistema é. Uma sessão nova
> lê o `docs/ESTADO_E_RETOMADA.md` (onde o trabalho está) e **só a spec da área que vai
> tocar** — não o código inteiro, não o `docs/HISTORICO_DE_SESSOES.md` (6.500 linhas de
> diário). O histórico guarda a evidência; a spec guarda a conclusão.
>
> Verificadas contra o código em **2026-09-14**, commit `e6433ae`.

## Índice e mapa de código

Ao mexer num caminho da coluna do meio, a spec da esquerda é a que precisa ser lida antes e
**atualizada depois**.

| spec | cobre (caminhos) | assunto |
| :--- | :--- | :--- |
| [`00_MASTER_SPECIFICATION.md`](00_MASTER_SPECIFICATION.md) | `Cargo.toml`, `crates/*/Cargo.toml`, estrutura geral | visão, versões-alvo, princípios, crates e seu estado |
| [`01_DATABASE_SCHEMA_POSTGRES.sql`](01_DATABASE_SCHEMA_POSTGRES.sql) | `crates/pw-storage/`, `scripts/*.sql` | esquema do banco (confere com o banco em execução: 12 tabelas) |
| [`02_MULTI_REALM_ARCHITECTURE.md`](02_MULTI_REALM_ARCHITECTURE.md) | `docker/`, `crates/pw-link/`, `crates/pw-bus/`, `crates/pw-auth/`, `crates/pw-protocol/src/{version,edition,codec,opcodes}.rs` | topologia de daemons e portas, login, versões, barramento |
| [`03_DATA_LOADER_SPEC.md`](03_DATA_LOADER_SPEC.md) | `crates/pw-data-loader/`, `specs/elements_*`, `specs/mapas/`, `specs/clsconfig_155/`, `data/realm_*` | formato de cada arquivo de dados e o que já é lido |
| [`04_PROTOCOLO_MUNDO_3D.md`](04_PROTOCOLO_MUNDO_3D.md) | `crates/pw-protocol/src/{packets,por_versao.rs}`, `crates/pw-wire/`, `crates/pw-gs/src/comandos.rs`, `specs/protocol/`, `tools/pw-rpcgen/`, `tools/pw-ir/` | subcomandos do `GamedataSend`: regras de layout, onde cada um é tratado |
| [`05_SIMULACAO_DO_MUNDO.md`](05_SIMULACAO_DO_MUNDO.md) | `crates/pw-gs/src/{world,bus_server,ai,combat,habilidades,entity,grid,npc}.rs` | regras de jogo portadas: spawns, visibilidade, IA, combate, economia, persistência |
| [`06_ADMIN_PANEL_AND_CPW_SPEC.md`](06_ADMIN_PANEL_AND_CPW_SPEC.md) | `web-admin/`, `tools/pw-patch-tool/` | painel administrativo e patcher (planejado) |
| `02_MIGRACAO_COMPATIBILIDADE_MULTI_REALM.sql` | — | migração histórica já aplicada; não editar |

Subpastas com dado de referência: `protocol/` (IR gerado pelo `pw-rpcgen`),
`elements_layouts/` (catálogo de layouts do `elements.data` + leitor Python),
`elements_155/` (`.cfg` do ADMVAL e a arqueologia do v156/v159), `mapas/` (terreno por
mapa), `clsconfig_155/` (leitor dos moldes de classe).

## Regra de manutenção

1. **Toda mudança de comportamento atualiza a spec da área no mesmo commit.** Mudou um
   layout, uma regra de jogo, uma porta, um formato lido, um estado de "falta" para
   "pronto": a linha correspondente muda junto.
2. **Spec descreve o presente.** Não é diário: sem "nesta sessão", sem narrativa. O relato
   com sintoma, causa e prova vai para o `docs/HISTORICO_DE_SESSOES.md` como item novo, e a
   spec cita o item (`B48`) quando a razão não é óbvia.
3. **Toda afirmação tem origem.** Número, layout ou regra cita de onde saiu: arquivo e linha
   do fonte original (`gs/npcsession.cpp:258`), IR, captura ou medida no arquivo de dados.
4. **Estado explícito** onde houver lista de funcionalidades: `confirmado` (visto em jogo),
   `testado` (teste automatizado, não visto em jogo), `parcial`, `falta`, `planejado`.
5. **Curta.** Uma spec que passa de ~400 linhas deve ser dividida. Tabela antes de prosa.
6. Ao fim, atualizar a data/commit do cabeçalho da spec tocada.
