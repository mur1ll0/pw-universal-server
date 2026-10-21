# Estudo: como editar e criar habilidades no cliente (B312)

> Pedido do Murillo (2026-10-20): análise de viabilidade, com engenharia reversa do cliente, para saber se é preciso
> editar instruções/recompilar o `elementskill.dll` ou se dá para levar as habilidades para outro sistema (o
> `elements.data`, uma tabela) mais fácil de manter. Este documento é a evidência, as opções e a recomendação.
> Contexto: B301–B311 (`docs/HISTORICO_DE_SESSOES.md`), spec 03 §3.10b e spec 07.

## 1. A pergunta

Hoje, no Estúdio:
- **servidor:** tudo editável no catálogo do realm (números, dano, roteiros de efeito, flags; B302–B310);
- **cliente:** só os requisitos (nível, SP, dinheiro, livro) e 6 manas, trocando bytes de tabelas medidas no DLL
  (B305–B311).

O resto do que o cliente confere ou mostra não se edita: quase toda a mana, a recarga, a execução, o alcance, a
descrição. Habilidade nova também não se cria.

## 2. O que o cliente realmente usa (medido no binário e no fonte)

| fato | evidência |
| :--- | :--- |
| O executável importa **36** funções do `elementskill.dll` no 1.5.5 e **23** no 1.2.6, não as 121 que ele exporta. | `dumpbin -imports` do `ELEMENTCLIENT.EXE` |
| Além delas, chama cerca de 36 métodos virtuais do objeto de `Create(id, nível)`, sem ler campos internos. | `m_pSkillCore->…` em `ElementClient/*.cpp` |
| A interface só passa tipos simples, 5 estruturas (`LearnRequirement`, `UseRequirement`, `PetRequirement`, `GoblinRequirement`, `GoblinUseRequirement`) e a interface abstrata `skill_array_interface<T>`; nenhuma assinatura importada leva STL. | `undname` das 36 importações |
| Executável e DLL são **Visual C++ 6.0** (linker 6.00; DLL 1.5.5 de 2015, 1.2.6 de 2007). | `dumpbin -headers` |
| As estruturas são iguais no fonte 1.5.3 e no 1.5.5. | `ElementSkill.h` dos dois |
| Os stubs `skillNNN.h` são fonte compartilhado com o servidor: os **efeitos** (`StateAttack`, `BlessMe`, `TakeEffect`, ódio) ficam sob `#ifdef _SKILL_SERVER` e **não estão no DLL do cliente**. | `ElementSkill/skill15.h`; `ElementSkill.vcxproj` define `_SKILL_CLIENT` |
| No cliente, cada stub tem só números por nível (mana, recarga, execução, alcance, raio, ângulo, precisão, requisitos), a **descrição** e, raramente, uma condição. | contagem nos 3.316 stubs: `GetMpcost` 3.316, `GetIntroduction` 3.316, `GetRequiredLevel` 3.130… |
| A descrição é `_snwprintf(formato do skillstr.txt, argumentos)`: em **3.309** habilidades os argumentos só dependem do nível, em 7 também da proficiência (que vem do servidor) e 1 não tem descrição. | `GetIntroduction` dos stubs |
| Código especial de verdade: `CheckComboSkExtraCondition` (18 stubs), `ComboSkEndAction` (8), `CheckHpCondition` (2). Cada um é uma expressão de uma linha (argumento de combo, faixa de vida em %). | stubs |
| `LoadSkillData` é só a lista de habilidades aprendidas que o servidor manda (`cmd_skill_data`). | `skillwrapper.cpp:17`, `EC_HostMsg.cpp:3597` |
| `VisibleState`/`TeamState` (ícones de estado) são 493 linhas de **dado** em `skill.cpp`. | `skill.cpp` |
| O `elements.data` **não define** habilidade nenhuma: só a referencia (serviço de NPC, livro, item, mascote). O cliente pega nome e descrição do `skillstr.txt` e o resto do DLL. | layouts; `EC_Skill.cpp:64,215` |
| O DLL instalado **não é o fonte**: o dinheiro do BR difere (B303); a mana quase toda é constante compartilhada entre funções ou fórmula no código (B311). | B303, B311 |

**Conclusão técnica.** Do ponto de vista do cliente, uma habilidade é **dado**: números por nível, uma descrição com
argumentos por nível e uma meia dúzia de condições de uma linha. Isso não está em dado no DLL atual — está compilado
como código, e por isso editar dá trabalho. Mas a interface que o executável usa é pequena, estável e sem STL.

## 3. As opções

### A. Ficar como está (trocar bytes de tabelas medidas)

- Já pronto (B305–B311).
- Cobre requisitos e 1/5 manas. Não cobre recarga, execução, alcance, descrição nem quase nenhuma mana. Não cria
  habilidade.
- Risco zero novo, alcance pequeno.

### B. Editar instruções do DLL atual

- Trocar a constante imediata de cada função (`mov eax, 8000`), ou dar a cada `GetMpcost` uma constante própria
  numa seção nova.
- **Exige um desmontador e um mapa por função** para cada um dos ~15 campos. Fórmulas (`24 + 15 × nível`) não se
  editam como número: teria de reescrever código.
- Não cria habilidade.
- Risco alto, frágil (vale para um binário), trabalho grande para alcance médio. **Não recomendado.**

### C. Recompilar o `elementskill.dll` a partir do fonte

