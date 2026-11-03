/* Mascotes no Estúdio (B394). Significado dos campos de `PET_ESSENCE` e `PET_EGG_ESSENCE`, as fórmulas de atributo por
   nível e o simulador de combate — tudo portado do que o nosso GS já faz, com a evidência do original:

   - campos: `ExpTypes.h:2983-3060` (cliente 1.5.5, `struct PET_ESSENCE`); o 1.2.6 (v7) tem só até `inhabit_type`;
   - fórmulas: `pet_dataman::GenerateBaseProp` (`gs/petdataman.h:23-76`, `petdataman.cpp:152-186`), em Rust em
     `pw_data_loader::pet::ModeloDeMascote` — aqui com as mesmas contas em `float` (Math.fround) e o truncamento `(int)`;
   - corpo de combate: `CreatePetBase` (`npcgenerator.cpp:2021-2040`) = `pw_gs::mascote::corpo_do_mascote`;
   - golpe: `AttackJudgement`/`HandleAttackMsg` (`actobject.cpp`) = `pw_gs::combat::resolver`.

   O simulador só tem o golpe normal: as habilidades do mascote e do monstro, os addons do `MONSTER_TYPE` e os
   adicionais do dono (`pet_enhance`) não entram — e a tela diz isso. */

import { IMUNIDADES } from "./monstro";

/* ---- tipos (`id_type` → `PET_CLASS_*`, `pet_dataman::LoadTemplate`; `pw_data_loader::pet::carregar_modelos`) ---- */
export interface TipoDeMascote { idType: number; classe: number; nome: string; explica: string; luta: boolean; so?: string }
export const TIPOS_DE_MASCOTE: TipoDeMascote[] = [
  { idType: 8782, classe: 1, nome: "Combate", luta: true, explica: "Luta ao lado do dono: ataca, defende, usa habilidades, ganha experiência." },
  { idType: 8781, classe: 0, nome: "Montaria", luta: false, explica: "Invocar é montar: o dono anda mais rápido, o mascote não luta." },
  { idType: 8783, classe: 2, nome: "Companhia", luta: false, explica: "Só acompanha o dono (visual), sem combate." },
  { idType: 28752, classe: 3, nome: "Invocação", luta: true, so: "1.5.5", explica: "Criatura invocada por habilidade: os atributos somam parte dos do dono (GenerateBaseProp2)." },
  { idType: 28913, classe: 4, nome: "Planta", luta: true, so: "1.5.5", explica: "Planta da Feiticeira/Místico: fica parada no lugar, com limite por grupo." },
  { idType: 37698, classe: 5, nome: "Evolução", luta: true, so: "1.5.5", explica: "Mascote evoluído: herda parte dos atributos e tem habilidade própria." },
];
export const tipoDoMascote = (idType: number) => TIPOS_DE_MASCOTE.find((t) => t.idType === idType);
/** Os que o simulador consegue montar: combate e evolução saem do `corpo_do_mascote`; invocação e planta dependem dos
    atributos do dono (`corpo_da_planta`, `GenerateBaseProp2`), que o Estúdio não tem. */
export const simulavel = (idType: number) => idType === 8782 || idType === 37698;

/** `inhabit_type` (ExpTypes.h:3038): 0 chão, 1 água, 2 ar, 3 chão+água, 4 chão+ar, 5 água+ar, 6 os três. O 1.2.6 aceita
    só 0–2 (o GS põe 0 no resto, `pet.rs`); o 1.5.5 vai até 6. */
