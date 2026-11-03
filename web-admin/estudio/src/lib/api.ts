/* Estúdio (B366): acesso às rotas do painel. O Estúdio não tem caminho de escrita próprio — só repete o que os
   Editores (`static/estudio.js`) já chamam: ficha, PUT de campos, clonar, espaços, textos, ícones e modelos.
   A sessão é o cookie do painel (mesma origem); o CSRF sai de `GET /api/sessao` (`painel/aplicacao.py:90`). */

let csrf: string | null = null;

export class ErroDaApi extends Error {
  status: number;
  constructor(mensagem: string, status: number) {
    super(mensagem);
    this.status = status;
  }
}

export async function sessao(): Promise<{ usuario?: string; csrf: string } | null> {
  const r = await fetch("/api/sessao", { credentials: "same-origin" });
  if (!r.ok) return null;
  const s = await r.json();
  csrf = s.csrf;
  return s;
}

export async function api<T = any>(caminho: string, opcoes: RequestInit = {}): Promise<T> {
  const cabecalhos: Record<string, string> = { ...(opcoes.headers as Record<string, string>) };
  if (opcoes.body) cabecalhos["Content-Type"] = "application/json";
  if (csrf) cabecalhos["X-CSRF-Token"] = csrf;
  const resposta = await fetch(caminho, { ...opcoes, headers: cabecalhos, credentials: "same-origin" });
  const dados = resposta.status === 204 ? null : await resposta.json().catch(() => null);
  if (!resposta.ok) {
    throw new ErroDaApi(dados?.mensagem || dados?.detail || dados?.codigo || `HTTP ${resposta.status}`, resposta.status);
  }
  return dados as T;
}

/* ---- tipos do que o pw-recursos devolve (`crates/pw-recursos/src/estudio.rs`) ---- */

export interface Produto { id: number; nome: string; icone: string | null }

export interface Referencia {
  tabela?: string;
  indice?: number;
  id: number;
  nome?: string;
  icone?: string | null;
  ausente?: boolean;
  /** B393: receita — o item de `targets_1_id_to_make`, que o jogo mostra no lugar dela. */
  produto?: Produto | null;
}

export interface Campo {
  nome: string;
  tipo: "int32" | "float" | "string" | "wstring" | string;
  tamanho: number;
  valor: number | string | null;
  rotulo: string | null;
  grupo: string;
  cita?: string;
  referencia?: Referencia;
  duvidoso?: boolean;
}

export interface Ficha {
  tabela: string;
  indice: number;
  id: number | null;
  espaco: string | null;
  nome: string;
  icone: string | null;
  produto?: Produto | null;
  campos: Campo[];
  usado_por: unknown[];
}

export interface Registro {
  indice: number;
  id: number | null;
  nome: string;
  icone: string | null;
  produto?: Produto | null;
  tabela?: string;
  /** B376: valores das `colunas` pedidas e, nas colunas `id_*`, o nome do registro citado. */
  campos?: Record<string, number | null>;
  nomes?: Record<string, string>;
}

export interface Pagina {
  total: number;
  pagina: number;
  por_pagina: number;
  registros: Registro[];
  /** B376: as colunas que a tabela tem, a ordem aplicada e os valores distintos das colunas `id_*`. */
  colunas?: string[];
  ordem?: string | null;
  opcoes?: Record<string, { valor: number; nome: string | null; registros: number }[]>;
}

/** B376: colunas, ordenação (`campo` ou `-campo`) e faixas por campo — valem sobre a tabela inteira. */
export interface FiltroDeTabela {
  colunas?: string[];
  ordem?: string;
  faixas?: Record<string, { min?: number | null; max?: number | null }>;
}

export function parametrosDoFiltro(f: FiltroDeTabela): Record<string, string> {
  const r: Record<string, string> = {};
  if (f.colunas?.length) r.colunas = f.colunas.join(",");
  if (f.ordem) r.ordem = f.ordem;
  const faixas = Object.entries(f.faixas ?? {})
    .filter(([, x]) => x.min != null || x.max != null)
    .map(([c, x]) => `${c}:${x.min ?? ""}:${x.max ?? ""}`);
  if (faixas.length) r.faixas = faixas.join(",");
  return r;
}

