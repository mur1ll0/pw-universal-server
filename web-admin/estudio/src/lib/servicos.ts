/* B392: os serviços de NPC (tabelas `NPC_*_SERVICE` do elements) no Estúdio — categorias da busca, que editor cada um
   usa e as regras que a tela mostra. Os campos são os dos layouts (`specs/elements_layouts/v156.json`, `v7.json`):
   SELL 8 abas × 32 itens (o v7 sem os custos de contribuição/força); MAKE 8 abas × 32 receitas; INSTALL/UNINSTALL 32
   pedras; TASK_IN/OUT 256 missões (v7: 32); TASK_MATTER 16 missões × 4 itens; SKILL 256 habilidades (v7: 128); PET
   LEARNSKILL 128; TRANSMIT 32 destinos; TRANSPORT 32 rotas; RESETPROP 15 entradas; WAR_TOWERBUILD 4 construções;
   ENGRAVE 16; RANDPROP 8 abas. */

export type Editor = "paginas" | "lista" | "teleporte" | "itens_de_missao" | "campos";

export interface CategoriaDeServico {
  chave: string;
  nome: string;
  grupo: string;
  tabelas: string[];
  editor: Editor;
  /** serviço simples (bit do NPC, sem tabela) */
  simples?: { campo: "combined_services" | "combined_services2"; bits: number[] };
  explica: string;
}

