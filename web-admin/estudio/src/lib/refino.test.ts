import { describe, expect, it } from "vitest";
import { descreverRefino, familiaDaTabela, filtrarRefinos, serveNaFamilia, usoDoRefino, type EfeitoDeRefino } from "./refino";

// Valores de `refine_factor` (equip_item.cpp:208-223) com param 7: (int)(7 × f + 0.1).
const fator = [1, 2, 3.05, 4.3, 5.75, 7.55, 9.95, 13, 17.05, 22.3, 29, 37.5];
const niveis = (p: number) => fator.map((f) => Math.trunc(Math.fround(p * f) + 0.1));
const ef = (id: number, tratador: string, soma: string[], param: number, uso: [number, number, number]): EfeitoDeRefino => ({
  id, indice: id, nome: `R${id}`, tratador, param, soma, evidencia: "", niveis: niveis(param),
  usado_por: { arma: uso[0], armadura: uso[1], acessorio: uso[2] },
});
const lista = [
  ef(1712, "refine_damage", ["ataque físico"], 7, [40, 0, 0]),
  ef(1760, "refine_magic_damage", ["ataque mágico", "ataque físico"], 5, [12, 0, 0]),
  ef(1800, "refine_max_hp", ["HP máximo"], 30, [0, 25, 0]),
];

describe("refino", () => {
  it("+1 e +12 pelo fator", () => {
    expect(lista[0].niveis[0]).toBe(7);
    expect(lista[0].niveis[11]).toBe(262); // 7 × 37,5 = 262,5 → 262
    expect(descreverRefino(lista[0])).toBe("Refino: +7 de ataque físico no +1; em +12 soma 262");
  });
  it("dois atributos", () => expect(descreverRefino(lista[1])).toBe("Refino: +5 de ataque mágico e ataque físico no +1; em +12 soma 187"));
  it("família pela tabela", () => {
    expect(familiaDaTabela("WEAPON_ESSENCE")).toBe("arma");
    expect(familiaDaTabela("FASHION_ESSENCE")).toBeNull();
  });
  it("serve na família pelo uso", () => {
    expect(serveNaFamilia(lista[2], "armadura")).toBe(true);
    expect(serveNaFamilia(lista[2], "arma")).toBe(false);
    expect(usoDoRefino(lista[0])).toBe("usado por 40 armas");
  });
  it("os da família primeiro; só da família filtra", () => {
    expect(filtrarRefinos(lista, "", "armadura", false)[0].id).toBe(1800);
    expect(filtrarRefinos(lista, "", "arma", true).map((e) => e.id)).toEqual([1712, 1760]);
    expect(filtrarRefinos(lista, "hp", null, false).map((e) => e.id)).toEqual([1800]);
  });
});
