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

export interface Referencia {
  tabela?: string;
  indice?: number;
  id: number;
  nome?: string;
  icone?: string | null;
  ausente?: boolean;
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
  campos: Campo[];
  usado_por: unknown[];
}

export interface Registro {
  indice: number;
  id: number | null;
  nome: string;
  icone: string | null;
  tabela?: string;
}

export interface Pagina {
  total: number;
  pagina: number;
  por_pagina: number;
  registros: Registro[];
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

export const listar = (tabela: string, busca: string, pagina: number, porPagina: number) =>
  api<Pagina>(rota(`tabelas/${tabela}`, { busca, pagina, por_pagina: porPagina }));

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

export const rotaDoGlb = (caminho: string) => rota("modelos/glb", { caminho }, false);

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
  recompensa_de: { id: number; nome: string; quando: string; quantidade: number; sorteio: boolean }[];
  vendido_por: (RefDeOrigem & { paginas: PaginaDeOrigem[]; npcs: RefDeOrigem[] })[];
  loja_gold: { arquivo: string; indice: number; nome: string; quantidade: number; preco: number }[];
  forjado_por: (RefDeOrigem & { chance: number | null; forjas: (RefDeOrigem & { paginas: PaginaDeOrigem[]; npcs: RefDeOrigem[] })[] })[];
}
export const origensDoItem = (id: number) => api<Origens>(rota(`origens/${id}`));

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
export const lerArea = (caminho: string, indice: number) => api<Area>(rota(`mapas/${encodeURIComponent(caminho)}/npcgen/areas/${indice}`));
export const salvarArea = (caminho: string, indice: number, campos: Record<string, number | boolean>, filhos?: Record<string, number | boolean>[]) =>
  api<{ resultado?: unknown }>(rota(`mapas/${encodeURIComponent(caminho)}/npcgen/areas/${indice}`), {
    method: "PUT", body: JSON.stringify({ campos, filhos: filhos ?? null }),
  });
export const clonarArea = (caminho: string, indice: number) =>
  api<{ resultado?: { indice?: number }; indice?: number }>(rota(`mapas/${encodeURIComponent(caminho)}/npcgen/areas/${indice}/clonar`), { method: "POST" });
export const removerArea = (caminho: string, indice: number) =>
  api(rota(`mapas/${encodeURIComponent(caminho)}/npcgen/areas/${indice}`), { method: "DELETE" });
export const alturaDoChao = (caminho: string, x: number, z: number) =>
  api<{ tem_terreno: boolean; y: number }>(rota(`mapas/${encodeURIComponent(caminho)}/altura`, { x, z }, false));
export const npcgenDoMapa = (caminho: string) => api<any>(rota(`mapas/${encodeURIComponent(caminho)}/npcgen`));
export const urlDaPeca = (caminho: string, peca: string) =>
  `/api/realms/${encodeURIComponent(contexto.realm)}/estudio/mapas/${encodeURIComponent(caminho)}/pecas/${peca}.png`;

/** Ponto (centro) e tamanho da área de um spawner, em unidades do mundo (`vPos_x/z`, `vExts_x/z`). */
export interface EscolhaNoMapa { x: number; z: number; largura: number; altura: number }
