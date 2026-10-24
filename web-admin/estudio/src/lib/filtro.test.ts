import { describe, expect, it } from "vitest";
import { parametrosDoFiltro } from "./api";
import { COLUNAS_DO_SELETOR, tabelaComColunas } from "./catalogo";

describe("B376: filtro da listagem de tabela", () => {
  it("monta colunas, ordem e faixas (lado vazio = sem limite)", () => {
    expect(parametrosDoFiltro({
      colunas: ["require_level", "level"], ordem: "-level",
      faixas: { require_level: { min: 30, max: 40 }, level: { min: null, max: 5 }, id_sub_type: { min: null, max: null } },
    })).toEqual({ colunas: "require_level,level", ordem: "-level", faixas: "require_level:30:40,level::5" });
  });
  it("sem nada pedido, não manda parâmetro", () => {
    expect(parametrosDoFiltro({})).toEqual({});
  });
  it("colunas só nas tabelas das categorias e nas pedras", () => {
    expect(tabelaComColunas("WEAPON_ESSENCE")).toBe(true);
    expect(tabelaComColunas("STONE_ESSENCE")).toBe(true);
    expect(tabelaComColunas("MEDICINE_ESSENCE")).toBe(false);
    expect(COLUNAS_DO_SELETOR.map((c) => c.campo)).toContain("require_level");
  });
});