export interface Rascunho {
  id: number;
  estado: string;
  descricao?: string;
}

/* ---- rotas ---- */

export const contexto = { realm: "", versao: "", rascunho: null as number | null };

function rota(resto: string, extra: Record<string, string | number> = {}, comRascunho = true): string {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(extra)) q.set(k, String(v));
  if (comRascunho && contexto.rascunho) q.set("rascunho", String(contexto.rascunho));
  const s = q.toString();
  return `/api/realms/${encodeURIComponent(contexto.realm)}/estudio/${resto}${s ? `?${s}` : ""}`;
}

export async function versaoDoRealm(realm: string): Promise<string> {
  return (await api<{ versao: string }>(`/api/realms/${encodeURIComponent(realm)}/estudio`)).versao;
}

export async function rascunhos(): Promise<Rascunho[]> {
  const todas = await api<Rascunho[]>(`/api/recursos/${encodeURIComponent(contexto.versao)}/alteracoes?limite=100`);
  return todas.filter((a) => a.estado === "rascunho");
}

export async function criarRascunho(descricao: string): Promise<number> {
  const operacao_id = (crypto.randomUUID ? crypto.randomUUID() : `${Date.now()}-${Math.random()}`).slice(0, 64);
  const r = await api<{ id: number }>(`/api/recursos/${encodeURIComponent(contexto.versao)}/rascunhos`, {
    method: "POST", body: JSON.stringify({ descricao, operacao_id }),
  });
  return r.id;
}

export const tabelas = () => api<{ tabelas: { nome: string; registros: number; espaco: string | null }[] }>(rota("tabelas"));

export const listar = (tabela: string, busca: string, pagina: number, porPagina: number, filtro: FiltroDeTabela = {}) =>
  api<Pagina>(rota(`tabelas/${tabela}`, { busca, pagina, por_pagina: porPagina, ...parametrosDoFiltro(filtro) }));

export const ficha = (tabela: string, indice: number) => api<Ficha>(rota(`tabelas/${tabela}/${indice}`));

export const salvar = (tabela: string, indice: number, campos: Record<string, number | string>) =>
  api<{ igual_ao_publicado?: boolean; resultado?: { campos_mudados: number } }>(rota(`tabelas/${tabela}/${indice}`), {
    method: "PUT", body: JSON.stringify({ campos }),
  });

export const clonar = (tabela: string, indice: number) =>
  api<{ resultado: { id: number; indice: number } }>(rota(`tabelas/${tabela}/${indice}/clonar`), { method: "POST" });

export const buscarNoEspaco = (espaco: string, busca: string, limite = 30) =>
  api<Registro[]>(rota(`espacos/${espaco}`, { busca, limite }));

export const textos = (arquivo: string, busca: string, pagina = 0) =>
  api<{ linhas: [number, string][]; total: number }>(rota(`textos/${arquivo}`, { busca, pagina }));

/** O texto da chave exata. A busca do servidor (`Estudio::textos`) também acha textos que contêm o número, na ordem
    do arquivo; a chave pode estar em qualquer página, então percorre até achar. `null` = não existe. */
export async function textoDaChave(arquivo: string, chave: number): Promise<string | null> {
  for (let pagina = 0; pagina < 200; pagina++) {
    const r = await textos(arquivo, String(chave), pagina);
    const achada = r.linhas.find(([k]) => k === chave);
    if (achada) return achada[1];
    if (!r.linhas.length) return null;
  }
  return null;
}

export const definirTexto = (arquivo: string, chave: number, texto: string) =>
  api(rota(`textos/${arquivo}/${chave}`), { method: "PUT", body: JSON.stringify({ texto }) });

export const novoTexto = (arquivo: string, id: number, texto: string) =>
  api(rota(`textos/${arquivo}`), { method: "POST", body: JSON.stringify({ id, texto }) });

/** GLB do modelo: pose parada, uma ação (`acao`, a prévia) ou, com `todas`, todas as ações-base (o que vai ao Blender, B380). */
export const rotaDoGlb = (caminho: string, acao?: string, todas = false, extra: Record<string, string> = {}) =>
  rota("modelos/glb", { ...(todas ? { caminho, todas: "true" } : acao ? { caminho, acao } : { caminho }), ...extra });