export const AMBIENTES = ["Chão", "Água", "Ar", "Chão e água", "Chão e ar", "Água e ar", "Chão, água e ar"];
/** `food_mask` (ExpTypes.h:3036): forragem, carne, verduras, frutas, água. */
export const COMIDAS: [number, string][] = [[0x1, "Forragem"], [0x2, "Carne"], [0x4, "Verduras"], [0x8, "Frutas"], [0x10, "Água"]];
/** `stand_mode` (ExpTypes.h:3039): só visual. */
export const POSTURAS = ["0 · quatro patas", "1 · duas patas", "2 · flutuando"];
/** `plant_group` (ExpTypes.h:3041): normal, final 1, final 2 (`plant_pet_manager::ActivePlant`, `petman.cpp:2303-2317`). */
export const GRUPOS_DE_PLANTA = ["0 · normal", "1 · final 1", "2 · final 2"];
/** `immune_type` do mascote (ExpTypes.h:3044-3045): as do monstro e mais os estados. */
export const IMUNIDADES_DO_MASCOTE: [number, string][] = [...IMUNIDADES,
  [0x40, "Atordoar"], [0x80, "Sono"], [0x100, "Lentidão"], [0x200, "Prender"], [0x400, "Selar"], [0x800, "Dano dobrado"], [0x1000, "Empurrão"]];

/* ---- rótulos e explicações dos campos ---- */
export const ROTULOS: Record<string, string> = {
  Name: "Nome", id_type: "Tipo", file_model: "Modelo (.ecm)", file_icon: "Ícone", character_combo_id: "Classes que podem usar",
  level_max: "Nível máximo", level_require: "Nível exigido", pet_snd_type: "Som dos passos",
  size: "Tamanho do corpo", damage_delay: "Atraso do dano (s)", attack_range: "Alcance do golpe", attack_speed: "Intervalo do golpe (s)",
  sight_range: "Visão", food_mask: "Come", inhabit_type: "Ambiente", stand_mode: "Postura", plant_group: "Grupo de planta",
  group_limit: "Limite do grupo", immune_type: "Imune a", player_gain_skill: "Habilidade que o dono ganha ao sacrificar",
  require_dye_count: "Tinturas para tingir", id_pet_egg_evolved: "Ovo depois de evoluir", cost_index: "Custo da evolução (índice)",
  hp_inherit_max_rate: "Herda vida (até %)", attack_inherit_max_rate: "Herda ataque (até %)", defence_inherit_max_rate: "Herda defesa (até %)",
  attack_level_inherit_max_rate: "Herda grau de ataque (até %)", defence_level_inherit_max_rate: "Herda grau de defesa (até %)",
  specific_skill: "Habilidade exclusiva", file_gfx_short: "Efeito do golpe (.gfx)", id_evolved_skill_rand: "Sorteio de habilidades da evolução",
  combined_switch: "Chaves",
  // ovo
  id_pet: "Mascote que nasce", money_hatched: "Moedas para chocar", money_restored: "Moedas para restaurar", honor_point: "Lealdade inicial",
  level: "Nível ao nascer", exp: "Experiência ao nascer", skill_point: "Pontos de habilidade", price: "Preço de venda (NPC)",
  shop_price: "Preço de compra (loja)", pile_num_max: "Empilha até", file_matter: "Modelo no chão",
};
export const EXPLICACOES: Record<string, string> = {
  level_require: "Entra nas fórmulas de vida, regeneração, defesa, acerto, esquiva e resistência (o mascote 'começa' neste nível).",
  level_max: "Até onde ele sobe de nível.",
  size: "Raio do corpo: soma ao alcance do golpe (npcgenerator.cpp:2033).",
  attack_range: "Distância do golpe; o GS soma o tamanho do corpo.",
  attack_speed: "Segundos entre golpes (em tiques de 50 ms: s × 20 + 0,1).",
  damage_delay: "Atraso entre o golpe e o dano; precisa ficar entre 0,1 e 10 s ou o GS recusa o mascote (petdataman.cpp:120-133).",
  sight_range: "Precisa ser ≥ 0,1 ou o GS recusa o mascote.",
  character_combo_id: "Bits das classes que podem chocar/usar (ExpTypes.h:2991).",
  player_gain_skill: "Planta (1.5.5): habilidade que o dono recebe quando ela é sacrificada.",
  honor_point: "Lealdade de 0 a 999: até 50 tira 40% do dano, até 150 tira 20%, até 500 normal, acima de 500 +20% (pet_filter.cpp:8).",
  money_hatched: "Cobrado na Gerente de Mascotes ao chocar (serviço 28, serviceprovider.cpp:3160).",
  money_restored: "Cobrado para voltar o mascote a ovo (ConvertPetDataToEggData, player.cpp:14360-14415).",
  level: "O mascote nasce neste nível; o dono precisa estar no máximo 35 níveis abaixo para invocar.",
};
export const rotuloDoMascote = (n: string) => ROTULOS[n] ?? (/^(hp|hp_gen|damage|speed|attack|armor|physic_defence|magic_defence)_[abcd]$/.test(n) ? n : "");

