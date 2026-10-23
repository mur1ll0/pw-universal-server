/* B371: o que cada campo do monstro (`MONSTER_ESSENCE`, `ExpTypes.h:2781-2960` do cliente 1.5.5) quer dizer, do jeito que o
   servidor usa (`cgame/gs/npcgenerator.cpp:205-345`, `aipolicy.h:918-926`, `ainpc.h:17-24`, `ainpc.cpp:209-230`). */

/** `id_strategy` → `aip.primary_strategy` (`npcgenerator.cpp:2600`), um de `STRATEGY_*` (`aipolicy.h:918-926`). */
export const ESTILOS_DE_LUTA: { valor: number; nome: string; explica: string }[] = [
  { valor: 0, nome: "Corpo a corpo", explica: "Corre até o alvo e bate." },
  { valor: 1, nome: "À distância", explica: "Ataca de longe com o golpe físico." },
  { valor: 2, nome: "Mágico", explica: "Luta com as habilidades; sem habilidade de ataque ou maldição o servidor avisa (npcgenerator.cpp:288)." },
  { valor: 3, nome: "Corpo a corpo + mágico", explica: "Mistura golpe e habilidade; também exige habilidade (npcgenerator.cpp:288)." },
  { valor: 4, nome: "Fixo", explica: "Não sai do lugar." },
  { valor: 5, nome: "Bicho (foge)", explica: "STRATEGY_CRITTER: criatura que não luta." },
  { valor: 6, nome: "Estaca", explica: "STRATEGY_STUB: alvo parado (boneco)." },
  { valor: 7, nome: "Fixo mágico", explica: "Não sai do lugar e usa habilidades." },
];

/** `aggressive_mode` (`ExpTypes.h`: 0 被动, 1 主动). */
export const ATITUDES = [
  { valor: 0, nome: "Passivo", explica: "Só luta se for atacado." },
  { valor: 1, nome: "Agressivo", explica: "Ataca quem entrar na distância de ódio." },
];

/** `patroll_mode`: o servidor só olha se é zero (`npcgenerator.cpp:244`); ≠ 0 anda quando está ocioso (`aipolicy.cpp:240`). */
export const PATRULHAS = [
  { valor: 0, nome: "Fica no lugar", explica: "Parado enquanto não luta." },
  { valor: 1, nome: "Anda à toa", explica: "Passeia pela área quando está ocioso." },
];

/** `inhabit_type` (`ExpTypes.h`: 地面、水下、空中、地面加水下、地面加空中、水下加空中、海陆空). */
export const HABITATS = ["Chão", "Água", "Ar", "Chão e água", "Chão e ar", "Água e ar", "Chão, água e ar"];

/** `aggro_strategy_N_id` → `aggro_policy` (`ainpc.h:17-24`), sorteado pelos pesos quando o monstro nasce
    (`npcgenerator.cpp:2562-2563`). No GS 1.5.5 as de 0 a 4 criam a mesma política (`ainpc.cpp:209-225`); os nomes 1–2 são
    os do cliente (`ExpTypes.h`: 普通、杀低级、杀血少). */
export const POLITICAS_DE_ODIO = [
  { valor: 0, nome: "Normal", explica: "Odeia quem causa mais dano." },
  { valor: 1, nome: "Mata os de nível baixo", explica: "Nome do cliente; no GS 1.5.5 age como Normal." },
  { valor: 2, nome: "Mata os de vida baixa", explica: "Nome do cliente; no GS 1.5.5 age como Normal." },
  { valor: 3, nome: "Política 3", explica: "No GS 1.5.5 age como Normal." },
  { valor: 4, nome: "Chefe", explica: "No GS 1.5.5 age como Normal." },
  { valor: 5, nome: "Lacaio do chefe", explica: "aggro_minor_policy: segue o ódio do chefe." },
  { valor: 6, nome: "Mascote", explica: "pet_aggro_policy." },
  { valor: 7, nome: "Torre", explica: "turret_aggro_policy." },
];

