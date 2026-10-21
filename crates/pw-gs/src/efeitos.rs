//! Efeitos de estado — os *filtros* das habilidades.
//!
//! No original, o que uma habilidade faz além do dano mora em `StateAttack` (na vítima) e
//! `BlessMe` (em quem conjura), em cada stub `cskill/skills/skillNNN.h`. São sequências
//! de `SetProbability/SetTime/SetRatio/SetValue/SetAmount` seguidas de um `SetX(1)` que
//! cria o efeito `X` (`PlayerWrapper::SetX`, `cskill/skill/playerwrapper.cpp`). O efeito é
//! um `filter` (`cskill/skill/skillfilter.h`) guardado no `filter_man` do objeto
//! (`cgame/gs/filter_man.h:164-222`), que a cada segundo desconta o tempo
//! (`timeout_filter::Heartbeat`, `gs/filter.h:199-203`) e, ao entrar e sair, mexe nos
//! realces percentuais (`_en_percent`, `obj_interface.cpp:200-561`), nos estados visíveis
//! (`IncVisibleState` → `UPDATE_EXT_STATE`) e nos ícones (`InsertTeamVisibleState` →
//! `ICON_STATE_NOTIFY`, `actobject.h:1760-1860`).
//!
//! Aqui ficam as partes puras: o avaliador das expressões extraídas dos stubs
//! ([`expr`]), a tabela dos efeitos portados ([`Efeito`]), a coleção de filtros de um
//! objeto ([`Efeitos`]) e o roteiro ([`executar_roteiro`]). Quem aplica no mundo e avisa o
//! cliente é `bus_server/habilidades.rs`.

use std::collections::HashMap;

// =============================================================================
// Expressões
// =============================================================================

/// O avaliador das expressões dos roteiros — em `pw_data_loader::expr` desde o B308 (um caminho só, também para o
/// Estúdio validar o que se edita).
pub use pw_data_loader::expr;

// =============================================================================
// Os efeitos portados
// =============================================================================

/// Como um filtro novo convive com um do mesmo tipo (`filter_man::AddFilter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Convivencia {
    /// `FILTER_MASK_UNIQUE`: o novo substitui o antigo.
    Unico,
    /// `FILTER_MASK_WEAK`: com um antigo presente, o novo é descartado.
    Fraco,
    /// `FILTER_MASK_MERGE`: o antigo absorve o novo (`Merge`).
    Fundir,
    /// Nenhuma das três: os dois ficam.
    Livre,
}

/// Os efeitos com porte. O nome é o do setter do stub (`SetSlow` → `Slow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Efeito {
    Slow,
    Speedup,
    Fix,
    Dizzy,
    Sleep,
    Sealed,
    Toxic,
    Bleeding,
    Burning,
    Flood,
    Thunder,
    Fallen,
    Incattack,
    Decattack,
    Incmagic,
    Decmagic,
    Incdefence,
    Decdefence,
    Incresist,
    Decresist,
    Incdodge,
    Decdodge,
    Incaccuracy,
    Decaccuracy,
    /// `filter_Inchpsteal` (1.5.5, B137): % de roubo de vida no golpe físico corpo a corpo.
    Inchpsteal,
    /// `filter_Incdamagedodge`: % de chance de o dano do golpe sumir.
    Incdamagedodge,
    /// `filter_Incdebuffdodge`: % de chance de escapar de maldição (a regra ainda sem porte).
    Incdebuffdodge,
    Fastattack,
    Slowattack,
    Fastpray,
    Slowpray,
    Inchurt,
    Dechurt,
    Incsmite,
    Hpgen,
    Mpgen,
    Inchp,
    Dechp,
    Powerup,
    Invincible,
    Firearrow,
    /// `filter_Wingshield` (`cskill/skill/skillfilter.h:4136-4232`): um escudo que **absorve**
    /// dano até acabar e, de quebra, injeta mana a cada 3 s. A Barreira de Asa (249) o aplica
    /// com `SetAmount(60 + 75 × nível)`, `SetValue(4 + 6 × nível)` e `SetTime(20000)`
    /// (`cskill/skills/skill249.h:257-262`).
    Wingshield,
    /// `filter_Ironshield` (`cskill/skill/skillfilter.h:3922-3973`): `UNIQUE|BUFF|REMOVE_ON_DEATH|
    /// HEARTBEAT`, `EnhanceScaleDefense(ratio × 100)`, `HSTATE_IRONSHIELD` 65 (`statedef.h:256`) e
    /// `VSTATE_BLESSED`. A Aura de Aço (77) o aplica aos aliados em volta (`SetIronshield`,
    /// `playerwrapper.cpp`).
    Ironshield,
    /// `filter_Retort` (`cskill/skill/skillfilter.h:1450-1505`), a **Muralha de Espinhos**
    /// (306) do 1.2.6: todo golpe **físico corpo a corpo** que acerta devolve ao atacante
    /// `(int)(physic_damage × ratio)` — o dano bruto do golpe, antes da defesa — como um
    /// golpe mágico que sempre acerta e passa pela defesa física dele (`attack_attr =
    /// MAGIC_ATTACK`, `attack_rate` 1000). Não devolve golpe de longe (`short_range > 0`),
    /// nem golpe não físico, nem ≤ 1. O `gs` 1.2.6 faz o mesmo (`filter_Retort::AdjustDamage`,
    /// VA 0x8310e96). A 306 do 1.2.6 dá `SetRatio(0,05·L + 0,1)` por 600 s (B120).
    Retort,
    /// `filter_Retort2` (`skillfilter.h:14617-14672`), a 306 do 1.5.5: igual ao `Retort`, mas
    /// o golpe de **habilidade** usa o `value` (`_ratio_skill`, [`Filtro::fator_de_habilidade`])
    /// em vez do `ratio`.
    Retort2,
    /// `filter_Fairyform` (`cskill/skill/skillfilter.h:16819-16875`), a **Forma Sombria**
    /// (2570) do Tormentador: enquanto dura, o jogador muda de forma (`ChangeShape(1 |
    /// FORM_CLASS << 6)` → `PLAYER_CHGSHAPE`), tem o equipamento trancado
    /// (`LockEquipment`), e ganha `_speed`% de velocidade e `_defense`% de defesa
    /// (`EnhanceSpeed`, `EnhanceScaleDefense`). O roteiro dá `SetTime(16000 + 3000 × nível)`,
    /// `SetRatio(0,04 × nível)` e `SetValue(0,6 × nível)` (`cskill/skills/skill2570.h:240-243`),
    /// e `SetFairyform` os converte em `(int)(100 × ratio)` e `(int)(100 × value)`
    /// (`cskill/skill/playerwrapper.cpp:5247-5257`).
    Fairyform,
    /// `filter_Foxform` (`cskill/skill/skillfilter.h:4606-4648`, `skillfilter.cpp:389-413`), o
    /// **Chamado da Raposa** (312) da Feiticeira: sem tempo — fica até a 312 ser lançada de
    /// novo (`SetFoxform` o tira quando `GetForm() == FORM_CLASS`, `playerwrapper.cpp:2539-2551`).
    /// Na forma: `EventChange` para `FORM_CLASS`, equipamento trancado, `ImpairScaleMaxMP`
    /// (`_decmp` = 100 × ratio), `EnhanceScaleDefense` (`_incdefence` = 100 × amount),
    /// `EnhanceScaleAttack` — a **precisão** (`_incaccuracy` = 100 × probability) — e
    /// `ChangeShape(_shape | FORM_CLASS << 6)`, `_shape` = `GetValueInt()`. O roteiro da 312 dá
    /// ratio 0,35 − 0,05 × L, amount 0,3 + 0,3 × L, probability 0,5 + 0,5 × L, value 1
    /// (`cskill/skills/skill312.h:163-167`; o `gs` 1.2.6 faz o mesmo, `filter_Foxform::OnAttach`
    /// em VA 0x830b0f2). No [`Filtro`]: `razao` = mana, `escala_defesa` = defesa,
    /// `por_segundo` = precisão, `contador` = `_shape`.
    Foxform,
    /// `filter_Swiftform` (`cskill/skill/skillfilter.h:4841-4876`, `skillfilter.cpp:415-431`), a
    /// **Traje de Flores** (762) da Feiticeira: `UNIQUE|BUFF|REMOVE_ON_DEATH|HEARTBEAT`,
    /// `EnhanceSpeed(_incspeed)` (andar e correr, `obj_interface.cpp:432-436`) com
    /// `_incspeed = (int)(100 × amount)` e o tempo do roteiro (`SetSwiftform`,
    /// `playerwrapper.cpp:2553-2557`), ícone `HSTATE_SWIFTFORM` 81 e o estado visível das
    /// flores, `VSTATE_FOXFORM` 32 (`statedef.h:44`, `:274`). O `gs` 1.2.6 é o mesmo filtro
    /// (`SetSwiftform` VA 0x8305f80: `amount` de +0x74, `time` de +0x80; máscara 0x16004;
    /// `OnAttach` VA 0x830b29e: ícone 0x51), mas liga o estado **31** — a tradução dos bits é do
    /// `WorldProtocol` 1.2.6. O roteiro da 762 dá 300 s e `amount` 0,05 + 0,1·L. A velocidade vai
    /// na `razao`.
    Swiftform,
    /// `filter_Jingji` (`cskill/skill/skillfilter.h:4702-4783`), a **Barreira de Espinhos** (310)
    /// da Feiticeira: `UNIQUE|BUFF|HEARTBEAT|REMOVE_ON_DEATH|ADJUST_DAMAGE|TRANSFERABLE_BUFF`, com o
    /// `_filter_id` do `Retort` (`FILTER_RETORT`, `:4736`) — um substitui o outro. Todo golpe
    /// recebido cai a `_reduce = 1 − value` (piso 0,01) e o físico corpo a corpo devolve
    /// `(int)(bruto × ratio)` como os espinhos do `Retort`. Ícone `HSTATE_JINGJI` 77, estado
    /// `VSTATE_JINGJI` 31 (30 no `gs` 1.2.6, VA 0x83176b5). `SetJingji(ratio, value,
    /// time)` sem dado (`playerwrapper.cpp:1552-1556`; 1.2.6 VA 0x83022a0). No [`Filtro`]:
    /// `fator` = ratio, `fator_de_habilidade` = `_reduce`.
    Jingji,
    /// `filter_Magicleak` (`skillfilter.h:1688-1746`), a **Alma da Raposa** (318): `DEBUFF|
    /// REMOVE_ON_DEATH|HEARTBEAT|TRANSFERABLE_DEBUFF` (sem `UNIQUE`), `_damage = amount / tempo` e,
    /// a cada 3 s (ou no fim), `DrainMana(_damage × _counter)`. Ícone `HSTATE_MAGICLEAK` 16, estado
    /// `VSTATE_MAGICLEAK` 15 — os mesmos no `gs` 1.2.6 (`push 0x10`/`0xf`). `SetMagicleak` com dado
    /// (`playerwrapper.cpp:1037-1051`; `amount` de +0x74 no 1.2.6, VA 0x830074c).
    Magicleak,
    /// `filter_Canti` (`skillfilter.h:4497-4553`), o **Ferir** (320): `UNIQUE|DEBUFF|HEARTBEAT|
    /// REMOVE_ON_DEATH|ADJUST_DAMAGE|TRANSFERABLE_DEBUFF`, `FILTER_SPIRITCURSE` (o mesmo do
    /// `Noregain` e do `Apleak`: as três maldições se substituem). Todo dano recebido × `1 +
    /// ratio`. Ícone `HSTATE_CANTI` 79, sem estado. `SetCanti` só com `ratio > 0` e o dado
    /// (`playerwrapper.cpp:2513-2523`; 1.2.6 VA 0x8305cac). `fator` = ratio.
    Canti,
    /// `filter_Noregain` (`skillfilter.h:4652-4700`), o **Enfraquecer** (321): `UNIQUE|DEBUFF|
    /// REMOVE_ON_DEATH|HEARTBEAT|TRANSFERABLE_DEBUFF`, `FILTER_SPIRITCURSE`;
    /// `ImpairScaleHPGen/MPGen(2000)` — o `Result2(…, −2000 %, 0)` dá negativo e o
    /// `UpdatePlayerMPHPGen` corta em 0 (`playertemplate.h:880-894`): sem regeneração. Ícone
    /// `HSTATE_NOREGAIN` 78, estado `VSTATE_NOREGAIN` 23 (iguais no 1.2.6, `push 0x4e`/`0x17`).
    Noregain,
    /// `filter_Apleak` (`skillfilter.h:4555-4604`), o **Devastar** (322): `UNIQUE|DEBUFF|
    /// REMOVE_ON_DEATH|HEARTBEAT|DO_DAMAGE|TRANSFERABLE_DEBUFF`, `FILTER_SPIRITCURSE`; a cada golpe
    /// que **a vítima recebe** (`EF_DoDamage`, `gs/actobject.cpp:757`) `ModifyAP(−value)`. Ícone
    /// `HSTATE_APLEAK` 80 (`push 0x50` no 1.2.6). Monstro não tem chi: vale para o jogador
    /// amaldiçoado no PvP ([`Efeitos::chi_ao_apanhar`], B221).
    Apleak,
    /// `filter_Reducewood` (`skillfilter.h:2806-2860`), o ○Enxame de Ferroadas (635): `MERGE|DEBUFF|
    /// REMOVE_ON_DEATH|HEARTBEAT|TRANSFERABLE_DEBUFF` com `Merge` vazio — o novo é descartado e o
    /// antigo fica como está (por isso [`Convivencia::Fraco`]). `ImpairScaleResistance(1, ratio)`:
    /// resistência de **madeira** −`(int)(ratio × 100)`%. Ícone `HSTATE_REDUCEWOOD` 54 (`push 0x36`
    /// no 1.2.6). `SetReducewood` com dado (`playerwrapper.cpp:1982-1992`; 1.2.6 VA 0x8303aba).
    Reducewood,
    /// `filter_Freemove` (`skillfilter.h:4400-4449`), a ○Traje de Flores (764): `WEAK|BUFF|HEARTBEAT|
    /// REMOVE_ON_DEATH|TRANSFERABLE_BUFF`; imune a lento, preso, atordoado e sono
    /// (`IncImmuneMask(IMMUNESLOW|FIX|DIZZY|SLEEP)`, que os `SetSlow/Fix/Dizzy/Sleep` consultam).
    /// Ícone `HSTATE_FREEMOVE` 72 (`push 0x48` no 1.2.6, que liga a máscara pelo `SetImmuneMask`).
    /// `SetFreemove(time)` sem dado (`playerwrapper.cpp:2405-2409`).
    Freemove,
    /// `filter_Giant` (`skillfilter.h:3975-4025`), a **Força Divina** (83) do Bárbaro: `UNIQUE|BUFF|
    /// REMOVE_ON_DEATH|HEARTBEAT|TRANSFERABLE_BUFF`, `EnhanceScaleDamage((int)(ratio × 100))` — dano
    /// +%. Ícone `HSTATE_GIANT` 66, `VSTATE_BLESSED` (`SetGiant` sem dado, `playerwrapper.cpp:2339`;
    /// `gs` 1.2.6: `push 0x42`/`push 1`, VA 0x8315bcc).
    Giant,
    /// `filter_Devilstate` (`skillfilter.h:4027-4082`), o **Sangue Fervente** (111): `UNIQUE|BUFF|…`,
    /// `ImpairScaleMaxHP(hpdec)` e `EnhanceScaleAttack(r)` — vida máxima −`ratio`%, precisão
    /// +`amount`% (`SetDevilstate(amount × 100 + 0,00001, ratio × 100 + 0,00001)`,
    /// `playerwrapper.cpp:2399`). Ícone 67, `VSTATE_BLESSED` (1.2.6 VA 0x8315dc2). `razao` = precisão,
    /// `escala_defesa` = a vida tirada.
    Devilstate,
    /// `filter_Tigerform` (`skillfilter.h:3738-3782`, `skillfilter.cpp:361-387`), o **Rugido do
    /// Tigre** (112): só `WEAK` (sem tempo, sobrevive à morte, como a raposa); lançar de novo na
    /// forma desfaz (`SetTigerform`, `playerwrapper.cpp:1785-1798`). Na forma: `EventChange`,
    /// equipamento trancado, vida máxima +`100 × probability`%, velocidade +`100 × amount`%, dano
    /// +`100 × value`% (negativo: −20 % no nível 1) e `ChangeShape(ratio | FORM_CLASS << 6)` — no
    /// `gs` 1.2.6 `ChangeShape(1)` fixo (VA 0x830afd0). Ícone `HSTATE_TIGERFORM` 47 sem parâmetro.
    /// `razao` = velocidade, `por_segundo` = vida, `escala_defesa` = dano, `contador` = `_shape`.
    Tigerform,
    /// `filter_Toxicblade` (`skillfilter.h:2406-2477`), as **Presas Venenosas** (155): `MERGE|BUFF|
    /// HEARTBEAT|REMOVE_ON_DEATH|TRANSLATE_SEND_MSG|TRANSFERABLE_BUFF`; todo golpe **físico** de quem
    /// o tem soma `ratio × 0,5 × (dano baixo + alto da arma)` ao dano de **madeira**
    /// (`magic_damage[1]`), como a flecha faz no fogo. `Merge`: o tempo e o `ratio` do novo. Ícone
    /// `HSTATE_TOXICBLADE` 40 (1.2.6 `push 0x28`).
    Toxicblade,
    /// `filter_Apgen` (`skillfilter.h:3687-3736`), a **Fúria Bestial** (188): `UNIQUE|BUFF|…|DO_DAMAGE`;
    /// a cada golpe **recebido** (`EF_DoDamage`, `gs/actobject.cpp:757`) `ModifyAP(value)`. Ícone
    /// `HSTATE_APGEN` 63 (1.2.6 `push 0x3f`). `por_segundo` = o chi por golpe.
    Apgen,
    /// `filter_Apgencont2` (`skillfilter.h:5846-5910`), da seita (646, 654, 2416): `WEAK|
    /// REMOVE_ON_DEATH|HEARTBEAT` e a máscara do `value` (> 0,5 bênção); `_ap = amount / tempo` e, a
    /// cada 3 s, `ModifyAP(_ap × _counter)`. Ícone `HSTATE_APGENCONT` 102 sempre; `VSTATE_APGENCONT`
    /// 50 só com `showicon` (`escala_defesa` = 1).
    Apgencont2,
    /// `filter_Beastieform` (`skillfilter.h:10061-10121`), o **Encantar** (1826, 1929, 1930):
    /// `UNIQUE|DEBUFF|REMOVE_ON_DEATH|HEARTBEAT`; velocidade +`ratio × 100`% e a forma de fera. No
    /// monstro — o único alvo de maldição no `pw-gs` — o `change_shape` é vazio (`gimp.h:289`): fica a
    /// velocidade. Sem ícone.
    Beastieform,
    /// `filter_Inccritresistance` (`skillfilter.h:11989-12036`, 1985/1987): `UNIQUE|BUFF|…`,
    /// `EnhanceCritResistance(value)` — tira pontos da chance de crítico de quem bate
    /// (`actobject.cpp:739`). Ícone 205. `por_segundo` = os pontos.
    Inccritresistance,
    /// `filter_Insertvstate` (`skillfilter.h:7768-7819`, 1828, 2231): só o visual — estado `amount`
    /// (`contador`), ícone `ratio` (`por_segundo`), máscara do `value`; `HEARTBEAT|REMOVE_ON_DEATH`,
    /// sem `UNIQUE`/`WEAK`/`MERGE` (`SetInsertvstate`, `playerwrapper.cpp:3127-3135`).
    Insertvstate,
    /// `filter_Palsy` (`skillfilter.h:14858-14889`, 2236/2237): `UNIQUE|HEARTBEAT|NOSAVE|
    /// REMOVE_ON_DEATH` e a máscara do `value`; `IncIdleSealMode(MODE_INDEX_STUN)` — não age, como o
    /// atordoado. Ícone 255, estado `VSTATE_PALSY` 117.
    Palsy,
    /// `filter_Sealed2` (`skillfilter.h:7943-7994`, 1826/1929/1930): `WEAK|HEARTBEAT|REMOVE_ON_DEATH` e
    /// `DEBUFF` só com `value > 0,5`; `IncIdleSealMode(MODE_INDEX_SILENT)` — selado. Ícone 140;
    /// `VSTATE_SEALED` 8 só com `showicon` (`escala_defesa` = 1).
    Sealed2,
    /// `filter_Soulretort2` (`skillfilter.h:12611-12710`, base `filter_Soulbase`), a **Barreira
    /// Fulgurante** (1829): `MERGE|HEARTBEAT|ADJUST_DAMAGE|SAVE_DB_DATA` — nem bênção nem maldição e
    /// **sem** `REMOVE_ON_DEATH`. Cada golpe recebido, com `rand() % 100 < trigger_prob` e mana para o
    /// custo, devolve `(int)(físico × ratio)` como golpe mágico — **também de longe** (só não
    /// devolve outro espinho, `AT_STATE_ALL_RETORT`); o `del_on_trigger` apaga o filtro ao disparar
    /// (1) ou ao primeiro golpe (2). `SetSoulretort2(time, value, (int)probability, (int)ratio,
    /// amount > 1,5 ? 2 : amount > 0,5 ? 1 : 0)` (`playerwrapper.cpp:4596-4606`). Ícone 215, estado
    /// 102. `fator` = ratio, `por_segundo` = chance, `contador` = custo, `escala_defesa` = apagar.
    Soulretort2,
    /// `filter_Incrementalhpgen` (`skillfilter.h:11129-11185`), da planta Erva Vital (1501): `BUFF|
    /// REMOVE_ON_DEATH|HEARTBEAT` (sem `UNIQUE`); a cada 3 s cura `_health × _counter` e o `_health`
    /// cresce `_inc_each × _counter` (`SetIncrementalhpgen(time, (int)value, (int)ratio)`,
    /// `playerwrapper.cpp:4308-4312`, sem dado). Ícone 187. `por_segundo` = `_health`,
    /// `escala_defesa` = `_inc_each`. B222.
    Incrementalhpgen,
    /// `filter_Transportdamagetoleader` (`skillfilter.h:12318-12377`), da planta da Feiticeira
    /// (1858): `UNIQUE|REMOVE_ON_DEATH|HEARTBEAT|ADJUST_DAMAGE`; cada golpe que a planta recebe,
    /// com o dono a até `value` m, tira do **dono** `(dano × damage_adjust × ratio)` (o
    /// `DrainLeaderHPMP` → `GM_MSG_DRAIN_HP_MP`: no máximo até 1 de vida, com `be_hurt`,
    /// `actobject.cpp:399-418`). Sem ícone. `fator` = ratio, `fator_de_habilidade` = o alcance.
    /// `parcial`: aqui o dano é o que cai na planta (o original usa o `damage_entry` do
    /// `AdjustDamage`, antes do resto dos ajustes). B222.
    Transportdamagetoleader,
    /// `filter_Auracurse` (`skillfilter.h:8693-8756`, base `filter_Aurabase`): `UNIQUE|REMOVE_ON_DEATH|
    /// HEARTBEAT`; a cada 3 s (e no último segundo) o `DoAura` faz o `RegionEnchant1` da habilidade
    /// `value` no nível `amount`, raio `ratio`, com `helpful` 0 — o roteiro dela em cada inimigo do
    /// raio — e põe o portador em combate. Sem ícone. `SetAuracurse` com dado (`playerwrapper.cpp:3474-3482`).
    /// Sacrifício 1794 → 1795, Borrão Pontiagudo 1358/1718/1720, Prisão de Gelo 1806, Tornado de
    /// Lâminas 1816, Olho Místico 1862. `fator` = raio, `por_segundo` = habilidade, `escala_defesa` =
    /// nível. B227.
    Auracurse,
    /// `filter_Aurabless` (mesmo bloco): o `DoAura` com `helpful` 1 no grupo dentro do raio
    /// (`TeamEnchant` → `SendMsgToTeam(msg, range, except_self)`, `obj_interface.cpp:1267-1283`) ou
    /// só no portador sem grupo. A Bênção de Gaia 1854 → 1861.
    Aurabless,
    /// `healing_potion_filter` / `mana_potion_filter` (`gs/potion_filter.h:6-130`): a poção
    /// não cura de uma vez — ela reparte o total pelo tempo e entrega **um pedaço por
    /// batimento de 1 s**. Não vem de roteiro de habilidade; quem cria é o uso do item.
    PocaoDeVida,
    PocaoDeMana,
    /// `filter_Rebirth` (`cskill/skill/skillfilter.h:9027-9070`), de `SetRebirth(probability,
    /// ratio)` (`playerwrapper.cpp:3589-3593`, sem dado): antes de morrer, com `probability`%
    /// de chance, cura `ratio` (0,01..1) da vida máxima, avisa `ENCHANT_RESULT` da 1085 e se
    /// desfaz (`BeforeDeath`). `UNIQUE|REMOVE_ON_DEATH|BEFORE_DEATH|TRANSFERABLE_BUFF`, ícone
    /// `HSTATE_REBIRTH` 155. No 1.5.5 o põem a 330 (Curar Mascote), 1096, 1280, 1281 e 2411; no
    /// `gs` 1.2.6 o filtro não existe.
    Rebirth,
    /// `filter_Decregiondmg` (`skillfilter.h:19899-19950`), de `SetDecregiondmg`
    /// (`playerwrapper.cpp:6044-6052`): multiplica por `1 − ratio` o golpe recebido de quem não é
    /// jogador **quando `attack_attr < 0`** (`TranslateRecvAttack`). No fonte do 1.5.5 o
    /// `attack_attr` só recebe `PHYSIC_ATTACK`, `PHYSIC_ATTACK_HIT_DEFINITE`, `MAGIC_ATTACK` ou o
    /// `attr` do stub (0..7 em todas as 3.316) — nunca negativo —, então o original nunca reduz:
    /// fica o ícone `HSTATE_DECREGIONDMG` 328 pelo tempo. `UNIQUE|REMOVE_ON_DEATH`; a máscara de
    /// bênção/maldição vem do `amount` (> 1: nenhuma — o caso da 330). Não existe no `gs` 1.2.6.
    Decregiondmg,
}

