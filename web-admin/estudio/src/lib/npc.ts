/* B392: o NPC de serviço (`NPC_ESSENCE`) no Estúdio — o que cada campo faz, com a evidência do original.

   Como o cliente monta a janela do NPC (`CDlgNPC::PopupNPCDialog`, `EvolvedPWClient/ElementClient/DlgNPC.cpp:339-1135`):
   o nome, a fala inicial (`hello_msg`, :441) e uma lista de linhas, nesta ordem —
     1. os serviços simples ligados nos bits de `combined_services` e `combined_services2` (:448-850; alguns só aparecem
        em certa condição, ex.: clã só para o líder);
     2. um por serviço com tabela, na ordem fixa do vetor `a_uiService` (:409-436, laço :852-1015), com o `Name` do
        serviço como texto. Comprar (`NPC_BUY_SERVICE`) não vira linha (:868-870): é a aba de venda da janela da loja,
        como o reparo, que só liga os botões da loja (:1431-1440). Decompor só aparece a quem tem habilidade de produção
        (:932-950);
     3. as missões que o NPC dá/recebe (`CheckTaskService`, :1100; serviços `id_task_*` lidos em :1195-1210);
     4. "Sair" (texto 503, :1131).
   O GS (`cgame/gs/npcgenerator.cpp:415-440`) monta o NPC a partir do monstro `id_src_monster` — sem um monstro
   válido o NPC é descartado e não nasce (:421-427). */

export type Atende = "sim" | "nao" | null;

/** Um serviço ligado a uma vaga do NPC. */
export interface Vinculado { campo: string; tabela: string; indice: number; id: number; nome: string }

/** Uma linha da janela do NPC (prévia, `JanelaDoNpc.svelte`). */
export interface LinhaDoMenu { texto: string; tipo: "simples" | "servico" | "missao" | "sair"; chave?: string; nota?: string }

export interface ServicoDoNpc {
  /** campo do `NPC_ESSENCE` */
  campo: string;
  /** tabela do serviço (espaço ESSENCE) */
  tabela: string;
  nome: string;
  grupo: string;
  /** como aparece na janela do NPC */
  menu: "linha" | "na_loja" | "missoes" | "condicional";
  /** o nosso servidor (pw-gs) atende o serviço — despacho `bus_server.rs::servico_de_npc`; `null` = não se aplica */
  atende: Atende;
  explica: string;
}

