import { describe, expect, it } from "vitest";
import { parametrosDoFiltro } from "./api";
import { COLUNAS_DO_SELETOR, TABELAS_DE_ITEM, tabelaComColunas } from "./catalogo";

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

describe("B377: só tabelas de item no seletor de item", () => {
  it("segue os case do generate_item (generate_item_temp.h)", () => {
    for (const t of ["WEAPON_ESSENCE", "MATERIAL_ESSENCE", "MEDICINE_ESSENCE", "PET_EGG_ESSENCE", "REVIVESCROLL_ESSENCE"]) expect(TABELAS_DE_ITEM.has(t)).toBe(true);
    for (const t of ["WEAPON_MAJOR_TYPE", "WEAPON_SUB_TYPE", "MONSTER_ESSENCE", "NPC_ESSENCE", "MINE_ESSENCE", "NPC_SELL_SERVICE", "SUITE_ESSENCE", "PET_ESSENCE"])
      expect(TABELAS_DE_ITEM.has(t)).toBe(false);
  });
});