/* ---- fórmulas por nível (`petdataman.h:23-76`) ---- */
export interface Formula {
  chave: keyof AtributosDoMascote; nome: string;
  /** campos a, b, c (d) na ordem do arquivo */
  campos: string[];
  /** a conta como o original escreve, para mostrar na tela */
  texto: string;
}
export const FORMULAS: Formula[] = [
  { chave: "vida", nome: "Vida", campos: ["hp_a", "hp_b", "hp_c"], texto: "a × (nível − b × nível exigido + c)" },
  { chave: "regeneracao", nome: "Regeneração de vida (por s)", campos: ["hp_gen_a", "hp_gen_b", "hp_gen_c"], texto: "a × (nível − b × nível exigido + c)" },
  { chave: "dano", nome: "Dano físico", campos: ["damage_a", "damage_b", "damage_c", "damage_d"], texto: "a × (b × nível² + c × nível + d)" },
  { chave: "acerto", nome: "Acerto", campos: ["attack_a", "attack_b", "attack_c"], texto: "a × (nível − b × nível exigido + c)" },
  { chave: "esquiva", nome: "Esquiva", campos: ["armor_a", "armor_b", "armor_c"], texto: "a × (nível − b × nível exigido + c)" },
  { chave: "defesa", nome: "Defesa física", campos: ["physic_defence_a", "physic_defence_b", "physic_defence_c", "physic_defence_d"], texto: "a × (b × (nível − c × nível exigido) + d)" },
  { chave: "resistencia", nome: "Defesa mágica (as 5)", campos: ["magic_defence_a", "magic_defence_b", "magic_defence_c", "magic_defence_d"], texto: "a × (b × (nível − c × nível exigido) + d)" },
  { chave: "correr", nome: "Velocidade (correr)", campos: ["speed_a", "speed_b"], texto: "a + b × (nível − 1); andar = metade" },
];
/** Só o `GenerateBaseProp2` (invocação e planta) usa (`petdataman.cpp:79-82`). */
export const FORMULAS_DE_INVOCACAO = [
  { campo: "mp_a", nome: "Mana", texto: "a × (nível + 5)" }, { campo: "mp_gen_a", nome: "Regeneração de mana", texto: "a × (nível + 5)" },
  { campo: "attack_degree_a", nome: "Grau de ataque", texto: "a × (nível + 5)" }, { campo: "defence_degree_a", nome: "Grau de defesa", texto: "a × (nível + 5)" },
];

export interface AtributosDoMascote {
  vida: number; regeneracao: number; dano: number; acerto: number; esquiva: number; defesa: number; resistencia: number;
  correr: number; andar: number;
}

type Valores = Record<string, number | string | undefined>;
const F = Math.fround;
const n32 = (v: Valores, c: string) => F(Number(v[c] ?? 0) || 0);
/** `(int)` do C: trunca em direção a zero. */
const int = (x: number) => Math.trunc(x);

