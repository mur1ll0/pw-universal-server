/* Estúdio G-Shop: a Loja Gold (`data\gshop*.data`) — rotas, tipos e a lógica da vitrine.
   Evidência (cliente 1.5.5, `F:\PW\1.5.5\EvolvedPWClient\ElementClient`; GS 1.5.5, `cgame/gs`):
   - preço em centavos de gold: `GetCashText` mostra `n/100` gold e `n%100` (`EC_GameUIMan.cpp:7951-7963`);
   - validade (`time`) em segundos, 0 = permanente (`GSHOP_ITEM`, `CCommon/globaldataman.h:113`; o GS a manda como
     `expire_time`, `template/globaldataman.cpp:179`);
   - período: `type` -1 sempre, 0 por datas (time_t gravado em GMT, `playermall.cpp:195`), 1 semanal (bits 0–6 =
     domingo–sábado, `tm_wday`, `playermall.cpp:84`), 2 mensal (bit k = dia k+1, `globaldataman.cpp:168` e
     `playermall.cpp:90`);
   - selo (`status`): 1 novo, 2 promoção, 3 recomendado, 4–12 desconto (paga (status−3)/10 do preço cheio,
     `GetOriginalPrice`, `EC_Shop.cpp:340-348`), 13 relâmpago, 14–19 VIP 1–6 (`DlgQShopItem.cpp:276-296, 455-462`). */
import { api, contexto } from "./api";

export const LOJAS = ["gshop.data", "gshop1.data", "gshop2.data"] as const;
export type ArquivoDaLoja = (typeof LOJAS)[number];

/** O que cada arquivo é no cliente 1.5.5 (`globaldataman.cpp:614-760`, `DlgQShop.cpp:1087-1116, 1705-1736`). */
export const DESCRICAO_DA_LOJA: Record<ArquivoDaLoja, string> = {
  "gshop.data": "Loja Gold (Win_QShop, paga em gold)",
  "gshop1.data": "Segunda loja (Win_BackShop, paga em dividendos; globaldata_getgshopitems2)",
  "gshop2.data": "Loja VIP (globaldata_getgshopitems3: entra no fim da Loja Gold, na aba 5)",
};

/** Linha da listagem (`Estudio::loja`): as 8 colunas do B254 e as 6 do G-Shop. */
export interface Oferta {
  indice: number;
  aba: number;
  sub: number;
  id: number;
  quantidade: number;
  nome: string;
  preco: number;
  item: string | null;
  icone: string | null;
  selo: number;
  vip: number | null;
  brinde: number | null;
  opcoes: number;
  imagem: string;
  /** Preço da opção que o cartão mostra: a primeira com preço (`GetFirstBuyStatus`, `DlgQShop.cpp:914-929`); 0 = desativada. */
  mostrado: number;
}

export interface Conferencia { erros: string[]; avisos: string[] }

export interface Loja {
  arquivo: string;
  formato: "V155Vip" | "V15xSemVip" | "V126" | string;
  carimbo: number;
  no_rascunho: boolean;
  ofertas: Oferta[];
  categorias: { nome: string; subcategorias: string[] }[];
  conferencias: Record<string, Conferencia>;
  campos: [string, unknown][];
}

export function linhaParaOferta(l: unknown[]): Oferta {
  const n = (x: unknown) => (typeof x === "number" ? x : 0);
  return {
    indice: n(l[0]), aba: n(l[1]), sub: n(l[2]), id: n(l[3]), quantidade: n(l[4]), nome: String(l[5] ?? ""), preco: n(l[6]),
    item: (l[7] as string | null) ?? null, icone: (l[8] as string | null) ?? null, selo: n(l[9]),
    vip: typeof l[10] === "number" ? l[10] : null, brinde: typeof l[11] === "number" ? l[11] : null, opcoes: n(l[12]),
    imagem: String(l[13] ?? ""), mostrado: l.length > 14 ? n(l[14]) : n(l[6]),
  };
}

/* ---- rotas (`painel/recursos.py` → `pw-recursos`) ---- */

