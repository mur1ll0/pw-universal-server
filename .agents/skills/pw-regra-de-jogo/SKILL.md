---
name: pw-regra-de-jogo
description: Portar ou corrigir uma regra de jogo do servidor original de Perfect World para o pw-gs — combate, habilidades, IA de monstro, experiência e nível, regeneração, recarga, reviver, missões, loja, visibilidade. Usar ao implementar mecânica em crates/pw-gs ou quando um comportamento em jogo difere do original.
---

# Regra de jogo = porte do `cgame/gs/`

Estado de cada regra e o que já foi portado: `specs/05_SIMULACAO_DO_MUNDO.md` — leia antes.

## 1. Achar a regra no original

Fonte: `F:\PW\1.5.5\EvolvedPWServer\cgame\gs\`. Pontos de entrada frequentes:

| assunto | onde procurar |
| :--- | :--- |
| golpe, dano, defesa | `actobject.cpp` (`AttackJudgement`, `HandleAttackMsg`), `player_template` |
| habilidade | stubs do cliente `EvolvedPWClient/ElementSkill/skillNNN.h` (`Calculate`, `GetExecutetime`, `GetCoolingtime`), `gs/skill*` |
| IA, movimento de NPC | `aipolicy.cpp`, `ainpc.cpp`, `npcsession.cpp`, `petnpc.cpp` |
| jogador (exp, nível, regeneração, reviver) | `player_imp.cpp/.h`, `player.cpp`, `playertemplate.cpp` |
| comandos do jogador | `playercmd.cpp` |
| serviços de NPC | `serviceprovider.cpp` |
| nascimento | `npcgenerator.cpp` |

```bash
grep -rn "<função ou constante>" "F:/PW/1.5.5/EvolvedPWServer/cgame/gs" --include=*.cpp --include=*.h
```

## 2. Portar

- **Unidades do original**, e diga quais no código: ms × ticks de 50 ms × segundos; velocidade
  ×256 no fio; deslocamentos inteiros (`>> 1`) em vez de divisão.
- **Ordem das operações** do original (piso/teto, arredondamento, inteiro × float).
- **Sorteio fora da conta** (como `combat::Rolagens`): a função recebe a rolagem e o teste usa
  número fechado.
- **Número do dado vem do dado** (`elements.data`, `ptemplate.conf`, `tasks.data`) — ver skill
  `pw-dados-do-realm`. Sem a fonte do número: não inventar; deixar `falta` documentado no
  código e na spec.
- Comentário com `arquivo:linha` do original na função portada.
- O que chega ao cliente passa pela skill `pw-protocolo-cliente`.
- Estado que precisa sobreviver: o autosave de 60 s grava o que está na entidade — mudar só o
  pacote e não a entidade é o defeito clássico (a experiência hoje).

## 3. Testar

- Unitário com números fechados tirados do original (ex.: `habilidades.rs` confere a conta do
  stub à mão).
- Integração em `crates/pw-gs/tests/` com mundo real (`subcomandos_no_mundo.rs`,
  `combate_real.rs`, `achados_do_teste_em_jogo.rs`), dados do realm e banco.
- Onde a VM 1.2.6 tiver captura do mesmo fluxo, compare (`pw-pcapdiff --subcomando N`).

## 4. Fechar

- Roteiro de teste em jogo para o Murillo: o que fazer, o que ver, que número esperar.
- Atualizar `specs/05_SIMULACAO_DO_MUNDO.md` (linha da regra, estado, constantes) pela skill
  `pw-atualizar-specs`.