/** `GenerateBaseProp` num nível, com as contas em `float` como o `ModeloDeMascote::atributos`. */
export function atributosNoNivel(v: Valores, nivel: number): AtributosDoMascote {
  const n = F(nivel), req = F(Number(v.level_require ?? 0) || 0);
  const linear = (a: string, b: string, c: string) => int(F(n32(v, a) * F(F(n - F(n32(v, b) * req)) + n32(v, c))));
  const comD = (a: string, b: string, c: string, d: string) => int(F(n32(v, a) * F(F(n32(v, b) * F(n - F(n32(v, c) * req))) + n32(v, d))));
  const correr = F(n32(v, "speed_a") + F(n32(v, "speed_b") * F(nivel - 1)));
  return {
    vida: linear("hp_a", "hp_b", "hp_c"),
    regeneracao: linear("hp_gen_a", "hp_gen_b", "hp_gen_c"),
    dano: int(F(n32(v, "damage_a") * F(F(F(F(n32(v, "damage_b") * n) * n) + F(n32(v, "damage_c") * n)) + n32(v, "damage_d")))),
    acerto: linear("attack_a", "attack_b", "attack_c"),
    esquiva: linear("armor_a", "armor_b", "armor_c"),
    defesa: comD("physic_defence_a", "physic_defence_b", "physic_defence_c", "physic_defence_d"),
    resistencia: comD("magic_defence_a", "magic_defence_b", "magic_defence_c", "magic_defence_d"),
    correr, andar: F(correr * 0.5),
  };
}

/** `nivel_de_lealdade` (`[50, 150, 500, 999]`) e `__pet_damage_adjust` (`pet_filter.cpp:8`): −40, −20, 0, +20 %. */
export const FAIXAS_DE_LEALDADE = [
  { ate: 50, nome: "Rebelde (0–50)", ajuste: -40 }, { ate: 150, nome: "Indiferente (51–150)", ajuste: -20 },
  { ate: 500, nome: "Leal (151–500)", ajuste: 0 }, { ate: 999, nome: "Fiel (501–999)", ajuste: 20 },
];
export const ajusteDeLealdade = (pontos: number) => (FAIXAS_DE_LEALDADE.find((f) => pontos <= f.ate) ?? FAIXAS_DE_LEALDADE[3]).ajuste;

/* ---- combatentes ---- */
export interface Combatente {
  nome: string; tipo: "mascote" | "monstro"; nivel: number; icone?: string | null;
  vida: number; regeneracao: number;
  danoMin: number; danoMax: number;
  /** `magic_damages_ext_N`: faixa por classe (metal, madeira, água, fogo, terra) */
  danoMagico: [number, number][];
  acerto: number; esquiva: number; defesa: number; resistencias: number[];
  grauDeAtaque: number; grauDeDefesa: number; imunidades: number;
  /** em tiques de 50 ms */
  intervalo: number; atraso: number; alcance: number;
}

/** O corpo de combate do mascote (`corpo_do_mascote`): atributos do nível, dano com a lealdade (divisão inteira),
    graus 0, sem dano mágico, alcance + corpo, imunidade do modelo. */
export function mascoteNoNivel(v: Valores, nivel: number, lealdade: number, nome = String(v.Name ?? "")): Combatente {
  const a = atributosNoNivel(v, Math.max(1, nivel));
  const dano = Math.max(0, Math.trunc((a.dano * (100 + ajusteDeLealdade(lealdade))) / 100));
  const tiques = (s: number) => int(F(F(F(s) * 20) + F(0.1)));
  return {
    nome, tipo: "mascote", nivel,
    vida: Math.max(1, a.vida), regeneracao: Math.max(0, a.regeneracao),
    danoMin: dano, danoMax: dano, danoMagico: [[0, 0], [0, 0], [0, 0], [0, 0], [0, 0]],
    acerto: a.acerto, esquiva: a.esquiva, defesa: a.defesa, resistencias: [a.resistencia, a.resistencia, a.resistencia, a.resistencia, a.resistencia],
    grauDeAtaque: 0, grauDeDefesa: 0, imunidades: Number(v.immune_type ?? 0) | 0,
    intervalo: tiques(Number(v.attack_speed ?? 0)), atraso: tiques(Number(v.damage_delay ?? 0)),
    alcance: F(Number(v.attack_range ?? 0) + Number(v.size ?? 0)),
  };
}