/** Ícone do item: o `file_icon` em bytes GBK, em hexadecimal (B188). */
export const urlDoIcone = (hex: string | null | undefined) => (hex ? `/api/icones/m/${hex}.png` : "");

/** B366: nomes do atlas de ícones de item (só leitura, `painel/icones.py`). */
export const buscarIcones = (busca: string, limite = 60) =>
  api<{ total: number; icones: { arquivo: string; hex: string }[] }>(
    `/api/icones/m?${new URLSearchParams({ busca, limite: String(limite) })}`);

/** B206: efeitos que o cliente descreve, pelo texto ou pelo id (catálogo do `item_ext_prop.txt`, parâmetros 0). */
export const buscarEfeitosPorTexto = (busca: string) =>
  api<{ efeitos: { id: number; texto: string }[] }>(`/api/efeitos?${new URLSearchParams({ busca })}`);

/** B206: a descrição do efeito com os parâmetros do `EQUIPMENT_ADDON` (como a dica do cliente). */
export const descreverEfeito = (id: number, args: number[]) =>
  api<{ id: number; linhas: string[] }>(`/api/efeitos/${id}?${new URLSearchParams({ args: args.join(",") })}`);

export interface DescricaoDeEfeito { id: number; texto: string; args: number[] | null }

/** B366: a descrição de cada efeito como o cliente mostra, com o valor do `EQUIPMENT_ADDON` (a mesma rota da edição
    de personagem, B209: `GET /api/realms/{r}/efeitos?item=&ids=`; quem lê o valor é o GS do realm, então vale o
    `elements` publicado — efeito que só existe no rascunho volta sem texto). `item` só serve de contexto (≥ 1). */
export async function descreverEfeitos(ids: number[], item: number): Promise<DescricaoDeEfeito[]> {
  const validos = [...new Set(ids.filter((i) => i >= 1 && i <= 8191))];
  const r: DescricaoDeEfeito[] = [];
  for (let k = 0; k < validos.length; k += 150) {
    const q = new URLSearchParams({ item: String(Math.max(1, item)), ids: validos.slice(k, k + 150).join(",") });
    const d = await api<{ efeitos: { id: number; texto?: string; args?: number[] | null }[] }>(
      `/api/realms/${encodeURIComponent(contexto.realm)}/efeitos?${q}`);
    for (const e of d.efeitos) r.push({ id: e.id, texto: e.texto ?? "", args: e.args ?? null });
  }
  return r;
}

/** B366: link direto para os Editores do painel, na vista Tabelas, com o registro aberto (`static/painel.js`). */
export function linkDosEditores(tabela: string, indice: number): string {
  const q = new URLSearchParams({ realm: contexto.realm, tabela, indice: String(indice) });
  if (contexto.rascunho) q.set("rascunho", String(contexto.rascunho));
  return `/#editores?${q}`;
}

/* ---- B369: de onde vem o item (`Estudio::origens_do_item`, só leitura) ---- */
export interface RefDeOrigem { tabela: string; indice: number; id: number | null; nome: string; icone: string | null }
export interface PaginaDeOrigem { pagina: number; titulo: string }
export interface Origens {
  id: number;
  dropado_de: (RefDeOrigem & { chance: number; linhas: number })[];
  /* Drop global do GS (`extra_drops.sev`): `indice` nulo = monstro citado na tabela que o elements não tem. */
  dropado_de_global: (Omit<RefDeOrigem, "indice"> & { indice: number | null; chance: number; tabela_global: number; nome_da_tabela: string; tipo: "soma" | "substitui" })[];
  dropado_de_global_erro: string | null;
  recompensa_de: { id: number; nome: string; quando: string; quantidade: number; sorteio: boolean }[];
  vendido_por: (RefDeOrigem & { paginas: PaginaDeOrigem[]; npcs: RefDeOrigem[] })[];
  loja_gold: { arquivo: string; indice: number; nome: string; quantidade: number; preco: number }[];
  forjado_por: (RefDeOrigem & { chance: number | null; forjas: (RefDeOrigem & { paginas: PaginaDeOrigem[]; npcs: RefDeOrigem[] })[] })[];
}
export const origensDoItem = (id: number) => api<Origens>(rota(`origens/${id}`));

