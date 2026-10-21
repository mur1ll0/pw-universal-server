# 08 — `elementskill.dll` de dados (opção D)

> Estado: **fases 1 (contrato, B313), 2 (gabarito, B314) e 3 (DLL de dados com diferencial 100%, B315–B317)
> concluídas; fase 4 (jogo) confirmada pelo Murillo em 2026-10-21 — habilidades usáveis, níveis corretos**.
> Fase 5 em andamento (B321): o DLL de dados e o `habilidades.pwsk` publicados no `pw-recursos` 1.5.5 (versão 2, o
> launcher distribui), e o Estúdio lê e edita os números do cliente no `.pwsk`. Decisão do Murillo (2026-10-20): seguir a opção D do
> `docs/admin/ESTUDO_HABILIDADES_DO_CLIENTE.md`, começando pelo 1.5.5, e distribuir o nosso DLL pelo launcher.
> Fases: 1 contrato · 2 gabarito (medir o original inteiro) · 3 DLL de dados + diferencial 100% · 4 jogo · 5 Estúdio ·
> 6 habilidade nova · 7 1.2.6.

Objetivo: um `elementskill.dll` nosso que o `ELEMENTCLIENT.EXE` 1.5.5 BR (v156) carregue no lugar do original, com o
**mesmo contrato binário** e **nenhum código por habilidade**: tudo vem de um arquivo de dados gerado pelo Estúdio a
partir do mesmo modelo do catálogo do servidor.

## 1. O que vale como contrato

O executável e o DLL instalados são Visual C++ 6.0 (linker 6.00; DLL de 2015-12-22, exe de 2016-11-09). O fonte
`EvolvedPWClient/ElementSkill` **não é** o que gerou esse DLL: além dos números (B303), a interface é mais nova —
`skill_array_interface`, `GetJuniorCount/At`, `GetRequiredSkillCount/At` não existem em fonte nenhum que temos.
Regra: **assinaturas e layout pelo binário; comportamento pelo fonte, conferido no gabarito (fase 2)**.

Ferramentas: `tools/elementskill/numeros.cpp` (B303), `enderecos.cpp` (B305), `estruturas.cpp` (B313), `dumpbin
-imports/-exports/-disasm` e `undname` do VS2022.

## 2. As 36 funções que o executável importa (1.5.5)

`dumpbin -imports ELEMENTCLIENT.EXE`, demangled. O DLL original exporta 121; só estas são chamadas pelo nome.

- `static int ElementSkill::Condition(unsigned int,UseRequirement &,int)`
- `static ElementSkill * ElementSkill::Create(unsigned int,int)`
- `void ElementSkill::Destroy(void)`
- `static int ElementSkill::GetAbility(unsigned int)`
- `static int ElementSkill::GetAbilityPercent(unsigned int)`
- `static void ElementSkill::GetComboSkActivated(ComboSkillState const &,skill_array_interface<pair<unsigned int,int> > &)`
- `static int ElementSkill::GetComboSkPreSkill(unsigned int)`
- `static int ElementSkill::GetCommonCoolDown(unsigned int)`
- `static char const * ElementSkill::GetEffect(unsigned int)`
- `static int ElementSkill::GetExecuteTime(unsigned int,int)`
- `static char const * ElementSkill::GetIcon(unsigned int)`
- `static void ElementSkill::GetInherentSkills(int,skill_array_interface<unsigned int> &)`
- `static int ElementSkill::GetMaxAbility(unsigned int,int)`
- `static unsigned short const * ElementSkill::GetName(unsigned int)`
- `static char const * ElementSkill::GetNativeName(unsigned int)`
- `static int ElementSkill::GetRequiredBook(unsigned int,int)`
- `static int ElementSkill::GetRequiredLevel(unsigned int,int)`
- `static int ElementSkill::GetRequiredMoney(unsigned int,int)`
- `static int ElementSkill::GetRequiredRealmLevel(unsigned int,int)`
- `static int ElementSkill::GetRequiredSp(unsigned int,int)`
- `static char ElementSkill::GetType(unsigned int)`
- `static int ElementSkill::GetVersion(void)`
- `static int ElementSkill::GoblinCondition(unsigned int,GoblinUseRequirement &,int)`
- `static int ElementSkill::GoblinLearn(unsigned int,GoblinRequirement &,int)`
- `static void ElementSkill::InitStaticData(void)`
- `static bool ElementSkill::IsGoblinSkill(unsigned int)`
- `static bool ElementSkill::IsMovingSkill(unsigned int)`
- `static bool ElementSkill::IsOverridden(unsigned int)`
- `static int ElementSkill::LearnCondition(unsigned int,LearnRequirement &,int)`
- `static void ElementSkill::LoadSkillData(void *)`
- `static unsigned int ElementSkill::NextSkill(unsigned int)`
- `static int ElementSkill::PetLearn(unsigned int,PetRequirement &,int)`
- `static TeamState const * TeamState::Query(int)`
- `static VisibleState const * VisibleState::Query(int,int)`
- `static int ElementSkill::SetAbility(unsigned int,int)`
- `static int ElementSkill::SetLevel(unsigned int,int)`

