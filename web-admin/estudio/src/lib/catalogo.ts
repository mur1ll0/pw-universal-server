/* O que o Estúdio mostra (B366, aprovado pelo Murillo em 2026-10-23). Campo que não existe no layout da versão do
   realm (v156 no 1.5.5, v7 no 1.2.6) simplesmente não aparece: as boxes filtram pelos `campos` da ficha. O resto dos
   campos continua nos Editores. */

export interface Subcategoria {
  chave: string;
  nome: string;
  tabela: string;
  /** tabelas de categoria (maior/menor) quando existem */
  maior?: string;
  menor?: string;
  /** só nesta versão de cliente */
  so?: string;
  pronta: boolean;
}

export interface Categoria {
  chave: string;
  nome: string;
  icone: string;
  pronta: boolean;
  subcategorias: Subcategoria[];
}

/* Subcategorias de Equipáveis: os compartimentos de equipamento do cliente (`EC_IvtrTypes.h:56-95`) e as partes do
   `equip_mask` (`ExpTypes.h:312-324, 736-741`). Nesta fase só Armas tem tela própria; as outras abrem a busca e
   levam aos Editores. */
export const CATEGORIAS: Categoria[] = [
  {
    chave: "equipaveis", nome: "Equipáveis", icone: "espada", pronta: true, subcategorias: [
      { chave: "armas", nome: "Armas", tabela: "WEAPON_ESSENCE", maior: "WEAPON_MAJOR_TYPE", menor: "WEAPON_SUB_TYPE", pronta: true },
      { chave: "armaduras", nome: "Armaduras", tabela: "ARMOR_ESSENCE", maior: "ARMOR_MAJOR_TYPE", menor: "ARMOR_SUB_TYPE", pronta: true },
      { chave: "ornamentos", nome: "Ornamentos", tabela: "DECORATION_ESSENCE", maior: "DECORATION_MAJOR_TYPE", menor: "DECORATION_SUB_TYPE", pronta: true },
      { chave: "moda", nome: "Moda", tabela: "FASHION_ESSENCE", maior: "FASHION_MAJOR_TYPE", menor: "FASHION_SUB_TYPE", pronta: true },
      { chave: "voo", nome: "Voo", tabela: "FLYSWORD_ESSENCE", pronta: true },
      { chave: "asas", nome: "Asas (Arqueiro)", tabela: "WINGMANWING_ESSENCE", pronta: true },
      { chave: "municao", nome: "Munição", tabela: "PROJECTILE_ESSENCE", maior: "PROJECTILE_TYPE", pronta: true },
      { chave: "livros", nome: "Livros", tabela: "BIBLE_ESSENCE", pronta: true },
      { chave: "cartas", nome: "Cartas de General", tabela: "POKER_ESSENCE", menor: "POKER_SUB_TYPE", so: "1.5.5", pronta: true },
      { chave: "daimon", nome: "Equipamento de Daimon", tabela: "GOBLIN_EQUIP_ESSENCE", maior: "GOBLIN_EQUIP_TYPE", so: "1.5.5", pronta: true },
    ],
  },
  { chave: "npcs", nome: "NPCs", icone: "npc", pronta: false, subcategorias: [] },
  { chave: "monstros", nome: "Monstros", icone: "monstro", pronta: true, subcategorias: [
    { chave: "monstros", nome: "Monstros", tabela: "MONSTER_ESSENCE", maior: "MONSTER_TYPE", pronta: true },
  ] },
  { chave: "mascotes", nome: "Mascotes", icone: "pata", pronta: false, subcategorias: [] },
  { chave: "itens", nome: "Itens", icone: "caixa", pronta: false, subcategorias: [] },
];

export const subcategoriaPorTabela = (tabela: string) =>
  CATEGORIAS.flatMap((c) => c.subcategorias).find((s) => s.tabela === tabela);

/* ---- B376: colunas do seletor de item (filtro e ordenação sobre a tabela inteira, `GET tabelas/{T}?colunas=`) ----
   Nomes do layout (`specs/elements_layouts/v156.json` e `v7.json`, listas achatadas a partir de 1): WEAPON (4),
   ARMOR (7), DECORATION (10) e FASHION (84) têm `id_major_type`, `id_sub_type`, `level` e `require_level`;
   FLYSWORD (23) `level` e `require_player_level_min`; WINGMANWING (24) `require_player_level_min`; STONE (36)
   `id_sub_type` e `level`; GOBLIN_EQUIP (122, só v156) `req_goblin_level`; POKER (185, só v156) `id_sub_type`,
   `require_level` e `rank`. Pede-se a lista toda: o pw-recursos devolve em `colunas` só as que a tabela tem. */