/** O monstro como o GS o monta (`MonsterEntity::do_template` sobre `pw_data_loader::monstros`), sem o addon do
    `MONSTER_TYPE` (o nosso GS também não sorteia). */
export function monstroDaFicha(v: Valores, nome = String(v.Name ?? "")): Combatente {
  const i = (c: string) => Math.trunc(Number(v[c] ?? 0) || 0);
  return {
    nome, tipo: "monstro", nivel: i("level"),
    vida: Math.max(1, i("life")), regeneracao: Math.max(0, i("hp_regenerate")),
    danoMin: i("damage_min"), danoMax: Math.max(i("damage_min"), i("damage_max")),
    danoMagico: [1, 2, 3, 4, 5].map((k) => [i(`magic_damages_ext_${k}_damage_min`), i(`magic_damages_ext_${k}_damage_max`)] as [number, number]),
    acerto: i("attack"), esquiva: i("armor"), defesa: i("defence"), resistencias: [1, 2, 3, 4, 5].map((k) => i(`magic_defences_${k}`)),
    grauDeAtaque: i("attack_degree"), grauDeDefesa: i("defend_degree"), imunidades: i("immune_type"),
    intervalo: int(F(F(F(Number(v.attack_speed ?? 0)) * 20) + F(0.5))), atraso: int(F(F(Number(v.damage_delay ?? 0)) * 20)),
    alcance: F(Number(v.attack_range ?? 0)),
  };
}

/* ---- o golpe (`pw_gs::combat`) ---- */
/** `AttackJudgement`: `rate / (rate + (armor >> 1))`, piso 0,05, sem teto. */
export function chanceDeAcerto(acerto: number, esquiva: number): number {
  const d = acerto + (esquiva >> 1);
  if (d === 0) return 0.05;
  return Math.max(F(acerto / d), 0.05);
}
/** `GetDamageReduce`: `def / (def + 40 × nível do atacante − 25)`, teto 0,95. */
export function reducaoPorDefesa(defesa: number, nivelDoAtacante: number): number {
  const d = F(F(defesa + F(40 * nivelDoAtacante)) - 25);
  if (d === 0) return 0;
  return Math.min(F(defesa / d), 0.95);
}

/** O dano de um golpe que acertou, dado o físico e os mágicos já sorteados (`resolver`, passos 4–7): defesa por classe
    com o nível de quem bate, imunidades, graus e o piso de 1. `null` = nada passou (imune a tudo). Mascote e monstro
    são NPC: sem crítico, sem punição de nível, sem ajuste de camada (`IS_HUMANSIDE` só para jogador). */
export function danoDoGolpe(a: Combatente, b: Combatente, fisico: number, magico: number[]): number | null {
  let total = 0, passou = false;
  if (fisico !== 0) {
    if (!(b.imunidades & 0x1)) { total = F(total + Math.max(0, F(fisico * F(1 - reducaoPorDefesa(b.defesa, a.nivel))))); passou = true; }
  }
  for (let k = 0; k < 5; k++) {
    if (!magico[k]) continue;
    if (b.imunidades & (1 << (k + 1))) continue;
    total = F(total + Math.max(0, F(magico[k] * F(1 - reducaoPorDefesa(b.resistencias[k], a.nivel)))));
    passou = true;
  }
  if (!passou) return null;
  const ajuste = a.grauDeAtaque - b.grauDeDefesa;
  const bruto = F(total + 0.5);
  const d = ajuste >= 0 ? F(bruto * F(1 + F(ajuste * 0.01))) : F(bruto / F(1 - F(ajuste * 0.012)));
  return Math.max(1, Math.trunc(d));
}

/** Sorteios do original: físico uniforme (`abase::Rand(low, high)`), elemental `RandNormal` (média de dois uniformes). */
const sorteioFisico = (min: number, max: number, r: () => number) => (min >= max ? min : min + Math.floor(r() * (max - min + 1)));
const sorteioElemental = (min: number, max: number, r: () => number) => (min >= max ? min : Math.trunc(((r() + r()) * 0.5) * (max - min + 1)) + min);

