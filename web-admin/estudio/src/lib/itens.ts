/* Itens no Estúdio: as tabelas de item que não são Equipáveis (`TABELAS_DE_ITEM` menos as de `BOXES_DO_EQUIPAVEL`),
   agrupadas como o jogador entende, mais as Minas (o recurso do mapa que dá os materiais). Cada tabela foi conferida no
   layout das duas versões (`specs/elements_layouts/v156.json` e `v7.json`) e no servidor 1.5.5: o `generate_item`
   (`cgame/gs/template/generate_item_temp.h:5-145`), a classe que o GS dá ao item (`template/setclassid.cpp:23-289`) e o
   tratador de uso (`cgame/gs/item/*.cpp`). Classe -1 = o GS cria o item, mas usar não faz nada (material, missão…). */

import type { Subcategoria } from "./catalogo";

export interface SubDeItem extends Subcategoria {
  grupo: string;
  /** o que o servidor faz com o item, com a evidência (aparece na busca e no topo da edição) */
  oQueE: string;
}

const s = (grupo: string, chave: string, nome: string, tabela: string, oQueE: string, extra: Partial<SubDeItem> = {}): SubDeItem =>
  ({ grupo, chave, nome, tabela, oQueE, pronta: true, ...extra });

export const SUBCATEGORIAS_DE_ITENS: SubDeItem[] = [
  // Materiais
  s("Materiais", "materiais", "Materiais", "MATERIAL_ESSENCE",
    "Matéria-prima de receita e de desmontar; usar não faz nada (classe -1, setclassid.cpp:284-286). Sai de mina, drop e loja; entra nas receitas (RECIPE_ESSENCE).",
    { maior: "MATERIAL_MAJOR_TYPE", menor: "MATERIAL_SUB_TYPE" }),
  s("Materiais", "materiais-cla", "Materiais de clã", "FACTION_MATERIAL_ESSENCE",
    "Material de construção do clã: vale material_count[8] materiais equivalentes (ExpTypes.h:1542); usar não faz nada (classe -1, setclassid.cpp:231-232).", { so: "1.5.5" }),
  s("Materiais", "elementos", "Pedras elementais", "ELEMENT_ESSENCE",
    "元石 (ExpTypes.h:1768): o que o voo gasta para durar mais (time_increase_per_element do voo); usar não faz nada (classe -1, setclassid.cpp:106-107)."),
  // Poções e remédios
  s("Poções e remédios", "remedios", "Remédios", "MEDICINE_ESSENCE",
    "A categoria decide o efeito: 1794 vida no tempo, 1802 mana no tempo, 1810 vida e mana na hora, 1815 antídoto, 2038 antídoto imediato; outra categoria não gera item (generate_item_temp.h:921-933; item_potion.cpp).",
    { maior: "MEDICINE_MAJOR_TYPE", menor: "MEDICINE_SUB_TYPE" }),
  s("Poções e remédios", "amuleto-hp", "Amuletos de vida", "AUTOHP_ESSENCE",
    "Equipado, cura sozinho quando a vida cai abaixo da proporção, gastando o total de pontos do amuleto (item_amulet.cpp; player.cpp:9105-9130)."),
  s("Poções e remédios", "amuleto-mp", "Amuletos de mana", "AUTOMP_ESSENCE",
    "Equipado, repõe a mana sozinho quando ela cai abaixo da proporção, gastando os pontos do amuleto (item_amulet.cpp; player.cpp:9105-9130)."),
  s("Poções e remédios", "exp-dobrada", "Experiência dobrada", "DOUBLE_EXP_ESSENCE",
    "Dá double_exp_time segundos de experiência dobrada (classe CLS_ITEM_DBL_EXP, setclassid.cpp:175-176; ExpTypes.h:1088)."),
  // Pergaminhos
  s("Pergaminhos", "cidade", "Pergaminhos da cidade", "TOWNSCROLL_ESSENCE",
    "Volta à cidade de renascer (ReturnToTown, item_townscroll.cpp) depois de use_time segundos de uso (×20 ticks, generate_item_temp.h:1281-1282)."),
  s("Pergaminhos", "teleporte", "Pergaminhos de teleporte", "TRANSMITSCROLL_ESSENCE",
    "Leva a um ponto de teleporte já descoberto (ReturnWaypoint, item_townscroll.cpp; classe CLS_ITEM_TOWNSCROLL2)."),
  s("Pergaminhos", "reviver", "Pergaminhos de reviver", "REVIVESCROLL_ESSENCE",
    "Classe -1 no GS 1.5.5 (setclassid.cpp:184-188): usar pela bolsa não faz nada; use_time em segundos, cool_time em ms (ExpTypes.h:1739)."),
  s("Pergaminhos", "uniao", "Pergaminhos UNIONSCROLL", "UNIONSCROLL_ESSENCE",
    "Classe -1 no GS 1.5.5 (setclassid.cpp:184-188): usar não faz nada; use_time em segundos (ExpTypes.h:1711)."),
  s("Pergaminhos", "marca", "Marca de teleporte", "FIX_POSITION_TRANSMIT_ESSENCE",
    "Grava um ponto e teleporta para ele (classe CLS_ITEM_FIX_POSITION_TRANSMIT, setclassid.cpp:280-281; ExpTypes.h:4134).", { so: "1.5.5" }),
  s("Pergaminhos", "reunir", "Ordem de reunir", "CONGREGATE_ESSENCE",
    "集结令 (ExpTypes.h:2502): chama o grupo/clã para as áreas listadas (classe CLS_ITEM_CONGREGATE).", { so: "1.5.5" }),
  // Livros de habilidade
  s("Livros de habilidade", "livros", "Livros de habilidade", "SKILLTOME_ESSENCE",
    "O livro não guarda a habilidade: é a habilidade que o exige para aprender (item_exigido do catálogo, consumido no aprendizado, skill.cpp:79-84); usar não faz nada (classe -1, setclassid.cpp:184-188).",
    { menor: "SKILLTOME_SUB_TYPE" }),
  s("Livros de habilidade", "item-habilidade", "Itens que lançam habilidade", "SKILLMATTER_ESSENCE",
    "技能书本体 (ExpTypes.h:2373): usar lança a habilidade id_skill no nível level_skill (classe CLS_ITEM_SKILL_TRIGGER)."),
  s("Livros de habilidade", "proficiencia", "Aumentar proficiência", "INC_SKILL_ABILITY_ESSENCE",
    "Aumenta a proficiência (inc_ratio) da habilidade id_skill a partir do nível level_required (ExpTypes.h:2397; classe CLS_ITEM_INCSKILLABILITY).", { so: "1.5.5" }),
  s("Livros de habilidade", "habilidade-equipada", "Habilidade por equipamento", "DYNSKILLEQUIP_ESSENCE",
    "Ter o item dá as habilidades id_skill_N (ExpTypes.h:983; classe CLS_ITEM_DYNSKILL).", { so: "1.5.5" }),
  // Pedras e refino
  s("Pedras e refino", "pedras", "Pedras", "STONE_ESSENCE",
    "Incrusta-se no furo da peça: o efeito depende de onde vai (arma, armadura, acessório), pago no NPC (install_price, serviceprovider.cpp:1374-1385); remover custa uninstall_price (item_list.cpp:205).",
    { menor: "STONE_SUB_TYPE" }),
  s("Pedras e refino", "talismas", "Talismãs de refino", "REFINE_TICKET_ESSENCE",
    "Usado junto no refino, muda as chances (RefineItemAddon, player.cpp:11677-11810; equip_item.cpp:192-206)."),
  s("Pedras e refino", "runa-ataque", "Runas de ataque", "DAMAGERUNE_ESSENCE",
    "攻击优化符 (ExpTypes.h:1179): aumenta o dano da arma na faixa de nível (classe CLS_ITEM_DAMAGE_RUNE).", { menor: "DAMAGERUNE_SUB_TYPE" }),
  s("Pedras e refino", "runa-defesa", "Runas de defesa", "ARMORRUNE_ESSENCE",
    "防御优化符 (ExpTypes.h:1222): reduz o dano por um tempo (classe de resistência ou de defesa pelo damage_type, setclassid.cpp:48-66).",
    { menor: "ARMORRUNE_SUB_TYPE" }),
  s("Pedras e refino", "amolar", "Pedras de amolar", "SHARPENER_ESSENCE",
    "磨刀石 (ExpTypes.h:2471): põe efeitos temporários (addon_N por addon_time) nas peças do equip_mask (classe CLS_ITEM_SHARPENER).", { so: "1.5.5" }),
  // Aparência
  s("Aparência", "corantes", "Corantes", "DYE_TICKET_ESSENCE",
    "Tinge moda (e mascote) com uma cor sorteada nas faixas de matiz, saturação e brilho (ExpTypes.h:1112); usar sozinho não faz nada (classe -1)."),
  s("Aparência", "cirurgia", "Cirurgia plástica", "FACETICKET_ESSENCE",
    "整容卷 (ExpTypes.h:2088): libera mudar o rosto.", { maior: "FACETICKET_MAJOR_TYPE", menor: "FACETICKET_SUB_TYPE" }),
  s("Aparência", "transformacao", "Pílulas de transformação", "FACEPILL_ESSENCE",
    "变形丸 (ExpTypes.h:2129): muda a aparência por duration (classe CLS_ITEM_FACEPILL).", { maior: "FACEPILL_MAJOR_TYPE", menor: "FACEPILL_SUB_TYPE" }),
  // Missão
  s("Itens de missão", "missao", "Itens de missão", "TASKMATTER_ESSENCE",
    "Item que a missão dá e pede; não cai no chão (o cliente não monta modelo, EC_Matter.cpp:469-475); usar não faz nada (classe -1)."),
  s("Itens de missão", "missao-comum", "Itens comuns de missão", "TASKNORMALMATTER_ESSENCE",
    "用于任务的普通物品 (ExpTypes.h:1840): material comum usado por missão; vai também para o armazém de materiais (player.cpp:7582)."),
  s("Itens de missão", "sorteio-missao", "Sorteio de missão", "TASKDICE_ESSENCE",
    "Usar dá uma das missões da lista pela probabilidade (ExpTypes.h:1866; classe CLS_ITEM_TASKDICE)."),
  // Mascote e Daimon
  s("Mascote e Daimon", "ovos", "Ovos de mascote", "PET_EGG_ESSENCE",
    "宠物蛋 (ExpTypes.h:2203): choca o mascote id_pet no NPC (classe CLS_ITEM_PET_EGG)."),
  s("Mascote e Daimon", "comida", "Comida de mascote", "PET_FOOD_ESSENCE",
    "宠物食品 (ExpTypes.h:2241): dá lealdade (hornor) e experiência ao mascote (classe CLS_ITEM_PET_FOOD)."),
  s("Mascote e Daimon", "daimon", "Daimon", "GOBLIN_ESSENCE", "小精灵本体 (ExpTypes.h:3092): o Daimon (classe CLS_ITEM_ELF).", { so: "1.5.5" }),
  s("Mascote e Daimon", "pilula-daimon", "Pílulas de experiência do Daimon", "GOBLIN_EXPPILL_ESSENCE",
    "小精灵经验丸 (ExpTypes.h:3172): experiência para o Daimon (classe CLS_ITEM_ELF_EXPPILL).", { so: "1.5.5" }),
  s("Mascote e Daimon", "alma", "Almas de monstro", "MONSTER_SPIRIT_ESSENCE",
    "怪物元魂 (ExpTypes.h:2535; classe CLS_ITEM_SOUL).", { so: "1.5.5" }),
  // Fogos e diversos
  s("Fogos e diversos", "fogos", "Fogos de artifício", "FIREWORKS_ESSENCE",
    "Toca o efeito file_fw (.gfx) na frente do personagem por time_to_fire segundos (EC_Player.cpp:9022-9024; classe CLS_ITEM_FIREWORKS)."),
  s("Fogos e diversos", "fogos2", "Fogos de artifício 2", "FIREWORKS2_ESSENCE", "烟花2本体 (ExpTypes.h:2317; classe CLS_ITEM_FIREWORKS2).", { so: "1.5.5" }),
  s("Fogos e diversos", "alto-falante", "Alto-falantes", "SPEAKER_ESSENCE", "Mensagem para o servidor todo (classe CLS_ITEM_BUGLE, setclassid.cpp:164-165)."),
  s("Fogos e diversos", "arremesso", "Arremessáveis", "TOSSMATTER_ESSENCE", "Item atirado no alvo com o dano da faixa (classe CLS_ITEM_TOSSMATTER)."),
  s("Fogos e diversos", "aljava", "Aljavas", "QUIVER_ESSENCE", "箭囊 (ExpTypes.h:1996): dá de num_min a num_max munições id_projectile.", { menor: "QUIVER_SUB_TYPE" }),
  s("Fogos e diversos", "catapulta", "Convocar catapulta", "WAR_TANKCALLIN_ESSENCE", "投石车招唤券 (ExpTypes.h:2353; classe CLS_ITEM_CONTROLLER)."),
  s("Fogos e diversos", "gerador-gm", "Geradores do GM", "GM_GENERATOR_ESSENCE",
    "GM 物品生成器 (ExpTypes.h:2178): faz nascer id_object (classe CLS_ITEM_MOBGEN).", { menor: "GM_GENERATOR_TYPE" }),
  s("Fogos e diversos", "barraca", "Certificado de barraca", "SELL_CERTIFICATE_ESSENCE", "售卖凭证 (ExpTypes.h:3195; classe CLS_ITEM_STALLCARD).", { so: "1.5.5" }),
  s("Fogos e diversos", "usar-no-alvo", "Usar no alvo", "TARGET_ITEM_ESSENCE", "对目标使用物品 (ExpTypes.h:3222): lança id_skill no alvo (classe CLS_ITEM_SKILLTRIGGER2).", { so: "1.5.5" }),
  s("Fogos e diversos", "ver-alvo", "Ver atributos do alvo", "LOOK_INFO_ESSENCE", "查看目标属性道具 (ExpTypes.h:3264; classe CLS_ITEM_QUERYOTHERPROPERTY).", { so: "1.5.5" }),
  s("Fogos e diversos", "casamento", "Reserva de casamento", "WEDDING_BOOKCARD_ESSENCE", "婚礼预约凭证 (ExpTypes.h:2423).", { so: "1.5.5" }),
  s("Fogos e diversos", "convite", "Convite de casamento", "WEDDING_INVITECARD_ESSENCE", "婚礼请柬 (ExpTypes.h:2449).", { so: "1.5.5" }),
  s("Fogos e diversos", "faccao", "Fichas de facção", "FORCE_TOKEN_ESSENCE", "势力令牌 (ExpTypes.h:962): reputação de facção (classe CLS_ITEM_FORCE_TICKET).", { so: "1.5.5" }),
  s("Fogos e diversos", "dinheiro", "Conversíveis em dinheiro", "MONEY_CONVERTIBLE_ESSENCE", "Classe -1 (setclassid.cpp:247-248): usar não faz nada.", { so: "1.5.5" }),
  s("Fogos e diversos", "ficha-loja", "Fichas de loja", "SHOP_TOKEN_ESSENCE", "商城代币 (ExpTypes.h:1290); classe -1 (setclassid.cpp:271-273).", { so: "1.5.5" }),
  s("Fogos e diversos", "funcao", "Itens de função do cliente", "UNIVERSAL_TOKEN_ESSENCE", "通用客户端功能物品 (ExpTypes.h:1312); classe -1.", { so: "1.5.5" }),
  s("Fogos e diversos", "pacote", "Pacotes por classe", "ITEM_PACKAGE_BY_PROFESSION_ESSENCE",
    "通用职业物品包 (ExpTypes.h:1473): dá o item da classe e do gênero (classe CLS_ITEM_OCCUP_PACKAGE).", { so: "1.5.5" }),
  // Cartas e Astrolábio (1.5.5)
  s("Cartas e Astrolábio", "pacote-cartas", "Pacotes de cartas", "POKER_DICE_ESSENCE", "Sorteia uma carta da lista (ExpTypes.h:619; classe CLS_ITEM_GENERALCARD_DICE).", { so: "1.5.5" }),
  s("Cartas e Astrolábio", "astrolabio", "Astrolábios", "ASTROLABE_ESSENCE", "星盘 (ExpTypes.h:1367; classe CLS_ITEM_ASTROLABE).", { so: "1.5.5" }),
  s("Cartas e Astrolábio", "astro-efeito", "Astrolábio: sortear efeito", "ASTROLABE_RANDOM_ADDON_ESSENCE", "星盘属性随机道具 (ExpTypes.h:1405).", { so: "1.5.5" }),
  s("Cartas e Astrolábio", "astro-aptidao", "Astrolábio: aptidão", "ASTROLABE_INC_INNER_POINT_VALUE_ESSENCE", "星盘涨资质道具 (ExpTypes.h:1426).", { so: "1.5.5" }),
  s("Cartas e Astrolábio", "astro-exp", "Astrolábio: experiência", "ASTROLABE_INC_EXP_ESSENCE", "星盘涨经验道具 (ExpTypes.h:1452).", { so: "1.5.5" }),
  // Minas: não é item (o generate_item devolve -1), é o recurso do mapa que dá os materiais.
  s("Minas (no mapa)", "minas", "Minas e recursos", "MINE_ESSENCE",
    "Não é item: nasce nas áreas de recurso do npcgen (npcgenerator.cpp:3888-3932) e, ao ser colhida, dá um dos materiais sorteado quando ela nasceu (npcgenerator.cpp:5003-5016; matter.cpp:265-440).",
    { menor: "MINE_TYPE" }),
];