/// O que se sabe de cada efeito: convivência, se é bênção (`FILTER_MASK_BUFF`) ou
/// maldição (`FILTER_MASK_DEBUFF`), o ícone (`HSTATE_*`) e o estado visível (`VSTATE_*`),
/// com os números de `cskill/skill/statedef.h`.
pub struct Ficha {
    pub convivencia: Convivencia,
    pub bencao: bool,
    pub maldicao: bool,
    pub icone: u16,
    pub visivel: u16,
}

const VS_BLESSED: u16 = 1;
const VS_CURSED: u16 = 2;
const VS_INFAUST: u16 = 9;

impl Efeito {
    pub fn do_setter(nome: &str) -> Option<Self> {
        use Efeito::*;
        Some(match nome {
            "Slow" => Slow,
            "Speedup" => Speedup,
            "Fix" => Fix,
            "Dizzy" => Dizzy,
            "Sleep" => Sleep,
            "Sealed" => Sealed,
            "Toxic" => Toxic,
            "Bleeding" => Bleeding,
            "Burning" => Burning,
            "Flood" => Flood,
            "Thunder" => Thunder,
            "Fallen" => Fallen,
            "Incattack" => Incattack,
            "Decattack" => Decattack,
            "Incmagic" => Incmagic,
            "Decmagic" => Decmagic,
            "Incdefence" => Incdefence,
            "Decdefence" => Decdefence,
            "Incresist" => Incresist,
            "Decresist" => Decresist,
            "Incdodge" => Incdodge,
            "Decdodge" => Decdodge,
            "Incaccuracy" => Incaccuracy,
            "Decaccuracy" => Decaccuracy,
            "Inchpsteal" => Inchpsteal,
            "Incdamagedodge" => Incdamagedodge,
            "Incdebuffdodge" => Incdebuffdodge,
            "Fastattack" => Fastattack,
            "Slowattack" => Slowattack,
            "Fastpray" => Fastpray,
            "Slowpray" => Slowpray,
            "Inchurt" => Inchurt,
            "Dechurt" => Dechurt,
            "Incsmite" => Incsmite,
            "Hpgen" => Hpgen,
            "Mpgen" => Mpgen,
            "Inchp" => Inchp,
            "Dechp" => Dechp,
            "Powerup" => Powerup,
            "Invincible" => Invincible,
            "Firearrow" => Firearrow,
            "Wingshield" => Wingshield,
            "Ironshield" => Ironshield,
            "Retort" => Retort,
            "Retort2" => Retort2,
            "Fairyform" => Fairyform,
            "Foxform" => Foxform,
            "Swiftform" => Swiftform,
            "Jingji" => Jingji,
            "Magicleak" => Magicleak,
            "Canti" => Canti,
            "Noregain" => Noregain,
            "Apleak" => Apleak,
            "Reducewood" => Reducewood,
            "Freemove" => Freemove,
            "Giant" => Giant,
            "Devilstate" => Devilstate,
            "Tigerform" => Tigerform,
            "Toxicblade" => Toxicblade,
            "Apgen" => Apgen,
            "Apgencont2" => Apgencont2,
            "Beastieform" => Beastieform,
            "Inccritresistance" => Inccritresistance,
            "Insertvstate" => Insertvstate,
            "Palsy" => Palsy,
            "Sealed2" => Sealed2,
            "Soulretort2" => Soulretort2,
            "Incrementalhpgen" => Incrementalhpgen,
            "Transportdamagetoleader" => Transportdamagetoleader,
            "Auracurse" => Auracurse,
            "Aurabless" => Aurabless,
            "Rebirth" => Rebirth,
            "Decregiondmg" => Decregiondmg,
            _ => return None,
        })
    }