/** Para onde o item vai: receitas que o gastam, minas que o dão, habilidades que o exigem (`Estudio::usos_do_item`). */
export interface UsosDoItem {
  id: number;
  ingrediente_de: (RefDeOrigem & { quantidade: number; faz: (RefDeOrigem | { id: number; nome: null })[] })[];
  minas: (RefDeOrigem & { chance: number; vida: number; valida: boolean; motivo?: string })[];
  habilidades: { id: number; nome: string; niveis: number[] }[];
}
export const usosDoItem = (id: number) => api<UsosDoItem>(rota(`usos/${id}`));

/** A mina como o GS a monta (`Estudio::mina_do_gs`, `npcgenerator.cpp:1280-1365`), ou o motivo da recusa. */
export type RefOuId = RefDeOrigem | { id: number; nome: null };
export interface MinaDoGs {
  valida: boolean;
  motivo?: string;
  mina?: {
    quantidade: number; quantidade_bonus: number; chance_bonus: number; tempo_minimo: number; tempo_maximo: number; nivel: number;
    exp: number; sp: number; materiais: { item: RefOuId; chance: number; vida: number }[]; ferramenta: RefOuId | null;
    missao_de_entrada: number; missao_de_saida: number; ininterrupta: boolean; permanente: boolean; gasta_ferramenta: boolean;
    distancia: number; coletores: number; chance_de_sucesso: number; tipo: number;
    monstros_ao_colher: { monstro: RefOuId; quantidade: number; raio: number; vida: number }[];
  };
}
export const minaDoGs = (indice: number) => api<MinaDoGs>(rota(`minas/${indice}`));

/* ---- B371: monstros — habilidades, onde nasce e as áreas do npcgen (rotas dos Editores, B243/B248/B284) ---- */
export interface Habilidade { id: number; nome: string; type: number; cls: number; max_level: number }
export const buscarHabilidades = (busca: string) =>
  api<{ habilidades: Habilidade[]; total: number }>(rota("habilidades", { busca }));

const cacheDeHabilidade = new Map<number, Promise<Habilidade | null>>();
/** Nome e tipo de uma habilidade pelo id exato (a busca também acha ids que contêm o número). */
export function habilidadePorId(id: number): Promise<Habilidade | null> {
  if (!cacheDeHabilidade.has(id)) {
    cacheDeHabilidade.set(id, buscarHabilidades(String(id)).then((r) => r.habilidades.find((h) => h.id === id) ?? null).catch(() => null));
  }
  return cacheDeHabilidade.get(id)!;
}

export interface Local { area: number; caminho: string; mapa: string; quantidade: number; tipo: string; x: number; z: number }
export const nascimentos = (id: number) => api<{ id: number; locais: Local[] }>(rota("nascimentos", { id }));

export interface MapaDoRealm { caminho: string; id: number; nome: string; colunas: number; linhas: number; npcgen: boolean }
export const mapasDoRealm = () => api<MapaDoRealm[]>(rota("mapas", {}, false));

export interface Area { indice: number; campos: Record<string, number | boolean>; filhos: Record<string, number | boolean>[] }
/** Áreas do npcgen: "areas" (criatura, B243) ou "recursos" (área de recurso: as minas, `NPCGENFILERESAREA`). */
export type TipoDeArea = "areas" | "recursos";
const rotaDaArea = (caminho: string, tipo: TipoDeArea, indice: number) => `mapas/${encodeURIComponent(caminho)}/npcgen/${tipo}/${indice}`;
export const lerArea = (caminho: string, indice: number, tipo: TipoDeArea = "areas") => api<Area>(rota(rotaDaArea(caminho, tipo, indice)));
export const salvarArea = (caminho: string, indice: number, campos: Record<string, number | boolean>, filhos?: Record<string, number | boolean>[], tipo: TipoDeArea = "areas") =>
  api<{ resultado?: unknown }>(rota(rotaDaArea(caminho, tipo, indice)), {
    method: "PUT", body: JSON.stringify({ campos, filhos: filhos ?? null }),
  });
