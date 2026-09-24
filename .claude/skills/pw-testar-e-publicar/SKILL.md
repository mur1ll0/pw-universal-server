---
name: pw-testar-e-publicar
description: Rodar a suíte de testes do pw-universal-server com o banco, publicar nos contêineres do realm de teste 155, ler logs e consultar o banco. Usar antes de dizer que algo funciona, ao publicar para o Murillo testar em jogo, e ao investigar um relato de teste.
---

# Testar e publicar

> **Nenhum comando desta skill roda sem filtro de saída.** A suíte inteira despejada no
> contexto custa mais de dez mil tokens para dizer "passou". Os comandos abaixo já vêm com
> o filtro certo — use-os como estão, e relate o consumo por etapa (ver o agente
> `pw-server-dev`, seção "Contexto é recurso").

## 1. Suíte — sempre com o banco, sempre filtrada

A rodada inteira demora mais de 10 minutos: mande para **segundo plano** e leia só o fim.

```bash
TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \
  cargo test --workspace --no-fail-fast -- --test-threads=2 2>&1 \
  | grep -E "^test result:" | awk '{p+=$4; f+=$6} END {print "passaram:", p, "falharam:", f}'
```

Duas linhas de saída, e o número que interessa. Para ver **quais** falharam, troque o filtro
por `grep -E "^test .* FAILED|^test result: FAILED"` — só os nomes, sem o rastro de pânico.
Só vá ao `panicked at` de um teste específico depois de saber qual é:

```bash
... cargo test -p pw-gs --test subcomandos_no_mundo <nome_do_teste> 2>&1 | tail -20
```

- Sem `TEST_DATABASE_URL` os testes de integração **passam sem verificar nada**.
- **`--test-threads=2`**: no paralelismo máximo, `aceitar_forma_o_grupo…` e
  `a_consulta_de_jogador…` falham por contenção no pool do Postgres e passam isoladas (B74,
  B80). Com 2 fios a rodada inteira passa — é assim que vale a pena medir.
- Referência (2026-09-24, B98): **674 testes, 0 falhas** com dois fios.
- Teste que cria dado no banco começa com `comum::limpar_sobras_de_teste(&pool)`
  (`crates/pw-storage/tests/comum/mod.rs`): apaga sobras de execuções anteriores. Teste novo
  que cria realm ou conta usa esse módulo e os mesmos padrões de nome (`t_*`, e conta com
  prefixo listado no SQL) — senão volta a sujar o banco.
- Por crate, para iterar: `cargo test -p pw-gs --test subcomandos_no_mundo`.
- Binário de teste velho no Windows: se o comportamento não bate com o código, `touch` no
  arquivo de teste.
- Se a referência mudar, atualizar `docs/ESTADO_E_RETOMADA.md` §2 e esta skill.

## 2. Publicar no realm de teste (só quando o Murillo pediu)

```bash
cd docker && docker compose build pw-world-155 pw-realm-155 2>&1 | tail -5 \
  && docker compose up -d --remove-orphans pw-world-155 pw-realm-155 2>&1 | tail -5
```

O `tail -5` não é enfeite: o build despeja centenas de linhas de compilação que ninguém lê.
Se o build falhar, aí sim `| grep -E "^error" -A 5`.

São dois serviços: o link (29004) e um servidor de mundo com os mapas 1 e 161 (`WORLD_TAGS`). Mudou só dado do realm
(`data/realm_155/config`)? Basta `docker compose restart` dos mundos. O 1.2.6 é igual, com
`pw-world-126 pw-realm-126` (link na 29000, dados em `data/realm_126/config`).

Conferir a subida:

```bash
docker logs --tail 40 pw-world-155   # "N monstros, N NPCs e N recursos", terreno carregado
docker logs --tail 40 pw-realm-155       # "Gateway pw-link escutando na porta 29004"
```

## 3. Ler um teste em jogo

```bash
docker logs --since 30m pw-realm-155 2>&1 | grep -v "Gamedata recebido"
docker logs --since 30m pw-world-155 2>&1 | grep -E "mundo:|WARN|ERROR"
```

"subcomando N ainda não tratado" no mundo = comando que o cliente mandou e ninguém trata
(consulte `python tools/pw-ir/consultar_ir.py c2s N`).

Do lado do cliente: `F:\PW\1.5.5\1.5.5 BR\Perfect World 1.5.5 BR\element\logs\EC.log` (e
`AF.log`) e o overlay `d_rtdebug`.

## 4. Banco

```bash
docker exec pw-postgres psql -U pw_admin -d pw_database -c "<sql>"
```

- Podem existir realms `t_*` das rodadas dos últimos 15 minutos: filtre com
  `where realm_id not like 't\_%'`. Contas reais: `admin` e `testuser`.
- Personagem em jogo: o `pw-gs` sobrescreve nível/exp/moedas/posição no autosave de 60 s —
  editar pelo banco só com o personagem fora do jogo.
- Correção de dado de realm vira script em `scripts/AAAA_MM_DD_<assunto>.sql`, com cabeçalho
  explicando o porquê e cláusula que protege quem já jogou.

## 5. O que entregar ao Murillo

Resultado da suíte com números, o que foi publicado, e o roteiro: o que fazer em jogo, o que
deve aparecer, e o que olhar no log/overlay se não aparecer.

E a **tabela de consumo por etapa** (análise, implementação, testes, documentação), com o
total do bloco. Etapa cara ganha uma linha de explicação.