No 1.2.6 são 23 (sem `GetComboSkActivated`, `GetComboSkPreSkill`, `GetCommonCoolDown`, `GetInherentSkills`,
`GetMaxAbility`, `GetRequiredLevel`, `GetRequiredRealmLevel`, `Goblin*`, `IsGoblinSkill`, `IsMovingSkill`,
`IsOverridden`, `InitStaticData`) — fase 7.

## 3. A vtable de `ElementSkill` (53 slots)

Medida no DLL instalado: a vtable exportada da base (`??_7ElementSkill@GNET@@6B@`) contra os endereços das virtuais
da base exportadas; 50 slots com nome. Os slots 16–18 são puras (`_purecall` repetido), identificadas desmontando a
classe derivada (`Create(1, 1)`): leem o vetor de pré-requisitos do stub (`+0x42`, pares de 8 bytes) — o mesmo padrão
de `GetJuniorCount/At/GetJunior` (42–44). O objeto é criado e destruído pelo DLL (`Create`/`Destroy`); o executável
não lê campo nenhum dele.

| slot | método |
| ---: | :--- |
| 0 | `virtual unsigned short const * GetName(void)` |
| 1 | `virtual char const * GetNativeName(void)` |
| 2 | `virtual char GetType(void)const ` |
| 3 | `virtual char const * GetIcon(void)` |
| 4 | `virtual unsigned short const * GetIntroduction(unsigned short *,int,SkillStr const &)` |
| 5 | `virtual int GetCls(void)const ` |
| 6 | `virtual int GetCoolingTime(void)` |
| 7 | `virtual int GetExecuteTime(void)` |
| 8 | `virtual int GetTargetType(void)` |
| 9 | `virtual float GetPrayRange(float,float)` |
| 10 | `virtual int GetRank(void)` |
| 11 | `virtual int GetRequiredLevel(void)` |
| 12 | `virtual int GetRequiredSp(void)` |
| 13 | `virtual int GetRequiredBook(void)` |
| 14 | `virtual int GetRequiredMoney(void)` |
| 15 | `virtual int GetRequiredRealmLevel(void)` |
| 16 | `virtual int GetRequiredSkillCount(void) — pura na base; na derivada: tamanho do vetor de pré-requisitos (stub+0x42, pares de 8 bytes)` |
| 17 | `virtual pair<unsigned int,int> const * GetRequiredSkillAt(int) — pura; ret 4` |
| 18 | `virtual void GetRequiredSkill(skill_array_interface<pair<unsigned int,int>> &) — pura; chama clear()/assign()` |
| 19 | `virtual int GetShowOrder(void)` |
| 20 | `virtual int SetLevel(int)` |
| 21 | `virtual int GetMaxLevel(void)` |
| 22 | `virtual bool IsWarmup(void)` |
| 23 | `virtual bool IsAutoAttack(void)` |
| 24 | `virtual bool IsInstant(void)` |
| 25 | `virtual bool IsDurative(void)` |
| 26 | `virtual int GetRangeType(void)` |
| 27 | `virtual int GetCastEnv(void)` |
| 28 | `virtual int * GetRequiredGenius(int)` |
| 29 | `virtual char const * GetEffect(void)` |
| 30 | `virtual char const * GetElseEffect(void)` |
| 31 | `virtual int GetMpCost(void)` |
| 32 | `virtual int GetApCost(void)` |
| 33 | `virtual int GetArrowCost(void)` |
| 34 | `virtual bool ValidWeapon(int)const ` |
| 35 | `virtual bool IsAllowLand(void)` |
| 36 | `virtual bool IsAllowWater(void)` |
| 37 | `virtual bool IsAllowAir(void)` |
| 38 | `virtual bool GetNotuseInCombat(void)` |
| 39 | `virtual bool IsMovingSkill(void)` |
| 40 | `virtual char GetAllowForms(void)` |
| 41 | `virtual bool Interrupt(void)` |
| 42 | `virtual int GetJuniorCount(void)const ` |
| 43 | `virtual pair<unsigned int,int> const * GetJuniorAt(int)const ` |
| 44 | `virtual void GetJunior(skill_array_interface<pair<unsigned int,int> > &)const ` |
| 45 | `virtual int GetClsLimit(void)` |
| 46 | `virtual int GetCommonCoolDown(void)` |
| 47 | `virtual int GetCommonCoolDownTime(void)` |
| 48 | `virtual int GetItemCost(void)` |
| 49 | `virtual int GetItemCostCount(void)` |
| 50 | `virtual int GetComboSkPreSkill(void)` |
| 51 | `virtual bool CheckHpCondition(int,int)` |
| 52 | `virtual bool CheckComboSkExtraCondition(void)` |