/* ---- o que se espera sem sorte (média) ---- */
export interface Esperado {
  chance: number; danoMedio: number; danoMin: number; danoMax: number;
  /** dano por segundo já com a chance de acerto */
  dps: number;
  /** dano por segundo menos a regeneração do alvo */
  liquido: number;
  /** segundos para derrubar o alvo (Infinity se a regeneração dele cobre) */
  tempo: number;
  golpesParaMatar: number;
  imune: boolean;
  /** o golpe mais forte derruba o alvo de vida cheia: a média engana (só as lutas sorteadas dizem quanto demora) */
  umGolpe: boolean;
}
export function esperado(a: Combatente, b: Combatente): Esperado {
  const chance = chanceDeAcerto(a.acerto, b.esquiva);
  const media = (x: number, y: number) => (x + Math.max(x, y)) / 2;
  const mag = a.danoMagico.map(([x, y]) => media(x, y));
  const dMin = danoDoGolpe(a, b, a.danoMin, a.danoMagico.map(([x]) => x));
  const dMax = danoDoGolpe(a, b, a.danoMax, a.danoMagico.map(([x, y]) => Math.max(x, y)));
  const dMed = danoDoGolpe(a, b, media(a.danoMin, a.danoMax), mag);
  const imune = dMed === null;
  const porGolpe = imune ? 0 : dMed!;
  const segundos = Math.max(1, a.intervalo) / 20;
  const dps = (chance * porGolpe) / segundos;
  const liquido = dps - b.regeneracao;
  return {
    chance, danoMedio: porGolpe, danoMin: dMin ?? 0, danoMax: dMax ?? 0, dps, liquido,
    tempo: liquido > 0 ? b.vida / liquido : Infinity,
    golpesParaMatar: porGolpe > 0 ? Math.ceil(b.vida / porGolpe) : Infinity,
    imune, umGolpe: !imune && (dMax ?? 0) >= b.vida,
  };
}

/* ---- a luta, tique a tique ---- */
export interface Ponto { t: number; a: number; b: number }
export interface Luta {
  vencedor: "a" | "b" | "empate"; segundos: number;
  golpes: [number, number]; acertos: [number, number]; dano: [number, number];
  vidaFinal: [number, number];
  /** a vida dos dois a cada segundo (só quando pedido) */
  curva?: Ponto[];
  /** os primeiros golpes, para o relato */
  relato?: string[];
}
/** Limite de uma luta: 10 minutos (12 000 tiques). Passou disso, empate. */
export const LIMITE_EM_TIQUES = 12_000;

/** Uma luta: os dois começam juntos, cada um golpeia a cada `intervalo` tiques e o dano chega `atraso` tiques depois
    (se quem bateu ainda estiver vivo). A cada 20 tiques (1 s) os vivos regeneram (`gnpc_imp::OnHeartbeat`,
    `npc.cpp:1946-1958`: em combate, `hp_gen` por batimento). Distância fixa no alcance: sem perseguição nem fuga. */
