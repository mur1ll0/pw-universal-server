/* Chances que somam 100% (B366).

   Unidade e regra do original: cada chance é `float` de 0 a 1 (`addons[i].probability_addon`, `ExpTypes.h:177-280`).
   Ao carregar, o servidor tira as linhas de id 0 e compacta a lista (`eliminate_zero_item`,
   `cgame/gs/template/elementdataman.cpp:270-296`) e divide cada peso pela soma (`NormalizeRandom`, `:298-320`); soma
   ≈ 0 (≤ 0,00001) vira `[1, 0, …]`. O sorteio percorre a lista subtraindo (`RandSelect`, `itemdataman.h:33-46`).
   Então a soma exata não é exigida pelo servidor, mas a tela grava pesos que somam 1 — o que o jogo vai usar é o que
   se vê — e a sobra do arredondamento para `float` vai para a última linha com peso. */

export const LIMIAR_DE_SOMA = 0.00001;

/** `NormalizeRandom` do servidor: divide pela soma; soma ≈ 0 → o primeiro fica com tudo. */
export function normalizar(pesos: number[]): number[] {
  if (!pesos.length) return [];
  const soma = pesos.reduce((s, p) => s + Math.max(0, p), 0);
  if (soma > LIMIAR_DE_SOMA) return pesos.map((p) => Math.max(0, p) / soma);
  return pesos.map((_, i) => (i === 0 ? 1 : 0));
}

/** Porcentagens para mostrar (0–100), já normalizadas como o servidor faria. */
export function porcentagens(pesos: number[]): number[] {
  return normalizar(pesos).map((p) => p * 100);
}

/**
 * Muda a chance da linha `i` para `valor` (0–1) e reparte o resto entre as outras, na proporção que elas já tinham;
 * se as outras estão todas em 0, o resto vai igual para cada uma. Uma linha só fica sempre em 1.
 */
export function ajustar(pesos: number[], i: number, valor: number): number[] {
  const n = pesos.length;
  if (n === 0) return [];
  if (n === 1) return [1];
  const v = Math.min(1, Math.max(0, valor));
  const atuais = normalizar(pesos);
  const resto = 1 - v;
  const outras = atuais.reduce((s, p, k) => (k === i ? s : s + p), 0);
  return atuais.map((p, k) => {
    if (k === i) return v;
    return outras > LIMIAR_DE_SOMA ? (p / outras) * resto : resto / (n - 1);
  });
}

/** Linha nova: entra com a parte igual (1/(n+1)) e as outras encolhem na mesma proporção. */
export function acrescentar(pesos: number[]): number[] {
  const n = pesos.length;
  if (n === 0) return [1];
  const parte = 1 / (n + 1);
  return [...normalizar(pesos).map((p) => p * (1 - parte)), parte];
}

/** Tira a linha `i` e devolve o peso dela às outras, proporcionalmente. */
export function remover(pesos: number[], i: number): number[] {
  const resto = pesos.filter((_, k) => k !== i);
  return resto.length ? normalizar(resto) : [];
}

/**
 * Valores para gravar em `float` (32 bits): cada um arredondado com `Math.fround`, e a última linha com peso recebe
 * o que falta para a soma (em dupla precisão, como o `double t` do `NormalizeRandom`) dar 1.
 */
export function paraGravar(pesos: number[]): number[] {
  const p = normalizar(pesos).map((x) => Math.fround(x));
  let ultima = -1;
  for (let k = p.length - 1; k >= 0; k--) if (p[k] > 0) { ultima = k; break; }
  if (ultima < 0) return p;
  const outras = p.reduce((s, x, k) => (k === ultima ? s : s + x), 0);
  p[ultima] = Math.fround(Math.max(0, 1 - outras));
  return p;
}

/** Soma em dupla precisão, como o servidor (`double t`). */
export const soma = (pesos: number[]) => pesos.reduce((s, p) => s + p, 0);

/* ---- listas de pares (id, chance) do layout: `addons_N_id_addon` + `addons_N_probability_addon` ---- */

export interface Linha {
  id: number;
  peso: number;
}

/** Cartão da tela: a linha com o nome do registro citado. */
export interface Cartao extends Linha {
  nome: string;
  /** texto do cliente com o valor do efeito ("Ataque físico +2"); vazio = sem texto */
  descricao?: string;
}

export interface Lista {
  /** campos de id e de chance, na ordem do arquivo (o número do layout começa em 1, B365) */
  ids: string[];
  chances: string[];
}

/** Acha a lista `prefixo_N_<id>` / `prefixo_N_<chance>` nos nomes dos campos, ordenada pelo N. */
export function acharLista(nomes: string[], prefixo: string, campoId: string, campoChance: string): Lista | null {
  const re = new RegExp(`^${prefixo}_(\\d+)_${campoId}$`);
  const numeros = nomes.map((n) => n.match(re)).filter((m): m is RegExpMatchArray => !!m).map((m) => Number(m[1]));
  numeros.sort((a, b) => a - b);
  const ids = numeros.map((k) => `${prefixo}_${k}_${campoId}`);
  const chances = numeros.map((k) => `${prefixo}_${k}_${campoChance}`);
  if (!ids.length || chances.some((c) => !nomes.includes(c))) return null;
  return { ids, chances };
}

/** As linhas com id ≠ 0, na ordem (o que sobra depois do `eliminate_zero_item`). */
export function lerLista(lista: Lista, valor: (nome: string) => number): Linha[] {
  const r: Linha[] = [];
  lista.ids.forEach((c, k) => {
    const id = valor(c);
    if (id) r.push({ id, peso: valor(lista.chances[k]) });
  });
  return r;
}

/** Campos a gravar: as linhas compactadas no começo, com as chances de `paraGravar`, e zeros no resto. */
export function gravarLista(lista: Lista, linhas: Linha[]): Record<string, number> {
  if (linhas.length > lista.ids.length) throw new Error(`no máximo ${lista.ids.length} linhas`);
  const pesos = paraGravar(linhas.map((l) => l.peso));
  const campos: Record<string, number> = {};
  lista.ids.forEach((c, k) => {
    campos[c] = k < linhas.length ? linhas[k].id : 0;
    campos[lista.chances[k]] = k < linhas.length ? pesos[k] : 0;
  });
  return campos;
}