/* Na ordem do vetor `a_uiService` do cliente (DlgNPC.cpp:409-436); depois os que ficam fora dele. */
export const SERVICOS_DO_NPC: ServicoDoNpc[] = [
  { campo: "id_talk_service", tabela: "NPC_TALK_SERVICE", nome: "Conversa", grupo: "Conversa", menu: "linha", atende: null,
    explica: "Abre a conversa (TALK_PROC) do serviço: janelas em árvore com botões." },
  { campo: "id_sell_service", tabela: "NPC_SELL_SERVICE", nome: "Loja", grupo: "Comércio", menu: "linha", atende: "sim",
    explica: "Vende ao jogador: até 8 abas de 32 itens; o preço é o do item." },
  { campo: "id_buy_service", tabela: "NPC_BUY_SERVICE", nome: "Compra do jogador", grupo: "Comércio", menu: "na_loja", atende: "sim",
    explica: "Não vira linha: é a parte de vender da janela da loja (DlgNPC.cpp:868-870)." },
  { campo: "id_install_service", tabela: "NPC_INSTALL_SERVICE", nome: "Incrustar", grupo: "Fabricação e pedras", menu: "linha", atende: "sim",
    explica: "Põe pedra nos furos do equipamento." },
  { campo: "id_uninstall_service", tabela: "NPC_UNINSTALL_SERVICE", nome: "Retirar pedras", grupo: "Fabricação e pedras", menu: "linha", atende: "sim",
    explica: "Tira as pedras do equipamento." },
  { campo: "id_skill_service", tabela: "NPC_SKILL_SERVICE", nome: "Habilidades", grupo: "Personagem", menu: "linha", atende: "sim",
    explica: "Treinador: as habilidades que ensina (o livro exigido sai da habilidade)." },
  { campo: "id_heal_service", tabela: "NPC_HEAL_SERVICE", nome: "Cura", grupo: "Personagem", menu: "linha", atende: "sim", explica: "Cura o jogador." },
  { campo: "id_transmit_service", tabela: "NPC_TRANSMIT_SERVICE", nome: "Teleporte", grupo: "Viagem", menu: "linha", atende: "sim",
    explica: "Leva a pontos do world_targets com custo e nível mínimo (até 32 destinos)." },
  { campo: "id_transport_service", tabela: "NPC_TRANSPORT_SERVICE", nome: "Transporte", grupo: "Viagem", menu: "linha", atende: "nao", explica: "Rotas de transporte." },
  { campo: "id_proxy_service", tabela: "NPC_PROXY_SERVICE", nome: "Consignação", grupo: "Comércio", menu: "linha", atende: "nao", explica: "Venda por conta do jogador." },
  { campo: "id_storage_service", tabela: "NPC_STORAGE_SERVICE", nome: "Armazém", grupo: "Personagem", menu: "linha", atende: "sim", explica: "Abre o armazém." },
  { campo: "id_make_service", tabela: "NPC_MAKE_SERVICE", nome: "Produção (forja)", grupo: "Fabricação e pedras", menu: "linha", atende: "sim",
    explica: "Receitas em até 8 abas de 32, com a habilidade de produção exigida." },
  { campo: "id_decompose_service", tabela: "NPC_DECOMPOSE_SERVICE", nome: "Decompor", grupo: "Fabricação e pedras", menu: "condicional", atende: "nao",
    explica: "Só aparece a quem tem habilidade de produção (DlgNPC.cpp:932-950)." },
  { campo: "id_identify_service", tabela: "NPC_IDENTIFY_SERVICE", nome: "Avaliar", grupo: "Fabricação e pedras", menu: "linha", atende: "sim",
    explica: "Avalia o item pela taxa; taxa negativa = não avalia (B379)." },
  { campo: "id_war_towerbuild_service", tabela: "NPC_WAR_TOWERBUILD_SERVICE", nome: "Torres da guerra", grupo: "Guerra e território", menu: "linha", atende: "nao",
    explica: "Construção de torres na guerra de território." },
  { campo: "id_resetprop_service", tabela: "NPC_RESETPROP_SERVICE", nome: "Restaurar atributos", grupo: "Personagem", menu: "linha", atende: "sim",
    explica: "Devolve pontos de atributo pelo item exigido." },
  { campo: "id_petname_service", tabela: "NPC_PETNAME_SERVICE", nome: "Mascote: renomear", grupo: "Mascote e Daimon", menu: "linha", atende: "sim", explica: "" },
  { campo: "id_petlearnskill_service", tabela: "NPC_PETLEARNSKILL_SERVICE", nome: "Mascote: aprender habilidade", grupo: "Mascote e Daimon", menu: "linha", atende: "sim", explica: "" },
  { campo: "id_petforgetskill_service", tabela: "NPC_PETFORGETSKILL_SERVICE", nome: "Mascote: esquecer habilidade", grupo: "Mascote e Daimon", menu: "linha", atende: "sim", explica: "" },
  { campo: "id_equipbind_service", tabela: "NPC_EQUIPBIND_SERVICE", nome: "Vincular equipamento", grupo: "Fabricação e pedras", menu: "linha", atende: "nao", explica: "" },
  { campo: "id_equipdestroy_service", tabela: "NPC_EQUIPDESTROY_SERVICE", nome: "Destruir equipamento", grupo: "Fabricação e pedras", menu: "linha", atende: "nao", explica: "" },
  { campo: "id_equipundestroy_service", tabela: "NPC_EQUIPUNDESTROY_SERVICE", nome: "Desfazer destruição", grupo: "Fabricação e pedras", menu: "linha", atende: "nao", explica: "" },
  { campo: "id_goblin_skill_service", tabela: "NPC_SKILL_SERVICE", nome: "Daimon: habilidades", grupo: "Mascote e Daimon", menu: "linha", atende: "sim",
    explica: "Uma lista de habilidades (NPC_SKILL_SERVICE) para o Daimon (npcgenerator.cpp:1055-1061)." },
  { campo: "id_engrave_service", tabela: "NPC_ENGRAVE_SERVICE", nome: "Gravar", grupo: "Fabricação e pedras", menu: "linha", atende: "nao", explica: "" },
  { campo: "id_randprop_service", tabela: "NPC_RANDPROP_SERVICE", nome: "Atributo aleatório", grupo: "Fabricação e pedras", menu: "linha", atende: "nao", explica: "" },
  { campo: "id_force_service", tabela: "NPC_FORCE_SERVICE", nome: "Força (facção)", grupo: "Guerra e território", menu: "condicional", atende: "nao",
    explica: "Aparece conforme a força do jogador (DlgNPC.cpp:1010-1017)." },
  // Fora do vetor do menu:
  { campo: "id_repair_service", tabela: "NPC_REPAIR_SERVICE", nome: "Reparo", grupo: "Comércio", menu: "na_loja", atende: "sim",
    explica: "Não vira linha: liga os botões Reparar e Reparar tudo da janela da loja (DlgNPC.cpp:1431-1440)." },
  { campo: "id_task_out_service", tabela: "NPC_TASK_OUT_SERVICE", nome: "Dar missão", grupo: "Missões", menu: "missoes", atende: "sim",
    explica: "As missões que o NPC oferece (até 256)." },
  { campo: "id_task_in_service", tabela: "NPC_TASK_IN_SERVICE", nome: "Receber missão", grupo: "Missões", menu: "missoes", atende: "sim",
    explica: "As missões que o NPC recebe na entrega (até 256)." },
  { campo: "id_task_matter_service", tabela: "NPC_TASK_MATTER_SERVICE", nome: "Item de missão", grupo: "Missões", menu: "missoes", atende: "sim",
    explica: "Itens que o NPC entrega para uma missão (até 144 linhas)." },
  { campo: "id_cross_server_service", tabela: "NPC_CROSS_SERVER_SERVICE", nome: "Servidor cruzado", grupo: "Guerra e território", menu: "linha", atende: "nao", explica: "" },
];