export const clonarArea = (caminho: string, indice: number, tipo: TipoDeArea = "areas") =>
  api<{ resultado?: { indice?: number }; indice?: number }>(rota(`${rotaDaArea(caminho, tipo, indice)}/clonar`), { method: "POST" });
export const removerArea = (caminho: string, indice: number, tipo: TipoDeArea = "areas") =>
  api(rota(rotaDaArea(caminho, tipo, indice)), { method: "DELETE" });
export const alturaDoChao = (caminho: string, x: number, z: number) =>
  api<{ tem_terreno: boolean; y: number }>(rota(`mapas/${encodeURIComponent(caminho)}/altura`, { x, z }, false));
export const npcgenDoMapa = (caminho: string) => api<any>(rota(`mapas/${encodeURIComponent(caminho)}/npcgen`));
export const urlDaPeca = (caminho: string, peca: string) =>
  `/api/realms/${encodeURIComponent(contexto.realm)}/estudio/mapas/${encodeURIComponent(caminho)}/pecas/${peca}.png`;

/** Ponto (centro) e tamanho da área de um spawner, em unidades do mundo (`vPos_x/z`, `vExts_x/z`). */
export interface EscolhaNoMapa { x: number; z: number; largura: number; altura: number; /** B392: graus (0 = +x, 90 = +z) */ direcao?: number }

/* ---- B373: modelo e ações do monstro (rotas dos Editores, B272-B300; `modelos/base` nova) ---- */
export interface FichaDoModelo {
  tipo: string; smd: string; smd_lido: boolean; versao: number;
  combinadas: { nome: string; bases: string[]; bases_ausentes: string[]; filhas: unknown[] }[];
  bases_do_smd: string[] | null;
  diagnostico_de_monstro: { indice: number; pedido: string; estado: string; alternativa: string | null }[] | null;
  /** B384: nome do cliente → nome legível (a mesma tabela do GLB exportado, `admin.rs::TRADUCOES_DE_ACAO`). */
  nomes_legiveis?: Record<string, string>;
}
export const fichaDoModelo = (caminho: string, extra: Record<string, string> = {}) =>
  api<FichaDoModelo>(rota("modelos/ficha", { caminho, ...extra }));
export const trocarBase = (caminho: string, acao: string, base: string) =>
  api(rota("modelos/base"), { method: "POST", body: JSON.stringify({ caminho, acao, base }) });
export const acaoNova = (caminho: string, origem: string, nova: string, base?: string) =>
  api(rota("modelos/acao"), { method: "POST", body: JSON.stringify(base ? { caminho, origem, nova, base } : { caminho, origem, nova }) });
/** Animações do glTF (Blender) como ações do .smd (`acoes_glb`, B295). */
export async function importarAcoesGlb(smd: string, glb: ArrayBuffer) {
  const r = await fetch(rota("modelos/acoes_glb", { smd }), {
    method: "POST", credentials: "same-origin", body: glb,
    headers: { "Content-Type": "application/octet-stream", ...(csrf ? { "X-CSRF-Token": csrf } : {}) },
  });
  const d = await r.json().catch(() => null);
  if (!r.ok) throw new ErroDaApi(d?.mensagem || d?.detail || `HTTP ${r.status}`, r.status);
  return d;
}

/** B374: um modelo inteiro (esqueleto, pele, texturas, animações, .smd e .ecm) a partir de um GLB, em `<pasta>\<título>.*`
    (`modelos/novo`, B298/B299); `molde` = o .ecm de onde o novo herda o cabeçalho e as ações. */
/** B380: GLB no modelo (.ecm) do monstro — só o modelo (malha e texturas) ou só as animações, com o mesmo esqueleto. */
export async function importarGlbNoModelo(caminho: string, modo: "modelo" | "animacoes", glb: ArrayBuffer, extra: { como?: "equipamento" | "matter"; validar?: boolean } = {}) {
  const q: Record<string, string> = { caminho, modo };
  if (extra.como) q.como = extra.como;
  if (extra.validar) q.validar = "true";
  const r = await fetch(rota("modelos/importar_glb", q), {
    method: "POST", credentials: "same-origin", body: glb,
    headers: { "Content-Type": "application/octet-stream", ...(csrf ? { "X-CSRF-Token": csrf } : {}) },
  });
  const d = await r.json().catch(() => null);
  if (!r.ok) throw new ErroDaApi(d?.mensagem || d?.detail || `HTTP ${r.status}`, r.status);
  return d;
}
/** B380: os arquivos do modelo antigo que nada mais usa saem no rascunho. */
export const removerModeloSemUso = (caminho: string) =>
  api<{ removidos: { caminho: string; acao: string }[]; mantidos?: { caminho: string; motivo: string }[]; motivo?: string; usado_por?: { tabela: string; id: number | null; nome: string } }>(
    rota("modelos/sem_uso"), { method: "POST", body: JSON.stringify({ caminho }) });
