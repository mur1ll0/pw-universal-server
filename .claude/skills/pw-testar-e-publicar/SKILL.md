---
name: pw-testar-e-publicar
description: Rodar a suíte de testes do pw-universal-server com o banco, publicar nos contêineres do realm de teste 155BR, ler logs e consultar o banco. Usar antes de dizer que algo funciona, ao publicar para o Murillo testar em jogo, e ao investigar um relato de teste.
---

# Testar e publicar

## 1. Suíte — sempre com o banco

```bash
TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \
  cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result|FAILED|panicked"
```

- Sem `TEST_DATABASE_URL` os testes de integração **passam sem verificar nada**.
- Referência (2026-09-14, B49): **512 passando, 2 falhando** — as duas do 1.2.6 em
  `pw-data-loader/tests/loader_tests.rs`. Qualquer outra falha é nova.
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
cd docker && docker compose build pw-world-155br pw-realm-155br \
  && docker compose up -d --remove-orphans pw-world-155br pw-realm-155br
```

São dois serviços: o link (29004) e um servidor de mundo com os mapas 1 e 161 (`WORLD_TAGS`). Mudou só dado do realm
(`data/realm_155BR/config`)? Basta `docker compose restart` dos mundos.

Conferir a subida:

```bash
docker logs --tail 40 pw-world-155br   # "N monstros, N NPCs e N recursos", terreno carregado
docker logs --tail 40 pw-realm-155br       # "Gateway pw-link escutando na porta 29004"
```

## 3. Ler um teste em jogo

```bash
docker logs --since 30m pw-realm-155br 2>&1 | grep -v "Gamedata recebido"
docker logs --since 30m pw-world-155br 2>&1 | grep -E "mundo:|WARN|ERROR"
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