## 4. Interfaces e classes auxiliares

- **`skill_array_interface<T>`** — implementada pelo executável, chamada pelo DLL. Pela desmontagem de
  `GetRequiredSkill`, `GetJunior` e `GetComboSkActivated` (`10002410`):
  - `[+4]` = `assign(T const* inicio, T const* fim)`;
  - `[+8]` = `clear()`;
  - `[+0x10]` = `push_back(T const&)`; o `GetComboSkActivated` acrescenta sem limpar antes;
  - `[+0]` e `[+0xC]` não são usadas.

  `T` = `pair<unsigned int, int>` (8 bytes) ou `unsigned int`.
- **`SkillStr`** — implementada pelo executável e passada a `GetIntroduction`; vtable exportada: `[0]` =
  `wchar_t* Find(int) const` (o texto `N×10+1` do `skillstr.txt`), `[1]` sem nome (provável destrutor). O DLL só
  chama `Find`.
- **`VisibleState`** (ícone/efeito de estado; `Query(int prof, int id)`): vtable `[GetID, GetName, GetHH, GetEffect]`.
- **`TeamState`** (`Query(int id)`): vtable `[GetID, GetName, GetIcon]`.
- Os dados de estado são 493 linhas em `ElementSkill/skill.cpp` (`VisibleStateImp`/`TeamStateImp`).

## 5. Estruturas (tamanho provado com página de guarda)

`tools/elementskill/estruturas.cpp`: a estrutura colada no fim de uma página cuja seguinte é proibida, a função do DLL
chamada para as 3.316 habilidades nos níveis 1 e 2 (6.632 chamadas). No tamanho do fonte, 0 violações; com 4 bytes a
menos, violação — o último campo é lido.

| estrutura | campos (fonte `ElementSkill.h`, iguais no 1.5.3) | tamanho | 4 bytes a menos |
| :--- | :--- | ---: | :--- |
| `LearnRequirement` | `level, sp, money, profession, rank, realm_level` | 24 | 246 violações |
| `UseRequirement` | `mp, ap, form, weapon, freepackage, arrow, move_env, bool is_combat, hp, max_hp, ComboSkillState{skillid, arg[3]}` | 56 | 6.632 |
| `PetRequirement` | `level, sp, int* list, lsize` | 16 | 2.128 |
| `GoblinRequirement` | `genius[5], profession, sp, money, level, mp` | 40 | 188 |
| `GoblinUseRequirement` | `mp, ap, genius[5], profession, level, move_env` | 44 | 0 (o último campo não foi lido com estas entradas) |

