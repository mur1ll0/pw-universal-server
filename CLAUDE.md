# pw-universal-server

Reimplementação em Rust do servidor de Perfect World que serve o cliente original. Alvo
atual: 1.5.5 (realm `realm_155`). Idioma do projeto: português.

## Toda sessão

Esta pasta configura a thread principal como o agente **`pw-server-dev`**
(`.claude/settings.json` → `.claude/agents/pw-server-dev.md`). Se por algum motivo ele não
estiver ativo, siga as mesmas instruções: comece pela skill **`pw-retomar-sessao`**.

| ler | para quê |
| :--- | :--- |
| `docs/ESTADO_E_RETOMADA.md` §0, §3.3, §5 | onde o trabalho está e a fila |
| `specs/README.md` → **só** a spec da área | como o sistema é |
| `docs/HISTORICO_DE_SESSOES.md` | só o item citado (`grep`), nunca inteiro |

Skills do projeto: `pw-retomar-sessao`, `pw-atualizar-specs`, `pw-protocolo-cliente`,
`pw-dados-do-realm`, `pw-regra-de-jogo`, `pw-testar-e-publicar`.

## Atualizar as specs conforme altera o projeto

**Toda alteração de comportamento atualiza a spec da área na mesma entrega** (mapa
caminho→spec em `specs/README.md`), mais `docs/ESTADO_E_RETOMADA.md` e um item no
`docs/HISTORICO_DE_SESSOES.md` ao fim do bloco — skill `pw-atualizar-specs`. As specs são
o que permite a cada sessão ler pouco; desatualizadas, fazem a próxima sessão construir
sobre fato falso. Um hook de fim de turno (`.claude/hooks/lembrar_specs.sh`) cobra isso
quando há código alterado sem spec nem estado alterados.

## Regras que não se negociam

- Evidência do original (fonte C++ com `arquivo:linha`, IR, captura, arquivo de dados) —
  nunca palpite nem número inventado.
- O binário do cliente é o juiz; S2C de tamanho errado é descartado em silêncio.
- Arquivo de dados fecha no último byte, pelo carregador do cliente.
- Testes só valem com `TEST_DATABASE_URL` definido (o `pw-storage` isola tudo no schema `test`, preservando o `public`).
- **Contexto é recurso:** saída de ferramenta entra filtrada (`grep`/`awk`/`tail`), rodada
  longa vai para segundo plano, e o que se traz é o número, não a lista. Cada bloco de
  trabalho termina com a tabela de consumo de tokens por etapa — agente `pw-server-dev`,
  seção "Contexto é recurso", e skill `pw-testar-e-publicar`.