    /// `skillfilter.h`, classe `filter_<Nome>` de cada um (os números em `statedef.h`).
    pub fn ficha(self) -> Ficha {
        use Convivencia::*;
        use Efeito::*;
        let f = |convivencia, bencao, icone, visivel| Ficha {
            convivencia,
            bencao,
            maldicao: !bencao,
            icone,
            visivel,
        };
        match self {
            // `filter_Slow`: UNIQUE|DEBUFF, VSTATE_SLOW 4, HSTATE_SLOW 3.
            Slow => f(Unico, false, 3, 4),
            // `filter_Speedup`: MERGE|BUFF, VSTATE_BLESSED, HSTATE_SPEEDUP 44.
            Speedup => f(Fundir, true, 44, VS_BLESSED),
            // `filter_Fix`: WEAK|DEBUFF, VSTATE_FIX 7, HSTATE_FIX 8.
            Fix => f(Fraco, false, 8, 7),
            // `filter_Dizzy`: WEAK|DEBUFF, VSTATE_DIZZY 5, HSTATE_DIZZY 1.
            Dizzy => f(Fraco, false, 1, 5),
            // `filter_Sleep`: WEAK|DEBUFF, VSTATE_SLEEP 6, HSTATE_SLEEP 2.
            Sleep => f(Fraco, false, 2, 6),
            // `filter_Sealed`: WEAK|DEBUFF, VSTATE_SEALED 8, HSTATE_SEALED 9.
            Sealed => f(Fraco, false, 9, 8),
            // `filter_Wounded` e filhos: sem UNIQUE/WEAK/MERGE (contra monstro).
            Toxic => f(Livre, false, 13, 11),
            Bleeding => f(Livre, false, 17, 14),
            Burning => f(Livre, false, 14, 12),
            // `filter_Frozen`: só o ícone HSTATE_FROZEN 73.
            Flood => f(Livre, false, 73, 0),
            Thunder => f(Livre, false, 12, 10),
            Fallen => f(Livre, false, 15, 13),
            Incattack => f(Fundir, true, 32, VS_BLESSED),
            Decattack => f(Unico, false, 20, VS_CURSED),
            Incmagic => f(Fundir, true, 46, VS_BLESSED),
            Decmagic => f(Unico, false, 43, VS_CURSED),
            Incdefence => f(Fundir, true, 30, VS_BLESSED),
            Decdefence => f(Unico, false, 18, VS_CURSED),
            Incresist => f(Unico, true, 31, VS_BLESSED),
            Decresist => f(Unico, false, 19, VS_CURSED),
            Incdodge => f(Fundir, true, 34, VS_BLESSED),
            Decdodge => f(Unico, false, 25, VS_CURSED),
            Incaccuracy => f(Unico, true, 36, VS_BLESSED),
            Decaccuracy => f(Unico, false, 24, VS_INFAUST),
            // `skillfilter.h:8808-8862`: MERGE|BUFF, HSTATE_INCHPSTEAL 151, sem estado visível.
            Inchpsteal => f(Fundir, true, 151, 0),
            // `:8940-8990`/`:8991-9040`: UNIQUE, HSTATE 153/154, VSTATE_INCDAMAGEDODGE 65 /
            // VSTATE_INCDEBUFFDODGE 64 (`statedef.h:82-83`, `:351-352`).
            Incdamagedodge => f(Unico, true, 153, 65),
            Incdebuffdodge => f(Unico, true, 154, 64),
            // `filter_Crazy`: MERGE|BUFF.
            Fastattack => f(Fundir, true, 33, VS_BLESSED),
            // `filter_Tardy`: UNIQUE|DEBUFF.
            Slowattack => f(Unico, false, 22, VS_INFAUST),
            Fastpray => f(Unico, true, 37, 0),
            Slowpray => f(Unico, false, 23, VS_INFAUST),
            Inchurt => f(Unico, false, 21, VS_INFAUST),
            Dechurt => f(Unico, true, 35, 0),
            // `filter_Incsmite`: WEAK|BUFF.
            Incsmite => f(Fraco, true, 74, VS_BLESSED),
            Hpgen => f(Livre, true, 41, 0),
            Mpgen => f(Livre, true, 42, 0),
            Inchp => f(Fundir, true, 28, VS_BLESSED),
            // `filter_Dechp`: UNIQUE, DEBUFF pelo construtor.
            Dechp => f(Unico, false, 45, VS_CURSED),
            // `filter_Powerup`: UNIQUE|BUFF, VSTATE_POWERUP 17, HSTATE_POWERUP 60.
            Powerup => f(Unico, true, 60, 17),
            // `SetInvincibleFilter` + `filter_Icon(HSTATE_INVINCIBLE 76)` quando `showicon`.
            Invincible => f(Unico, true, 76, 0),
            // `filter_Firearrow` (`cskill/skill/skillfilter.h:4233-4290`):
            // `WEAK|BUFF|HEARTBEAT|REMOVE_ON_DEATH|TRANSLATE_SEND_MSG|TRANSFERABLE_BUFF`,
            // com `HSTATE_FIREARROW` na lista visível à equipe e `VSTATE_FIREARROW` no
            // estado. **Fraco**, não único: com um já ativo, o novo é descartado.
            Firearrow => f(Fraco, true, 70, 30),
            // `filter_Wingshield`: `UNIQUE|BUFF|HEARTBEAT|REMOVE_ON_DEATH|ADJUST_DAMAGE|
            // TRANSFERABLE_BUFF`, `HSTATE_WINGSHIELD` 69 e `VSTATE_WINGSHIELD` 29
            // (`cskill/skill/statedef.h:40,261`).
            Wingshield => f(Unico, true, 69, 29),
            Ironshield => f(Unico, true, 65, VS_BLESSED),
            Swiftform => f(Unico, true, 81, 32),
            Jingji => f(Unico, true, 77, 31),
            Magicleak => f(Livre, false, 16, 15),
            Canti => f(Unico, false, 79, 0),
            Noregain => f(Unico, false, 78, 23),
            Apleak => f(Unico, false, 80, 0),
            Reducewood => f(Fraco, false, 54, 0),
            Freemove => f(Fraco, true, 72, 0),
            Giant => f(Unico, true, 66, VS_BLESSED),
            Devilstate => f(Unico, true, 67, VS_BLESSED),
            Tigerform => Ficha {
                convivencia: Fraco,
                bencao: false,
                maldicao: false,
                icone: 47,
                visivel: 0,
            },
            Toxicblade => f(Fundir, true, 40, 0),
            Apgen => f(Unico, true, 63, 0),
            // Máscara e estado pelo roteiro (`Filtro::mascara`, `escala_defesa`).
            Apgencont2 => f(Fraco, true, 102, 0),
            Beastieform => f(Unico, false, 0, 0),
            Inccritresistance => f(Unico, true, 205, 0),
            Insertvstate => Ficha {
                convivencia: Livre,
                bencao: false,
                maldicao: false,
                icone: 0,
                visivel: 0,
            },
            Palsy => f(Unico, false, 255, 117),
            Sealed2 => f(Fraco, false, 140, 0),
            Incrementalhpgen => f(Livre, true, 187, 0),
            Auracurse | Aurabless => Ficha {
                convivencia: Unico,
                bencao: false,
                maldicao: false,
                icone: 0,
                visivel: 0,
            },
            Transportdamagetoleader => Ficha {
                convivencia: Unico,
                bencao: false,
                maldicao: false,
                icone: 0,
                visivel: 0,
            },
            Soulretort2 => Ficha {
                convivencia: Fundir,
                bencao: false,
                maldicao: false,
                icone: 215,
                visivel: 102,
            },
            // `FILTER_MASK_UNIQUE | BUFF | HEARTBEAT | REMOVE_ON_DEATH | ADJUST_DAMAGE |
            // TRANSFERABLE_BUFF`; `VSTATE_RETORT` 3 e `HSTATE_RETORT` 4 / `HSTATE_RETORT2` 253
            // (`statedef.h:11,183,451`; o `gs` 1.2.6 empurra 3 e 4, VA 0x8310f8e).
            Retort => f(Unico, true, 4, 3),
            Retort2 => f(Unico, true, 253, 3),
            // `filter_Fairyform`: `FILTER_MASK_WEAK | FILTER_MASK_HEARTBEAT` — nem bênção nem
            // maldição (o Dispersar não o tira) e **sem** `REMOVE_ON_DEATH`. O ícone é o
            // `HSTATE_FAIRYFORM` 279 (`statedef.h:477`, `InsertTeamVisibleState`); não há
            // `VSTATE`: o que o cliente desenha é a forma, pelo `PLAYER_CHGSHAPE`.
            Fairyform => Ficha {
                convivencia: Fraco,
                bencao: false,
                maldicao: false,
                icone: 279,
                visivel: 0,
            },
            // `filter_Foxform`: só `FILTER_MASK_WEAK` (`skillfilter.h:4611`) — sem batimento,
            // sem `REMOVE_ON_DEATH`. Ícone `HSTATE_FOXFORM` 75 (`statedef.h:268`; o `gs`
            // 1.2.6 empurra 0x4b), **sem parâmetro**: `InsertTeamVisibleState(state)`.
            Foxform => Ficha {
                convivencia: Fraco,
                bencao: false,
                maldicao: false,
                icone: 75,
                visivel: 0,
            },
            // Sem ícone e sem estado visual: o original não acende nenhum (`potion_filter.h`).
            PocaoDeVida | PocaoDeMana => f(Fundir, true, 0, 0),
            // Nem `BUFF` nem `DEBUFF`: o Dispersar não os tira.
            Rebirth => Ficha {
                convivencia: Unico,
                bencao: false,
                maldicao: false,
                icone: 155,
                visivel: 0,
            },
            Decregiondmg => Ficha {
                convivencia: Unico,
                bencao: false,
                maldicao: false,
                icone: 328,
                visivel: 0,
            },
        }
    }

    /// Mesmo `_filter_id`: o `Jingji` usa o `FILTER_RETORT` dos espinhos (`skillfilter.h:1481`,
    /// `:4736`); `Canti`, `Noregain` e `Apleak` usam o `FILTER_SPIRITCURSE` (`:4538`, `:4585`,
    /// `:4678`).
    pub fn mesmo_filtro(self, outro: Efeito) -> bool {
        use Efeito::*;
        self == outro
            || matches!((self, outro), (Retort | Retort2 | Jingji, Retort | Retort2 | Jingji))
            || matches!((self, outro), (Canti | Noregain | Apleak, Canti | Noregain | Apleak))
    }

    /// Dano ao longo do tempo (`filter_Wounded`), com a classe de dano do
    /// `CalcMagicDamage` de cada `Set` (`playerwrapper.cpp`): `None` = físico.
    pub fn dano_no_tempo(self) -> Option<Option<usize>> {
        use Efeito::*;
        match self {
            Bleeding => Some(None),
            Thunder => Some(Some(0)),
            Toxic => Some(Some(1)),
            Flood => Some(Some(2)),
            Burning => Some(Some(3)),
            Fallen => Some(Some(4)),
            _ => None,
        }
    }
}

/// `FORM_CLASS` (`cskill/skill/skill.h:84`): a forma de classe (raposa, Forma Sombria).
pub const FORMA_DE_CLASSE: u8 = 1;

/// Um filtro vivo num objeto.
#[derive(Debug, Clone, PartialEq)]
pub struct Filtro {
    pub efeito: Efeito,
    /// `_timeout`, em segundos.
    pub restante_s: i32,
    /// `(int)(ratio * 100)` — a porcentagem dos realces.
    pub razao: i32,
    /// `ratio` sem arredondar, para `Inchurt`/`Dechurt` (`1 ± ratio`).
    pub fator: f32,
    /// `_damage` / `_health` por segundo, e `_point` do `Incsmite`.
    pub por_segundo: i32,
    pub contador: i32,
    /// Quem causou — o dono do dano do `filter_Wounded`.
    pub origem: i64,
    /// Se mostra ícone (`Invincible` só com `showicon`).
    pub icone: bool,
    /// `_amount` do `filter_Wingshield`: quanto de dano o escudo ainda aguenta. Cai a cada
    /// golpe e, abaixo de 6, o filtro se apaga (`skillfilter.h:4168-4195`).
    pub absorve: f32,
    /// `_defense` do `filter_Fairyform`: porcentagem somada à defesa (`EnhanceScaleDefense`).
    /// A velocidade vai na `razao`, como nos outros realces de velocidade.
    pub escala_defesa: i32,
    /// `_ratio_skill` do `filter_Retort2`: a parte do golpe de **habilidade** que os espinhos
    /// devolvem (o `Value` do roteiro; a 306 do 1.5.5 dá `0,02·L`). O `filter_Retort` do
    /// 1.2.6 não distingue: usa o `ratio` nos dois (`skillfilter.h:1478-1484`, `:14644-14650`).
    pub fator_de_habilidade: f32,
    /// A máscara de bênção/maldição quando o setter a escolhe pelo roteiro (`value > 0,5` →
    /// `FILTER_MASK_BUFF` etc., `Apgencont2`/`Sealed2`/`Palsy`/`Insertvstate`); `None` = a da ficha.
    pub mascara: Option<(bool, bool)>,
}

impl Filtro {
    /// `FILTER_MASK_BUFF` / `FILTER_MASK_DEBUFF` deste filtro (o que o `ClearSpecFilter` olha).
    pub fn bencao_e_maldicao(&self) -> (bool, bool) {
        self.mascara.unwrap_or_else(|| {
            let fi = self.efeito.ficha();
            (fi.bencao, fi.maldicao)
        })
    }
}