export function lutar(a: Combatente, b: Combatente, r: () => number = Math.random, detalhe = false): Luta {
  const lados = [a, b];
  const vida = [a.vida, b.vida];
  const golpes: [number, number] = [0, 0], acertos: [number, number] = [0, 0], dano: [number, number] = [0, 0];
  const proximo = [0, 0];
  const pendentes: { t: number; de: number; d: number }[] = [];
  const curva: Ponto[] | undefined = detalhe ? [{ t: 0, a: vida[0], b: vida[1] }] : undefined;
  const relato: string[] | undefined = detalhe ? [] : undefined;
  let t = 0;
  for (; t <= LIMITE_EM_TIQUES; t++) {
    // golpes que saem neste tique
    for (let k = 0; k < 2; k++) {
      if (vida[k] <= 0 || t < proximo[k]) continue;
      const x = lados[k], y = lados[1 - k];
      proximo[k] = t + Math.max(1, x.intervalo);
      golpes[k]++;
      if (r() > chanceDeAcerto(x.acerto, y.esquiva)) {
        if (relato && relato.length < 14) relato.push(`${(t / 20).toFixed(2)} s · ${x.nome} errou`);
        continue;
      }
      const fis = sorteioFisico(x.danoMin, x.danoMax, r);
      const mag = x.danoMagico.map(([m, M]) => sorteioElemental(m, M, r));
      const d = danoDoGolpe(x, y, fis, mag);
      if (d === null) { if (relato && relato.length < 14) relato.push(`${(t / 20).toFixed(2)} s · ${x.nome}: imune`); continue; }
      acertos[k]++;
      pendentes.push({ t: t + Math.max(0, x.atraso), de: k, d });
    }
    // dano que chega neste tique
    for (let i = pendentes.length - 1; i >= 0; i--) {
      const p = pendentes[i];
      if (p.t !== t) continue;
      pendentes.splice(i, 1);
      if (vida[p.de] <= 0) continue;
      const alvo = 1 - p.de;
      if (vida[alvo] <= 0) continue;
      vida[alvo] = Math.max(0, vida[alvo] - p.d);
      dano[p.de] += p.d;
      if (relato && relato.length < 14) relato.push(`${(t / 20).toFixed(2)} s · ${lados[p.de].nome} acertou ${p.d.toLocaleString("pt-BR")}`);
    }
    if (vida[0] <= 0 || vida[1] <= 0) break;
    if (t > 0 && t % 20 === 0) {
      for (let k = 0; k < 2; k++) vida[k] = Math.min(lados[k].vida, vida[k] + lados[k].regeneracao);
      curva?.push({ t: t / 20, a: vida[0], b: vida[1] });
    }
  }
  curva?.push({ t: t / 20, a: vida[0], b: vida[1] });
  const vencedor = vida[0] > 0 && vida[1] <= 0 ? "a" : vida[1] > 0 && vida[0] <= 0 ? "b" : "empate";
  return { vencedor, segundos: Math.min(t, LIMITE_EM_TIQUES) / 20, golpes, acertos, dano, vidaFinal: [vida[0], vida[1]], curva, relato };
}

export interface Resumo {
  lutas: number; vitorias: [number, number]; empates: number;
  segundosMedio: number; vidaRestante: [number, number];
  /** duração média das lutas que cada um venceu (NaN se não venceu nenhuma) */
  segundosAteVencer: [number, number];
  exemplo: Luta;
}
/** Muitas lutas para ter a chance de vitória (a sorte pesa no acerto e no sorteio do dano). */
export function simular(a: Combatente, b: Combatente, lutas = 400, r: () => number = Math.random): Resumo {
  let va = 0, vb = 0, emp = 0, seg = 0, ra = 0, rb = 0, sa = 0, sb = 0;
  for (let i = 0; i < lutas; i++) {
    const l = lutar(a, b, r);
    seg += l.segundos;
    if (l.vencedor === "a") { va++; ra += l.vidaFinal[0] / a.vida; sa += l.segundos; }
    else if (l.vencedor === "b") { vb++; rb += l.vidaFinal[1] / b.vida; sb += l.segundos; }
    else emp++;
  }
  return {
    lutas, vitorias: [va, vb], empates: emp, segundosMedio: seg / lutas,
    vidaRestante: [va ? ra / va : 0, vb ? rb / vb : 0],
    segundosAteVencer: [va ? sa / va : NaN, vb ? sb / vb : NaN],
    exemplo: lutar(a, b, r, true),
  };
}

/** Gerador com semente (para teste e para "refazer a mesma luta"). */
export function sementeira(semente: number): () => number {
  let s = semente >>> 0 || 1;
  return () => { s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0; return s / 4294967296; };
}