export const COLUNAS_DO_SELETOR: { campo: string; titulo: string }[] = [
  { campo: "require_level", titulo: "Nív. mín." },
  { campo: "require_player_level_min", titulo: "Nív. mín." },
  { campo: "req_goblin_level", titulo: "Nív. Daimon" },
  { campo: "level", titulo: "Nível" },
  { campo: "rank", titulo: "Grau" },
  { campo: "id_major_type", titulo: "Categoria" },
  { campo: "id_sub_type", titulo: "Subcategoria" },
];

/* ---- B377: o que pode ser item (drop, item ligado, ovo) ----
   As tabelas para as quais o GS gera item: os `case` do `generate_item` (`cgame/gs/template/generate_item_temp.h:5-145`,
   1.5.5). `*_TYPE`, `*_SERVICE`, `MONSTER`, `NPC`, `MINE`, `SUITE`, `PET_ESSENCE` e as demais caem no `default` e
   devolvem -1 (:147-153): como drop, o monstro não solta nada. */
export const TABELAS_DE_ITEM = new Set([
  "WEAPON_ESSENCE", "ARMOR_ESSENCE", "DECORATION_ESSENCE", "PROJECTILE_ESSENCE", "QUIVER_ESSENCE", "STONE_ESSENCE",
  "TASKDICE_ESSENCE", "FLYSWORD_ESSENCE", "FASHION_ESSENCE", "GOBLIN_ESSENCE", "GOBLIN_EQUIP_ESSENCE", "GOBLIN_EXPPILL_ESSENCE",
  "SELL_CERTIFICATE_ESSENCE", "TARGET_ITEM_ESSENCE", "LOOK_INFO_ESSENCE", "INC_SKILL_ABILITY_ESSENCE", "WEDDING_BOOKCARD_ESSENCE",
  "WEDDING_INVITECARD_ESSENCE", "SHARPENER_ESSENCE", "POKER_ESSENCE", "ASTROLABE_ESSENCE", "WINGMANWING_ESSENCE",
  "MEDICINE_ESSENCE", "MATERIAL_ESSENCE", "DYE_TICKET_ESSENCE", "ELEMENT_ESSENCE", "SKILLTOME_ESSENCE", "TOWNSCROLL_ESSENCE",
  "DAMAGERUNE_ESSENCE", "ARMORRUNE_ESSENCE", "TASKMATTER_ESSENCE", "TOSSMATTER_ESSENCE", "TASKNORMALMATTER_ESSENCE",
  "FACEPILL_ESSENCE", "FACETICKET_ESSENCE", "GM_GENERATOR_ESSENCE", "PET_EGG_ESSENCE", "PET_FOOD_ESSENCE", "FIREWORKS_ESSENCE",
  "WAR_TANKCALLIN_ESSENCE", "SKILLMATTER_ESSENCE", "REFINE_TICKET_ESSENCE", "BIBLE_ESSENCE", "SPEAKER_ESSENCE", "AUTOMP_ESSENCE",
  "AUTOHP_ESSENCE", "DOUBLE_EXP_ESSENCE", "TRANSMITSCROLL_ESSENCE", "FACTION_MATERIAL_ESSENCE", "CONGREGATE_ESSENCE",
  "FORCE_TOKEN_ESSENCE", "DYNSKILLEQUIP_ESSENCE", "MONEY_CONVERTIBLE_ESSENCE", "POKER_DICE_ESSENCE", "MONSTER_SPIRIT_ESSENCE",
  "SHOP_TOKEN_ESSENCE", "UNIVERSAL_TOKEN_ESSENCE", "ASTROLABE_RANDOM_ADDON_ESSENCE", "ASTROLABE_INC_INNER_POINT_VALUE_ESSENCE",
  "ASTROLABE_INC_EXP_ESSENCE", "ITEM_PACKAGE_BY_PROFESSION_ESSENCE", "FIREWORKS2_ESSENCE", "FIX_POSITION_TRANSMIT_ESSENCE",
  "UNIONSCROLL_ESSENCE", "REVIVESCROLL_ESSENCE",
]);