function rotaDaLoja(resto: string, extra: Record<string, string | number> = {}, comRascunho = true) {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(extra)) q.set(k, String(v));
  if (comRascunho && contexto.rascunho) q.set("rascunho", String(contexto.rascunho));
  const s = q.toString();
  return `/api/realms/${encodeURIComponent(contexto.realm)}/estudio/${resto}${s ? `?${s}` : ""}`;
}

export async function lerLoja(arquivo: string): Promise<Loja> {
  const r = await api<any>(rotaDaLoja(`loja/${arquivo}`));
  return { ...r, ofertas: (r.ofertas as unknown[][]).map(linhaParaOferta), conferencias: r.conferencias ?? {} };
}

export interface ImagemDeAba { caminho: string; existe: boolean; medida: { largura: number; altura: number } | null }
export interface TemaDasAbas {
  tema: number;
  dialogo: string;
  botoes_de_aba: number;
  botoes_de_subaba: number;
  imagem_compartilhada: boolean;
  abas: { indice: number; texto_do_dialogo: string; imagens: { normal?: ImagemDeAba; selecionada?: ImagemDeAba } }[];
}
export const lerAbas = (arquivo: string) => api<{ arquivo: string; temas: TemaDasAbas[] }>(rotaDaLoja(`loja/${arquivo}/abas`));

export const lerOferta = (arquivo: string, i: number) =>
  api<{ indice: number; campos: Record<string, number | string>; item: { nome: string; tabela: string; indice: number } | null }>(rotaDaLoja(`loja/${arquivo}/${i}`));

export const gravarOferta = (arquivo: string, i: number, campos: Record<string, number | string>) =>
  api<{ igual_ao_publicado?: boolean }>(rotaDaLoja(`loja/${arquivo}/${i}`), { method: "PUT", body: JSON.stringify({ campos }) });

export const novaOferta = (arquivo: string, base: number, campos: Record<string, number | string>) =>
  api<{ resultado: { indice: number } }>(rotaDaLoja(`loja/${arquivo}`), { method: "POST", body: JSON.stringify({ base, campos }) });

export const removerOferta = (arquivo: string, i: number) =>
  api<{ igual_ao_publicado?: boolean }>(rotaDaLoja(`loja/${arquivo}/${i}`), { method: "DELETE" });

export const conferirOferta = (arquivo: string, i: number, campos: Record<string, number | string>) =>
  api<Conferencia>(rotaDaLoja(`loja/${arquivo}/${i}/conferir`), { method: "POST", body: JSON.stringify({ campos }) });

export const mudarCategoria = (arquivo: string, pedido: { acao: "renomear" | "nova" | "remover"; categoria: number; sub?: number; nome?: string }) =>
  api(rotaDaLoja(`loja/${arquivo}/categorias`), { method: "POST", body: JSON.stringify(pedido) });

async function enviarPng(caminho: string, png: Blob) {
  const { sessao } = await import("./api");
  const s = await sessao();
  const r = await fetch(caminho, {
    method: "POST", credentials: "same-origin", body: png,
    headers: { "Content-Type": "application/octet-stream", ...(s?.csrf ? { "X-CSRF-Token": s.csrf } : {}) },
  });
  const d = await r.json().catch(() => null);
  if (!r.ok) throw new Error(d?.mensagem || d?.detail || `HTTP ${r.status}`);
  return d;
}

export const trocarImagemDaAba = (arquivo: string, aba: number, tema: number, estado: "normal" | "selecionada", png: Blob) =>
  enviarPng(rotaDaLoja(`loja/${arquivo}/abas/${aba}/imagem`, { tema, estado }), png);

export const trocarImagemDaOferta = (arquivo: string, i: number, png: Blob) => enviarPng(rotaDaLoja(`loja/${arquivo}/${i}/imagem`), png);

/** Imagem de interface do cliente em PNG; `versao` muda depois de uma troca (o painel guarda PNG em cache por um dia). */
export const urlDaImagem = (caminho: string, versao = 0) => rotaDaLoja("imagem", { caminho, v: versao });

/* ---- números como o cliente mostra ---- */