/// O que um segundo de filtros faz ([`Efeitos::batida`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Tique {
    Dano { origem: i64, valor: i32 },
    Cura(i32),
    Mana(i32),
    /// `ModifyAP` do `filter_Apgencont2`.
    Chi(i32),
    /// O `DoAura` de uma aura: lançar o roteiro de `habilidade` no `nivel` em quem está a `raio` m
    /// (inimigos, ou o grupo quando `amigavel`).
    Aura { habilidade: i32, nivel: i32, raio: f32, amigavel: bool },
}

/// Soma dos realces vivos — o `_en_percent` (e o `_crit_rate`) que os filtros mexem.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Realce {
    pub dano: i32,
    pub magia: i32,
    pub defesa: i32,
    pub evasao: i32,
    pub precisao: i32,
    /// `_en_percent.attack_speed` — **negativo** acelera (`EnhanceScaleAttackSpeed` subtrai).
    pub velocidade_de_ataque: i32,
    pub velocidade: i32,
    pub vida: i32,
    /// `_en_percent.max_mp` (`Enhance/ImpairScaleMaxMP`), aplicado no `UpdateMana`.
    pub mana: i32,
    pub resistencia: i32,
    pub critico: i32,
    /// `DecPrayTime`/`IncPrayTime` — porcentagem a menos no tempo de conjuração.
    pub conjuracao: i32,
    /// Multiplica o dano recebido (`AdjustDamage` de `Inchurt`/`Dechurt`).
    pub dano_recebido: f32,
    /// `_en_percent.swim_speed` (`EnhanceSwimSpeed`, `obj_interface.cpp:475-478`).
    pub natacao: i32,
    /// `_hp_steal_rate`, `_damage_dodge_rate`, `_debuff_dodge_rate` (%), B137.
    pub roubo_de_vida: i32,
    pub esquiva_de_dano: i32,
    pub esquiva_de_maldicao: i32,
    /// `EnhanceHPGen`/`EnhanceMPGen` (pontos por batimento) das passivas comuns. B203.
    pub regen_vida: i32,
    pub regen_mana: i32,
    /// `_en_percent.hp_gen`/`mp_gen` dos filtros (`ImpairScaleHPGen/MPGen`): o `Noregain` tira
    /// 2000 %, o que zera a regeneração.
    pub regen_pct: i32,
    /// `ImpairScaleResistance(escola, r)` — por escola (metal, madeira, água, fogo, terra),
    /// além da `resistencia` comum. O `Reducewood` mexe na 1.
    pub resistencias: [i32; 5],
    /// `_crit_resistance` (`EnhanceCritResistance`): pontos a menos na chance de crítico de quem
    /// bate.
    pub resistencia_a_critico: i32,
    /// B330 — `SetSkillInc(escola, inc)` das passivas `Incgold`/`Incwood`/`Incwater`/`Incfire`/`Incearth`
    /// (`playerwrapper.cpp:64-68`, `skillwrapper.cpp:957-965`): **atribui** (`skillinc[m] = inc`), então vale a
    /// última na ordem do mapa de habilidades (id crescente). O `Skill::Set<escola>damage` multiplica o dano
    /// da escola por `1 + inc` (`skill.cpp:899-903`).
    pub elemento: [f32; 5],
    /// B330 — `_skill_enhance2` (`Addskilldamage`, `EnhanceSkillDamage2((int)(inc × 100))`,
    /// `playerwrapper.cpp:1835-1843`): golpe de habilidade × `(100 + x)/100` no `AdjustDamage` do alvo
    /// (`npc.cpp:1767-1768`, `player.cpp:9652-9653`).
    pub realce_de_habilidade2: i32,
    /// B330 — `_near/_far_normal/skill_dmg_reduce` (`Incnear…`/`Incfar…dmgreduce`, `playerwrapper.cpp:1845-1879`):
    /// a atenuação por distância do golpe de jogador ou mascote que **este** recebe (`actobject.cpp:520-538`).
    pub reducao_perto_normal: f32,
    pub reducao_perto_habilidade: f32,
    pub reducao_longe_normal: f32,
    pub reducao_longe_habilidade: f32,
    /// B330 — `_invisible_passive`/`_anti_invisible_passive` (`IncInvisiblePassive`, `IncAntiInvisiblePassive`,
    /// `playerwrapper.cpp:1379-1403`). Entram no grau de invisibilidade e de detecção
    /// (`UpdatePlayerInvisible`, `playertemplate.h:1135-1147`) — `falta`: o nosso mundo não tem furtividade
    /// (o `_invisible_active`), então ainda não há quem os use.
    pub invisivel_passivo: i32,
    pub anti_invisivel_passivo: i32,
    /// B330 — `_en_point.attack_range` do `Incrange` com arco (`playerwrapper.cpp:2411-2425`,
    /// `obj_interface.cpp:268-271`), em metros.
    pub alcance: f32,
    /// B330 — `_en_point.flight_speed` do `Incfeather` com asas (`playerwrapper.cpp:2289-2302`,
    /// `EnhanceFlySpeed`, `obj_interface.cpp:487-490`), em m/s.
    pub voo: f32,
    /// B330 — `_resurrect_exp_lost_reduce` (`Reduceresurrectexplost`, `playerwrapper.cpp:1477-1484`,
    /// `player_imp.h:3797`): a perda ao renascer vai × `(100 − x)/100` (`player.cpp:8718`).
    pub reducao_da_perda_ao_renascer: i32,
    /// B330 — `IMMUNEDROP` (`Immunedrop`, `playerwrapper.cpp:1486-1493`, `statedef.h:563`): imune ao
    /// `Dropmoneyondeath` (`playerwrapper.cpp:4637-4643`) — `falta`: esse efeito ainda não tem porte.
    pub imune_a_queda: bool,
}

impl Realce {
    /// Soma as passivas (de forma, `EventChange`, e comuns, B203) a este realce.
    pub fn somar(&mut self, p: &Realce) {
        self.dano += p.dano;
        self.magia += p.magia;
        self.defesa += p.defesa;
        self.precisao += p.precisao;
        self.resistencia += p.resistencia;
        self.critico += p.critico;
        self.natacao += p.natacao;
        self.regen_vida += p.regen_vida;
        self.regen_mana += p.regen_mana;
        // B330: só as passivas comuns mexem nestes (nenhum filtro nem forma os toca).
        for i in 0..5 {
            self.elemento[i] += p.elemento[i];
        }
        self.realce_de_habilidade2 += p.realce_de_habilidade2;
        self.reducao_perto_normal += p.reducao_perto_normal;
        self.reducao_perto_habilidade += p.reducao_perto_habilidade;
        self.reducao_longe_normal += p.reducao_longe_normal;
        self.reducao_longe_habilidade += p.reducao_longe_habilidade;
        self.invisivel_passivo += p.invisivel_passivo;
        self.anti_invisivel_passivo += p.anti_invisivel_passivo;
        self.alcance += p.alcance;
        self.voo += p.voo;
        self.reducao_da_perda_ao_renascer += p.reducao_da_perda_ao_renascer;
        self.imune_a_queda |= p.imune_a_queda;
    }
}

/// `WeaponClass` (`cskill/skill/skillwrapper.h:54-66`) — o `id_major_type` da arma (B203,
/// conferido nos `elements` dos dois realms).
pub mod classe_de_arma {
    pub const ESPADA: i32 = 1;
    pub const LANCA: i32 = 5;
    pub const MACHADO: i32 = 9;
    pub const ARCO: i32 = 13;
    pub const PUNHO: i32 = 182;
    pub const ADAGA: i32 = 23749;
    pub const TALISMA: i32 = 25333;
    pub const CIMITARRA: i32 = 44878;
}

/// As passivas comuns que o jogador conhece, somadas (B203): as `EVENT_RESET`, sempre
/// (`SkillWrapper::EventReset`, `skillwrapper.cpp:612-630`), e as `EVENT_WIELD` com a arma vestida
/// (`EventWield(player, weapon_class)`, `:652-670`), aplicadas na entrada (`:1261-1264`). O
/// `TakeEffect` de cada uma, no nível dela, com os setters do `playerwrapper.cpp`:
/// `Adddefence` (`:1809`, `EnhanceScaleDefense(100 × m)`), `Addresistance` (`:1819`, as cinco
/// resistências `× 100`), `Inccrit` (`:1346`, `EnhanceCrit`), `Inchpgen`/`Incmpgen` (`:1495`,
/// `:1335`, pontos), `Incswim` (`:2272`), e os de arma, que só valem com a classe certa
/// (`intarg != WEAPONCLASS_* → false`): `Incsword`/`Incspear`/`Inchammer`/`Incbow`/`Incboxing`/
/// `Incdagger`/`Incscimitar` (`EnhanceScaleDamage(inc × 100)`, `:754-930`, `:2436`),
/// `Inctalisman` (`EnhanceScaleMagicDamage`, `:896`) e `Inchitrate` (arco, `EnhanceScaleAttack`,
/// `:1357`). B330: o dano de habilidade por escola (`SetIncgold…earth` → `SetSkillInc`),
/// `Addskilldamage`, a redução por distância (`Incnear…`/`Incfar…dmgreduce`), a invisibilidade
/// passiva, `Incrange` (com arco), `Incfeather` (com asas: o `EventWield(65535)` do
/// `angel_wing_item::OnActivate`, `item_flysword.cpp:107-110`), `Reduceresurrectexplost` e
/// `Immunedrop` — ver os campos de [`Realce`]. **Falta**: as `EVENT_ENTER`.
///
/// A ordem é a do mapa de habilidades do original (`std::map`, id crescente): o `SetSkillInc`
/// atribui, e a última passiva de uma escola é a que vale.
pub fn passivas_comuns(
    tabela: &pw_data_loader::habilidades::TabelaDeHabilidades,
    conhecidas: &std::collections::HashMap<u32, u8>,
    classe_da_arma: i32,
    asas: bool,
) -> Realce {
    use pw_data_loader::habilidades::{EVENT_RESET, EVENT_WIELD};
    let mut r = Realce::default();
    let mut ordem: Vec<(u32, u8)> = conhecidas.iter().map(|(&id, &nivel)| (id, nivel)).collect();
    ordem.sort_unstable();
    for (id, nivel) in ordem {
        let Some(h) = tabela.get(id) else { continue };
        let evento = h.eventflag.unwrap_or(0);
        if evento != EVENT_RESET && !(evento == EVENT_WIELD && (classe_da_arma > 0 || asas)) {
            continue;
        }
        for (_, setter, e) in h.efeito_passivo.as_deref().unwrap_or_default() {
            let Some(v) = expr::avaliar(e, &|n| (n == "L").then_some(nivel as f64)) else {
                continue;
            };
            let v = v as f32;
            let da_arma = |c: i32| classe_da_arma == c;
            use classe_de_arma::*;
            match setter.as_str() {
                "Adddefence" => r.defesa += (v * 100.0) as i32,
                "Addresistance" => r.resistencia += (v * 100.0) as i32,
                "Inccrit" => r.critico += v as i32,
                "Inchpgen" => r.regen_vida += v as i32,
                "Incmpgen" => r.regen_mana += v as i32,
                "Incswim" => r.natacao += (100.0 * v) as i32,
                "Incsword" if da_arma(ESPADA) => r.dano += (v * 100.0) as i32,
                "Incspear" if da_arma(LANCA) => r.dano += (v * 100.0) as i32,
                "Inchammer" if da_arma(MACHADO) => r.dano += (v * 100.0) as i32,
                "Incbow" if da_arma(ARCO) => r.dano += (v * 100.0) as i32,
                "Incboxing" if da_arma(PUNHO) => r.dano += (v * 100.0) as i32,
                "Incdagger" if da_arma(ADAGA) => r.dano += (v * 100.0) as i32,
                "Incscimitar" if da_arma(CIMITARRA) => r.dano += (v * 100.0) as i32,
                "Inctalisman" if da_arma(TALISMA) => r.magia += (v * 100.0) as i32,
                "Inchitrate" if da_arma(ARCO) => r.precisao += (100.0 * v) as i32,
                "Incgold" => r.elemento[0] = v,
                "Incwood" => r.elemento[1] = v,
                "Incwater" => r.elemento[2] = v,
                "Incfire" => r.elemento[3] = v,
                "Incearth" => r.elemento[4] = v,
                "Addskilldamage" => r.realce_de_habilidade2 += (v * 100.0) as i32,
                "Incnearnormaldmgreduce" => r.reducao_perto_normal += v,
                "Incnearskilldmgreduce" => r.reducao_perto_habilidade += v,
                "Incfarnormaldmgreduce" => r.reducao_longe_normal += v,
                "Incfarskilldmgreduce" => r.reducao_longe_habilidade += v,
                // Os setters de `int` truncam o `double` da expressão.
                "Incinvisiblepassive" => r.invisivel_passivo += v as i32,
                "Incantiinvisiblepassive" => r.anti_invisivel_passivo += v as i32,
                "Incrange" if da_arma(ARCO) => r.alcance += v,
                "Incfeather" if asas => r.voo += v,
                "Reduceresurrectexplost" => r.reducao_da_perda_ao_renascer += v as i32,
                "Immunedrop" => r.imune_a_queda = true,
                _ => {}
            }
        }
    }
    r
}

/// As passivas `EVENT_CHANGE` que o jogador conhece, somadas (`SkillWrapper::EventChange`,
/// `cskill/skill/skillwrapper.cpp:589-610`): o `TakeEffect` de cada uma, no nível dela, com os
/// setters de `playerwrapper.cpp` — `SetIncswim` (`:2272-2285`, `EnhanceSwimSpeed(100 × inc)`),
/// `SetIncfight` (`:2559-2566`, `EnhanceScaleDamage(100 × inc)`), `SetAdddefence`
/// (`EnhanceScaleDefense(100 × m)`) e `SetInccrit` (`EnhanceCrit(point)`). Vale enquanto o
/// jogador está na forma de classe; ao sair, o `UndoEffect` tira o mesmo tanto.
pub fn passivas_de_forma(
    tabela: &pw_data_loader::habilidades::TabelaDeHabilidades,
    conhecidas: &std::collections::HashMap<u32, u8>,
) -> Realce {
    let mut r = Realce::default();
    for (&id, &nivel) in conhecidas {
        let Some(h) = tabela.get(id) else { continue };
        if h.eventflag != Some(pw_data_loader::habilidades::EVENT_CHANGE) {
            continue;
        }
        for (_, setter, e) in h.ao_mudar_de_forma.as_deref().unwrap_or_default() {
            let Some(v) = expr::avaliar(e, &|n| (n == "L").then_some(nivel as f64)) else {
                continue;
            };
            match setter.as_str() {
                "Incswim" => r.natacao += (100.0 * v as f32) as i32,
                "Incfight" => r.dano += (v as f32 * 100.0) as i32,
                "Adddefence" => r.defesa += (v as f32 * 100.0) as i32,
                "Inccrit" => r.critico += v as i32,
                _ => {}
            }
        }
    }
    r
}

