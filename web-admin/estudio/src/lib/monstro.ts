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
  { valor: 0, nome: "Normal", explica: "Ataca quem juntou mais ódio (dano, provocação). O ódio de quem some passa depois do tempo de ódio." },
  { valor: 1, nome: "Nível baixo (cliente)", explica: "O cliente chama de \"mata os de nível baixo\", mas o GS 1.5.5 cria a política Normal (ainpc.cpp:214)." },
  { valor: 2, nome: "Vida baixa (cliente)", explica: "O cliente chama de \"mata os de vida baixa\", mas o GS 1.5.5 cria a política Normal (ainpc.cpp:217)." },
  { valor: 3, nome: "Política 3", explica: "Igual à Normal no GS 1.5.5 (ainpc.cpp:220)." },
  { valor: 4, nome: "Chefe", explica: "Igual à Normal no GS 1.5.5 (ainpc.cpp:223); o chefe repassa o alvo aos lacaios." },
  { valor: 5, nome: "Lacaio do chefe", explica: "Quando o chefe repassa o alvo, joga fora o próprio ódio e passa a atacar só o alvo do chefe (aiaggro.h:9-27)." },
  { valor: 6, nome: "Mascote", explica: "Política de mascote (pet_aggro_policy): não é para monstro comum." },
  { valor: 7, nome: "Torre", explica: "Política de torre (turret_aggro_policy): não é para monstro comum." },
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
  const lv = n.match(/^local_var_([0-9]+)$/);
  if (lv) return `Variável local ${lv[1]} (valor inicial)`;
  let m = n.match(/^magic_defences_(\d+)$/);
  if (m) return `Defesa de ${ELEMENTOS[Number(m[1]) - 1] ?? m[1]}`;
  m = n.match(/^magic_damages_ext_(\d+)_damage_(min|max)$/);
  if (m) return `Dano extra de ${ELEMENTOS[Number(m[1]) - 1] ?? m[1]} (${m[2] === "min" ? "mín." : "máx."})`;
  const extra: Record<string, string> = {
    id_pet_egg_captured: "Ovo de mascote ao ser capturado",
    short_range_mode: "Distância do golpe",
    file_gfx_short: "Efeito do golpe (.gfx)",
    file_gfx_short_hit: "Efeito do acerto (.gfx)",
    no_accept_player_buff: "Não aceita bênção de jogador",
    id_building: "Instalação da base do clã",
    drop_for_no_profit_time: "Drop ignora o tempo sem lucro",
    attack_degree: "Nível de ataque",
    defend_degree: "Nível de defesa",
    invisible_lvl: "Nível de invisibilidade",
    uninvisible_lvl: "Nível de detecção",
    highest_frequency: "IA sempre ativa",
    no_auto_fight: "Não entra em luta sozinho",
    fixed_direction: "Direção fixa",
    stand_mode: "Postura do modelo",
    role_in_war: "Papel na batalha",
    common_strategy: "Roteiro de IA (aipolicy)",
    id_skill: "Habilidade única (antiga)",
    skill_level: "Nível da habilidade única (antiga)",
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
  invisibilidade: ["invisible_lvl", "uninvisible_lvl"],
  // B373: o que era "Avançado", cada um no lugar do que afeta.
  ia: ["common_strategy", "highest_frequency", "no_auto_fight", "/^local_var_[0-9]+$/"],
  movimentoExtra: ["fixed_direction", "stand_mode"],
  defesaExtra: ["no_accept_player_buff"],
  eventos: ["role_in_war", "id_building", "domain_related", "hp_adjust_common_value", "defence_adjust_common_value", "attack_adjust_common_value"],
  legado: ["id_skill", "skill_level"],
};

/** `faction` (阵营组合代码): o "lado" do monstro, bit k = lado k. Os nomes são os do `ENEMY_FACTION_CONFIG.enemy_factions`
    (`ExpTypes.h`, comentário dos 32 lados) e batem com o `faction.h` do GS (humano 0x2, bestial 0x4, alado 0x8, maré 0x10,
    espírito 0x100). Quem é inimigo de cada lado fica nesse `ENEMY_FACTION_CONFIG` (nos Editores). */
const NOMES_DOS_LADOS = ["(vazio)", "Humano comum", "Bestial comum", "Alado comum", "Povo da Maré comum", "Chefe mundial", "Amigável do cenário",
  "Hostil do cenário", "Espírito comum", "Nome vermelho", "(vazio)", "Ataque — guerra de território", "Defesa — guerra de território",
  "Ataque — guerra de clã", "Defesa — guerra de clã", "Duelo", "(vazio)", "Imortal (仙)", "Demônio (魔)", "(vazio)", "(vazio)",
  "NPC comum (função, guarda)", "Guarda da guerra de território", "NPC especial", "(vazio)", "Mascote", "Invocação", "(vazio)",
  "Monstro do lado Imortal", "Monstro do lado Demônio", "Monstro comum", "(vazio)"];
