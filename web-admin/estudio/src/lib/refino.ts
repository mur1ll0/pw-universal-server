/* Efeito do refino (`levelup_addon`) explicado (B383). O servidor diz o que cada efeito de refino soma e o valor de +1
   a +12 (`GET estudio/refinos` → `Estudio::refinos`): o GS grava `(int)(param1 × refine_factor[nível] + 0.1)`
   (`equip_item.cpp:208-223, 319, 330`) e o valor do nível substitui o anterior. O que cada tratador soma está em
   `pw_data_loader::refino::EFEITOS_DE_REFINO` (`item_addon.cpp:1505-1511`). */
import { api, contexto } from "./api";

export type Familia = "arma" | "armadura" | "acessorio";

export interface EfeitoDeRefino {
  id: number;
  indice: number;
  nome: string;
  tratador: string;
  param: number;
  soma: string[];
  evidencia: string;
  /** Valor em +1 … +12. */
  niveis: number[];
  usado_por: Record<Familia, number>;
}
export interface Refinos { fator: number[]; efeitos: EfeitoDeRefino[] }

const cache = new Map<string, Promise<Refinos>>();
/** Os efeitos de refino do rascunho (uma vez por realm e rascunho). */
export function refinos(): Promise<Refinos> {
  const chave = `${contexto.realm}|${contexto.rascunho ?? ""}`;
  let p = cache.get(chave);
  if (!p) {
    const q = contexto.rascunho ? `?rascunho=${contexto.rascunho}` : "";
    p = api<Refinos>(`/api/realms/${encodeURIComponent(contexto.realm)}/estudio/refinos${q}`);
    p.catch(() => cache.delete(chave));
    cache.set(chave, p);
  }
  return p;
}

/** A família pela tabela (`get_item_refine_addon` só lê arma, armadura e acessório, `itemdataman.cpp:205-228`). */
export function familiaDaTabela(tabela: string): Familia | null {
  return tabela === "WEAPON_ESSENCE" ? "arma" : tabela === "ARMOR_ESSENCE" ? "armadura" : tabela === "DECORATION_ESSENCE" ? "acessorio" : null;
}
export const NOME_DA_FAMILIA: Record<Familia, string> = { arma: "armas", armadura: "armaduras", acessorio: "acessórios" };

/** "ataque mágico e ataque físico" */
export const oQueSoma = (e: Pick<EfeitoDeRefino, "soma">) => e.soma.join(" e ");

/** "Refino: +7 de ataque físico no +1; em +12 soma 262". O crescimento não é linear (fator do servidor). */
export function descreverRefino(e: Pick<EfeitoDeRefino, "soma" | "niveis">): string {
  const um = e.niveis[0] ?? 0, doze = e.niveis[11] ?? 0;
  return `Refino: +${um} de ${oQueSoma(e)} no +1; em +12 soma ${doze}`;
}

/** As famílias que usam o efeito no rascunho, ex.: "usado por 120 armas". */
export function usoDoRefino(e: Pick<EfeitoDeRefino, "usado_por">): string {
  const partes = (Object.keys(NOME_DA_FAMILIA) as Familia[]).filter((f) => e.usado_por[f]).map((f) => `${e.usado_por[f]} ${NOME_DA_FAMILIA[f]}`);
  return partes.length ? `usado por ${partes.join(", ")}` : "nenhuma peça usa";
}

/** Serve na família: alguma peça da família já usa (o GS não restringe; a regra que vale é o uso do jogo). */
export const serveNaFamilia = (e: Pick<EfeitoDeRefino, "usado_por">, f: Familia | null) => !f || e.usado_por[f] > 0;

/** Filtro e ordem do seletor: os da família primeiro, depois por tipo e parâmetro. */
export function filtrarRefinos(lista: EfeitoDeRefino[], busca: string, familia: Familia | null, soDaFamilia: boolean): EfeitoDeRefino[] {
  const b = busca.trim().toLowerCase();
  return lista
    .filter((e) => !soDaFamilia || serveNaFamilia(e, familia))
    .filter((e) => !b || String(e.id) === b || e.nome.toLowerCase().includes(b) || oQueSoma(e).toLowerCase().includes(b) || e.tratador.includes(b))
    .sort((x, y) => Number(serveNaFamilia(y, familia)) - Number(serveNaFamilia(x, familia))
      || x.tratador.localeCompare(y.tratador) || x.param - y.param || x.id - y.id);
}