/** Os grupos na ordem da lista, com as subcategorias que existem na versão. */
export function gruposDeItens(versao: string, existentes?: Set<string>) {
  const g = new Map<string, SubDeItem[]>();
  for (const x of SUBCATEGORIAS_DE_ITENS) {
    if (x.so && x.so !== versao) continue;
    if (existentes?.size && !existentes.has(x.tabela)) continue;
    g.set(x.grupo, [...(g.get(x.grupo) ?? []), x]);
  }
  return [...g.entries()].map(([grupo, subs]) => ({ grupo, subs }));
}

export const subDeItemPorTabela = (tabela: string) => SUBCATEGORIAS_DE_ITENS.find((x) => x.tabela === tabela);

/* ---- Remédio (MEDICINE_ESSENCE): a categoria decide o que o GS faz ----
   `generate_medicine` (generate_item_temp.h:913-990) e `item_potion.cpp`. `cool_time` em ms (ExpTypes.h:852); valor
   abaixo de 1000 é multiplicado por 100 ao gerar o item (generate_item_temp.h:966, :975, :984, :993). Vida/mana "no
   tempo": um filtro reparte o total por batida de 1 s (potion_filter.h:16-64; tomar outra soma tempo e total, :30-42). */
export const CATEGORIAS_DE_REMEDIO: Record<number, { nome: string; familia: string }> = {
  1794: { nome: "vida ao longo do tempo", familia: "recarga de poção de vida (COOLDOWN_INDEX_HP_POTION)" },
  1802: { nome: "mana ao longo do tempo", familia: "recarga de poção de mana (COOLDOWN_INDEX_MP_POTION)" },
  1810: { nome: "vida e mana na hora", familia: "recarga de poção de vida e mana (COOLDOWN_INDEX_REJUVENATION_POTION)" },
  1815: { nome: "antídoto (enfraquece o veneno)", familia: "recarga de antídoto (COOLDOWN_INDEX_ANITDOTE_POTION)" },
  2038: { nome: "antídoto imediato (tira o veneno)", familia: "recarga de antídoto (COOLDOWN_INDEX_ANITDOTE_POTION)" },
};