export const LADOS: [number, string][] = NOMES_DOS_LADOS.map((n, k) => [(1 << k) >>> 0, `${k} · ${n}`] as [number, string]).filter(([, n]) => !n.endsWith("(vazio)"));

/** `monster_faction` (怪物的详细派系划分): grupos livres do designer — não têm nome em nenhum arquivo; o mesmo bit em dois
    monstros quer dizer "mesmo grupo" para a ajuda. */
export const GRUPOS: [number, string][] = Array.from({ length: 32 }, (_, k) => [(1 << k) >>> 0, `Grupo ${k + 1}`] as [number, string]);

/** Os bits ligados de uma máscara, como "1, 3, 7" (grupos começam em 1). */
export const bitsLigados = (v: number, base = 1) => Array.from({ length: 32 }, (_, k) => k).filter((k) => (v >>> 0) & ((1 << k) >>> 0)).map((k) => k + base);

/** Campos 0/1 (B373): aparecem como "não / sim". */
export const BOOLEANOS = new Set(["highest_frequency", "no_auto_fight", "fixed_direction", "no_accept_player_buff", "domain_related", "drop_protected",
  "drop_for_no_profit_time"]);

/** `role_in_war`: o papel na batalha (`instance/battleground_manager.cpp:437-446`: 1 destrói a construção-chave do lado, 0 conta
    como tropa comum ao morrer). Outros valores o GS não trata ali. */
export const PAPEIS_NA_GUERRA = ["0 · tropa comum", "1 · construção-chave"];

/** O que cada campo afeta no jogo, com a evidência (B373). */
export const EXPLICACOES: Record<string, string> = {
  common_strategy: "Roteiro de IA do aipolicy.data (gatilhos por vida, tempo, alvo…). 0 = nenhum; id que não existe vira 0 (npcgenerator.cpp:218-226).",
  highest_frequency: "Sim: a IA roda em todo ciclo, mesmo parado. Não: parado, roda 1 vez a cada 30 ciclos (npcgenerator.cpp:2458-2465, config.h:41). Use em chefe e em monstro com roteiro de tempo.",
  no_auto_fight: "Sim: a IA não começa luta sozinha (aipolicy.cpp:125) — nem o agressivo ataca; só reage por roteiro.",
  fixed_direction: "Sim: fica virado para a direção do spawn e não gira (npc.cpp:109).",
  stand_mode: "Só visual, do cliente: como o modelo fica de pé (quatro patas, duas, voando). O GS não usa.",
  no_accept_player_buff: "Sim: recusa habilidade de ajuda vinda de jogador (cura, bênção) (npc.cpp:1021).",
  role_in_war: "Na batalha (campo de guerra): ao morrer, 1 derruba a construção-chave do lado dele, 0 conta como tropa (battleground_manager.cpp:437-446).",
  id_building: "Instalação da base do clã ligada a este monstro: quando ele morre, a instalação cai (instance_manager.cpp:1539-1543).",
  domain_related: "Sim: o clã dono do território onde ele nasce passa a ser o dono dele (npcgenerator.cpp:2513-2517).",
  hp_adjust_common_value: "Id de uma variável global do mapa: o valor dela (−90 a 5000) soma em % na vida máxima ao nascer; fora disso vale 0 (npcgenerator.cpp:1914-1922).",
  defence_adjust_common_value: "Como o de vida, para a defesa (mesma função AdjustPropByCommonValue, npcgenerator.cpp:1912).",
  attack_adjust_common_value: "Como o de vida, para o ataque (mesma função AdjustPropByCommonValue, npcgenerator.cpp:1912).",
  id_skill: "Campo antigo: o GS 1.5.5 não usa (npcgenerator.cpp:225, comentado). Use as listas de habilidades.",
  skill_level: "Campo antigo, junto com o de cima: não usado.",
  id_pet_egg_captured: "O ovo de mascote que o jogador ganha ao capturar este monstro (PET_EGG_ESSENCE).",
  invisible_lvl: "Nível de invisibilidade: o GS soma o nível do monstro quando não é 0.",
  uninvisible_lvl: "Nível de detecção de invisíveis: o GS soma o nível do monstro; negativo vira 0.",
};
