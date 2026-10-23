/* Estado do Estúdio e rota por hash (B366). A rota leva o realm e o rascunho, para uma aba nova (ex.: "Criar item")
   abrir no mesmo contexto: `#/<tela>?realm=<id>&rascunho=<n>`. */
import { contexto } from "./api";

export interface Rota {
  caminho: string[];
  params: URLSearchParams;
}

function lerRota(): Rota {
  const h = location.hash.replace(/^#\/?/, "");
  const [c, q = ""] = h.split("?");
  return { caminho: c.split("/").filter(Boolean), params: new URLSearchParams(q) };
}

export const rota = $state<Rota>(lerRota());

window.addEventListener("hashchange", () => {
  const r = lerRota();
  rota.caminho = r.caminho;
  rota.params = r.params;
});

export const estudio = $state({
  realm: "",
  versao: "",
  rascunho: null as number | null,
  usuario: "",
  erro: "",
});

/** Vai para uma tela mantendo realm e rascunho. */
export function ir(caminho: string, extra: Record<string, string | number> = {}) {
  location.hash = `#/${caminho}?${parametros(extra)}`;
}

export function parametros(extra: Record<string, string | number> = {}) {
  const q = new URLSearchParams();
  if (estudio.realm) q.set("realm", estudio.realm);
  if (estudio.rascunho) q.set("rascunho", String(estudio.rascunho));
  for (const [k, v] of Object.entries(extra)) q.set(k, String(v));
  return q.toString();
}

export function definirRascunho(id: number | null) {
  estudio.rascunho = id;
  contexto.rascunho = id;
  const r = lerRota();
  if (id) r.params.set("rascunho", String(id));
  else r.params.delete("rascunho");
  history.replaceState(null, "", `#/${r.caminho.join("/")}?${r.params}`);
}

/* ---- avisos (toast) ---- */
export const avisos = $state<{ id: number; tipo: "ok" | "erro" | "aviso"; texto: string }[]>([]);
let proximo = 1;
export function avisar(tipo: "ok" | "erro" | "aviso", texto: string) {
  const id = proximo++;
  avisos.push({ id, tipo, texto });
  setTimeout(() => {
    const i = avisos.findIndex((a) => a.id === id);
    if (i >= 0) avisos.splice(i, 1);
  }, tipo === "erro" ? 9000 : 4500);
}

/* ---- canal entre abas: o item criado na aba nova volta para a de origem ---- */
export const NOME_DO_CANAL = "pw-estudio";
export interface MensagemDeItem {
  tipo: "item-criado";
  retorno: string;
  id: number;
  nome: string;
}