/** As tabelas cujo seletor ganha colunas: as das categorias do Estúdio e as pedras. */
export const tabelaComColunas = (tabela: string) => tabela === "STONE_ESSENCE" || !!subcategoriaPorTabela(tabela);

/* ---- boxes da edição de Equipável (B366 Armas; B370 todas as subcategorias) ----
   Uma lista só para todas as tabelas: o campo que a tabela (ou a versão) não tem não aparece. Entrada que começa com
   "/" é expressão regular sobre os nomes do layout (na ordem do arquivo). Fontes: `ExpTypes.h` — `WEAPON_ESSENCE`
   :177-305, `ARMOR_ESSENCE` :330-460, `DECORATION_ESSENCE` :470-580, `FASHION_ESSENCE` :745-800, e as demais
   pelo nome do layout. */

export interface Box {
  titulo: string;
  campos: string[];
}

export const BOXES_DO_EQUIPAVEL: Box[] = [
  { titulo: "Requisitos para equipar", campos: ["require_level", "require_player_level_min", "require_strength", "require_agility", "require_energy",
    "require_tili", "require_reputation", "require_projectile", "require_weapon_level_min", "require_weapon_level_max", "req_goblin_level", "gender",
    "/^require_control_point_[0-9]+$/"] },
  { titulo: "Combate", campos: ["level", "type", "damage_low", "damage_high", "damage_high_min", "damage_high_max", "magic_damage_low", "magic_damage_high",
    "magic_damage_high_min", "magic_damage_high_max", "attack_range", "short_range_mode", "damage_enhance", "damage_scale_enhance",
    "defence_low", "defence_high", "/^magic_defences_[0-9]+_(low|high)$/", "armor_enhance_low", "armor_enhance_high", "hp_enhance_low", "hp_enhance_high",
    "mp_enhance_low", "mp_enhance_high", "strength", "agility", "energy", "tili", "/^magic_[0-9]+$/",
    "rank", "max_level", "hp", "damage", "magic_damage", "defence", "/^magic_defence_[0-9]+$/", "vigour",
    "inc_hp", "inc_damage", "inc_magic_damage", "inc_defence", "/^inc_magic_defence_[0-9]+$/", "inc_vigour", "swallow_exp"] },
  { titulo: "Voo", campos: ["fly_mode", "speed_increase", "speed_increase_min", "speed_increase_max", "speed_rush_increase_min", "speed_rush_increase_max",
    "time_max_min", "time_max_max", "time_increase_per_element", "mp_launch", "mp_per_second", "max_refine_lvl",
    "/^refine_[0-9]+_(cost|speed_increase|speed_rush_increase)$/"] },
  { titulo: "Comércio", campos: ["price", "shop_price", "repairfee", "pile_num_max", "require_dye_count"] },
  { titulo: "Durabilidade e fabricação", campos: ["durability_min", "durability_max", "durability_drop_min", "durability_drop_max", "decompose_price", "decompose_time"] },
];

/** Expande as entradas de uma box nos campos que a ficha tem (regex na ordem do layout; nome solto só se existir). */
export function camposDaBox(box: Box, nomes: string[]): string[] {
  const r: string[] = [];
  for (const e of box.campos) {
    if (e.startsWith("/")) {
      const re = new RegExp(e.slice(1, e.lastIndexOf("/")));
      for (const n of nomes) if (re.test(n) && !r.includes(n)) r.push(n);
    } else if (nomes.includes(e) && !r.includes(e)) r.push(e);
  }
  return r;
}

/** Os 5 elementos das defesas mágicas, na ordem do arquivo: 金木水火土 (`ExpTypes.h:368`). */
export const ELEMENTOS = ["Metal", "Madeira", "Água", "Fogo", "Terra"];

/** Rótulos dos campos próprios das outras subcategorias que o `rotulos.json` não tem — do comentário de cada campo no
    `ExpTypes.h` do cliente 1.5.5 (linha ao lado). */