/** Preço em centavos → texto em gold, como `GetCashText`: "12" ou "12,50". */
export function precoEmGold(centavos: number): string {
  const g = Math.floor(centavos / 100), c = centavos % 100;
  return c === 0 ? `${g.toLocaleString("pt-BR")}` : `${g.toLocaleString("pt-BR")},${String(c).padStart(2, "0")}`;
}

/** Texto em gold ("12,5" ou "12,50") → centavos; `null` se não é número. */
export function goldEmCentavos(texto: string): number | null {
  const t = texto.trim().replace(/\./g, "").replace(",", ".");
  if (!/^\d+(\.\d{1,2})?$/.test(t)) return null;
  return Math.round(Number(t) * 100);
}

export interface Selo { valor: number; nome: string; curto: string; cor: "novo" | "promo" | "hot" | "desconto" | "relampago" | "vip" | "" }

export function selo(status: number): Selo {
  if (status === 1) return { valor: 1, nome: "Novo", curto: "NOVO", cor: "novo" };
  if (status === 2) return { valor: 2, nome: "Promoção", curto: "PROMO", cor: "promo" };
  if (status === 3) return { valor: 3, nome: "Recomendado", curto: "HOT", cor: "hot" };
  if (status >= 4 && status <= 12) {
    const paga = status - 3;
    return { valor: status, nome: `Desconto: paga ${paga * 10}% (${100 - paga * 10}% off)`, curto: `-${100 - paga * 10}%`, cor: "desconto" };
  }
  if (status === 13) return { valor: 13, nome: "Relâmpago", curto: "RELÂMPAGO", cor: "relampago" };
  if (status >= 14 && status <= 19) return { valor: status, nome: `VIP ${status - 13}`, curto: `VIP ${status - 13}`, cor: "vip" };
  return { valor: status, nome: status === 0 ? "Nenhum" : `Desconhecido (${status})`, curto: "", cor: "" };
}

export const SELOS: Selo[] = Array.from({ length: 20 }, (_, i) => selo(i));

/** Preço cheio riscado do desconto (`GetOriginalPrice`, sem o arredondamento opcional da interface). */
export function precoCheio(centavos: number, status: number): number | null {
  if (status < 4 || status > 12) return null;
  return Math.round((centavos * 10) / (status - 3));
}

/* ---- tempo ---- */

/** Validade em segundos → partes (0 = permanente). */
export function validadeEmPartes(segundos: number): { dias: number; horas: number; minutos: number } {
  return { dias: Math.floor(segundos / 86400), horas: Math.floor((segundos % 86400) / 3600), minutos: Math.floor((segundos % 3600) / 60) };
}

export function validadeEmTexto(segundos: number): string {
  if (!segundos) return "permanente";
  const p = validadeEmPartes(segundos);
  return [p.dias && `${p.dias} d`, p.horas && `${p.horas} h`, p.minutos && `${p.minutos} min`].filter(Boolean).join(" ") || `${segundos} s`;
}

/** time_t (GMT, como o editor original grava) → valor de `<input type="datetime-local">` em UTC; 0 → "". */
export function dataParaCampo(t: number): string {
  if (!t) return "";
  return new Date(t * 1000).toISOString().slice(0, 16);
}

/** Valor de `<input type="datetime-local">` lido como UTC → time_t; "" → 0. */
export function campoParaData(v: string): number {
  if (!v) return 0;
  const ms = Date.parse(`${v}:00Z`);
  return Number.isFinite(ms) ? Math.floor(ms / 1000) : 0;
}

export const DIAS_DA_SEMANA = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"];

export function bitsParaDias(bits: number, mensal: boolean): number[] {
  const n = mensal ? 31 : 7;
  return Array.from({ length: n }, (_, k) => k).filter((k) => (bits >>> k) & 1);
}

export function diasParaBits(dias: number[]): number {
  return dias.reduce((b, k) => (b | (1 << k)) >>> 0, 0);
}

/* ---- vitrine ---- */

export interface FiltroDaVitrine {
  aba: number | null;
  /** -1 = todas as subabas (o "todos" da aba do cliente, `Btn_SubType-1`). */
  sub: number;
  busca: string;
  selo: number | null;
  soVip: boolean;
  soBrinde: boolean;
  desativadas: boolean;
  soComProblema: boolean;
  /** `cliente`: a ordem da vitrine do jogo; `arquivo`: a do goods_index. */
  ordem: "cliente" | "arquivo";
}