export const GRUPOS_DE_SERVICO = ["Conversa", "Comércio", "Fabricação e pedras", "Missões", "Viagem", "Personagem", "Mascote e Daimon", "Guerra e território"];

export const servicoDoCampo = (campo: string) => SERVICOS_DO_NPC.find((s) => s.campo === campo);
export const servicosDaTabela = (tabela: string) => SERVICOS_DO_NPC.filter((s) => s.tabela === tabela);

export interface Simples {
  bit: number;
  nome: string;
  atende: Atende;
}

/* `combined_services` (`ExpTypes.h:3763-3767` do cliente; `template/exptypes.h` do GS 1.5.5), nomes traduzidos do
   comentário. "Atende" = o despacho de serviço do pw-gs tem o tratador (B392). */
export const SIMPLES: Simples[] = [
  { bit: 0, nome: "Esquecer habilidade de produção", atende: null },
  { bit: 1, nome: "Furar equipamento", atende: "sim" },
  { bit: 3, nome: "Descobrir posto de teleporte (com o ponto abaixo)", atende: null },
  { bit: 4, nome: "Clã (criar, subir, renomear, desfazer)", atende: null },
  { bit: 5, nome: "Cirurgia (trocar o rosto)", atende: "sim" },
  { bit: 6, nome: "Correio", atende: null },
  { bit: 7, nome: "Leilão", atende: null },
  { bit: 8, nome: "Cartão de experiência dupla", atende: null },
  { bit: 9, nome: "Chocar ovo de mascote", atende: "sim" },
  { bit: 10, nome: "Restaurar mascote em ovo", atende: "sim" },
  { bit: 11, nome: "Gestão da guerra de território", atende: null },
  { bit: 12, nome: "Sair do campo de batalha", atende: null },
  { bit: 13, nome: "Consignação de cartões de ponto", atende: null },
  { bit: 14, nome: "Refinar equipamento", atende: "sim" },
  { bit: 15, nome: "Tingir (equipamento e mascote)", atende: null },
  { bit: 16, nome: "Reverter (扭转乾坤)", atende: null },
  { bit: 17, nome: "Daimon: redistribuir atributos", atende: null },
  { bit: 18, nome: "Daimon: redistribuir habilidades", atende: null },
  { bit: 19, nome: "Daimon: esquecer habilidade", atende: "sim" },
  { bit: 20, nome: "Daimon: refinar", atende: "sim" },
  { bit: 21, nome: "Daimon: decompor", atende: "sim" },
  { bit: 22, nome: "Daimon: tirar equipamento", atende: null },
  { bit: 23, nome: "Reparar equipamento destruído", atende: null },
  { bit: 24, nome: "Plataforma de comércio", atende: null },
  { bit: 25, nome: "Trocar Celestial/Demoníaco", atende: null },
  { bit: 26, nome: "Casamento", atende: "sim" },
  { bit: 27, nome: "Abrir base do clã", atende: null },
  { bit: 28, nome: "Subir nível da base e entregar", atende: null },
  { bit: 29, nome: "Trocar materiais da base", atende: null },
  { bit: 30, nome: "Ver armazém sem senha", atende: null },
  { bit: 31, nome: "Ranking dos sacos de pancada", atende: null },
];