/// O estado visível do `invincible_filter` no 1.5.5 (`IncVisibleState(49)`).
pub const ESTADO_VISIVEL_INVENCIVEL: usize = 49;

/// Os filtros de um objeto.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Efeitos {
    pub filtros: Vec<Filtro>,
    /// `_invincible_timeout` do `SetInvincibleFilter` (sem ícone).
    pub invencivel_s: i32,
    /// O `invincible_filter(FILTER_INVINCIBLE)` sem prazo que o GM liga e desliga
    /// (`GMCMD_TOGGLE_INVINCIBLE`, `gs/playercmd.cpp:4895-4910`): dano não entra e o estado
    /// visível 49 acende, como o filtro com prazo (`invincible_filter.cpp:13-36`).
    pub gm_invencivel: bool,
    /// O `gm_invisible_filter` (`GMCMD_TOGGLE_INVISIBLE`, `gs/playercmd.cpp:4879-4893`): some da
    /// vista dos outros e o ataque que chega perde a facção (`TranslateRecvAttack` →
    /// `target_faction = 0`, `invisible_filter.cpp:115-123`) — não fere.
    pub gm_invisivel: bool,
}

impl Efeitos {
    /// `filter_man::AddFilter` com a convivência de cada efeito. `true` quando algo mudou.
    pub fn adicionar(&mut self, novo: Filtro) -> bool {
        let ficha = novo.efeito.ficha();
        // `filter_man::AddFilter` procura pelo `_filter_id` (`gs/filter_man.h:177-207`).
        if let Some(i) = self.filtros.iter().position(|f| f.efeito.mesmo_filtro(novo.efeito)) {
            match ficha.convivencia {
                Convivencia::Unico => {
                    self.filtros.retain(|f| !f.efeito.mesmo_filtro(novo.efeito));
                }
                Convivencia::Fraco => return false,
                Convivencia::Fundir => {
                    let velho = &mut self.filtros[i];
                    if novo.efeito.dano_no_tempo().is_some() {
                        // `filter_Wounded::Merge`.
                        let resto = velho.por_segundo * velho.restante_s;
                        velho.restante_s = novo.restante_s.max(1);
                        let r = resto / velho.restante_s;
                        velho.por_segundo = (novo.por_segundo + if r != 0 { r } else { 1 }).max(1);
                    } else if matches!(novo.efeito, Efeito::PocaoDeVida | Efeito::PocaoDeMana) {
                        // `healing_potion_filter::Merge` (`potion_filter.h:30-42`): soma o
                        // tempo e o total das duas, e reparte de novo.
                        let total = velho.por_segundo * velho.restante_s
                            + novo.por_segundo * novo.restante_s;
                        velho.restante_s += novo.restante_s;
                        velho.por_segundo = (total / velho.restante_s.max(1)).max(1);
                        return true;
                    } else if novo.efeito == Efeito::Soulretort2 {
                        // `filter_Soulretort2::Merge`: mesmo `ratio`, fica o tempo maior; maior,
                        // substitui tempo e `ratio`; menor, nada.
                        if novo.fator == velho.fator {
                            velho.restante_s = velho.restante_s.max(novo.restante_s);
                        } else if novo.fator > velho.fator {
                            *velho = novo;
                        }
                    } else if novo.efeito == Efeito::Inchpsteal {
                        // `filter_Inchpsteal::Merge` (`skillfilter.h:8840-8854`): igual renova o
                        // tempo, maior substitui, menor não muda nada.
                        if novo.razao >= velho.razao {
                            velho.restante_s = novo.restante_s;
                            velho.razao = novo.razao;
                            velho.fator = novo.fator;
                        }
                    } else {
                        // `Merge` dos realces: o tempo e a razão do novo.
                        velho.restante_s = novo.restante_s;
                        velho.razao = novo.razao;
                        velho.fator = novo.fator;
                    }
                    return true;
                }
                Convivencia::Livre => {}
            }
        }
        self.filtros.push(novo);
        true
    }

    /// Um segundo (`FILTER_MASK_HEARTBEAT`, `Heartbeat(1)`). Devolve os tiques e se algum
    /// filtro acabou.
    pub fn batida(&mut self) -> (Vec<Tique>, bool) {
        let mut tiques = Vec::new();
        for f in &mut self.filtros {
            // `filter_Foxform`/`filter_Tigerform` não têm `FILTER_MASK_HEARTBEAT` nem tempo.
            if matches!(f.efeito, Efeito::Foxform | Efeito::Tigerform) {
                continue;
            }
            // `filter_Wounded::Heartbeat`, `filter_Hpgen/Mpgen::Heartbeat`: a cada 3 s, ou no
            // último, o acumulado.
            // A poção entrega todo segundo (`healing_potion_filter::Heartbeat`), sem o
            // acúmulo de 3 s dos filtros de habilidade.
            if matches!(f.efeito, Efeito::PocaoDeVida | Efeito::PocaoDeMana) {
                let v = f.por_segundo.min(if f.restante_s <= 1 {
                    i32::MAX
                } else {
                    f.por_segundo
                });
                tiques.push(if f.efeito == Efeito::PocaoDeVida {
                    Tique::Cura(v)
                } else {
                    Tique::Mana(v)
                });
                f.restante_s -= 1;
                continue;
            }
            // `filter_Aurabase::Heartbeat` (`skillfilter.h`): `_counter += tick`; com `_counter >= 3` ou
            // `tick >= _timeout`, `DoAura()` e `_counter -= 3`; depois o tempo.
            if matches!(f.efeito, Efeito::Auracurse | Efeito::Aurabless) {
                f.contador += 1;
                if f.contador >= 3 || 1 >= f.restante_s {
                    tiques.push(Tique::Aura {
                        habilidade: f.por_segundo,
                        nivel: f.escala_defesa,
                        raio: f.fator,
                        amigavel: f.efeito == Efeito::Aurabless,
                    });
                    f.contador -= 3;
                }
                f.restante_s -= 1;
                continue;
            }
            // `filter_Wingshield::Heartbeat`: a cada 3 s injeta `_mpgen` **inteiro**, não o
            // acumulado (`skillfilter.h:4207-4218`).
            if f.efeito == Efeito::Wingshield {
                f.contador += 1;
                if f.contador >= 3 || 1 >= f.restante_s {
                    if f.por_segundo > 0 {
                        tiques.push(Tique::Mana(f.por_segundo));
                    }
                    f.contador -= 3;
                }
                f.restante_s -= 1;
                continue;
            }
            let acumula = f.efeito.dano_no_tempo().is_some()
                || matches!(
                    f.efeito,
                    Efeito::Hpgen | Efeito::Mpgen | Efeito::Magicleak | Efeito::Apgencont2 | Efeito::Incrementalhpgen
                );
            if acumula {
                f.contador += 1;
                if f.contador >= 3 || 1 >= f.restante_s {
                    let v = f.por_segundo * f.contador;
                    // `filter_Incrementalhpgen::Heartbeat`: `_health += _inc_each × _counter`.
                    if f.efeito == Efeito::Incrementalhpgen {
                        f.por_segundo += f.escala_defesa * f.contador;
                    }
                    f.contador -= 3;
                    match f.efeito {
                        Efeito::Hpgen | Efeito::Incrementalhpgen => tiques.push(Tique::Cura(v)),
                        Efeito::Mpgen => tiques.push(Tique::Mana(v)),
                        // `filter_Magicleak::Heartbeat`: `DrainMana(_damage × _counter)`.
                        Efeito::Magicleak => tiques.push(Tique::Mana(-v)),
                        // `filter_Apgencont2::Heartbeat`: `ModifyAP(_ap × _counter)`.
                        Efeito::Apgencont2 => tiques.push(Tique::Chi(v)),
                        _ => tiques.push(Tique::Dano {
                            origem: f.origem,
                            valor: v,
                        }),
                    }
                }
            }
            f.restante_s -= 1;
        }
        let antes = self.filtros.len();
        self.filtros.retain(|f| f.restante_s > 0);
        let invencivel_antes = self.invencivel_s > 0;
        if self.invencivel_s > 0 {
            self.invencivel_s -= 1;
        }
        // O `invincible_filter` que acaba também muda o estado visível (49, 1.5.5).
        let fim_do_invencivel = invencivel_antes && self.invencivel_s == 0;
        (tiques, self.filtros.len() != antes || fim_do_invencivel)
    }

    /// `filter_Wingshield::AdjustDamage` (`cskill/skill/skillfilter.h:4168-4195`).
    ///
    /// O original compara **um quinto** do golpe com o escudo. Enquanto couber ali, o dano
    /// passa a 20% e o escudo perde quatro vezes o que absorveu; quando não cabe mais, o
    /// dano é reduzido na proporção do que sobrou e o escudo zera. Abaixo de 6 o filtro se
    /// apaga.
    /// `filter_Rebirth::BeforeDeath`: com o filtro e o dado a favor (`rand() % 100 <
    /// chance`), ele se desfaz e devolve a fração da vida máxima a curar. Dado contra: nada, e o
    /// filtro sai com a morte (`REMOVE_ON_DEATH`).
    pub fn renascer(&mut self, dado: i32) -> Option<f32> {
        let i = self
            .filtros
            .iter()
            .position(|f| f.efeito == Efeito::Rebirth)?;
        if dado >= self.filtros[i].razao {
            return None;
        }
        Some(self.filtros.remove(i).fator.clamp(0.01, 1.0))
    }

    pub fn escudo_absorve(&mut self, dano: i32) -> i32 {
        let Some(f) = self
            .filtros
            .iter_mut()
            .find(|f| f.efeito == Efeito::Wingshield)
        else {
            return dano;
        };
        let quinto = dano as f32 * 0.2;
        let saida = if quinto < f.absorve {
            f.absorve -= quinto * 4.0;
            quinto
        } else if quinto > 1.0 {
            let r = 1.0 - f.absorve / quinto;
            f.absorve = 0.0;
            dano as f32 * r
        } else {
            dano as f32
        };
        if f.absorve < 6.0 {
            f.restante_s = 0;
        }
        saida.max(0.0) as i32
    }

    /// `ClearSpecFilter(FILTER_MASK_BUFF/DEBUFF)`.
    pub fn limpar(&mut self, bencaos: bool) -> bool {
        let antes = self.filtros.len();
        self.filtros.retain(|f| {
            let (bencao, maldicao) = f.bencao_e_maldicao();
            if bencaos {
                !bencao
            } else {
                !maldicao
            }
        });
        antes != self.filtros.len()
    }

    /// `FILTER_MASK_REMOVE_ON_DEATH` — todos os portados têm, **menos as formas**: o
    /// `Fairyform` é só `WEAK | HEARTBEAT` (`skillfilter.h:16822-16825`) e acaba pelo tempo; o
    /// `Foxform` é só `WEAK` (`:4611`) e fica até a 312 ser lançada de novo.
    pub fn ao_morrer(&mut self) {
        self.filtros
            .retain(|f| matches!(f.efeito, Efeito::Fairyform | Efeito::Foxform | Efeito::Tigerform | Efeito::Soulretort2));
        self.invencivel_s = 0;
    }

    /// `gnpc_imp::Reborn` (`gs/npc.cpp:1970-1980`): o monstro que volta do gerador limpa as
    /// maldições (`ClearSpecFilter(FILTER_MASK_DEBUFF)`) — o sangramento e todo dano no tempo
    /// inclusive — e não volta invencível.
    pub fn ao_renascer(&mut self) {
        self.filtros.retain(|f| !f.efeito.ficha().maldicao);
        self.invencivel_s = 0;
    }

    /// `Sleep::DoDamage`: dano acorda.
    pub fn ao_receber_dano(&mut self) -> bool {
        let antes = self.filtros.len();
        self.filtros.retain(|f| f.efeito != Efeito::Sleep);
        antes != self.filtros.len()
    }

    fn tem(&self, e: Efeito) -> bool {
        self.filtros.iter().any(|f| f.efeito == e)
    }

    /// A forma do objeto: `(_shape, FORM_CLASS)` na Forma Sombria (`_shape` 1,
    /// `skillfilter.h:16853`) e na da raposa (`_shape` = o `value` da 312, 1), nenhuma fora
    /// delas. `FORM_CLASS` = 1 (`cskill/skill/skill.h:84`). O byte que vai ao cliente é da
    /// versão (`WorldProtocol::byte_de_forma`): 65 no 1.5.5, 1 no 1.2.6.
    pub fn forma(&self) -> Option<(u8, u8)> {
        self.filtros.iter().find_map(|f| match f.efeito {
            Efeito::Fairyform => Some((1, FORMA_DE_CLASSE)),
            Efeito::Foxform | Efeito::Tigerform => Some((f.contador as u8, FORMA_DE_CLASSE)),
            _ => None,
        })
    }

    /// `GetForm()` (`gs/actobject.h:1058`): 0 fora de forma, `FORM_CLASS` numa forma de classe.
    /// É o que o `allow_forms` das habilidades testa (`cskill/skill/skill.cpp:128`).
    pub fn forma_atual(&self) -> u8 {
        self.forma().map_or(0, |(_, forma)| forma)
    }

    /// `SetFoxform` com a raposa já ativa: `RemoveFilter(FILTER_FOXFORM)` e volta à forma
    /// humana (`playerwrapper.cpp:2541-2546`). `true` quando havia o que tirar.
    pub fn desfazer_raposa(&mut self) -> bool {
        let antes = self.filtros.len();
        self.filtros.retain(|f| f.efeito != Efeito::Foxform);
        antes != self.filtros.len()
    }

    /// `_lock_equipment` (`LockEquipment(true)` no `filter_Fairyform::OnAttach`): vestir,
    /// trocar, mover para o corpo e descartar peça são recusados com
    /// `ERR_EQUIPMENT_IS_LOCKED` (`gs/player.cpp:7874, 7991, 8077, 8258`).
    pub fn equipamento_travado(&self) -> bool {
        self.tem(Efeito::Fairyform) || self.tem(Efeito::Foxform) || self.tem(Efeito::Tigerform)
    }