const ROTULOS_EXTRA: Record<string, string> = {
  require_player_level_min: "Nível mínimo do personagem", // :1238
  require_weapon_level_min: "Nível mínimo da arma", // :1193
  require_weapon_level_max: "Nível máximo da arma",
  type: "Tipo de munição", // :1951 (PROJECTILE_ESSENCE)
  damage_enhance: "Ataque físico extra da arma", // :1965
  damage_scale_enhance: "Ataque físico extra da arma (proporção)", // :1966
  fly_mode: "Modo de voo", // :1594
  speed_increase: "Aumento da velocidade de voo", // :1605
  speed_increase_min: "Aumento da velocidade de voo (mín.)", // :1584
  speed_increase_max: "Aumento da velocidade de voo (máx.)",
  speed_rush_increase_min: "Aumento da velocidade acelerada (mín.)", // :1586
  speed_rush_increase_max: "Aumento da velocidade acelerada (máx.)",
  time_max_min: "Tempo máximo de voo (mín.)", // :1589
  time_max_max: "Tempo máximo de voo (máx.)",
  time_increase_per_element: "Tempo de voo por pedra elemental", // :1592
  max_refine_lvl: "Nível máximo de refino",
  mp_launch: "MP para levantar voo", // :1636
  mp_per_second: "MP por segundo de voo", // :1637
  rank: "Grau", // :685
  max_level: "Nível máximo", // :686
  hp: "HP inicial", // :688
  damage: "Ataque físico inicial",
  magic_damage: "Ataque mágico inicial",
  vigour: "Vigor inicial", // :693
  inc_hp: "HP por nível", // :695
  inc_damage: "Ataque físico por nível",
  inc_magic_damage: "Ataque mágico por nível",
  inc_defence: "Defesa por nível",
  inc_vigour: "Vigor por nível",
  swallow_exp: "Experiência ao ser absorvida", // :627
  req_goblin_level: "Nível mínimo do Daimon", // :3153
  strength: "Força",
  agility: "Agilidade",
  energy: "Energia",
  tili: "Vitalidade",
  gender: "Gênero", // :776
  require_dye_count: "Tinturas para tingir", // :769
};

/** Modo de voo (`fly_mode`, `ExpTypes.h:1594`: 0 espada voadora, 1 asas, 2 rodas de fogo). */
export const MODOS_DE_VOO = ["0 · espada voadora", "1 · asas", "2 · rodas de fogo"];

/** Rótulo de campo que o `rotulos.json` não tem, montado pela regra do layout. */
export function rotuloGerado(n: string): string | null {
  if (ROTULOS_EXTRA[n]) return ROTULOS_EXTRA[n];
  let m = n.match(/^magic_defences_(\d+)_(low|high)$/);
  if (m) return `Defesa de ${ELEMENTOS[Number(m[1]) - 1] ?? m[1]} (${m[2] === "low" ? "mín." : "máx."})`;
  m = n.match(/^(inc_)?magic_defence_(\d+)$/);
  if (m) return `${m[1] ? "Aumento da defesa" : "Defesa"} de ${ELEMENTOS[Number(m[2]) - 1] ?? m[2]}`;
  m = n.match(/^magic_(\d+)$/);
  if (m) return `Magia de ${ELEMENTOS[Number(m[1]) - 1] ?? m[1]}`;
  m = n.match(/^refine_(\d+)_(cost|speed_increase|speed_rush_increase)$/);
  if (m) return `Refino ${m[1]}: ${{ cost: "custo", speed_increase: "velocidade", speed_rush_increase: "velocidade acelerada" }[m[2]]}`;
  m = n.match(/^require_control_point_(\d+)$/);
  if (m) return `Pontos de controle exigidos ${m[1]}`;
  return null;
}

/** Gênero (`FASHION_ESSENCE.gender`, `ExpTypes.h:776`). */
export const GENEROS = ["0 · masculino", "1 · feminino"];

/** Partes do corpo que o modelo cobre (`equip_location` de armadura e moda). Não é máscara: o cliente usa o valor como
    índice do `enumSkinShow*` (`EC_Player.h:94-104`; `EC_Player.cpp:4633-4640` recusa fora de 1–8 na armadura; a moda
    também usa 9 capacete e 10 mão, `EC_IvtrFashion.cpp:191-216`). Conferido nos dados: armadura 1, perneira 3, sapato 4,
    vestido 8. */
export const PARTES_DO_MODELO = ["0 · nenhuma", "1 · corpo", "2 · pulso", "3 · parte de baixo", "4 · pé", "5 · corpo e parte de baixo",
  "6 · corpo e pulso", "7 · parte de baixo e pé", "8 · corpo, parte de baixo e pulso", "9 · capacete", "10 · mão"];

