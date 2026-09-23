# pw-universal-server — instruções para o Codex

Reimplementação em Rust do servidor do MMO Perfect World que serve o **cliente original sem
modificação**. Dono do projeto: Murillo. Idioma de tudo (conversa, documentação, commits,
identificadores novos): **português**.

Estas são as mesmas diretrizes do agente do Claude (`.claude/agents/pw-server-dev.md`). As
duas ferramentas trabalham no mesmo repositório e seguem as mesmas regras. Se algo mudar
aqui, mude lá também, e vice-versa.

## Onde o projeto está

- **1.5.5 (realm `realm_155`, porta 29004, cliente BR v156)**: jogável no básico desde
  2026-09-23 (login, criação, mundo, combate, habilidades, missões, loja, montaria,
  teleporte). O que falta está em `docs/ESTADO_E_RETOMADA.md` §5.
- **1.2.6 (realm `realm_126`)**: **frente atual.** Levar ao 1.2.6 o que o 1.5.5 já resolveu,
  pelo `WorldProtocol` da versão (`crates/pw-protocol/src/versions/v126/`). Painel de
  paridade em `docs/ESTADO_E_RETOMADA.md` §5D.
- Depois: banco (Contexto E), painel `pw-admin` (G), atualizador/launcher (H).

## Início de toda sessão

Siga `.claude/skills/pw-retomar-sessao/SKILL.md` (é um arquivo: leia com `sed`/`cat`).
Leia **só** isto antes do primeiro pedido:

| ler | para quê |
| :--- | :--- |
| `docs/ESTADO_E_RETOMADA.md` §0, §3, §5 | onde o trabalho está e a fila |
| `specs/README.md` → **só** a spec da área | como o sistema é |
| `docs/HISTORICO_DE_SESSOES.md` | só o item citado (`grep -n "B87"`), **nunca inteiro** |

As "skills" do projeto são guias em `.claude/skills/<nome>/SKILL.md`. Leia o guia da tarefa
antes de começá-la:

| tarefa | guia |
| :--- | :--- |
| início de sessão, retomada | `pw-retomar-sessao` |
| comando do mundo 3D (S2C/C2S) ou pacote GNET, "não aparece na tela" | `pw-protocolo-cliente` |
| ler/ligar arquivo de dados (`elements.data`, `tasks.data`, `npcgen.data`, `.hmap`…) | `pw-dados-do-realm` |
| portar regra de jogo do `cgame/gs/` | `pw-regra-de-jogo` |
| rodar testes com o banco, publicar nos contêineres, ler logs | `pw-testar-e-publicar` |
| fechar um bloco de trabalho, antes de commitar | `pw-atualizar-specs` |

## Regras que não se negociam

1. **Evidência, nunca palpite.** Todo layout, número, unidade ou regra sai do fonte original
   (`F:\PW\1.5.5\EvolvedPWServer\cgame\gs`, `F:\PW\1.5.5\EvolvedPWClient`,
   `F:\PW\1.7.2\172Source` para campos mais novos; para o 1.2.6, os fontes 1.5.3 e as
   capturas em `docs/evidencias/126/`), do IR em `specs/protocol/`, de captura ou do próprio
   arquivo de dados. Cite `arquivo:linha` no código. Onde não há evidência, o código diz que
   falta. Não invente número.
2. **O binário do cliente é o juiz** quando fonte e binário discordam.
3. **S2C com tamanho errado é descartado em silêncio pelo cliente.** Todo codificador novo ou
   alterado segue `pw-protocolo-cliente`.
4. **Arquivo de dados: o carregador do cliente é o juiz, e o arquivo fecha no último byte.**
5. **Regra de jogo é porte do `cgame/gs/`**, nas unidades do original.
6. **Teste só vale com o banco** (`TEST_DATABASE_URL`; o `pw-storage` isola tudo no schema
   `test`). Sem ele os testes de integração passam sem verificar nada. Informe o resultado com
   números. Falha é falha.
7. **Um caminho de escrita por layout.** Diferença entre versões só no `WorldProtocol` da
   versão (`versions/v126`, `v155`…): nada de `if versao == …` espalhado.
8. **Nada que espere o banco no caminho do jogo** (o `world.tick` segura o mundo inteiro).
9. Não commite, não faça push e não publique nos contêineres sem o Murillo pedir, a menos que
   ele já tenha pedido isso para a tarefa em curso.

## Specs: atualizar junto com o código (obrigatório)

`specs/` é a descrição **curta e atual** do sistema. É ela que permite ler pouco em cada
sessão, e só serve se estiver certa.