/** A recarga que o GS usa (ms): abaixo de 1000, ×100 (generate_item_temp.h:966). */
export const recargaDoRemedio = (coolTime: number) => (coolTime < 1000 ? coolTime * 100 : coolTime);

export function regraDoRemedio(c: { id_major_type: number; require_level: number; cool_time: number; hp_add_total: number;
  hp_add_time: number; mp_add_total: number; mp_add_time: number }): { linhas: string[]; avisos: string[] } {
  const cat = CATEGORIAS_DE_REMEDIO[c.id_major_type];
  const avisos: string[] = [];
  if (!cat) return { linhas: [], avisos: [`Categoria ${c.id_major_type} não é de remédio no GS: o item não é gerado (generate_item_temp.h:932-933).`] };
  const linhas: string[] = [];
  const ms = recargaDoRemedio(c.cool_time);
  const seg = (x: number) => `${(x / 1000).toLocaleString("pt-BR", { maximumFractionDigits: 2 })} s`;
  if (c.id_major_type === 1794 || c.id_major_type === 1802) {
    const [total, tempo, oque] = c.id_major_type === 1794 ? [c.hp_add_total, c.hp_add_time, "vida"] : [c.mp_add_total, c.mp_add_time, "mana"];
    if (tempo > 0) {
      const porSeg = Math.trunc(total / tempo);
      linhas.push(`Recupera ${total} de ${oque} em ${tempo} s: ${porSeg} por segundo (potion_filter.h:24, divisão inteira)`
        + (porSeg * tempo !== total ? ` — no total ${porSeg * tempo}: o resto da divisão se perde (potion_filter.h:59-65).` : "."));
    }
    if (!tempo || !total || !Math.trunc(total / tempo)) avisos.push(`Total e tempo têm de dar pelo menos 1 por segundo (o GS exige, generate_item_temp.h:971-972).`);
    linhas.push("Tomar outra enquanto cura soma o tempo e o que falta (potion_filter.h:30-42).");
  } else if (c.id_major_type === 1810) {
    linhas.push(`Recupera na hora ${c.hp_add_total} de vida e ${c.mp_add_total} de mana (item_potion.cpp:55-70); os tempos não são usados.`);
  } else {
    linhas.push(c.id_major_type === 1815 ? "Enfraquece o veneno ativo (FMID_ANTIDOTE nos filtros de veneno, item_potion.cpp:74-92)."
      : "Tira todo o veneno (RemoveFilter de veneno, item_potion.cpp:94-110).");
  }
  linhas.push(`Recarga ${seg(ms)}${ms !== c.cool_time ? ` (o campo diz ${c.cool_time}; abaixo de 1000 o GS multiplica por 100)` : ""}, família: ${cat.familia}.`);
  if (c.require_level > 0) linhas.push(`Nível mínimo ${c.require_level} (o maior nível que o personagem já teve, item_potion.cpp:20).`);
  return { linhas, avisos };
}