/** Listas de efeitos fixos (só ids, sem chance): `BIBLE_ESSENCE.id_addons_N`, `PROJECTILE_ESSENCE.id_addonN`, `POKER_ESSENCE.addon_N`. */
export const EFEITOS_FIXOS = ["/^id_addons_[0-9]+$/", "/^id_addon[0-9]+$/", "/^addon_[0-9]+$/"];

/** Grupos de chances simples (um `float` por posição) que o servidor normaliza ao carregar (`elementdataman.cpp:626-628`). */
export const CHANCES_SIMPLES = [
  { titulo: "Furos no drop", prefixo: "drop_probability_socket", rotulo: (k: number) => `${k} furo${k === 1 ? "" : "s"}` },
  { titulo: "Furos ao fabricar", prefixo: "make_probability_socket", rotulo: (k: number) => `${k} furo${k === 1 ? "" : "s"}` },
  { titulo: "Quantos efeitos add-on nascem", prefixo: "probability_addon_num", rotulo: (k: number) => `${k} efeito${k === 1 ? "" : "s"}` },
];

/** Listas (id, chance) da arma; os ids são do espaço `ADDON` (`EQUIPMENT_ADDON`). */
export const LISTAS_DA_ARMA = [
  { titulo: "Efeitos add-on", prefixo: "addons", id: "id_addon", chance: "probability_addon",
    explica: "Sorteados quando a arma nasce (drop ou fabricação): primeiro quantos, depois quais (generate_item_temp.h:234-246)." },
  { titulo: "Efeitos aleatórios", prefixo: "rands", id: "id_rand", chance: "probability_rand",
    explica: "Usados ao reforjar as propriedades (generate_addon_from_rands, itemdataman.cpp:1059-1072)." },
  { titulo: "Efeitos únicos", prefixo: "uniques", id: "id_unique", chance: "probability_unique", recolhida: true, chanceGeral: "probability_unique",
    explica: "Com a chance geral de efeito único, um deles entra no lugar (generate_template_addon)." },
];

/** Distância do golpe (`short_range_mode`, `ExpTypes.h`: 0 longe, 1 perto, 2 assassino). */
export const DISTANCIAS = ["0 · longe", "1 · perto", "2 · assassino (perto)"];

/* Classes: bit `1 << classe` (`item_astrolabe.cpp:221`); o 1.5.5 tem 12 (`localmacro.h:16`, `EC_RoleTypes.h:53-64`),
   o 1.2.6 tem 8 (0–7). Nomes como o cliente escreve (`painel/textos.py:109`). */
const NOMES_DAS_CLASSES = ["Guerreiro", "Mago", "Espiritualista", "Feiticeira", "Bárbaro", "Mercenário", "Arqueiro",
  "Sacerdote", "Arcano", "Místico", "Retalhador", "Tormentador"];

export const classesDaVersao = (versao: string) => NOMES_DAS_CLASSES.slice(0, versao.startsWith("1.2") ? 8 : 12);

/* Regras do item (`proc_type`): `ITEM_PROC_TYPE_*` do servidor 1.5.5 (`cgame/gs/item.h:309-326`). */
export const REGRAS_DO_ITEM: [number, string][] = [
  [0x0001, "Não cai ao morrer"],
  [0x0002, "Não pode ser jogado no chão"],
  [0x0004, "Não pode ser vendido ao NPC"],
  [0x0008, "Item de loja (dinheiro real)"],
  [0x0010, "Não pode ser trocado entre jogadores"],
  [0x0020, "Item de missão"],
  [0x0040, "Vincula ao equipar"],
  [0x0080, "Pode ser desvinculado"],
  [0x0100, "Some ao sair do mapa"],
  [0x0200, "Uso automático"],
  [0x0400, "Cai ao morrer"],
  [0x0800, "Cai ao desconectar"],
  [0x1000, "Não pode ser consertado"],
  [0x2000, "Quebra se morto em PK"],
  [0x4000, "Não entra no armazém da conta"],
  [0x8000, "Já vinculado"],
  [0x10000, "Pode ser negociado na web"],
  [0x20000, "Comprado na loja (exceto brinde)"],
];

/** Campos de referência com cartão (item, quantidade ao lado). */
export const ITENS_LIGADOS = [
  { campo: "element_id", quantidade: "element_num", titulo: "Item gerado ao desmontar", espaco: "ESSENCE" },
  { campo: "id_drop_after_damaged", quantidade: "num_drop_after_damaged", titulo: "Item que cai ao quebrar", espaco: "ESSENCE" },
];