export async function importarModeloGlb(pasta: string, titulo: string, molde: string, glb: ArrayBuffer, extra: { ganchos?: string; validar?: boolean } = {}) {
  const q: Record<string, string> = molde ? { pasta, titulo, molde } : { pasta, titulo };
  if (extra.ganchos) q.ganchos = extra.ganchos;
  if (extra.validar) q.validar = "true";
  const r = await fetch(rota("modelos/novo", q), {
    method: "POST", credentials: "same-origin", body: glb,
    headers: { "Content-Type": "application/octet-stream", ...(csrf ? { "X-CSRF-Token": csrf } : {}) },
  });
  const d = await r.json().catch(() => null);
  if (!r.ok) throw new ErroDaApi(d?.mensagem || d?.detail || `HTTP ${r.status}`, r.status);
  return d;
}

/* ---- B383: pele de armadura/moda (`realname` sobre o corpo do jogador) ---- */
export interface CorpoDoGrupo { classe: number; nome: string; ecm: string }
export interface GrupoDePele { grupo: string; arquivos: { caminho: string; existe: boolean }[]; corpos: CorpoDoGrupo[] }
export interface PeleDoEquipamento {
  realname: string; genero: number; grupos: GrupoDePele[];
  usado_por: { tabela: string; indice: number; id: number | null; nome: string }[];
}
export const peleDoEquipamento = (realname: string, genero: number) =>
  api<PeleDoEquipamento>(rota("equipamentos/pele", { realname, genero }));
export async function importarPeleDoEquipamento(realname: string, genero: number, grupo: string, classe: number | null, validar: boolean, glb: ArrayBuffer) {
  const q: Record<string, string | number> = { realname, genero, grupo };
  if (classe != null) q.classe = classe;
  if (validar) q.validar = "true";
  const r = await fetch(rota("equipamentos/pele", q), {
    method: "POST", credentials: "same-origin", body: glb,
    headers: { "Content-Type": "application/octet-stream", ...(csrf ? { "X-CSRF-Token": csrf } : {}) },
  });
  const d = await r.json().catch(() => null);
  if (!r.ok) throw new ErroDaApi(d?.mensagem || d?.detail || `HTTP ${r.status}`, r.status);
  return d;
}

/* ---- B392: NPCs, serviços, conversas e missões ---- */

export interface ServicoListado {
  indice: number; id: number; nome: string; npcs: number; nomes_dos_npcs: string[];
  resumo: { itens: number; abas: number; destinos: number; lista: number }; achado_na_lista: boolean;
}
/** Serviços de uma tabela `NPC_*_SERVICE` com quantos NPCs usam cada um (`Estudio::servicos`). */
export const listarServicos = (tabela: string, busca: string, uso: "" | "em_uso" | "sem_uso", pagina: number, porPagina = 50) =>
  api<{ total: number; pagina: number; por_pagina: number; registros: ServicoListado[] }>(
    rota(`servicos/${tabela}`, { busca, pagina, por_pagina: porPagina, ...(uso ? { uso } : {}) }));

export interface MissaoResumida {
  id: number; nome: string | null; ausente?: boolean; tipo?: number; nivel_min?: number; nivel_max?: number;
  mae?: number | null; filhas?: number; npc_entrega?: number; npc_premio?: number;
}
/** Nome e dados de lista de missões pelo id e as que o tasks.data liga ao NPC (`Estudio::missoes_resumo`). */
export const resumoDeMissoes = (ids: number[], npc?: number) =>
  api<{ missoes: MissaoResumida[]; do_npc: null | { entrega: MissaoResumida[]; premia: MissaoResumida[] } }>(
    rota("missoes_resumo", { ids: ids.join(","), ...(npc ? { npc } : {}) }));