/* ---- Amuleto (AUTOHP/AUTOMP): `item_amulet.cpp` e `gplayer_imp::Heartbeat` (player.cpp:9105-9130) ---- */
export function regraDoAmuleto(total: number, gatilho: number, coolTime: number, vida: boolean): string[] {
  const oque = vida ? "vida" : "mana";
  return [
    `Equipado: quando a ${oque} fica abaixo de ${(gatilho * 100).toLocaleString("pt-BR", { maximumFractionDigits: 1 })}% do máximo, repõe o que falta (player.cpp:9113-9128).`,
    `Gasta do total do amuleto: ${total} pontos; vazio, para (base_amulet::OnAutoTrigger).`,
    `Recarga entre um disparo e outro: ${(coolTime / 1000).toLocaleString("pt-BR", { maximumFractionDigits: 2 })} s (cool_time em ms, itemdataman.cpp:1815-1819).`,
  ];
}

/* ---- Pedra (STONE_ESSENCE) ----
   `require_equip` do layout é o `combined_switch` do ExpTypes (ExpTypes.h:2053-2065): bits que o GS confere ao incrustar
   (`IsStoneFit`, item.cpp:332-353) e o cliente ao mostrar (DlgInstall.cpp:142-154). */
export const BITS_DA_PEDRA: [number, string][] = [
  [0x1, "Não serve em arma"],
  [0x2, "Não serve em armadura"],
  [0x4, "Serve em colar"],
  [0x8, "Serve em cinto"],
];
/** Onde a pedra serve pela máscara (`IsStoneFit`: arma e armadura salvo o bit; acessório só com colar ou cinto). */
export function ondeAPedraServe(mascara: number) {
  return { arma: (mascara & 0x1) === 0, armadura: (mascara & 0x2) === 0, acessorio: (mascara & 0xc) !== 0 };
}