export const FILTRO_INICIAL: FiltroDaVitrine = { aba: 0, sub: -1, busca: "", selo: null, soVip: false, soBrinde: false, desativadas: true, soComProblema: false, ordem: "cliente" };

/** A vitrine na ordem do arquivo (o índice é o `goods_index` da compra), com os filtros aplicados. A busca casa nome,
    nome do item, id do item ou o índice (`#12`). */
export function filtrarVitrine(ofertas: Oferta[], f: FiltroDaVitrine, conferencias: Record<string, Conferencia> = {}): Oferta[] {
  const termo = f.busca.trim().toLowerCase();
  const indiceBuscado = /^#\d+$/.test(termo) ? Number(termo.slice(1)) : null;
  return ofertas
    .filter((o) => {
      if (f.aba !== null && o.aba !== f.aba) return false;
      if (f.aba !== null && f.sub >= 0 && o.sub !== f.sub) return false;
      if (indiceBuscado !== null) { if (o.indice !== indiceBuscado) return false; }
      else if (termo && !(o.nome.toLowerCase().includes(termo) || (o.item ?? "").toLowerCase().includes(termo) || String(o.id) === termo)) return false;
      if (f.selo !== null && (f.selo === 4 ? o.selo < 4 || o.selo > 12 : f.selo === 14 ? o.selo < 14 || o.selo > 19 : o.selo !== f.selo)) return false;
      if (f.soVip && !(o.vip && o.vip > 0)) return false;
      if (f.soBrinde && !(o.brinde && o.brinde > 0)) return false;
      if (!f.desativadas && o.mostrado === 0) return false;
      if (f.soComProblema && !(conferencias[String(o.indice)]?.erros.length)) return false;
      return true;
    })
    .sort((a, b) => (f.ordem === "cliente" ? grupoNaVitrine(a) - grupoNaVitrine(b) : 0) || a.indice - b.indice);
}

/** Grupo da oferta na vitrine do cliente 1.5.5 (`GShopCollector`, `DlgQShop.cpp:170-232`): pelo selo da 1ª opção com
    preço — relâmpago (13), recomendado (3), promoção (2), novo (1), o resto; dentro do grupo, a ordem do arquivo. A
    desativada (sem preço) o cliente não mostra: vai para o fim. */
export function grupoNaVitrine(o: Oferta): number {
  if (o.mostrado === 0) return 5;
  return ({ 13: 0, 3: 1, 2: 2, 1: 3 } as Record<number, number>)[o.selo] ?? 4;
}

/** Quantas ofertas há em cada aba e subaba (para a árvore). */
export function contarPorAba(ofertas: Oferta[]): Map<string, number> {
  const m = new Map<string, number>();
  for (const o of ofertas) {
    m.set(`${o.aba}`, (m.get(`${o.aba}`) ?? 0) + 1);
    m.set(`${o.aba}/${o.sub}`, (m.get(`${o.aba}/${o.sub}`) ?? 0) + 1);
  }
  return m;
}

/** Texto em UTF-16 que cabe num `wchar[n]` (com o NUL): `szName` 32, `desc` 512, nome de aba 64. */
export function cabeEmWchar(texto: string, n: number): boolean {
  let u = 0;
  for (const c of texto) u += c.codePointAt(0)! > 0xffff ? 2 : 1;
  return u + 1 <= n;
}

/** Campos da oferta por versão: o que o layout não tem aparece desligado com a explicação. */
export function camposDaVersao(formato: string) {
  const v126 = formato === "V126";
  return {
    v126,
    periodo: !v126,
    seloPorOpcao: !v126,
    vip: formato === "V155Vip",
    brinde: !v126,
    donos: !v126,
    limite: formato === "V155Vip",
    explicacao126: "O gshop.data do 1.2.6 tem buy de 12 bytes (preço, data, validade) e um selo só para a oferta (status@196, load_malldata do gs 1.2.6): período, grupo, VIP, brinde, NPCs donos e limite não existem nesse layout.",
  };
}