/** `short_range_mode` do monstro: 1 perto, 0 longe (`ExpTypes.h`; o contrário da ordem da arma). */
export const DISTANCIAS_DO_MONSTRO = ["0 · longe", "1 · perto"];

/** `immune_type` (`ExpTypes.h`): bits de baixo para cima — físico, metal, madeira, água… (fogo e terra seguem 金木水火土). */
export const IMUNIDADES: [number, string][] = [[0x1, "Físico"], [0x2, "Metal"], [0x4, "Madeira"], [0x8, "Água"], [0x10, "Fogo"], [0x20, "Terra"]];

/** `after_death` (`ExpTypes.h`: 自爆 0x1, 重生 0x2). */
export const APOS_A_MORTE: [number, string][] = [[0x1, "Explode"], [0x2, "Renasce"]];

/** `combined_switch` = `MONSTER_COMBINED_SWITCH` (`cgame/gs/template/exptypes.h:2962-2968`). */
export const CHAVES_DO_MONSTRO: [number, string][] = [
  [0x1, "Só quem o invocou pode atacar"], [0x2, "Jogador não consegue selecionar"], [0x4, "Não mostra imunidade na cabeça"],
  [0x8, "Grava ranking de dano ao morrer"],
];

export const ELEMENTOS = ["Metal", "Madeira", "Água", "Fogo", "Terra"];

/** Rótulos que o `rotulos.json` não tem, do comentário do campo no `ExpTypes.h`. */
export function rotuloDoMonstro(n: string): string | null {
  let m = n.match(/^magic_defences_(\d+)$/);
  if (m) return `Defesa de ${ELEMENTOS[Number(m[1]) - 1] ?? m[1]}`;
  m = n.match(/^magic_damages_ext_(\d+)_damage_(min|max)$/);
  if (m) return `Dano extra de ${ELEMENTOS[Number(m[1]) - 1] ?? m[1]} (${m[2] === "min" ? "mín." : "máx."})`;
  const extra: Record<string, string> = {
    id_pet_egg_captured: "Ovo de mascote ao ser capturado",
    short_range_mode: "Distância do golpe",
    no_accept_player_buff: "Não aceita bênção de jogador",
    id_building: "Instalação da base do clã",
    drop_for_no_profit_time: "Drop ignora o tempo sem lucro",
    attack_degree: "Nível de ataque",
    defend_degree: "Nível de defesa",
    invisible_lvl: "Nível de invisibilidade",
    uninvisible_lvl: "Nível de detecção",
    highest_frequency: "Sempre na frequência máxima (IA)",
    max_move_range: "Distância máxima de perseguição",
    damage_delay: "Atraso do dano do golpe (s)",
    domain_related: "Ligado a território",
    hp_adjust_common_value: "Ajuste de vida (variável global)",
    defence_adjust_common_value: "Ajuste de defesa (variável global)",
    attack_adjust_common_value: "Ajuste de ataque (variável global)",
  };
  return extra[n] ?? null;
}

export const CAMPOS_DO_MONSTRO = {
  vida: ["life", "hp_regenerate", "defence", "/^magic_defences_[0-9]+$/", "armor"],
  ataque: ["attack", "damage_min", "damage_max", "magic_damage_min", "magic_damage_max", "/^magic_damages_ext_[0-9]+_damage_(min|max)$/",
    "attack_range", "attack_speed", "damage_delay", "attack_degree", "defend_degree"],
  recompensa: ["exp", "skillpoint", "money_average", "money_var"],
  movimento: ["walk_speed", "run_speed", "fly_speed", "swim_speed"],
  visao: ["sight_range", "aggro_range", "aggro_time", "max_move_range"],
  avancado: ["faction", "monster_faction", "monster_faction_ask_help", "monster_faction_can_help", "role_in_war", "invisible_lvl", "uninvisible_lvl",
    "no_accept_player_buff", "highest_frequency", "no_auto_fight", "fixed_direction", "id_building", "hp_adjust_common_value",
    "defence_adjust_common_value", "attack_adjust_common_value", "stand_mode", "domain_related", "/^local_var_[0-9]+$/", "id_skill", "skill_level"],
};