/* ---- Talismã de refino (REFINE_TICKET_ESSENCE) ----
   Tabela de refino do GS (`refine_table`, equip_item.cpp:192-206: sucesso, nada muda, cai um nível, volta a zero, por
   nível atual) com o ajuste do talismã (`RefineItemAddon`, player.cpp:11719-11747; `RefineAddon`, equip_item.cpp:265-273):
   `ext_succeed_prob` soma no sucesso, `ext_reserved_prob` na chance de só cair um nível (os dois presos a 0–1);
   `fail_reserve_level` põe 2,0 em "nada muda" e troca o sucesso por `fail_ext_succeed_prob[nível]`. O sorteio é o
   `abase::RandSelect` (arandomgen.h:129-139): acumulado sobre um número em [0, 1); o que passa de 1 nunca sai e, se a
   soma não chega a 1, o resto cai no primeiro (sucesso). */
export const TABELA_DE_REFINO: [number, number, number, number][] = [
  [0.5, 0.7, 0, 0], [0.3, 0, 0, 1], [0.3, 0, 0, 1], [0.3, 0, 0, 1], [0.3, 0, 0, 1], [0.3, 0, 0, 1], [0.3, 0, 0, 1],
  [0.3, 0, 0, 1], [0.25, 0, 0, 1], [0.2, 0, 0, 1], [0.12, 0, 0, 1], [0.05, 0, 0, 1],
];

