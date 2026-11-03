import { describe, expect, it } from "vitest";
import { acharLista, acrescentar, ajustar, gravarLista, lerLista, normalizar, paraGravar, porcentagens, remover, soma } from "./chances";

const perto = (a: number[], b: number[]) => a.forEach((x, k) => expect(x).toBeCloseTo(b[k], 6));

describe("normalizar (NormalizeRandom do servidor)", () => {
  it("divide pela soma", () => perto(normalizar([2, 1, 1]), [0.5, 0.25, 0.25]));
  it("soma zero vira [1, 0, …]", () => expect(normalizar([0, 0, 0])).toEqual([1, 0, 0]));
  it("abaixo do limiar de 0,00001 também", () => expect(normalizar([0.000001, 0])).toEqual([1, 0]));
  it("porcentagens somam 100", () => expect(soma(porcentagens([3, 7, 0.5]))).toBeCloseTo(100, 9));
});

describe("ajustar", () => {
  it("as outras se repartem na proporção que tinham", () => {
    const r = ajustar([0.5, 0.25, 0.25], 0, 0.8);
    perto(r, [0.8, 0.1, 0.1]);
    expect(soma(r)).toBeCloseTo(1, 12);
  });
  it("outras zeradas recebem partes iguais", () => perto(ajustar([1, 0, 0], 0, 0.4), [0.4, 0.3, 0.3]));
  it("linha única fica em 100%", () => expect(ajustar([0.3], 0, 0.1)).toEqual([1]));
  it("valor fora de 0–1 é limitado", () => perto(ajustar([0.5, 0.5], 1, 1.7), [0, 1]));
  it("muitos ajustes seguidos não perdem a soma", () => {
    let p = [0.2, 0.2, 0.2, 0.2, 0.2];
    for (let k = 0; k < 200; k++) p = ajustar(p, k % 5, ((k * 37) % 100) / 100);
    expect(soma(p)).toBeCloseTo(1, 9);
  });
});

describe("acrescentar e remover", () => {
  it("linha nova entra com a parte igual", () => perto(acrescentar([0.5, 0.5]), [1 / 3, 1 / 3, 1 / 3]));
  it("primeira linha fica com tudo", () => expect(acrescentar([])).toEqual([1]));
  it("remover devolve o peso às outras", () => perto(remover([0.5, 0.3, 0.2], 0), [0.6, 0.4]));
});

describe("paraGravar (float de 32 bits)", () => {
  it("cada valor é representável em float e a soma dá 1", () => {
    const p = paraGravar([1, 1, 1]);
    p.forEach((x) => expect(Math.fround(x)).toBe(x));
    expect(Math.abs(soma(p) - 1)).toBeLessThan(1e-7);
  });
  it("a sobra vai para a última linha com peso", () => {
    const p = paraGravar([1, 1, 1, 0]);
    expect(p[3]).toBe(0);
    expect(p[2]).toBe(Math.fround(1 - (p[0] + p[1])));
  });
});

describe("listas do layout", () => {
  const nomes = ["ID", "addons_2_id_addon", "addons_2_probability_addon", "addons_1_id_addon", "addons_1_probability_addon",
    "addons_10_id_addon", "addons_10_probability_addon"];
  const lista = acharLista(nomes, "addons", "id_addon", "probability_addon")!;
  it("ordena pelo número (começa em 1)", () => expect(lista.ids).toEqual(["addons_1_id_addon", "addons_2_id_addon", "addons_10_id_addon"]));
  it("lê só as linhas com id", () => {
    const v: Record<string, number> = { addons_1_id_addon: 0, addons_2_id_addon: 7, addons_2_probability_addon: 0.5, addons_10_id_addon: 9, addons_10_probability_addon: 0.5 };
    expect(lerLista(lista, (n) => v[n] ?? 0)).toEqual([{ id: 7, peso: 0.5 }, { id: 9, peso: 0.5 }]);
  });
  it("grava compactado, com zeros no fim", () => {
    const c = gravarLista(lista, [{ id: 7, peso: 3 }, { id: 9, peso: 1 }]);
    expect(c).toEqual({ addons_1_id_addon: 7, addons_1_probability_addon: 0.75, addons_2_id_addon: 9, addons_2_probability_addon: 0.25,
      addons_10_id_addon: 0, addons_10_probability_addon: 0 });
  });
  it("recusa mais linhas que o layout tem", () => expect(() => gravarLista(lista, [1, 2, 3, 4].map((id) => ({ id, peso: 1 })))).toThrow());
});
