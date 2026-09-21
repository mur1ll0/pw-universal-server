---
name: pw-retomar-sessao
description: Início de sessão no pw-universal-server — lê o estado atual, confere repositório e contêineres, e escolhe só a spec da área a tocar. Usar no começo de toda sessão nova e ao retomar depois de compactação de contexto.
---

# Retomar a sessão com o mínimo de contexto

Objetivo: saber onde o trabalho está e o que o pedido exige **sem** ler o código inteiro nem
o histórico de 6.500 linhas.

## 1. Estado (sempre)

Leia do `docs/ESTADO_E_RETOMADA.md`:

- o cabeçalho (data e commit da última atualização);
- **§0 "Em uma tela"**;
- **§3.3** (o que está publicado e aguarda teste em jogo);
- **§5 "O que falta"** — a fila.

As outras seções (ambiente, clientes, material em disco, regras) só quando o pedido precisar.

## 2. O que mudou desde a última atualização

```bash
git status --short
git log --oneline -15
docker ps --format "{{.Names}}\t{{.Status}}"
```

Se há commits **depois** do commit citado no cabeçalho do estado, leia as mensagens deles:
o estado pode estar atrasado. Se estiver, avise e atualize (skill `pw-atualizar-specs`) antes
de construir em cima.

## 3. Escolher a spec

Abra `specs/README.md` e, pelo mapa "cobre (caminhos)", leia **só** a(s) spec(s) da área do
pedido:

| pedido sobre | spec |
| :--- | :--- |
| login, portas, docker, realms, versão, barramento | `02_MULTI_REALM_ARCHITECTURE.md` |
| `elements`/`tasks`/`npcgen`/`aipolicy`/`.conf`/`.hmap` | `03_DATA_LOADER_SPEC.md` |
| comando que não aparece na tela, layout, `PorVersao`, IR | `04_PROTOCOLO_MUNDO_3D.md` |
| combate, IA, habilidade, visibilidade, loja, exp, reviver | `05_SIMULACAO_DO_MUNDO.md` |
| banco | `01_DATABASE_SCHEMA_POSTGRES.sql` |
| painel, patcher | `06_ADMIN_PANEL_AND_CPW_SPEC.md` |
| visão geral, crates | `00_MASTER_SPECIFICATION.md` |

Evidência de um item citado (`B42`, `A46`): procure só ele no histórico,
`grep -n "^42\. \*\*" docs/HISTORICO_DE_SESSOES.md` (a série B fica depois de
"## 4. Próximo passo"; a A em "## 2."), e leia a partir dali.

## 4. Responder

Em poucas linhas, antes de começar o trabalho: onde o projeto está, o que aguarda teste, e
como o pedido se encaixa na fila (ou se muda a prioridade). Se o pedido for vago ("continua"),
proponha o primeiro item de §3.3 (teste pendente) ou de §5A.