## 6. Comportamento (pelo fonte, a conferir no gabarito)

- Estáticas que só criam o objeto e perguntam: `GetRequired*`, `GetExecuteTime`, `GetIcon`, `GetName`, `GetEffect`…
  (`ElementSkill.cpp`).
- `Condition`, `LearnCondition`, `PetLearn`, `GoblinCondition`, `GoblinLearn`: lógica da base (`ElementSkill.cpp`,
  `skill.cpp`), uma só para todas as habilidades, sobre os números do stub.
- Por habilidade, no cliente (`#ifdef _SKILL_CLIENT`): números por nível, a descrição
  (`_snwprintf(formato, argumentos)`, argumentos só do nível em 3.309, também da proficiência em 7) e as condições de
  uma linha `CheckComboSkExtraCondition` (18), `ComboSkEndAction` (8), `CheckHpCondition` (2).
- `LoadSkillData` guarda a lista que o servidor manda (`cmd_skill_data`: id, nível, proficiência).

## 7. O gabarito (fase 2, B314)

`tools/elementskill/gabarito.cpp` mede, no DLL original, tudo o que o contrato expõe; o DLL de dados só vale se o
mesmo programa, rodado nele, der saída idêntica. `tools/elementskill/formatos.py` tira do `skillstr.txt` os formatos de
descrição (texto `N×10+1`). Uma linha JSON:
- **por habilidade:** as estáticas por id; `ValidWeapon` (slot 34) em todos os ids de 0 a 65.535; `CheckHpCondition`
  (slot 51) em 0..100/100 e 0..1000/1000 de 7 em 7.
- **por nível:** todos os slots sem argumento; `GetPrayRange` (slot 9) em 6 pares (alcance, bônus); `GetRequiredGenius`
  (28); pré-requisitos e juniores pelos dois caminhos (`Count/At` e a interface); a descrição crua (um `SkillStr`
  sonda cujo `Find` devolve `%08x|…` × 24 — as palavras de 32 bits dos argumentos, `double` inclusive) e com o formato
  real, com proficiência 0 e 500; as estáticas por nível; e `LearnCondition`, `Condition`, `PetLearn`,
  `GoblinLearn/Condition` com sondas sorteadas por semente fixa.
- **globais:** `GetVersion`, `GetInherentSkills` (classes 0–63), `GetComboSkActivated`, `VisibleState::Query`
  (profissão 0–15 × id 0–4.095), `TeamState::Query` (0–4.095).

Chamada que cai vira `"erro"` no gabarito (o DLL de dados tem de se comportar igual).

**Resultado no DLL 1.5.5 (B314):**
- **Tamanho:** 26.705 linhas, 90 MB; não vai para o repositório, porque o medidor a reproduz. As linhas são 3.316 por
  habilidade, 20.676 por nível, a versão, 4 classes com habilidades inerentes, 2.368 estados visíveis e 340 de equipe.
- **Chamadas:** nenhuma caiu.
- **Armas:** 1.901 habilidades aceitam todas; 1.415 têm lista, de até 20 ids.
- **Vida:** `CheckHpCondition` só restringe em 2.
- **Combo (corrigido no B315):** a primeira sonda caía e o gabarito omitia o caso; a interface tem `push_back` em
  `[+0x10]` (§4). Com ela, 32 linhas de combo e nenhuma queda (ex.: 2562 ativa 2602/2603/2604 conforme `arg[0]`,
  intervalo 45.000).
- **Descrição crua:** confere com o fonte. Na 1, nível 2, as palavras são `2 | 9 | 172,3 (double)`, ou seja,
  `L, −5 + 7L, 1,9L² + 64L + 36,7`. As palavras seguintes são lixo da pilha e variam entre rodadas. O número de
  argumentos de cada habilidade vem do fonte e se confere pelo texto renderizado.
- **Proficiência:** muda a descrição em 61 níveis, das 7 habilidades esperadas.
- **Determinismo:** duas rodadas dão saída idêntica, exceto as palavras de lixo e **37 níveis de 6 habilidades**
  (715, 1754, 2075, 2158, 2162, 3063). Nelas o texto traduzido do `skillstr.txt` tem `%` sem escape ("100% a…"
  vira a conversão `% a`), e o cliente original mostra lixo da pilha. É defeito do texto, corrigível no Estúdio
  (`%` → `%%`); no diferencial, esses níveis são indeterminados também no original.

