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
      { chave: "armaduras", nome: "Armaduras", tabela: "ARMOR_ESSENCE", maior: "ARMOR_MAJOR_TYPE", menor: "ARMOR_SUB_TYPE", pronta: false },
      { chave: "ornamentos", nome: "Ornamentos", tabela: "DECORATION_ESSENCE", maior: "DECORATION_MAJOR_TYPE", menor: "DECORATION_SUB_TYPE", pronta: false },
      { chave: "moda", nome: "Moda", tabela: "FASHION_ESSENCE", maior: "FASHION_MAJOR_TYPE", menor: "FASHION_SUB_TYPE", pronta: false },
      { chave: "voo", nome: "Voo", tabela: "FLYSWORD_ESSENCE", pronta: false },
      { chave: "asas", nome: "Asas (Arqueiro)", tabela: "WINGMANWING_ESSENCE", pronta: false },
      { chave: "municao", nome: "Munição", tabela: "PROJECTILE_ESSENCE", maior: "PROJECTILE_TYPE", pronta: false },
      { chave: "livros", nome: "Livros", tabela: "BIBLE_ESSENCE", pronta: false },
      { chave: "cartas", nome: "Cartas de General", tabela: "POKER_ESSENCE", menor: "POKER_SUB_TYPE", so: "1.5.5", pronta: false },
      { chave: "daimon", nome: "Equipamento de Daimon", tabela: "GOBLIN_EQUIP_ESSENCE", maior: "GOBLIN_EQUIP_TYPE", so: "1.5.5", pronta: false },
    ],
  },
  { chave: "npcs", nome: "NPCs", icone: "npc", pronta: false, subcategorias: [] },
  { chave: "monstros", nome: "Monstros", icone: "monstro", pronta: false, subcategorias: [] },
  { chave: "mascotes", nome: "Mascotes", icone: "pata", pronta: false, subcategorias: [] },
  { chave: "itens", nome: "Itens", icone: "caixa", pronta: false, subcategorias: [] },
];

export const subcategoriaPorTabela = (tabela: string) =>
  CATEGORIAS.flatMap((c) => c.subcategorias).find((s) => s.tabela === tabela);

/* ---- boxes da tela de Armas (`WEAPON_ESSENCE`, `ExpTypes.h:177-305`) ---- */

export interface Box {
  titulo: string;
  campos: string[];
}

export const BOXES_DA_ARMA: Box[] = [
  { titulo: "Requisitos para equipar", campos: ["require_level", "require_strength", "require_agility", "require_energy", "require_tili", "require_reputation", "require_projectile"] },
  { titulo: "Combate", campos: ["level", "damage_low", "damage_high_min", "damage_high_max", "magic_damage_low", "magic_damage_high_min", "magic_damage_high_max", "attack_range", "short_range_mode"] },
  { titulo: "Comércio", campos: ["price", "shop_price", "repairfee", "pile_num_max"] },
  { titulo: "Durabilidade e fabricação", campos: ["durability_min", "durability_max", "durability_drop_min", "durability_drop_max", "decompose_price", "decompose_time"] },
];

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