export interface OpcaoDeConversa { id: number; texto: string; param: number; funcao?: string }
export interface JanelaDeConversa { id: number; id_pai: number; texto: string; opcoes: OpcaoDeConversa[] }
export interface Conversa { indice: number; id: number; texto: string; janelas: JanelaDeConversa[]; usado_por?: { tabela: string; indice: number; id: number; nome: string; campo?: string }[] }
export interface FuncaoDeConversa { codigo: number; nome: string; rotulo: string }

export const funcoesDeConversa = () => api<FuncaoDeConversa[]>(rota("funcoes", {}, false));
export const listarConversas = (busca: string, pagina = 0, porPagina = 50) =>
  api<{ total: number; registros: { indice: number; id: number; nome: string; janelas: number }[] }>(rota("conversas", { busca, pagina, por_pagina: porPagina }));
export const lerConversa = (indice: number) => api<Conversa>(rota(`conversas/${indice}`));
export const salvarConversa = (indice: number, c: Pick<Conversa, "texto" | "janelas">) =>
  api<{ resultado?: { id: number; mudou: boolean }; igual_ao_publicado?: boolean }>(rota(`conversas/${indice}`), {
    method: "PUT", body: JSON.stringify({ conversa: c }) });
export async function novaConversa(base?: number): Promise<{ indice: number; id: number }> {
  const r = await api<{ indice?: number; id?: number; resultado?: { indice: number; id: number } }>(rota("conversas", base != null ? { base } : {}), { method: "POST" });
  return { indice: r.resultado?.indice ?? r.indice ?? -1, id: r.resultado?.id ?? r.id ?? 0 };
}
/** O índice da conversa de um id (a busca do servidor acha pelo id exato). */
export async function conversaPorId(id: number): Promise<Conversa | null> {
  const r = await listarConversas(String(id), 0, 50);
  const achada = r.registros.find((x) => x.id === id);
  return achada ? lerConversa(achada.indice) : null;
}

export interface LinhaDeMissao { id: number; nome: string; tipo: number; nivel_min: number; nivel_max: number; mae: number | null; filhas: number; npc_entrega: number; npc_premio: number }
export async function listarMissoes(busca: string, topo: boolean, pagina: number, porPagina = 100) {
  const r = await api<{ missoes: number; de_topo: number; total: number; pagina: number; versao_do_arquivo: number; linhas: unknown[][] }>(
    rota("missoes", { busca, topo: topo ? "true" : "false", pagina, por_pagina: porPagina }));
  const linhas: LinhaDeMissao[] = r.linhas.map((l) => ({ id: l[0] as number, nome: l[1] as string, tipo: l[2] as number, nivel_min: l[3] as number,
    nivel_max: l[4] as number, mae: (l[5] as number | null) ?? null, filhas: l[6] as number, npc_entrega: l[7] as number, npc_premio: l[8] as number }));
  return { ...r, linhas };
}
export const lerMissao = (id: number) => api<any>(rota(`missoes/${id}`));
export const editarMissao = (id: number, campos: Record<string, unknown>) =>
  api<{ resultado?: unknown }>(rota(`missoes/${id}`), { method: "PUT", body: JSON.stringify({ campos }) });
export const clonarMissao = (id: number) => api<{ resultado?: { id: number }; id?: number }>(rota(`missoes/${id}/clonar`), { method: "POST" });

/** Posição do NPC no rastreador de missões do cliente (`task_npc.data`; o cliente só o consulta no mapa principal). */
export const posicaoNoRastreador = (id: number, pos: [number, number, number]) =>
  api(rota(`rastreador/${id}`), { method: "PUT", body: JSON.stringify({ campos: { pos } }) });

export interface PontoDeTeleporte { id: number; nome: string | null; dominio: number; pos: number[]; usado_por: { nome: string; indice: number }[] }
export const teleportesDoMapa = (caminho: string) =>
  api<{ mapa: number; pontos: PontoDeTeleporte[] }>(rota(`mapas/${encodeURIComponent(caminho)}/teleportes`));