## 8. O modelo do cliente (fase 3, B315)

`tools/elementskill/gerar_modelo.py` junta o gabarito, o fonte e `descricoes.py` num JSON — tudo o que o DLL de dados
devolve. Uma entrada por habilidade:
- estáticas, armas (`"todas"` ou a lista), faixa de vida, condição de combo `[i, "=="|">=", v]`, intervalo do combo,
  e os 5 textos dos slots 0/1/3/29/30, que são constantes por habilidade;
- por nível:
  - os 25 slots `int` e os 13 de byte (as listas `SLOTS_INT`/`SLOTS_BYTE`);
  - gênio, pré-requisitos e juniores;
  - as 7 estáticas por nível;
  - o alcance `[a, b, c]`;
  - os argumentos da descrição.

Mais os estados visíveis e de equipe, as inerentes por classe e a versão.

**Contagem pelo binário (B323).** O número de palavras que cada stub passa ao `_snwprintf` sai do
binário (`argumentos_da_descricao.py` + `vtable_dos_stubs.cpp`). O `GetIntroduction` é o slot 12 da vtable do stub nas
3.316 habilidades, e o `add esp, N` depois do `call [1031B02C]` (`_snwprintf` na IAT) dá N/4 − 3 palavras. O fonte erra
em **221** (210 com menos argumentos que o binário: o BR pôs números novos nas descrições; 11 com mais). Onde diverge, o
argumento vira um bloco só com as palavras do gabarito. As 7 de proficiência batem. Com isso, o diferencial segue com 0 campos
diferentes, e os textos que consomem além dos argumentos do **binário** caem de 65 para **62**. Os 3 que saíram (2897,
3279 e mais um) eram defeito nosso: o DLL de dados mostrava 0 onde o original mostrava o número.

**Textos com `%` sem escape (B323).** A correção escapa só a conversão com flag de espaço (`"100% de"` → `"100%% de"`),
que é sempre uma porcentagem do português. No 1.5.5 ela muda **63** textos, e depois dela nenhum consome além do que o
binário passa (eram 62). O 63º é o 1181, em que o `"% d"` é a última conversão: não desloca nada e só tira o número
grudado em "100…e precisão". No 1.2.6 ela muda **6** textos: 703, 715, 724, 785, 787 e 819. Aplicada pelo Estúdio
(`PUT …/texto`) em rascunho e publicada.

**Descrição.** As expressões do fonte, avaliadas com a semântica do C, batem palavra por palavra com o binário em 3.268
das 3.316 habilidades. As 48 restantes diferem em 1 ulp de `double` (o VC6 calcula em x87) ou têm valor mudado pelo
BR. Por isso o DLL **não recalcula**: guarda por nível as palavras exatas do binário, na quantidade dada pelos tipos do
fonte (`int` = 1 palavra, `double` = 2). Em 7 habilidades o argumento é `GetAbility()` ou `GetMaxability()`, e o modelo
marca `proficiencia` ou `proficiencia_max`.

**Alcance.** `d = a·alcance_da_arma + b + c·bônus`, com a regra da base (`skill.h:251-272`): ataque ou maldição com
auto-ataque dá `d − 1` se `0,3·d ≥ 1`, senão `0,7·d`; salto dá `d`; `NoTarget` (tipo de área 2 ou 5, `skill.h:39`)
dá −1; o resto dá `d`. O `b` é `double`, pelo fonte. Sem forma no fonte (6 habilidades do BR), `d` sai das sondas
invertendo a regra. Formas medidas `(a, c)`: `(0,0)` 19.202 níveis, `(1,0)` 1.251, `(0,1)` 221, `(2,0)` 2 (3196 e
3197). **Zero níveis sem modelo.**

## 9. O arquivo `habilidades.pwsk` (fase 3, B316)