/* `combined_services2` (só no 1.5.5; `ExpTypes.h:3772-3775`). */
export const SIMPLES2: Simples[] = [
  { bit: 0, nome: "Entrar na guerra nacional", atende: null },
  { bit: 1, nome: "Sair do campo da guerra nacional", atende: null },
  { bit: 2, nome: "Assinar equipamento", atende: null },
  { bit: 3, nome: "Entrar no servidor cruzado (abandonado)", atende: null },
  { bit: 4, nome: "Voltar ao servidor de origem", atende: null },
  { bit: 5, nome: "Trocar o nome", atende: null },
  { bit: 6, nome: "Converter e trocar pedra da alma", atende: null },
  { bit: 7, nome: "Eleição do rei", atende: null },
  { bit: 8, nome: "Consignação", atende: null },
  { bit: 9, nome: "Desmontar moda", atende: null },
  { bit: 10, nome: "Renascer", atende: null },
  { bit: 11, nome: "Cartão-presente", atende: null },
  { bit: 12, nome: "Inscrição no campo dos carros de guerra", atende: null },
  { bit: 13, nome: "Renascer carta", atende: "sim" },
  { bit: 14, nome: "Fortalecer espada voadora", atende: null },
  { bit: 15, nome: "Abrir saque de clã", atende: null },
  { bit: 16, nome: "Loja exclusiva (乾坤袋)", atende: null },
  { bit: 17, nome: "Loja exclusiva (鸿利)", atende: null },
  { bit: 18, nome: "Trocar o sexo", atende: null },
  { bit: 19, nome: "Furar acessório", atende: "sim" },
  { bit: 20, nome: "Torre solo: escolher o andar", atende: null },
  { bit: 21, nome: "Torre solo: ranking (servidor)", atende: null },
  { bit: 22, nome: "Guerra de clãs cruzada: inscrição", atende: null },
  { bit: 23, nome: "Guerra de clãs cruzada: resultado da inscrição", atende: null },
  { bit: 24, nome: "Guerra de clãs cruzada: ranking", atende: null },
  { bit: 25, nome: "Guerra de clãs cruzada: ir ao campo", atende: null },
  { bit: 26, nome: "Guerra de clãs cruzada: entrar e sair", atende: null },
  { bit: 27, nome: "Torre solo: ranking (cruzado)", atende: null },
];

export const ligado = (valor: number, bit: number) => ((valor >>> 0) & (2 ** bit)) !== 0;
export const comBit = (valor: number, bit: number, ligar: boolean) => {
  const v = valor >>> 0, b = 2 ** bit;
  const r = ligar ? (v | b) >>> 0 : (v & ~b) >>> 0;
  // o campo é int32 no layout: volta ao valor com sinal
  return r > 0x7fffffff ? r - 0x100000000 : r;
};