export interface Talisma { ext_succeed_prob: number; ext_reserved_prob: number; fail_reserve_level: number; fail_ext_succeed_prob: number[] }

/** As chances reais de [sucesso, nada muda, cai um, zera] do refino no nível atual (0–11), com ou sem talismã. */
export function chancesDoRefino(nivel: number, t?: Talisma | null): [number, number, number, number] {
  const p = [...TABELA_DE_REFINO[nivel]] as [number, number, number, number];
  if (t) {
    const preso = (x: number) => Math.min(1, Math.max(0, x));
    p[0] += preso(t.ext_succeed_prob);
    p[2] += preso(t.ext_reserved_prob);
    if (t.fail_reserve_level) { p[1] += 2; p[0] = t.fail_ext_succeed_prob[nivel] ?? 0; }
  }
  const r: [number, number, number, number] = [0, 0, 0, 0];
  let acc = 0;
  for (let i = 0; i < 4; i++) {
    const lo = Math.min(Math.max(acc, 0), 1);
    acc += p[i];
    const hi = Math.min(Math.max(acc, 0), 1);
    r[i] = Math.max(0, hi - lo);
  }
  r[0] += Math.max(0, 1 - Math.min(Math.max(acc, 0), 1));
  return r;
}

/* ---- Mina (MINE_ESSENCE) ---- */
/** As regras de uma área de recurso do npcgen, como o GS as lê (npcgenerator.cpp:3886-3932). */
export const CAMPOS_DA_AREA_DE_RECURSO: Record<string, string> = {
  vPos_x: "Centro X", vPos_z: "Centro Z", vPos_y: "Altura (Y) do centro",
  fExtX: "Largura (X): a mina nasce no retângulo centro ± largura/2 (npcgenerator.cpp:3893)",
  fExtZ: "Profundidade (Z): centro ± profundidade/2 (npcgenerator.cpp:3893)",
  bAutoRevive: "Renasce depois de colhida (SetRespawn, npcgenerator.cpp:3904)",
  iMaxNum: "Limite de minas vivas da área (SetGenLimit, npcgenerator.cpp:3905)",
  idCtrl: "Controlador que liga/desliga a área (0 = sempre; InsertSpawner, npcgenerator.cpp:3930)",
  bInitGen: "Nasce ao subir o mapa (o GS 1.5.5 não lê este campo nas áreas de recurso)",
  bValidOnce: "Vale uma vez (o GS 1.5.5 não lê este campo nas áreas de recurso)",
  dwGenID: "Id do gerador (o GS 1.5.5 não lê este campo nas áreas de recurso)",
};
export const CAMPOS_DO_RECURSO: Record<string, string> = {
  idTemplate: "Mina (MINE_ESSENCE); mina que o GS recusa não nasce (npcgenerator.cpp:3913-3917)",
  dwNumber: "Quantas minas desta nascem na área (mine_count, npcgenerator.cpp:3919)",
  dwRefreshTime: "Tempo de renascer: 15 + este valor, em segundos, nunca menos de 15 (BASE_REBORN_TIME, config.h:106; npcgenerator.cpp:3920-3922)",
  fHeiOff: "Altura acima do chão (SetOffsetTerrain, npcgenerator.cpp:3923)",
  iResType: "Tipo de recurso (só o editor usa; o GS copia e não lê, npcgendata.cpp:228)",
};
/** Segundos para a mina renascer (npcgenerator.cpp:3920-3922). */
export const renascerDaMina = (dwRefreshTime: number) => Math.max(15, 15 + dwRefreshTime);