Escritor e leitor únicos: `pw_data_loader::habilidades_do_cliente` (`escrever`/`ler`; o layout está no cabeçalho do
módulo). O Estúdio usará o mesmo código na fase 5. As habilidades vão na **ordem do `NextSkill` do original** (a do
gabarito), porque o executável percorre nessa ordem; por isso o modelo as guarda em lista. Os textos são guardados como
bytes exatos (GBK, UTF-16LE), e o alcance como `f64` (`a`, `b`, `c`). Ferramenta:
`cargo run -p pw-data-loader --example escrever_habilidades_do_cliente -- <modelo.json> <habilidades.pwsk>`, que só
grava se a ida e volta reproduzir o modelo.

1.5.5: 3.316 habilidades, 4.353.007 bytes, ida e volta igual.

## 10. O DLL de dados e o diferencial (fase 3, B317)

`tools/elementskill/dll/elementskill_dados.cpp`, compilado por `dll/compilar.bat` (VS2022 x86, `/MT`, `/Zc:wchar_t-`
para `wchar_t` = `unsigned short` como no VC6). Ele:
- exporta as 36 funções com o nome exato que o executável importa (conferido por `dumpbin`; o DLL tem 97 exportações);
- monta a vtable de 53 slots na ordem do §3;
- lê `habilidades.pwsk` da mesma pasta, na primeira chamada.

**A lógica de base é porte do fonte:**
- `Condition` e `LearnCondition` (`ElementSkill.cpp:43-227`);
- `PetLearn` e `Goblin*` (`:455-606`);
- `GetAbilityPercent`;
- `GetComboSkActivated` (pós-combo na ordem do `NextSkill`);
- `IsOverridden` (`skillwrapper.cpp`);
- `LoadSkillData` (`cmd_skill_data` empacotado em 1 byte: `count` mais entradas de 5 bytes);
- `GetPrayRange` (`skill.h:251-272`).

**Descrição.** Montada com as palavras do arquivo e formatada pelo `_vsnwprintf` do `msvcrt.dll` do sistema, o mesmo
CRT do original, porque a formatação de `%.1f` muda entre CRTs. Isso evita diferença de arredondamento.

**Condição de combo (B317).** Medida pela API: o gabarito varre cada argumento de −1000 a 1000 por pré-combo (30
varreduras). Os stubs do BR têm faixas que o fonte não tem — 3231: `33 < arg0 ≤ 66`; 3232: `arg0 > 66` (desmontagem
dos stubs, `101AB430` e `101AB840`). O modelo guarda `{arg, faixas}`.

**Diferencial.** `tools/elementskill/diferencial.py` compara o gabarito do nosso DLL com o do original:
**0 campos diferentes em 26.767 linhas**. Duas exceções, medidas no próprio original:
- só valem as palavras cruas da descrição que são argumentos;
- o texto de **65 habilidades** cujo formato traduzido consome mais palavras do que os argumentos (`%` sem escape,
  "100% de" vira `% d`). O original mostra o que estiver na pilha: às vezes muda entre rodadas, às vezes é um endereço
  fixo do DLL original (`0x1001B001` = 268.546.049, visível no jogo). O nosso mostra 0. Corrigir o texto
  (`%` → `%%`) é trabalho do Estúdio.

**Sem dados (B318).** O executável confere `ELEMENTSKILL_VERSION != GetVersion()` logo ao iniciar (`EC_Game.cpp:363`,
"ElementSkill's version is incorrect!"). O original devolve 15, e o nosso tira a versão do `habilidades.pwsk`: sem o
arquivo, devolveria 0 e o cliente recusaria com essa mensagem, que não diz a causa. Agora, se o arquivo faltar, não
for `PWSK`, tiver formato desconhecido ou estiver truncado, o DLL mostra antes uma janela dizendo o motivo e o caminho
esperado (mesma pasta do DLL).

**Diferenças conhecidas (fora do gabarito):**
- `Create(id, nível < 1)` — o original lê fora da tabela, o nosso usa o nível 1;
- `NextSkill` de um id que não existe — o original é indefinido, o nosso devolve 0.

## 11. Fase 5 — distribuição e Estúdio (B321)

