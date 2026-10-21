# 08 — `elementskill.dll` de dados (opção D)

> Estado: **fase 1 (contrato) concluída — B313; fase 2 (gabarito) concluída — B314**. Decisão do Murillo (2026-10-20): seguir a opção D do
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

- **`skill_array_interface<T>`** — implementada pelo executável, chamada pelo DLL (desmontagem de `GetRequiredSkill`
  e `GetJunior`): `[+4]` = `assign(T const* inicio, T const* fim)`, `[+8]` = `clear()`; `[+0]` não é usada. `T` =
  `pair<unsigned int, int>` (8 bytes) ou `unsigned int`.
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
- **Combo:** `GetComboSkActivated` não devolveu nada com as sondas `{id, arg}` usadas — rever na fase 3.
- **Descrição crua:** confere com o fonte. Na 1, nível 2, as palavras são `2 | 9 | 172,3 (double)`, ou seja,
  `L, −5 + 7L, 1,9L² + 64L + 36,7`. As palavras seguintes são lixo da pilha e variam entre rodadas. O número de
  argumentos de cada habilidade vem do fonte e se confere pelo texto renderizado.
- **Proficiência:** muda a descrição em 61 níveis, das 7 habilidades esperadas.
- **Determinismo:** duas rodadas dão saída idêntica, exceto as palavras de lixo e **37 níveis de 6 habilidades**
  (715, 1754, 2075, 2158, 2162, 3063). Nelas o texto traduzido do `skillstr.txt` tem `%` sem escape ("100% a…"
  vira a conversão `% a`), e o cliente original mostra lixo da pilha. É defeito do texto, corrigível no Estúdio
  (`%` → `%%`); no diferencial, esses níveis são indeterminados também no original.