/* ---- Boxes da edição de Item (o campo que a tabela/versão não tem some) ---- */
export interface BoxDeItem { titulo: string; campos: string[]; explica?: string }

/** Campos tratados na box principal ou por box própria (não repetem em "Outros campos"). */
export const CAMPOS_PRINCIPAIS = ["ID", "Name", "file_icon", "file_matter", "id_major_type", "id_sub_type", "id_type", "price", "shop_price",
  "pile_num_max", "has_guid", "proc_type", "element_id", "element_num", "file_model"];

export const BOXES_DO_ITEM: BoxDeItem[] = [
  { titulo: "Comércio", campos: ["price", "shop_price", "pile_num_max", "has_guid"],
    explica: "price = o NPC paga ao comprar do jogador; shop_price = o NPC cobra (ExpTypes.h, 卖店价/店卖价); pile_num_max = pilha." },
  { titulo: "Desmontar", campos: ["decompose_price", "decompose_time", "element_id", "element_num"],
    explica: "Desmontar no NPC: custo, tempo e as pedras elementais que saem (ExpTypes.h:1024-1037)." },
  { titulo: "Coleta (o que o GS usa)", campos: ["level", "level_required", "id_equipment_required", "eliminate_tool", "time_min", "time_max", "exp", "skillpoint",
    "num1", "probability1", "num2", "probability2", "task_in", "task_out", "uninterruptable", "permenent", "max_gatherer", "gather_dist",
    "material_gain_ratio", "mine_type", "combined_switch", "action_start_id", "action_cycle_id"],
    explica: "Montados como o GS monta a mina (npcgenerator.cpp:1280-1365) e usados na coleta (matter.cpp:265-440)." },
  { titulo: "Monstros ao colher", campos: ["/^npcgen_[0-9]+_(id_monster|num|radius|life_time)$/", "/^aggros_[0-9]+_(monster_faction|radius|num)$/"],
    explica: "npcgen_N: nascem ao colher (mine_info.monster_list, npcgenerator.cpp:1367-1370; aggros :1340-1343); aggros: pedem ajuda aos monstros do lado (raio até 30)." },
  { titulo: "Uso", campos: ["require_level", "require_level_max", "use_time", "cool_time", "duration", "double_exp_time", "time_to_fire", "level", "color",
    "camera_scale", "character_combo_id", "require_strength", "require_agility", "require_player_level_min", "require_player_level_max",
    "require_weapon_level_min", "require_weapon_level_max", "damage_type", "damage_increased", "damage_reduce_percent", "damage_reduce_time",
    "damage_low", "damage_high_min", "damage_high_max", "attack_range", "use_on_pick", "no_use_in_combat", "use_in_combat", "use_in_sanctuary_only",
    "num_use_pertime", "target_type", "faction_target", "usage", "exp", "hornor", "food_type", "money_hatched", "money_restored", "honor_point",
    "skill_point", "power", "type", "id_pet", "id_object", "id_projectile", "num_min", "num_max", "id_token_shop", "life", "swallow_exp",
    "require_force", "reputation_add", "reputation_increase_ratio", "congregate_type", "addon_time", "equip_mask", "gfx_index", "point_value",
    "require_point_min", "require_point_max", "show_model", "num_sell_item", "num_buy_item", "max_name_length", "year", "month", "day",
    "bound_file", "unsymmetrical", "exp_factor", "init_strength", "init_agility", "init_energy", "init_tili", "max_refine_lvl",
    "total_point_value_min", "total_point_value_max", "base_addon", "level_required", "inc_ratio", "link_num", "action", "msg"] },
];