/** Explicação de cada campo da ficha, com a evidência. */
export const EXPLICACOES_DO_NPC: Record<string, string> = {
  hello_msg: "A primeira fala da janela do NPC, antes dos botões (DlgNPC.cpp:441). Até 256 caracteres, códigos de cor inclusos.",
  id_src_monster: "Obrigatório: o GS copia a ficha deste monstro (vida, defesa, ataque, tamanho…) para o NPC e descarta o NPC que não tem um monstro válido — ele não nasce (npcgenerator.cpp:421-427).",
  refresh_time: "Segundos para renascer; o GS guarda × 20 (tiques de 50 ms, npcgenerator.cpp:435).",
  attack_rule: "O GS 1.5.5 só copia o valor para o NPC (npcgenerator.cpp:436); nenhuma regra o lê.",
  tax_rate: "Taxa guardada no NPC (npcgenerator.cpp:437, 2661).",
  id_to_discover: "Ponto de teleporte que o jogador descobre falando com ele; só vale com o serviço simples \"Descobrir posto de teleporte\" ligado e precisa existir na lista de pontos (npcgenerator.cpp:880-897).",
  domain_related: "Ligado a território de clã (need_domain, npcgenerator.cpp:438).",
  no_collision: "Sem colisão: o GS não registra o corpo (npcgenerator.cpp:432) e o cliente atravessa (EC_NPC.cpp:2675).",
  id_mine: "Mina (MINE_ESSENCE) anexa ao NPC.",
  distance_service: "Chaves do NPC (combined_switch no GS): 1 = atende a qualquer distância (NCS_IGNORE_DISTANCE_CHECK, template/exptypes.h:3782; npcgenerator.cpp:439).",
  id_type: "Categoria do NPC (tabela NPC_TYPE).",
};

export const ROTULOS_DO_NPC: Record<string, string> = {
  Name: "Nome", id_type: "Tipo", hello_msg: "Fala inicial", id_src_monster: "Luta e corpo de (monstro)", refresh_time: "Renasce em (s)",
  attack_rule: "Regra de ataque", tax_rate: "Taxa", id_to_discover: "Ponto que descobre", domain_related: "Território de clã",
  no_collision: "Sem colisão", id_mine: "Mina anexa", distance_service: "Chaves (distância)", file_model: "Modelo (.ecm)",
};

/** Filtros da busca de NPCs: o que o NPC faz, pelas vagas preenchidas (faixa mín. 1 no campo, sobre a tabela inteira). */
export const FILTROS_DE_NPC: { chave: string; nome: string; campo: string }[] = [
  { chave: "loja", nome: "Lojas", campo: "id_sell_service" },
  { chave: "producao", nome: "Produção", campo: "id_make_service" },
  { chave: "teleporte", nome: "Teleporte", campo: "id_transmit_service" },
  { chave: "missao", nome: "Dá missões", campo: "id_task_out_service" },
  { chave: "entrega", nome: "Recebe missões", campo: "id_task_in_service" },
  { chave: "armazem", nome: "Armazém", campo: "id_storage_service" },
  { chave: "treinador", nome: "Treinadores", campo: "id_skill_service" },
  { chave: "mascote", nome: "Mascote", campo: "id_petname_service" },
  { chave: "conversa", nome: "Com conversa", campo: "id_talk_service" },
];

/** Direção do spawn: o GS usa `atan2(z, x) × 128/π` do `vDir` quando a área é um ponto (`npcgenerator.cpp:4348-4367`;
    `pw-gs::entity::direcao_do_gerador`); com tamanho, sorteia. Aqui em graus: 0 = +x (leste), 90 = +z (norte). */
export const grausDoVetor = (x: number, z: number) => {
  if (!x && !z) return 0;
  const g = (Math.atan2(z, x) * 180) / Math.PI;
  return Math.round(((g % 360) + 360) % 360);
};
export const vetorDosGraus = (g: number) => {
  const r = (g * Math.PI) / 180;
  return { x: Math.round(Math.cos(r) * 10000) / 10000, z: Math.round(Math.sin(r) * 10000) / 10000 };
};