export const CATEGORIAS_DE_SERVICO: CategoriaDeServico[] = [
  { chave: "conversa", nome: "Conversa", grupo: "Conversa", tabelas: ["NPC_TALK_SERVICE"], editor: "campos",
    explica: "O botão abre a conversa (id_dialog) — edite as janelas na aba Conversa do NPC." },
  { chave: "loja", nome: "Lojas", grupo: "Comércio", tabelas: ["NPC_SELL_SERVICE"], editor: "paginas",
    explica: "Até 8 abas de 32 itens. O preço é o do item (price/shop_price), não da loja." },
  { chave: "compra", nome: "Compra do jogador", grupo: "Comércio", tabelas: ["NPC_BUY_SERVICE"], editor: "campos",
    explica: "Sem lista: só liga a venda na janela da loja." },
  { chave: "reparo", nome: "Reparo", grupo: "Comércio", tabelas: ["NPC_REPAIR_SERVICE"], editor: "campos",
    explica: "Sem lista: liga os botões de reparo da janela da loja." },
  { chave: "consignacao", nome: "Consignação", grupo: "Comércio", tabelas: ["NPC_PROXY_SERVICE"], editor: "campos", explica: "" },
  { chave: "producao", nome: "Produção (forja)", grupo: "Fabricação e pedras", tabelas: ["NPC_MAKE_SERVICE"], editor: "paginas",
    explica: "Receitas em até 8 abas de 32, com a habilidade de produção exigida." },
  { chave: "decompor", nome: "Decompor", grupo: "Fabricação e pedras", tabelas: ["NPC_DECOMPOSE_SERVICE"], editor: "campos", explica: "" },
  { chave: "avaliar", nome: "Avaliar", grupo: "Fabricação e pedras", tabelas: ["NPC_IDENTIFY_SERVICE"], editor: "campos",
    explica: "Só a taxa: negativa = o NPC não avalia (B379, serviço 17)." },
  { chave: "pedras", nome: "Incrustar e retirar", grupo: "Fabricação e pedras", tabelas: ["NPC_INSTALL_SERVICE", "NPC_UNINSTALL_SERVICE"], editor: "lista", explica: "" },
  { chave: "equipamento", nome: "Vínculo e destruição", grupo: "Fabricação e pedras",
    tabelas: ["NPC_EQUIPBIND_SERVICE", "NPC_EQUIPDESTROY_SERVICE", "NPC_EQUIPUNDESTROY_SERVICE"], editor: "campos", explica: "" },
  { chave: "gravar", nome: "Gravar e atributo aleatório", grupo: "Fabricação e pedras", tabelas: ["NPC_ENGRAVE_SERVICE", "NPC_RANDPROP_SERVICE"], editor: "lista", explica: "" },
  { chave: "dar_missao", nome: "Dar missão", grupo: "Missões", tabelas: ["NPC_TASK_OUT_SERVICE"], editor: "lista",
    explica: "As missões que o NPC oferece." },
  { chave: "receber_missao", nome: "Receber missão", grupo: "Missões", tabelas: ["NPC_TASK_IN_SERVICE"], editor: "lista",
    explica: "As missões que o NPC recebe na entrega." },
  { chave: "itens_de_missao", nome: "Itens de missão", grupo: "Missões", tabelas: ["NPC_TASK_MATTER_SERVICE"], editor: "itens_de_missao",
    explica: "Até 16 missões, cada uma com até 4 itens que o NPC entrega." },
  { chave: "teleporte", nome: "Teleporte", grupo: "Viagem", tabelas: ["NPC_TRANSMIT_SERVICE"], editor: "teleporte",
    explica: "Até 32 destinos do world_targets, com custo e nível mínimo." },
  { chave: "transporte", nome: "Transporte", grupo: "Viagem", tabelas: ["NPC_TRANSPORT_SERVICE"], editor: "campos", explica: "" },
  { chave: "habilidades", nome: "Habilidades (treinador)", grupo: "Personagem", tabelas: ["NPC_SKILL_SERVICE"], editor: "lista",
    explica: "As habilidades que o treinador ensina. A mesma tabela serve às habilidades de Daimon." },
  { chave: "cura", nome: "Cura", grupo: "Personagem", tabelas: ["NPC_HEAL_SERVICE"], editor: "campos", explica: "" },
  { chave: "armazem", nome: "Armazém", grupo: "Personagem", tabelas: ["NPC_STORAGE_SERVICE"], editor: "campos", explica: "" },
  { chave: "atributos", nome: "Restaurar atributos", grupo: "Personagem", tabelas: ["NPC_RESETPROP_SERVICE"], editor: "campos", explica: "" },
  { chave: "mascote", nome: "Mascote", grupo: "Mascote e Daimon",
    tabelas: ["NPC_PETNAME_SERVICE", "NPC_PETLEARNSKILL_SERVICE", "NPC_PETFORGETSKILL_SERVICE"], editor: "lista", explica: "" },
  { chave: "ovo", nome: "Chocar e restaurar ovo", grupo: "Mascote e Daimon", tabelas: [], editor: "campos",
    simples: { campo: "combined_services", bits: [9, 10] },
    explica: "Não têm tabela: são bits do NPC (combined_services 9 e 10, ExpTypes.h:3763-3767). Vincular liga o bit." },
  { chave: "guerra", nome: "Guerra e território", grupo: "Guerra e território",
    tabelas: ["NPC_WAR_TOWERBUILD_SERVICE", "NPC_FORCE_SERVICE", "NPC_CROSS_SERVER_SERVICE"], editor: "campos", explica: "" },
];

export const GRUPOS_DE_CATEGORIA = [...new Set(CATEGORIAS_DE_SERVICO.map((c) => c.grupo))];

export const categoriaDaTabela = (tabela: string) => CATEGORIAS_DE_SERVICO.find((c) => c.tabelas.includes(tabela));

/** Nome curto de cada tabela (as categorias que juntam mais de uma). */
export const NOME_DA_TABELA: Record<string, string> = {
  NPC_INSTALL_SERVICE: "Incrustar", NPC_UNINSTALL_SERVICE: "Retirar pedras", NPC_EQUIPBIND_SERVICE: "Vincular",
  NPC_EQUIPDESTROY_SERVICE: "Destruir", NPC_EQUIPUNDESTROY_SERVICE: "Desfazer destruição", NPC_ENGRAVE_SERVICE: "Gravar",
  NPC_RANDPROP_SERVICE: "Atributo aleatório", NPC_PETNAME_SERVICE: "Renomear", NPC_PETLEARNSKILL_SERVICE: "Aprender habilidade",
  NPC_PETFORGETSKILL_SERVICE: "Esquecer habilidade", NPC_WAR_TOWERBUILD_SERVICE: "Torres", NPC_FORCE_SERVICE: "Força",
  NPC_CROSS_SERVER_SERVICE: "Servidor cruzado",
};