**Distribuição.** No `pw-recursos` 1.5.5, o rascunho 51 trocou o `elementskill.dll` pelo nosso (150.528 bytes,
`caa6edba…`) e acrescentou o `habilidades.pwsk` (4.353.097 bytes, `da8d30f0…`). Publicado como **versão 2**; o
launcher do 1.5.5 baixa os dois na próxima atualização.

**Estúdio.** Quando a versão tem `habilidades.pwsk`, a ficha da habilidade lê os números do cliente nele
(`cliente.fonte = "pwsk"`), e `PUT …/numeros_do_cliente` o reescreve pelo escritor único. Campos (`CAMPOS_DO_PWSK`):
`recarga_ms`, `execucao_ms`, `nivel_exigido`, `sp_exigido`, `item_exigido`, `dinheiro_exigido`, `reino_exigido`, `mp`,
`apcost`. Onde o mesmo número existe no slot da vtable e na estática por nível (`GetRequired*`/`GetExecuteTime`), a
edição grava os dois. Sem `.pwsk`, vale o caminho do B305 (bytes de tabela no DLL original). Arquivo que só existe no
rascunho (ação `adicionar`) passou a ser lido pelo `item_do_cliente`.

**Falta:**
- gerar o `.pwsk` a partir do modelo único, junto com o catálogo do servidor;
- editar a descrição (argumentos) e o alcance;
- a correção dos 65 textos com `%` sem escape;
- habilidade nova (fase 6);
- o 1.2.6 (fase 7).

## 12. Edição completa das habilidades no Estúdio (B324)

Com o `.pwsk` na versão, a ficha da habilidade (`GET …/estudio/habilidades/{id}`) traz em `cliente`:
- `campos`: os 9 números do B321;
- `descricao`: `conversoes` (o `%d`/`%.1f`… do texto atual, com o rascunho) e `niveis` (o valor de cada conversão por
  nível: `inteiro`, `real`, `proficiencia`, `fora_dos_argumentos` ou `nao_editavel`);
- `alcance`: `a`, `b` e `c` por nível.

A conta é pura e fica em `pw_data_loader::habilidades_do_cliente`: `conversoes`, `numeros_da_descricao` e
`definir_numeros_da_descricao`. As palavras do `_snwprintf` se percorrem na ordem do formato: `%d` lê 1, `%.1f` lê 2
(`double`), `%%` não lê. A proficiência fica no lugar dela, e as palavras que o texto não usa ficam como estão.

| rota (`PUT`, com `?rascunho=`) | o que grava |
| :--- | :--- |
| `…/descricao_numeros {niveis: [[número ou null por conversão] por nível]}` | as palavras da descrição no `.pwsk` |
| `…/alcance {a?, b?, c?: [por nível], servidor?}` | o alcance no `.pwsk`; com `servidor`, também `alcance.arma = a` (um só, inteiro) e `alcance.fixo = b` no `catalogo\habilidades.json` |
| `…/numeros_nos_dois {campo: [por nível]}` | o mesmo número no servidor e no cliente: `recarga_ms`, `execucao_ms`, `nivel_exigido`, `sp_exigido`, `dinheiro_exigido`, `item_exigido`, `mp` (o cliente primeiro, por ser o mais estrito) |

O servidor passou a editar também `execucao_ms` e `item_exigido` (`CAMPOS_EDITAVEIS`).

**Um modelo, dois artefatos.** O `.pwsk` (cliente) e o `catalogo\habilidades.json` (servidor) saem do mesmo pedido do
Estúdio nos campos que existem nos dois, e não divergem mais. O `.pwsk` inicial vem do gabarito do original (§7–§10).

**Versionamento.** Tudo passa pelo rascunho, publicação e reversão do `pw-recursos`:
- dois rascunhos abertos não editam o mesmo arquivo (409);
- tirar um item do rascunho (`DELETE …/rascunhos/{id}/arquivo`) libera o arquivo;
- publicar grava o `.pwsk` no manifesto do launcher e o catálogo no realm;
- reverter a alteração e publicar devolve **os dois** ao estado anterior (teste de integração).

O catálogo do servidor vale no jogo depois de reiniciar o mundo (o GS o lê ao subir); o `.pwsk`, depois de o jogador
atualizar pelo launcher.
