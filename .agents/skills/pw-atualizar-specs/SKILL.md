---
name: pw-atualizar-specs
description: Mantém specs/, docs/ESTADO_E_RETOMADA.md e docs/HISTORICO_DE_SESSOES.md em dia com o que foi alterado no pw-universal-server. Usar ao terminar qualquer bloco de trabalho que mude comportamento, antes de commitar, e sempre que uma spec se mostrar desatualizada.
---

# Atualizar as specs conforme o projeto muda

As specs são o cache de contexto do projeto: cada sessão lê uma spec curta em vez do código.
Spec desatualizada é pior que nenhuma — a próxima sessão constrói em cima de um fato falso.

## 1. O que mudou

```bash
git status --short
git diff --stat
git diff --name-only HEAD      # e, se já commitado: git diff --name-only <commit-do-estado>..HEAD
```

## 2. Qual spec cada caminho alterado exige

Use o mapa de `specs/README.md`. Resumo:

| alterou | atualizar |
| :--- | :--- |
| `crates/pw-protocol/src/packets`, `versions/`, `crates/pw-gs/src/comandos.rs`, tratamento de subcomando | `04_PROTOCOLO_MUNDO_3D.md` (§4 estratégia por versão, §5 fatos, §6 onde cada C2S é tratado) |
| `crates/pw-gs/src/{world,bus_server,ai,combat,habilidades,entity,npc}.rs` | `05_SIMULACAO_DO_MUNDO.md` (a linha da regra e o estado) |
| `crates/pw-data-loader/`, `specs/elements_*`, `specs/mapas`, dados do realm | `03_DATA_LOADER_SPEC.md` |
| `docker/`, `crates/pw-link/`, `crates/pw-bus/`, `version.rs`, `edition.rs` | `02_MULTI_REALM_ARCHITECTURE.md` |
| `crates/pw-storage/`, `scripts/*.sql`, esquema | `01_DATABASE_SCHEMA_POSTGRES.sql` (e conferir contra o banco) |
| crate novo, dependência entre crates, ferramenta nova | `00_MASTER_SPECIFICATION.md` |
| `web-admin/`, `tools/pw-patch-tool/` | `06_ADMIN_PANEL_AND_CPW_SPEC.md` |

Só teste, comentário ou refatoração sem mudança de comportamento: nenhuma spec — diga isso.

## 3. Como editar uma spec

- **Presente, não diário.** Troque a linha que ficou falsa; não acrescente "nesta sessão".
- **Estado explícito:** `confirmado` (Murillo viu em jogo), `testado` (teste automatizado),
  `parcial`, `falta`, `planejado`. Só `confirmado` com relato em jogo.
- **Origem de cada número/regra:** `arquivo:linha` do original, IR, captura ou medida.
  A narrativa fica no histórico; a spec cita o item (`B49`) quando o porquê não é óbvio.
- Números medidos (contagens, tamanhos) atualizados se mudaram.
- Atualize o cabeçalho: `Verificada contra o código em <data>, commit <hash>`.
- Passou de ~400 linhas: divida e registre no índice do `specs/README.md`.
- Criou área nova (crate, subsistema): nova linha no mapa do `specs/README.md`.

Depois de editar, procure afirmações antigas que contradizem a mudança:

```bash
grep -rn "<termo que mudou>" specs/ docs/ESTADO_E_RETOMADA.md
```

## 4. `docs/ESTADO_E_RETOMADA.md`

- §0: resumo e "o que está publicado e aguarda teste".
- §3: mover o que foi confirmado/testado; §3.3 com o roteiro de teste em jogo do que foi
  publicado.
- §5: tirar o que foi feito, acrescentar o que se descobriu que falta (com a evidência).
- §2: referência da suíte, se foi medida de novo.
- Cabeçalho: data e commit.

## 5. `docs/HISTORICO_DE_SESSOES.md`

Acrescente ao **fim da lista da série B** (depois do último item, antes de
"**Depois de "1.5.5 funcional" estar de fato provado**") um item com o próximo número:

```markdown
49. **Sessão AAAA-MM-DD: <o que mudou, em uma frase>.**

    ### a. Sintoma / pedido
    ### b. Causa, com a referência ao fonte (`arquivo:linha`) ou à medida
    ### c. Correção
    ### d. Provas (testes, números, log)
    ### e. O que continua faltando
```

Acrescente também a linha no índice da §8 do `ESTADO_E_RETOMADA.md`.

## 6. Conferência final

- `git diff --stat specs/ docs/` mostra as specs certas tocadas.
- Nenhuma spec ou estado diz algo que o diff acabou de tornar falso.