    /// `SetTigerform` na forma de classe: `RemoveFilter(FILTER_TIGERFORM)` — na raposa não há tigre
    /// a tirar e nada muda (`playerwrapper.cpp:1787-1792`).
    pub fn desfazer_tigre(&mut self) -> bool {
        let antes = self.filtros.len();
        self.filtros.retain(|f| f.efeito != Efeito::Tigerform);
        antes != self.filtros.len()
    }

    /// `filter_Transportdamagetoleader`: `(ratio, alcance)`.
    pub fn transporte_ao_dono(&self) -> Option<(f32, f32)> {
        self.filtros
            .iter()
            .find(|f| f.efeito == Efeito::Transportdamagetoleader)
            .map(|f| (f.fator, f.fator_de_habilidade))
    }

    /// O chi que cada golpe recebido mexe (`EF_DoDamage`): `filter_Apgen` dá, `filter_Apleak` tira.
    pub fn chi_ao_apanhar(&self) -> i32 {
        self.filtros
            .iter()
            .map(|f| match f.efeito {
                Efeito::Apgen => f.por_segundo,
                Efeito::Apleak => -f.por_segundo,
                _ => 0,
            })
            .sum()
    }

    /// `filter_Retort(2)::AdjustDamage`: quanto do golpe **físico corpo a corpo** (dano bruto
    /// `fisico`, antes da defesa) volta ao atacante — golpe normal ou de habilidade, de monstro
    /// ou de jogador. `Retort2` usa `_ratio_skill` no golpe de habilidade (`msg.skill_id ?
    /// _ratio_skill : _ratio`, `skillfilter.h:14646`); `Retort`, o `ratio` nos dois (`:1480`).
    /// `None` sem espinhos ou com o resultado ≤ 1 (`:1480-1484`, `:14646-14650`). O teto de
    /// 1.000.000 do 1.5.5 não existe no 1.2.6 e não muda nada abaixo dele.
    pub fn espinhos(&self, fisico: i32, de_habilidade: bool) -> Option<i32> {
        if fisico >= 1_000_000 {
            return None;
        }
        let f = self
            .filtros
            .iter()
            .find(|f| matches!(f.efeito, Efeito::Retort | Efeito::Retort2 | Efeito::Jingji))?;
        self.espinhos_do_filtro(f, fisico, de_habilidade)
    }

    /// `filter_Soulbase::AdjustDamage` + `filter_Soulretort2::DoSoulAdjustDamage`
    /// (`skillfilter.h:12611-12710`): `del_on_trigger` 2 apaga já; `rand() % 100 >= trigger_prob`
    /// ou sem mana para o custo, nada; senão devolve `(int)(físico × ratio)` (> 1), drena o custo e,
    /// com `del_on_trigger` 1, apaga. Vale de longe também. Devolve o dano e a mana gasta.
    pub fn espinhos_de_alma(&mut self, fisico: i32, dado: i32, mana: i32) -> Option<(i32, i32)> {
        let i = self.filtros.iter().position(|f| f.efeito == Efeito::Soulretort2)?;
        let f = self.filtros[i].clone();
        if f.escala_defesa == 2 {
            self.filtros.remove(i);
        }
        if dado >= f.por_segundo || (f.contador > 0 && mana < f.contador) {
            return None;
        }
        let dano = (fisico as f32 * f.fator) as i32;
        if dano <= 1 {
            return None;
        }
        if f.escala_defesa == 1 {
            self.filtros.retain(|g| g.efeito != Efeito::Soulretort2);
        }
        Some((dano, f.contador.max(0)))
    }

    fn espinhos_do_filtro(&self, f: &Filtro, fisico: i32, de_habilidade: bool) -> Option<i32> {
        // `filter_Jingji::AdjustDamage` devolve igual ao `Retort`, com o `ratio` (`skillfilter.h:4743-4768`).
        let razao = if de_habilidade && f.efeito == Efeito::Retort2 {
            f.fator_de_habilidade
        } else {
            f.fator
        };
        let dano = (fisico as f32 * razao) as i32;
        (dano > 1).then_some(dano)
    }

    /// `filter_Freemove` ligado: a máscara `IMMUNESLOW|FIX|DIZZY|SLEEP` recusa esses quatro
    /// (`SetSlow/Fix/Dizzy/Sleep`, `playerwrapper.cpp`, `GetImmuneMask()`).
    pub fn imune_a(&self, e: Efeito) -> bool {
        self.tem(Efeito::Freemove) && matches!(e, Efeito::Slow | Efeito::Fix | Efeito::Dizzy | Efeito::Sleep)
    }

    /// `MODE_INDEX_STUN`/`SLEEP`: não age.
    pub fn sem_acao(&self) -> bool {
        self.tem(Efeito::Dizzy) || self.tem(Efeito::Sleep) || self.tem(Efeito::Palsy)
    }
    /// `MODE_INDEX_ROOT`: não anda.
    pub fn preso(&self) -> bool {
        self.sem_acao() || self.tem(Efeito::Fix)
    }
    /// `MODE_INDEX_SILENT`: não conjura.
    pub fn selado(&self) -> bool {
        self.sem_acao() || self.tem(Efeito::Sealed) || self.tem(Efeito::Sealed2)
    }
    pub fn invencivel(&self) -> bool {
        self.invencivel_s > 0 || self.tem(Efeito::Invincible) || self.gm_invencivel || self.gm_invisivel
    }

    pub fn realce(&self) -> Realce {
        use Efeito::*;
        let mut r = Realce {
            dano_recebido: 1.0,
            ..Default::default()
        };
        for f in &self.filtros {
            let k = f.razao;
            match f.efeito {
                Slow => r.velocidade -= k,
                Speedup => r.velocidade += k,
                Incattack => r.dano += k,
                Decattack => r.dano -= k,
                Incmagic => r.magia += k,
                Decmagic => r.magia -= k,
                Incdefence | Ironshield => r.defesa += k,
                Decdefence => r.defesa -= k,
                Incresist => r.resistencia += k,
                Decresist => r.resistencia -= k,
                Incdodge => r.evasao += k,
                Decdodge => r.evasao -= k,
                Incaccuracy => r.precisao += k,
                Decaccuracy => r.precisao -= k,
                Inchpsteal => r.roubo_de_vida += k,
                Incdamagedodge => r.esquiva_de_dano += k,
                Incdebuffdodge => r.esquiva_de_maldicao += k,
                Fastattack => r.velocidade_de_ataque -= k,
                Slowattack => r.velocidade_de_ataque += k,
                Fastpray => r.conjuracao += k,
                Slowpray => r.conjuracao -= k,
                Inchp => r.vida += k,
                Dechp => r.vida -= k,
                Incsmite => r.critico += f.por_segundo,
                Swiftform => r.velocidade += k,
                Jingji => r.dano_recebido *= f.fator_de_habilidade,
                Canti => r.dano_recebido *= 1.0 + f.fator,
                Noregain => r.regen_pct -= 2000,
                Reducewood => r.resistencias[1] -= k,
                Giant => r.dano += k,
                Devilstate => {
                    r.precisao += k;
                    r.vida -= f.escala_defesa;
                }
                Tigerform => {
                    r.velocidade += k;
                    r.vida += f.por_segundo;
                    r.dano += f.escala_defesa;
                }
                Beastieform => r.velocidade += k,
                Inccritresistance => r.resistencia_a_critico += f.por_segundo,
                Fairyform => {
                    r.velocidade += k;
                    r.defesa += f.escala_defesa;
                }
                // `ImpairScaleMaxMP(_decmp)`, `EnhanceScaleDefense(_incdefence)`,
                // `EnhanceScaleAttack(_incaccuracy)` (`skillfilter.cpp:393-395`).
                Foxform => {
                    r.mana -= k;
                    r.defesa += f.escala_defesa;
                    r.precisao += f.por_segundo;
                }
                Inchurt => r.dano_recebido *= 1.0 + f.fator,
                Dechurt => r.dano_recebido *= 1.0 - f.fator,
                _ => {}
            }
        }
        r
    }

    /// Os seis `DWORD` de estado visível (`gactive_imp::UpdateVisibleState`,
    /// `actobject.cpp:1531-1590`).
    pub fn estados_visiveis(&self) -> [u32; 6] {
        let mut s = [0u32; 6];
        // `invincible_filter::OnAttach` → `IncVisibleState(49)` (`gs/invincible_filter.cpp:
        // 13-22`, 1.5.5): o efeito sobre o monstro que volta para casa. O `gs` 1.2.6 não liga
        // estado nenhum nesse filtro (VA 0x812f6ee), e o `UPDATE_EXT_STATE` do 1.2.6 só leva
        // os estados 0..31 — o bit 49 não chega lá, como no original.
        if self.invencivel_s > 0 || self.gm_invencivel {
            s[ESTADO_VISIVEL_INVENCIVEL / 32] |= 1 << (ESTADO_VISIVEL_INVENCIVEL % 32);
        }
        for f in &self.filtros {
            let v = match f.efeito {
                Efeito::Insertvstate => f.contador.max(0) as usize,
                // `if(_insertvstate) IncVisibleState(...)`.
                Efeito::Apgencont2 if f.escala_defesa != 0 => 50,
                Efeito::Sealed2 if f.escala_defesa != 0 => 8,
                e => e.ficha().visivel as usize,
            };
            if v > 0 && v < 192 {
                s[v / 32] |= 1 << (v % 32);
            }
        }
        s
    }

    /// Os ícones com o tempo restante (`_visible_team_state`, um parâmetro cada:
    /// `InsertTeamVisibleState(HSTATE, _timeout)`), sem repetir ícone.
    pub fn icones(&self) -> Vec<(u16, i32)> {
        let mut out: Vec<(u16, i32)> = Vec::new();
        for f in &self.filtros {
            if !f.icone {
                continue;
            }
            let h = if f.efeito == Efeito::Insertvstate {
                f.por_segundo.clamp(0, u16::MAX as i32) as u16
            } else {
                f.efeito.ficha().icone
            };
            if h != 0 && !out.iter().any(|(x, _)| *x == h) {
                // A raposa entra sem parâmetro (`InsertTeamVisibleState(HSTATE_FOXFORM)`):
                // [`SEM_PARAMETRO`] faz o `ICON_STATE_NOTIFY` mandar o ícone sem tempo.
                let param = if matches!(f.efeito, Efeito::Foxform | Efeito::Tigerform) {
                    pw_protocol::packets::s2c::SEM_PARAMETRO
                } else {
                    f.restante_s
                };
                out.push((h, param));
            }
        }
        out
    }
}

/// O dano que chega a um objeto, depois dos filtros dele: invencível não leva nada
/// (`SetInvincibleFilter`), `Inchurt`/`Dechurt` multiplicam (`AdjustDamage`,
/// `skillfilter.h:1838-1860`, `3239-3260`) e o sono acaba (`filter_Sleep::DoDamage`).
pub fn dano_recebido(efeitos: &mut Efeitos, dano: i32) -> i32 {
    if dano <= 0 {
        return dano;
    }
    if efeitos.invencivel() {
        return 0;
    }
    efeitos.ao_receber_dano();
    let dano = efeitos.escudo_absorve(dano);
    ((dano as f32) * efeitos.realce().dano_recebido) as i32
}

// =============================================================================
// O roteiro de StateAttack/BlessMe
// =============================================================================

/// Um efeito que passou no dado, com os parâmetros daquele momento do roteiro.
#[derive(Debug, Clone, PartialEq)]
pub struct Aplicacao {
    pub nome: String,
    pub efeito: Option<Efeito>,
    pub tempo_s: i32,
    pub razao: f32,
    pub valor: f32,
    pub quantia: f32,
    pub mostra_icone: bool,
    /// O `probability` do `PlayerWrapper` na hora do setter — o `SetRebirth` o usa como chance
    /// (`(int)(probability)`).
    pub probabilidade: f32,
}