/** Rótulos dos campos de item que o `rotulos.json` não tem, do comentário de cada campo na struct da tabela no `ExpTypes.h` 1.5.5. */
export const ROTULOS_DE_ITEM: Record<string, string> = {
  hp_add_total: "Vida: total", hp_add_time: "Vida: em quantos segundos", mp_add_total: "Mana: total", mp_add_time: "Mana: em quantos segundos",
  cool_time: "Recarga (ms)", require_level: "Nível mínimo", decompose_price: "Custo de desmontar", decompose_time: "Tempo de desmontar",
  element_id: "Pedra elemental ao desmontar", element_num: "Quantas pedras elementais",
  use_time: "Tempo de uso (s)",
  install_price: "Preço de incrustar", uninstall_price: "Preço de remover",
  weapon_desc: "Texto na arma", armor_desc: "Texto na armadura", decoration_desc: "Texto no acessório",
  id_addon_damage: "Efeito na arma", id_addon_defence: "Efeito na armadura", id_addon_decoration: "Efeito no acessório",
  color: "Cor (0–7)", level: "Nível",
  ext_succeed_prob: "Soma na chance de sucesso", ext_reserved_prob: "Soma na chance de cair só um nível",
  fail_reserve_level: "Falhar não tira nível", binding_only: "Só em peça vinculada", require_level_max: "Grau máximo da peça",
  desc: "Descrição curta",
  total_hp: "Pontos de vida do amuleto", total_mp: "Pontos de mana do amuleto", trigger_amount: "Dispara abaixo de (proporção)",
  double_exp_time: "Experiência dobrada (s)", time_to_fire: "Tempo do efeito (s)", file_fw: "Efeito (.gfx)",
  h_min: "Matiz mín.", h_max: "Matiz máx.", s_min: "Saturação mín.", s_max: "Saturação máx.", v_min: "Brilho mín.", v_max: "Brilho máx.",
  usage: "Uso do corante (0 moda/mascote, 1 tinta de assinatura)",
  id_skill: "Habilidade", level_skill: "Nível da habilidade", level_required: "Nível exigido", inc_ratio: "Proficiência somada",
  level_required_mine: "Nível de coleta", id_equipment_required: "Ferramenta exigida", eliminate_tool: "Gasta a ferramenta",
  time_min: "Tempo de coleta mín. (s)", time_max: "Tempo de coleta máx. (s)", exp: "Experiência", skillpoint: "SP",
  num1: "Quantidade", probability1: "Chance da quantidade (não usada pelo GS)", num2: "Quantidade bônus", probability2: "Chance da quantidade bônus",
  task_in: "Missão exigida", task_out: "Missão dada", uninterruptable: "Coleta não é interrompida", permenent: "Não some ao colher",
  max_gatherer: "Coletores ao mesmo tempo (1–20)", gather_dist: "Distância da coleta (4–20 m)", material_gain_ratio: "Chance de a coleta dar certo",
  mine_type: "Tipo de mina (0–1)", combined_switch: "Interruptores (dono, aviso ao colher)", file_model: "Modelo",
  action_start_id: "Ação de começar", action_cycle_id: "Ação em ciclo",
};