/* ---- leitura das listas de uma ficha (nomes do layout) ---- */

export interface Vaga { campo: string; pagina: number; posicao: number }

/** Grade em abas: `pages_P_goods_K_id` (loja) e `pages_P_id_goods_K` (produção). Índices a partir de 1 no layout. */
export function vagasEmAbas(nomes: string[]): { titulos: string[]; vagas: Vaga[]; colunasPorAba: number } | null {
  const vagas: Vaga[] = [];
  for (const n of nomes) {
    const m = /^pages_(\d+)_goods_(\d+)_id$/.exec(n) ?? /^pages_(\d+)_id_goods_(\d+)$/.exec(n);
    if (m) vagas.push({ campo: n, pagina: Number(m[1]) - 1, posicao: Number(m[2]) - 1 });
  }
  if (!vagas.length) return null;
  const abas = Math.max(...vagas.map((v) => v.pagina)) + 1;
  const titulos = Array.from({ length: abas }, (_, p) => `pages_${p + 1}_page_title`).filter((t) => nomes.includes(t));
  return { titulos, vagas, colunasPorAba: Math.max(...vagas.map((v) => v.posicao)) + 1 };
}

/** Lista simples de ids: `id_goods_K`, `id_tasks_K`, `id_skills_K`, `id_engrave_K`, `pages_K_id_recipe`. */
export function vagasEmLista(nomes: string[]): { prefixo: string; campos: string[] } | null {
  for (const p of ["id_tasks_", "id_skills_", "id_goods_", "id_engrave_"]) {
    const c = nomes.filter((n) => new RegExp(`^${p}\\d+$`).test(n)).sort((a, b) => Number(a.slice(p.length)) - Number(b.slice(p.length)));
    if (c.length) return { prefixo: p, campos: c };
  }
  const r = nomes.filter((n) => /^pages_\d+_id_recipe$/.test(n));
  return r.length ? { prefixo: "pages_", campos: r } : null;
}

/** O que a lista guarda, para escolher o seletor certo. */
export const espacoDaLista = (prefixo: string, tabela: string) =>
  prefixo === "id_tasks_" ? "MISSAO" : prefixo === "id_skills_" ? "HABILIDADE" : tabela === "NPC_RANDPROP_SERVICE" ? "RECIPE" : "ESSENCE";

/** Compacta uma lista de ids para gravar: os não nulos na frente, zeros no fim (a ordem é a do arquivo). */
export function compactar(ids: number[], tamanho: number): number[] {
  const v = ids.filter((x) => x > 0);
  return [...v, ...Array(Math.max(0, tamanho - v.length)).fill(0)].slice(0, tamanho);
}

/* ---- receita (RECIPE_ESSENCE): a regra do GS ----
   `recipe_manager::LoadTemplate` (`npcgenerator.cpp:1380-1430`): tempo = duration × 20 tiques, entre 1 e 10000;
   produzir (`player.cpp:16419-16423`): com chance `fail_probability` não sai nada (os materiais são gastos); senão
   sorteia um dos 4 resultados pelas chances (`abase::RandSelect`). */
export function regraDaReceita(falha: number, chances: number[]) {
  const soma = chances.reduce((a, b) => a + Math.max(0, b), 0);
  const sai = Math.max(0, Math.min(1, 1 - falha));
  return {
    falha: Math.max(0, Math.min(1, falha)),
    porResultado: chances.map((c) => (soma > 0 ? (sai * Math.max(0, c)) / soma : 0)),
    somaDosResultados: soma,
  };
}
export const tiquesDaReceita = (duracao: number) => Math.min(10000, Math.max(1, Math.trunc(20 * duracao)));

/** Vincular a um NPC (o painel de serviços aberto de dentro do NPC). */
export interface Vinculo {
  npc: string;
  /** a vaga do NPC (campo `id_*_service`) — define a categoria inicial */
  campo?: string;
  aoVincular: (tabela: string, id: number, nome: string) => void;
  /** serviço simples: ligar o bit no NPC */
  aoLigarBit?: (campo: "combined_services" | "combined_services2", bit: number) => void;
}