/// `PlayerWrapper` do lado da vítima: os parâmetros que os `SetX` guardam
/// (`playerwrapper.h:160-205`) e o dado (`ThrowDice`, `:170-178`), que **fixa** a
/// probabilidade em 100 ou 0 depois de rolar — o efeito seguinte sem `SetProbability`
/// herda o resultado.
/// Os efeitos que **não** consultam o dado no original.
///
/// No `cskill`, quem decide se a probabilidade entra é **cada setter**: dos 486
/// `PlayerWrapper::Set*` de `cskill/skill/playerwrapper.cpp`, 316 abrem com
/// `if (ThrowDice())` e estes 170 aplicam o filtro direto. `SetFirearrow`
/// (`playerwrapper.cpp:2333-2337`) é um deles — daí a Flecha Fulgurante (244) ser garantida,
/// embora o roteiro dela não tenha `SetProbability` (`cskill/skills/skill244.h:234-240`).
///
/// A lista foi **extraída do fonte**, não escrita à mão. `probability` nasce em zero
/// (`playerwrapper.h:61`) e `ThrowDice()` com zero devolve falso (`:169-178`): quem consulta o
/// dado sem `SetProbability` antes não aplica nada, e é assim no original.
const GARANTIDOS_SEM_DADO: &[&str] = &[
    "Absorbdamageincdefense",
    "Addball",
    "Adddefence",
    "Additionalattack",
    "Additionalheal",
    "Addmaxhp",
    "Addresistance",
    "Addskilldamage",
    "Airstreamlock",
    "Antiwater",
    "Apgen",
    "Apgen2",
    "Appendenchant",
    "Attachstatetoself",
    "Attachstatetotarget",
    "Attackattachstate1",
    "Attackattachstate2",
    "Attackattachstate3",
    "Attackattachstate4",
    "Aurabless2",
    "Aurabless3",
    "Auracurse2",
    "Auracurse4asn",
    "Beastieform",
    "Beattackattachstate1",
    "Beattackattachstate2",
    "Beattackattachstate3",
    "Beattackattachstate4",
    "Blessmagic",
    "Burningfeet",
    "Callupteammember",
    "Chanceofrebirth",
    "Charred",
    "Clearinvisible",
    "Clearinvisible2",
    "Comboid",
    "CommonCoolDown",
    "Debithurt",
    "Decdamagefromcrits",
    "Delaytransmit",
    "Denyattackcmd",
    "Devilstate",
    "Disappear",
    "Disturbrecover",
    "Dropmoneyondeath",
    "Earthguard",
    "Earthhurt",
    "Enmity",
    "Enternonpenaltypvp",
    "Entrap",
    "Entrap2",
    "Fairyform",
    "Fastprayincmagic",
    "Feathershield",
    "Filpball",
    "Firearrow",
    "Firehurt",
    "Fishform",
    "Flower1",
    "Flower2",
    "Flower3",
    "Flower4",
    "Foxform",
    "Freemove",
    "Freemoveapgen",
    "Frenetic",
    "Frighten",
    "Giant",
    "GiantForm",
    "Goldhurt",
    "Hardenskin",
    "Healsteal",
    "Homefeeling",
    "Immunedrop",
    "Incantiinvisiblepassive",
    "Incatkdefhp",
    "Incatkdefhp2",
    "Incattackondamage",
    "Incbow",
    "Incboxing",
    "Inccrit",
    "Incdagger",
    "Incdefencesmite",
    "Incdefensedegree",
    "Incearth",
    "Incfarnormaldmgreduce",
    "Incfarskilldmgreduce",
    "Incfeather",
    "Incfight",
    "Incfightproperty",
    "Incfire",
    "Incgold",
    "Inchammer",
    "Inchitrate",
    "Inchpgen",
    "Inchurt3",
    "Incinvisiblepassive",
    "Incmaxhpatkdfdlevel",
    "Incmpgen",
    "Incnearnormaldmgreduce",
    "Incnearskilldmgreduce",
    "Incpenres",
    "Incpetattackdegree",
    "Incpetdamage",
    "Incpetdefenddegree",
    "Incpetdefense",
    "Incpethp",
    "Incpetmagicdamage",
    "Incpetmagicdefense",
    "Incpetmp",
    "Incrange",
    "Incrementalhpgen",
    "Incresistmagic",
    "Incscimitar",
    "Incspear",
    "Incswim",
    "Incswimspeed",
    "Incsword",
    "Inctalisman",
    "Incwater",
    "Incwood",
    "Incwoodwaterdefense",
    "Insertvstate",
    "Ironshield",
    "Jingji",
    "Leavenonpenaltypvp",
    "Longjumptospouse",
    "Magicfrenetic",
    "MnfactionDecresist",
    "Moongod",
    "Panruo",
    "Perform",
    "Petsacrifice",
    "Physichurt",
    "Plantsuicide",
    "Powerup",
    "Queryotherinventory",
    "Rebirth",
    "Rebirth2",
    "Reduceresurrectexplost",
    "Repelonnormalattack",
    "Resurrect",
    "Retortmagic",
    "Returntown",
    "Sandstorm",
    "Shadowform",
    "Soulbeatback",
    "Soulretort",
    "Soulretort2",
    "Soulsealed",
    "Soulstun",
    "Specialphysichurt",
    "Specialslow",
    "Startcallup",
    "Stoneskin",
    "Summonpet2",
    "Summonplantpet",
    "Swiftform",
    "TalentData",
    "Thunderform",
    "Tigerform",
    "Transportdamagetopet",
    "Transportmptopet",
    "Vacuum",
    "Waterhurt",
    "Windshield",
    "Wingshield",
    "Woodhurt",
    "Xisui",
    "Yijin",
];

/// O efeito é aplicado sem passar pelo dado?
fn garantido_sem_dado(setter: &str) -> bool {
    GARANTIDOS_SEM_DADO.binary_search(&setter).is_ok()
}

pub fn executar_roteiro(
    passos: &[(String, String, String)],
    vars: &dyn Fn(&str) -> Option<f64>,
    dado: &mut dyn FnMut() -> i32,
) -> (Vec<Aplicacao>, Vec<String>) {
    // `probability` nasce zerado no `PlayerWrapper` (`playerwrapper.h:61`).
    let mut prob = 0.0f64;
    let mut tempo_s = 0i32;
    let (mut razao, mut valor, mut quantia) = (0.0f32, 0.0f32, 0.0f32);
    let mut icone = false;
    let mut aplicar = Vec::new();
    let mut nao_lidos = Vec::new();
    for (quem, setter, e) in passos {
        if quem != "V" {
            continue;
        }
        let Some(v) = expr::avaliar(e, vars) else {
            nao_lidos.push(format!("{setter}({e})"));
            continue;
        };
        match setter.as_str() {
            "Probability" => {
                if v >= 0.0 {
                    prob = v;
                }
            }
            // `SetTime`: `(int)((t + 0.00001) / 1000)`.
            "Time" => tempo_s = ((v as f32 + 0.00001) / 1000.0) as i32,
            "Ratio" => razao = v as f32,
            "Value" => valor = v as f32,
            "Amount" => quantia = v as f32,
            "Showicon" => icone = v != 0.0,
            nome => {
                // Quem não consulta o dado entra sempre — ver [`GARANTIDOS_SEM_DADO`].
                let passou = if garantido_sem_dado(nome) {
                    true
                } else if prob > 99.0 {
                    true
                } else if prob < 0.001 {
                    false
                } else {
                    let ok = (dado() as f64) < prob;
                    prob = if ok { 100.0 } else { 0.0 };
                    ok
                };
                if passou {
                    aplicar.push(Aplicacao {
                        nome: nome.to_string(),
                        efeito: Efeito::do_setter(nome),
                        tempo_s,
                        razao,
                        valor,
                        quantia,
                        mostra_icone: icone,
                        probabilidade: prob as f32,
                    });
                }
            }
        }
    }
    (aplicar, nao_lidos)
}

/// Variáveis de um roteiro. `L` é o nível; `P_X` quem conjura, `V_X` a vítima, `S_X` a
/// habilidade. Talentos (`S_T0..T2`) valem 0 — o servidor não tem talentos.
pub fn variaveis<'a>(
    nivel: i32,
    jogador: &'a HashMap<&'static str, f64>,
    vitima: &'a HashMap<&'static str, f64>,
    habilidade: &'a HashMap<&'static str, f64>,
) -> impl Fn(&str) -> Option<f64> + 'a {
    move |n: &str| {
        if n == "L" {
            return Some(nivel as f64);
        }
        if let Some(x) = n.strip_prefix("P_") {
            return jogador.get(x).copied();
        }
        if let Some(x) = n.strip_prefix("V_") {
            return vitima.get(x).copied();
        }
        if let Some(x) = n.strip_prefix("S_") {
            return match x {
                "T0" | "T1" | "T2" => Some(0.0),
                "Rand" => Some(rand::random::<u32>() as f64 % 100.0),
                _ => habilidade.get(x).copied(),
            };
        }
        None
    }
}

#[cfg(test)]
mod testes {
    /// B309: a lista que o Estúdio oferece (`pw_data_loader::roteiro::SETTERS_DO_GS`) é a do `Efeito::do_setter`:
    /// todo nome dela é portado, e todo setter dos dois catálogos é portado se e só se está nela.
    #[test]
    fn os_setters_do_estudio_sao_os_do_gs() {
        use pw_data_loader::roteiro::{ACUMULADORES, SETTERS_DO_GS};
        for n in SETTERS_DO_GS {
            assert!(Efeito::do_setter(n).is_some(), "{n} está na lista e o GS não porta");
        }
        let mut vistos = 0;
        for v in ["1.5.5", "1.2.6"] {
            let j: serde_json::Value = serde_json::from_str(pw_data_loader::habilidades::json_do_servidor(v).unwrap()).unwrap();
            for h in j["habilidades"].as_object().unwrap().values() {
                for r in ["no_alvo", "em_si"] {
                    for p in h[r].as_array().into_iter().flatten() {
                        let n = p[1].as_str().unwrap();
                        if ACUMULADORES.contains(&n) {
                            continue;
                        }
                        vistos += 1;
                        assert_eq!(Efeito::do_setter(n).is_some(), SETTERS_DO_GS.contains(&n), "{v}: {n}");
                    }
                }
            }
        }
        assert!(vistos > 1000);
    }

    /// B203: as passivas comuns do catálogo 1.5.5 — 2446 (`EVENT_RESET`, defesa e resistência
    /// `0,08·L`), 1188 (mana `2·L` por batimento), 1371 (`EVENT_WIELD`, espada `0,06·L`), 1263
    /// (adaga 0,15 e crítico 2 com qualquer arma) e 11 (`Incgold` `0,02·L`, B330).
    #[test]
    fn as_passivas_comuns_somam_o_que_o_take_effect_manda() {
        use std::collections::HashMap;
        let tabela = pw_data_loader::habilidades::TabelaDeHabilidades::do_155();
        let conhecidas: HashMap<u32, u8> = [(2446, 2), (1188, 3), (1371, 3), (1263, 1), (11, 5)].into_iter().collect();
        let com_espada = passivas_comuns(&tabela, &conhecidas, classe_de_arma::ESPADA, false);
        assert!((com_espada.elemento[0] - 0.1).abs() < 1e-6, "Incgold 0,02 × 5");
        assert_eq!((com_espada.defesa, com_espada.resistencia, com_espada.regen_mana), (16, 16, 6));
        assert_eq!((com_espada.dano, com_espada.critico), (18, 2), "espada da 1371; crítico da 1263 com arma");
        let com_adaga = passivas_comuns(&tabela, &conhecidas, classe_de_arma::ADAGA, false);
        assert_eq!((com_adaga.dano, com_adaga.critico), (15, 2));
        let sem_arma = passivas_comuns(&tabela, &conhecidas, 0, false);
        assert_eq!((sem_arma.dano, sem_arma.critico, sem_arma.defesa), (0, 0, 16), "sem arma, só as EVENT_RESET");
    }

    /// B330 — as passivas que faltavam, no catálogo 1.5.5: 2443 `Addskilldamage 0,02·L`, 2691
    /// `Incnearskilldmgreduce 0,03 + 0,02·L`, 255 `Incrange L + 1` (`EVENT_WIELD`, só com arco), 190
    /// `Incfeather 0,2·L` (`EVENT_WIELD`, só com asas), 1804 `Reduceresurrectexplost 100`, 1949 `Immunedrop`,
    /// 1081 `Incinvisiblepassive 2·L`; e o `SetSkillInc` que atribui: 2831 (`Incgold 0,25`) e 2832
    /// (`Incgold 0,2`) — vale a de id maior.
    #[test]
    fn as_passivas_de_fidelidade_do_b330() {
        use std::collections::HashMap;
        let tabela = pw_data_loader::habilidades::TabelaDeHabilidades::do_155();
        let c: HashMap<u32, u8> = [(2443, 10), (2691, 1), (255, 3), (190, 5), (1804, 1), (1949, 1), (1081, 4), (2831, 1), (2832, 1)]
            .into_iter()
            .collect();
        let arco = passivas_comuns(&tabela, &c, classe_de_arma::ARCO, false);
        assert_eq!(arco.realce_de_habilidade2, 20);
        assert!((arco.reducao_perto_habilidade - 0.05).abs() < 1e-6);
        assert_eq!((arco.alcance, arco.voo), (4.0, 0.0), "Incrange com arco; Incfeather sem asas não");
        assert_eq!((arco.reducao_da_perda_ao_renascer, arco.imune_a_queda, arco.invisivel_passivo), (100, true, 8));
        assert!((arco.elemento[0] - 0.2).abs() < 1e-6, "a 2832 vem depois da 2831 e o SetSkillInc atribui");
        let espada_e_asas = passivas_comuns(&tabela, &c, classe_de_arma::ESPADA, true);
        assert_eq!((espada_e_asas.alcance, espada_e_asas.voo), (0.0, 1.0), "Incrange sem arco não; Incfeather com asas");
        let so_asas = passivas_comuns(&tabela, &c, 0, true);
        assert_eq!(so_asas.voo, 1.0, "a asa sozinha já dispara o EventWield(65535)");
    }

    use super::*;

    /// B67 — a lista de efeitos garantidos saiu do fonte e está ordenada (a busca é binária).
    #[test]
    fn a_lista_de_garantidos_esta_ordenada_e_tem_o_firearrow() {
        let mut ordenada = GARANTIDOS_SEM_DADO.to_vec();
        ordenada.sort_unstable();
        assert_eq!(
            ordenada, GARANTIDOS_SEM_DADO,
            "a lista precisa estar ordenada"
        );
        assert!(
            garantido_sem_dado("Firearrow"),
            "SetFirearrow não consulta o dado no original"
        );
        assert!(garantido_sem_dado("Returntown"));
        // E quem consulta continua dependendo da probabilidade.
        assert!(
            !garantido_sem_dado("Speedup"),
            "SetSpeedup abre com if (ThrowDice())"
        );
        assert!(!garantido_sem_dado("Slow"));
    }

    /// B67 — sem `Probability`, o efeito garantido entra e o probabilístico não.
    ///
    /// É o comportamento do original: `probability` nasce em zero (`playerwrapper.h:61`) e
    /// `ThrowDice()` com zero é falso; `SetFirearrow` nem pergunta.
    #[test]
    fn sem_probability_o_garantido_entra_e_o_probabilistico_nao() {
        let passos: Vec<(String, String, String)> = vec![
            ("V".into(), "Time".into(), "600000".into()),
            ("V".into(), "Ratio".into(), "0.4".into()),
            ("V".into(), "Firearrow".into(), "1".into()),
            ("V".into(), "Slow".into(), "1".into()),
        ];
        let mut dado = || 50;
        let (aplicadas, _) = executar_roteiro(&passos, &sem_vars, &mut dado);
        let nomes: Vec<&str> = aplicadas.iter().map(|a| a.nome.as_str()).collect();
        assert_eq!(nomes, vec!["Firearrow"], "só o garantido devia entrar");
        assert_eq!(aplicadas[0].tempo_s, 600, "SetTime divide por 1000");
    }

    fn sem_vars(n: &str) -> Option<f64> {
        (n == "L").then_some(3.0)
    }

    #[test]
    fn a_expressao_do_stub_se_avalia_como_em_cpp() {
        assert_eq!(
            expr::avaliar("8 * L * L + 214.1 * L + 659.8", &sem_vars)
                .map(|v| (v * 10.0).round() / 10.0),
            Some(1374.1)
        );
        assert_eq!(expr::avaliar("1.0 * 5 + L", &sem_vars), Some(8.0));
        assert_eq!(expr::avaliar("L == 3 ? 100 : 0", &sem_vars), Some(100.0));
        assert_eq!(expr::avaliar("-5.5 + 7.5 * L", &sem_vars), Some(17.0));
        assert_eq!(expr::avaliar("INT(7.9) + 0.5f", &sem_vars), Some(7.5));
        assert_eq!(expr::avaliar("P_Maxhp * 2", &sem_vars), None);
    }

    fn passo(s: &str, e: &str) -> (String, String, String) {
        ("V".into(), s.into(), e.into())
    }