- O fonte existe e tem projeto (`ElementSkill.vcxproj`, toolset v142).
- **Problemas:**
  - o fonte não é o BR (dinheiro diferente, B303) — teria de ser corrigido pelo binário antes;
  - cada edição vira gerar C++ e compilar com MSVC x86 — toolchain do Windows na esteira do Estúdio, ou `clang-cl`
    em contêiner;
  - não há fonte do DLL do 1.2.6.
- Habilidade nova: sim (stub novo).
- Risco médio; manutenção pesada (código gerado + compilador em cada edição).

### D. **DLL de dados**: um `elementskill.dll` nosso, movido a tabela (recomendado)

- Um DLL novo, escrito por nós, que exporta as **mesmas 36 funções** (23 no 1.2.6) e devolve objetos com a **mesma
  vtable** (ordem medida no binário, B303/B304) — mas sem nenhum código por habilidade: tudo vem de um arquivo de
  dados ao lado dele (ex.: `element\data\habilidades.dat`).
- O arquivo de dados é gerado pelo Estúdio a partir do **mesmo modelo** que gera o catálogo do servidor: uma fonte da
  verdade, cliente e servidor nunca mais divergem (o problema do B303 some).
- **Como garantir que é igual ao original:** antes de existir, mede-se o DLL original inteiro — todo método, toda
  habilidade, todo nível, os argumentos exatos da descrição (sondando o `GetIntroduction` com o tipo de cada `%` do
  próprio formato) e as condições com entradas variadas. O DLL de dados só vale se reproduz **100%** dessa medição,
  o mesmo critério do B305–B311.
- **Habilidade nova:** uma linha na tabela, mais o que já se edita (nome e descrição no `skillstr.txt`, ícone,
  efeito visual, ensino no `elements`, catálogo do servidor).
- Trabalho: um DLL de C++ pequeno, compilado uma vez (VS2022 x86); depois, toda edição é dado.
- **Riscos:**
  - a interface tem de ser exata — mitigado pela medição diferencial;
  - o 1.2.6 não tem fonte do DLL: a interface dele sai do binário (23 importações; estruturas a confirmar por
    desmontagem);
  - comportamento do cliente fora do que o DLL devolve não muda (animação e efeito visual continuam do modelo e do
    `gfx`).

### E. Levar as habilidades para o `elements.data` (ou outra tabela do sistema)

- **Impossível sem modificar o executável.** O cliente não lê habilidade do `elements` (seção 2); ele pergunta tudo
  ao `elementskill.dll`.
- A opção D é a versão viável desta ideia: a "tabela do sistema" passa a ser o arquivo de dados que o nosso DLL lê,
  e o `elements` continua com o que já tem (quem ensina, livros).

### F. Modificar o executável

Fora do princípio do projeto (cliente original) e mais arriscado que D. **Não recomendado.**

## 4. Comparação

| | A (bytes) | B (instruções) | C (recompilar) | **D (DLL de dados)** | E (elements) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| mana, recarga, alcance, execução | parcial | sim, frágil | sim | **sim** | — |
| descrição (números) | não | não | sim | **sim** | — |
| habilidade nova | não | não | sim | **sim** | — |
| editar sem compilar | sim | sim | não | **sim** | — |
| cliente = servidor (uma fonte) | não | não | não | **sim** | — |
| 1.2.6 | sim | difícil | sem fonte | interface pelo binário | — |
| risco | nenhum | alto | médio | médio, medido | impossível |

## 5. Recomendação

**Opção D**, em fases com portão. Nenhuma fase vai ao jogo sem a anterior provada:

1. **Contrato** — a interface exata do 1.5.5:
   - as 36 importações;
   - a vtable de `ElementSkill`, `VisibleState` e `TeamState`;
   - as 5 estruturas e o `skill_array_interface`.

   Tudo do binário, conferido com o fonte.
2. **Medição completa do original** — todo valor que o cliente pode pedir, para toda habilidade e nível: números,
   argumentos exatos de descrição, condições sondadas, estados. É o "gabarito".
3. **DLL de dados 1.5.5** e **diferencial**: o gabarito medido no nosso DLL tem de dar 100% igual.
4. **Em jogo** (Murillo, dois clientes 1.5.5): o mesmo cliente com o DLL de dados, sem nenhuma diferença visível.
5. **Estúdio**:
   - o arquivo de dados gerado do modelo único;
   - edição de todos os campos do cliente;
   - o catálogo do servidor sai do mesmo modelo.
6. **Habilidade nova** de ponta a ponta.
7. **1.2.6** pelo mesmo caminho (interface pelo binário).

Até a fase 4 nada muda para o jogador: o DLL de dados devolve exatamente o que o original devolve.

## 6. O que precisa da decisão do Murillo

- **Princípio do projeto:** o servidor serve "o cliente original sem modificação". O B305 já distribui um
  `elementskill.dll` editado pelo launcher; a opção D distribui um `elementskill.dll` **nosso**. Na prática é a mesma
  categoria (arquivo do cliente distribuído pelo launcher), mas troca o binário inteiro. Precisa do seu aceite.
- **Por onde começar:** 1.5.5 (tem fonte para conferir) ou 1.2.6 (frente atual, sem fonte do DLL). A recomendação é
  1.5.5 primeiro, porque o fonte acelera o contrato e o 1.2.6 herda o método.
- **Formato do arquivo de dados:** binário próprio (rápido, pequeno) ou texto/JSON (legível). A recomendação é binário
  com cabeçalho de versão, gerado e lido por código nosso.
