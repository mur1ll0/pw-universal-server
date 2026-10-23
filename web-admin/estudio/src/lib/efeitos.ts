/* B368: todos os efeitos que uma peça da família pode ter, pela mesma regra da edição de personagem.

   O painel do personagem busca efeitos em `GET /api/realms/{r}/efeitos?item=&busca=` (B209/B214): o GS do realm diz,
   para cada id, se ele serve na família da peça (`validar_efeito`, `crates/pw-gs/src/bus_server/item_editado.rs:47`)
   pela evidência de `TabelaDeAddons::ligar_familias` (`crates/pw-data-loader/src/addons.rs:306-338`: 1. a família
   da classe no fonte; 2. as famílias cujas essências sorteiam o id em `addons`/`rands`/`uniques`; 3. a união das
   do mesmo tratador; 4. sem evidência, todas) e se entra na busca (fora: refino, pedra, temporário).
   A busca por texto de lá só devolve 30; aqui pedimos **todos** os ids do `EQUIPMENT_ADDON` (do rascunho) com
   `ids=` em lotes, e o resto se filtra na tela. Ids acima de 8191 não cabem no item (13 bits, `item_editado.rs:18`).
   Vale o `elements` publicado no GS: efeito novo só do rascunho não tem família nem texto até publicar. */
import { api, contexto, listar } from "./api";

export interface EfeitoDisponivel {
  id: number;
  nome: string;
  texto: string;
  habilidade: string;
}

const cache = new Map<string, Promise<EfeitoDisponivel[]>>();

/** Até `n` promessas ao mesmo tempo. */
async function emParalelo<T>(tarefas: (() => Promise<T>)[], n = 4): Promise<T[]> {
  const r: T[] = new Array(tarefas.length);
  let proxima = 0;
  await Promise.all(Array.from({ length: Math.min(n, tarefas.length) }, async () => {
    while (proxima < tarefas.length) { const k = proxima++; r[k] = await tarefas[k](); }
  }));
  return r;
}

/** Efeitos que servem na família de `tabela` (a peça de referência é o 1º registro dela, que está publicado). */
export function efeitosDaFamilia(tabela: string): Promise<EfeitoDisponivel[]> {
  const chave = `${contexto.realm}/${contexto.rascunho ?? 0}/${tabela}`;
  if (!cache.has(chave)) {
    const p = carregar(tabela);
    p.catch(() => cache.delete(chave));
    cache.set(chave, p);
  }
  return cache.get(chave)!;
}

async function carregar(tabela: string): Promise<EfeitoDisponivel[]> {
  const referencia = (await listar(tabela, "", 0, 1)).registros[0]?.id;
  if (!referencia) throw new Error(`${tabela} sem registro para servir de referência`);
  // Todos os ids do EQUIPMENT_ADDON (200 por página).
  const primeira = await listar("EQUIPMENT_ADDON", "", 0, 200);
  const paginas = Math.ceil(primeira.total / 200);
  const resto = await emParalelo(Array.from({ length: Math.max(0, paginas - 1) }, (_, k) => () => listar("EQUIPMENT_ADDON", "", k + 1, 200)));
  const nomes = new Map<number, string>();
  for (const p of [primeira, ...resto]) for (const r of p.registros) if (r.id && r.id >= 1 && r.id <= 8191) nomes.set(r.id, r.nome);
  const ids = [...nomes.keys()].sort((a, b) => a - b);
  // O que o GS diz de cada id nesta família (180 ids por pedido: o `ids` da rota vai até 1200 caracteres).
  const lotes = await emParalelo(Array.from({ length: Math.ceil(ids.length / 180) }, (_, k) => () =>
    api<{ efeitos: { id: number; recusa: string | null; busca?: boolean; texto?: string; habilidade?: { nome?: string } | null }[] }>(
      `/api/realms/${encodeURIComponent(contexto.realm)}/efeitos?${new URLSearchParams({ item: String(referencia), ids: ids.slice(k * 180, k * 180 + 180).join(",") })}`)));
  const r: EfeitoDisponivel[] = [];
  for (const l of lotes) for (const e of l.efeitos) {
    if (e.recusa || e.busca === false) continue;
    r.push({ id: e.id, nome: nomes.get(e.id) ?? "", texto: e.texto ?? "", habilidade: e.habilidade?.nome ?? "" });
  }
  return r.sort((a, b) => a.id - b.id);
}

/** Filtro da tela: id exato, ou texto do cliente / nome da habilidade / nome interno contendo a busca. */
export function filtrarEfeitos(lista: EfeitoDisponivel[], busca: string): EfeitoDisponivel[] {
  const alvo = busca.trim().toLowerCase();
  if (!alvo) return lista;
  if (/^\d+$/.test(alvo)) {
    const exato = lista.filter((e) => String(e.id) === alvo);
    return exato.length ? exato : lista.filter((e) => String(e.id).includes(alvo));
  }
  return lista.filter((e) => e.texto.toLowerCase().includes(alvo) || e.habilidade.toLowerCase().includes(alvo) || e.nome.toLowerCase().includes(alvo));
}