    /// A 10 (nível 3): Fallen 100%, Slow 90% com razão 0,15, Fix a 5 + L %. O dado que
    /// falha no Slow **zera** a probabilidade só até o próximo `SetProbability`.
    #[test]
    fn o_roteiro_rola_o_dado_e_herda_o_resultado() {
        let passos = vec![
            passo("Probability", "1.0 * 100"),
            passo("Time", "15000"),
            passo("Amount", "8 * L * L + 214.1 * L + 659.8"),
            passo("Fallen", "1"),
            passo("Probability", "1.0 * 90"),
            passo("Time", "15000"),
            passo("Ratio", "0.15"),
            passo("Slow", "1"),
            passo("Fix", "1"),
            passo("Probability", "1.0 * 5 + L"),
            passo("Time", "4000"),
            passo("Fix", "1"),
        ];
        let mut rolagens = vec![95, 0].into_iter();
        let (ap, nao) = executar_roteiro(&passos, &sem_vars, &mut || rolagens.next().unwrap());
        assert!(nao.is_empty());
        let nomes: Vec<_> = ap.iter().map(|a| (a.nome.as_str(), a.tempo_s)).collect();
        // Slow falhou (95 ≥ 90), e o Fix logo depois herdou o 0; o último Fix rolou 0 < 8.
        assert_eq!(nomes, vec![("Fallen", 15), ("Fix", 4)]);
    }

    /// B207: a Aura de Aço (77) aplica o `filter_Ironshield`: buff, ícone 65, defesa `+ratio×100`%.
    #[test]
    fn o_ironshield_soma_defesa() {
        let e = Efeito::do_setter("Ironshield").expect("portado");
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(e, 1800, 35)));
        assert_eq!(f.realce().defesa, 35);
        let ficha = e.ficha();
        assert!(!ficha.maldicao);
        assert_eq!(ficha.icone, 65);
    }

    /// B220: Traje de Flores (762) — `filter_Swiftform`, velocidade `+100 × amount`%, ícone 81,
    /// estado 32 (1.5.5).
    #[test]
    fn o_swiftform_acelera_e_acende_as_flores() {
        let e = Efeito::do_setter("Swiftform").expect("portado");
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(e, 300, 15)));
        assert_eq!(f.realce().velocidade, 15);
        assert_eq!(f.icones(), vec![(81, 300)]);
        assert_eq!(f.estados_visiveis()[1], 1, "VSTATE_FOXFORM 32");
    }

    /// B220: os filtros da Feiticeira — `Jingji` (310), `Canti`/`Noregain`/`Apleak` (320–322),
    /// `Magicleak` (318), `Reducewood` (635) e `Freemove` (764).
    #[test]
    fn os_filtros_da_feiticeira_seguem_o_skillfilter() {
        // Barreira de Espinhos: reduz o recebido, devolve como os espinhos e tira o `Retort`.
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(Efeito::Retort, 600, 10)));
        let mut j = filtro(Efeito::Jingji, 600, 0);
        j.fator = 0.2;
        j.fator_de_habilidade = 0.7;
        assert!(f.adicionar(j));
        assert_eq!(f.filtros.len(), 1, "FILTER_RETORT: o Jingji substitui o Retort");
        assert!((f.realce().dano_recebido - 0.7).abs() < 1e-6);
        assert_eq!(f.espinhos(100, false), Some(20));

        // As três maldições de espírito se substituem; o Canti aumenta o dano recebido.
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(Efeito::Noregain, 10, 0)));
        assert_eq!(f.realce().regen_pct, -2000);
        let mut c = filtro(Efeito::Canti, 10, 0);
        c.fator = 0.25;
        assert!(f.adicionar(c));
        assert_eq!(f.filtros.len(), 1, "FILTER_SPIRITCURSE");
        assert_eq!(f.realce().regen_pct, 0);
        assert!((f.realce().dano_recebido - 1.25).abs() < 1e-6);
        assert!(f.adicionar(filtro(Efeito::Apleak, 10, 0)));
        assert_eq!(f.filtros.len(), 1);

        // Reducewood: só a madeira; o segundo não muda o primeiro (`Merge` vazio).
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(Efeito::Reducewood, 10, 30)));
        assert!(!f.adicionar(filtro(Efeito::Reducewood, 99, 90)));
        assert_eq!(f.realce().resistencias, [0, -30, 0, 0, 0]);

        // Freemove: imune a lento, preso, atordoado e sono.
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(Efeito::Freemove, 10, 0)));
        assert!(f.imune_a(Efeito::Dizzy) && f.imune_a(Efeito::Slow));
        assert!(!f.imune_a(Efeito::Toxic));

        // Magicleak: drena o acumulado de 3 s.
        let mut f = Efeitos::default();
        let mut m = filtro(Efeito::Magicleak, 9, 0);
        m.por_segundo = 10;
        assert!(f.adicionar(m));
        let mut drenado = Vec::new();
        for _ in 0..3 {
            drenado.extend(f.batida().0);
        }
        assert_eq!(drenado, vec![Tique::Mana(-30)]);
    }

    /// B221: os filtros do Bárbaro — Força Divina, Sangue Fervente, Rugido do Tigre, Fúria Bestial.
    #[test]
    fn os_filtros_do_barbaro_seguem_o_skillfilter() {
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(Efeito::Giant, 1800, 12)));
        let mut d = filtro(Efeito::Devilstate, 600, 110);
        d.escala_defesa = 25;
        assert!(f.adicionar(d));
        let r = f.realce();
        assert_eq!((r.dano, r.precisao, r.vida), (12, 110, -25));

        // Tigre: forma de classe sem tempo, vida/velocidade/dano, sobrevive à morte, desfaz.
        let mut f = Efeitos::default();
        let mut t = filtro(Efeito::Tigerform, i32::MAX, 20);
        t.contador = 1;
        t.por_segundo = 10;
        t.escala_defesa = -20;
        assert!(f.adicionar(t));
        let r = f.realce();
        assert_eq!((r.velocidade, r.vida, r.dano), (20, 10, -20));
        assert_eq!(f.forma(), Some((1, FORMA_DE_CLASSE)));
        assert!(f.equipamento_travado());
        assert_eq!(f.icones(), vec![(47, pw_protocol::packets::s2c::SEM_PARAMETRO)]);
        f.ao_morrer();
        assert_eq!(f.forma_atual(), FORMA_DE_CLASSE, "WEAK: o tigre não sai com a morte");
        assert!(f.batida().0.is_empty() && f.forma().is_some(), "sem batimento");
        assert!(f.desfazer_tigre() && f.forma().is_none());

        // Fúria Bestial dá chi a cada golpe recebido; Devastar tira.
        let mut f = Efeitos::default();
        let mut g = filtro(Efeito::Apgen, 15, 0);
        g.por_segundo = 5;
        assert!(f.adicionar(g));
        let mut l = filtro(Efeito::Apleak, 15, 0);
        l.por_segundo = 2;
        assert!(f.adicionar(l));
        assert_eq!(f.chi_ao_apanhar(), 3);
    }

    /// B221: os filtros da Feiticeira 1.5.5 (seita/nível alto).
    #[test]
    fn os_filtros_da_feiticeira_155_seguem_o_skillfilter() {
        // Apgencont2: chi a cada 3 s; estado 50 só com `showicon`; máscara pelo roteiro.
        let mut f = Efeitos::default();
        let mut c = filtro(Efeito::Apgencont2, 3, 0);
        c.por_segundo = 33;
        c.mascara = Some((false, false));
        assert!(f.adicionar(c));
        assert_eq!(f.estados_visiveis()[1], 0);
        assert!(!f.limpar(true), "sem BUFF o Dispersar não tira");
        let mut chi = Vec::new();
        for _ in 0..3 {
            chi.extend(f.batida().0);
        }
        assert_eq!(chi, vec![Tique::Chi(99)]);

        // Palsy não age; Sealed2 sela; Insertvstate acende o estado e o ícone pedidos.
        let mut f = Efeitos::default();
        assert!(f.adicionar(filtro(Efeito::Palsy, 3, 0)));
        assert!(f.sem_acao() && f.selado());
        let mut f = Efeitos::default();
        let mut s2 = filtro(Efeito::Sealed2, 7, 0);
        s2.mascara = Some((false, false));
        assert!(f.adicionar(s2));
        assert!(f.selado() && !f.sem_acao());
        assert_eq!(f.icones(), vec![(140, 7)]);
        let mut f = Efeitos::default();
        let mut iv = filtro(Efeito::Insertvstate, 11, 0);
        iv.contador = 112;
        iv.por_segundo = 227;
        assert!(f.adicionar(iv));
        assert_eq!(f.estados_visiveis()[3], 1 << (112 - 96));
        assert_eq!(f.icones(), vec![(227, 11)]);

        // Inccritresistance: pontos de resistência a crítico.
        let mut f = Efeitos::default();
        let mut cr = filtro(Efeito::Inccritresistance, 7, 0);
        cr.por_segundo = 5;
        assert!(f.adicionar(cr));
        assert_eq!(f.realce().resistencia_a_critico, 5);

        // Soulretort2 da 1829: 100 % de chance, sem custo, não se apaga; também de longe.
        let mut f = Efeitos::default();
        let mut sr = filtro(Efeito::Soulretort2, 15, 0);
        sr.fator = 1.0;
        sr.por_segundo = 100;
        assert!(f.adicionar(sr));
        assert_eq!(f.espinhos_de_alma(300, 99, 0), Some((300, 0)));
        assert_eq!(f.filtros.len(), 1);
        f.ao_morrer();
        assert_eq!(f.filtros.len(), 1, "sem REMOVE_ON_DEATH");
        // Com custo e sem mana, não dispara; `del_on_trigger` 1 apaga ao disparar.
        f.filtros[0].contador = 50;
        assert_eq!(f.espinhos_de_alma(300, 0, 10), None);
        f.filtros[0].escala_defesa = 1;
        assert_eq!(f.espinhos_de_alma(300, 0, 60), Some((300, 50)));
        assert!(f.filtros.is_empty());
    }

    /// B222: a Erva Vital (1501) cura cada vez mais; a planta da Feiticeira passa o golpe ao dono.
    #[test]
    fn os_filtros_das_plantas() {
        let mut f = Efeitos::default();
        let mut c = filtro(Efeito::Incrementalhpgen, 12, 0);
        c.por_segundo = 10;
        c.escala_defesa = 5;
        assert!(f.adicionar(c));
        let mut curas = Vec::new();
        for _ in 0..6 {
            curas.extend(f.batida().0);
        }
        // 3 × 10 = 30; o `_health` sobe 5 × 3 = 15 → 25; 3 × 25 = 75.
        assert_eq!(curas, vec![Tique::Cura(30), Tique::Cura(75)]);
        let mut t = filtro(Efeito::Transportdamagetoleader, 60, 0);
        t.fator = 0.3;
        t.fator_de_habilidade = 45.0;
        assert!(f.adicionar(t));
        assert_eq!(f.transporte_ao_dono(), Some((0.3, 45.0)));
    }

    fn filtro(e: Efeito, s: i32, razao: i32) -> Filtro {
        Filtro {
            efeito: e,
            restante_s: s,
            razao,
            fator: razao as f32 / 100.0,
            por_segundo: 0,
            contador: 0,
            origem: 0,
            icone: true,
            absorve: 0.0,
            escala_defesa: 0,
            fator_de_habilidade: 0.0,
            mascara: None,
        }
    }

    #[test]
    fn unico_substitui_fraco_descarta_fundir_absorve() {
        let mut e = Efeitos::default();
        assert!(e.adicionar(filtro(Efeito::Slow, 10, 20)));
        assert!(e.adicionar(filtro(Efeito::Slow, 5, 50)));
        assert_eq!(e.filtros.len(), 1);
        assert_eq!(e.realce().velocidade, -50);
        assert!(e.adicionar(filtro(Efeito::Dizzy, 3, 0)));
        assert!(!e.adicionar(filtro(Efeito::Dizzy, 9, 0)));
        assert!(e.sem_acao() && e.preso() && e.selado());
        assert!(e.adicionar(filtro(Efeito::Incattack, 30, 10)));
        assert!(e.adicionar(filtro(Efeito::Incattack, 60, 25)));
        assert_eq!(
            e.filtros
                .iter()
                .filter(|f| f.efeito == Efeito::Incattack)
                .count(),
            1
        );
        assert_eq!(e.realce().dano, 25);
        assert_eq!(e.icones(), vec![(3, 5), (1, 3), (32, 60)]);
        let v = e.estados_visiveis();
        assert_eq!(v[0], (1 << 4) | (1 << 5) | (1 << 1));
    }

    /// `filter_Wounded`: dano total dividido pelo tempo, tique a cada 3 s com o acumulado.
    #[test]
    fn dano_no_tempo_tica_de_tres_em_tres() {
        let mut e = Efeitos::default();
        e.adicionar(Filtro {
            efeito: Efeito::Toxic,
            restante_s: 5,
            razao: 0,
            fator: 0.0,
            por_segundo: 20,
            contador: 0,
            origem: 7,
            icone: true,
            absorve: 0.0,
            escala_defesa: 0,
            fator_de_habilidade: 0.0,
            mascara: None,
        });
        let mut total = 0;
        for _ in 0..5 {
            for t in e.batida().0 {
                if let Tique::Dano { valor, origem } = t {
                    assert_eq!(origem, 7);
                    total += valor;
                }
            }
        }
        assert_eq!(total, 100);
        assert!(e.filtros.is_empty());
    }

    /// B132 — o sangramento (e todo dano no tempo) é maldição: o `Reborn` o tira
    /// (`ClearSpecFilter(FILTER_MASK_DEBUFF)`, `npc.cpp:1978`), e o monstro não volta sangrando
    /// com ódio de quem o pôs (Filhote de Doninha da WRA, 2026-09-27).
    #[test]
    fn o_renascimento_tira_o_dano_no_tempo() {
        use Efeito::*;
        for e in [Bleeding, Thunder, Toxic, Flood, Burning, Fallen] {
            assert!(e.ficha().maldicao, "{e:?} devia ser maldição");
            let mut ef = Efeitos::default();
            ef.filtros.push(filtro(e, 15, 10));
            ef.invencivel_s = 5;
            ef.ao_renascer();
            assert!(ef.filtros.is_empty(), "{e:?} sobreviveu ao renascimento");
            assert_eq!(ef.invencivel_s, 0);
        }
        let mut ef = Efeitos::default();
        ef.filtros.push(filtro(Bleeding, 15, 10));
        ef.ao_morrer();
        assert!(ef.filtros.is_empty(), "a morte tira o sangramento");
    }
}
