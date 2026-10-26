/* B373: prévia do item ao passar o mouse — a mesma dica da tela de personagem (`painel/dica.py`), de um item gerado
   agora pelo GS como se tivesse caído (`GET /api/realms/{r}/itens/{id}/exemplo`; o GS não grava nada). Cada linha traz
   os códigos de cor do cliente (`^RRGGBB`). */
import { api, buscarNoEspaco, contexto, linkDosEditores } from "./api";
import { categoriaDaTabela, subcategoriaPorTabela } from "./catalogo";
import { parametros } from "./estado.svelte";

/* B375: a dica fica presa ao ícone, logo acima dele (`base` = distância do pé da janela até o topo do ícone). */
export const dica = $state<{ visivel: boolean; x: number; base: number; linhas: string[] }>({ visivel: false, x: 0, base: 0, linhas: [] });

const cache = new Map<string, Promise<string[]>>();
let pedido = 0;

function linhasDe(id: number): Promise<string[]> {
  const chave = `${contexto.realm}:${id}`;
  if (!cache.has(chave)) {
    const p = api<{ linhas: string[] }>(`/api/realms/${encodeURIComponent(contexto.realm)}/itens/${id}/exemplo`).then((r) => r.linhas);
    p.catch(() => cache.delete(chave));
    cache.set(chave, p);
  }
  return cache.get(chave)!;
}

export async function mostrarDica(id: number, nome: string, alvo: HTMLElement) {
  const meu = ++pedido;
  const r = alvo.getBoundingClientRect();
  const x = Math.max(4, Math.min(r.left, window.innerWidth - 348));
  Object.assign(dica, { visivel: true, x, base: window.innerHeight - r.top + 6, linhas: [`^ffffff${nome || `Item ${id}`}`, "^b0b0b0…"] });
  try {
    const l = await linhasDe(id);
    if (meu === pedido) dica.linhas = l;
  } catch (e) {
    if (meu === pedido) dica.linhas = [`^ffffff${nome || `Item ${id}`}`, `^ff6060${(e as Error).message}`];
  }
}

export function esconderDica() {
  pedido++;
  dica.visivel = false;
}

/** Partes coloridas de uma linha da dica. */
export function partes(linha: string): { cor: string; texto: string }[] {
  const r: { cor: string; texto: string }[] = [];
  let cor = "ffffff";
  for (const p of linha.split(/(\^[0-9a-fA-F]{6})/)) {
    if (/^\^[0-9a-fA-F]{6}$/.test(p)) { cor = p.slice(1); continue; }
    if (p) r.push({ cor, texto: p });
  }
  return r;
}

/** Abre o item numa aba nova: na edição do Estúdio quando a tabela tem tela própria; senão, nos Editores. */
export async function abrirItemEmNovaAba(id: number) {
  const aba = window.open("", "_blank");
  try {
    const r = (await buscarNoEspaco("ESSENCE", String(id), 10)).find((x) => x.id === id && x.tabela);
    if (!r) throw new Error(`item ${id} não encontrado`);
    const sub = subcategoriaPorTabela(r.tabela!);
    const cat = categoriaDaTabela(r.tabela!) ?? "equipaveis";
    const url = sub?.pronta ? `${location.pathname}#/${cat}/${r.tabela}/${r.indice}?${parametros()}` : linkDosEditores(r.tabela!, r.indice);
    if (aba) aba.location.href = url; else window.open(url, "_blank");
  } catch (e) {
    aba?.close();
    throw e;
  }
}