- **Toda alteração de comportamento atualiza a spec da área na mesma entrega** (mapa
  caminho→spec em `specs/README.md`): layout, regra, formato lido, porta, serviço, e item que
  passou de `falta` para `testado`/`confirmado`.
- Ao fechar um bloco (antes de dizer que acabou ou de commitar), siga `pw-atualizar-specs`:
  spec da área, `docs/ESTADO_E_RETOMADA.md` (§0, §3, §5) e um item novo no
  `docs/HISTORICO_DE_SESSOES.md`. O item é o próximo número da série B (lista numerada
  `NN. **título**`): `grep -oE '^[0-9]+\. \*\*' docs/HISTORICO_DE_SESSOES.md | tail -1`.
  **Nunca reutilize um número** — Claude e Codex escrevem na mesma série, e a colisão de
  B77–B81 custou uma renumeração no merge de 2026-09-23 (B93).
- Se descobrir que uma spec está errada, corrija na hora e avise.
- Se a mudança não altera nada descrito (refatoração, teste, comentário), diga isso em uma
  linha.

## Contexto é recurso: economize tokens

O que gasta a janela de contexto é **saída de ferramenta que ninguém vai ler**. Um
`cargo test --workspace` sem filtro custa mais de dez mil tokens para responder se passou ou
não.

**Regra: cada comando traz para o contexto só o que vai ser lido.**

- Comando com muita saída sempre termina em filtro (`grep -E`, `awk`, `tail -n`, `head -n`).
  Nunca rode `cargo test`, `cargo build`, `docker compose build` ou `docker logs` sem filtro.
- Precisa de um número? Calcule no shell e traga só o número:
  ```bash
  TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \
    cargo test --workspace --no-fail-fast -- --test-threads=2 2>&1 \
    | grep -E '^test result|FAILED|panicked|^error' \
    | awk '/^test result/{p+=$4; f+=$6; next} {print} END {print "passou", p, "falhou", f}'
  ```
- Fonte C++: `grep -n` para achar a faixa, depois `sed -n 'A,Bp'` só nela. Nunca abra o
  arquivo inteiro.
- Não leia o histórico inteiro, o código inteiro nem specs de outras áreas "por garantia".
- Arquivos intermediários (scripts de edição, saídas de teste) ficam em uma pasta temporária,
  fora do repositório.
- Rodada longa (suíte, build de Docker): rode com a saída já filtrada e leia só o final. Se o
  build do Docker for pesado, ofereça ao Murillo o comando pronto para ele rodar.

## Relatório de consumo por etapa (obrigatório)

O Murillo usa esse relatório para ver se a sessão está eficiente. Anote o consumo no início do
trabalho e ao fim de cada etapa. Use o contador de tokens da sessão quando ele estiver
disponível (`/status` no Codex CLI ou o uso mostrado pela interface). Se não estiver, estime:
cerca de 1 token para cada 4 caracteres de tudo o que entrou no contexto (saídas de comando,
arquivos lidos) e saiu (código escrito). Nesse caso, diga que é estimativa.

Feche **toda resposta que encerra um bloco de trabalho** com esta tabela:

| etapa | consumo |
| :--- | ---: |
| 1. Análise (fonte original, IR, evidência) | ~N mil |
| 2. Implementação + testes | ~N mil |
| 3. Suíte + specs + docs | ~N mil |
| **total do bloco** | **~N mil** |

As etapas são fases do trabalho, não chamadas de ferramenta. Quando uma etapa sair cara, diga
**por quê** em uma linha (quase sempre é leitura de fonte C++), para a próxima sessão melhorar.

## Com o Murillo

- Ele testa em jogo com clientes originais (1.5.5 BR, e o 1.2.6). Ao entregar algo visível,
  diga **exatamente o que olhar na tela, em que ordem, e o que esperar no log e no overlay**
  (`d_rtdebug` no 1.5.5).
- O relato dele em jogo é evidência. Hipótese sua não é. Separe sempre "corrigido e testado",
  "corrigido, falta ver em jogo" e "diagnosticado".

## Infra (resumo; detalhe em `pw-testar-e-publicar`)

- Docker local: `docker/docker-compose.yml`. Contêineres `pw-realm-155`/`pw-world-155` e
  `pw-realm-126`/`pw-world-126`. PostgreSQL em `127.0.0.1:5432` (`pw_database`), Dragonfly.
- Dados de realm: `data/realm_155/config`, `data/realm_126/config`.
