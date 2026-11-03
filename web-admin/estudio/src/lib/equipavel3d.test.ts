import { describe, expect, it } from "vitest";
import { caminhosDaPele, camposDeModelo, ehPeleNoCorpo, ganchosExigidos, pastaSugerida, pelesDoCorpo } from "./equipavel3d";

describe("equipável no 3D", () => {
  it("campos de modelo pela tabela", () => {
    const arma = new Set(["file_model_right", "file_model_left", "file_matter"]);
    expect(camposDeModelo("WEAPON_ESSENCE", (c) => arma.has(c)).map((c) => c.campo)).toEqual(["file_model_right", "file_model_left"]);
    const voo = new Set(["file_model", "file_model2"]);
    expect(camposDeModelo("FLYSWORD_ESSENCE", (c) => voo.has(c)).map((c) => c.campo)).toEqual(["file_model", "file_model2"]);
  });
  it("ganchos do cliente", () => {
    expect(ganchosExigidos("WEAPON_ESSENCE", "file_model_right", 0)).toEqual(["CC_weapon", "CC_qiu"]);
    expect(ganchosExigidos("FLYSWORD_ESSENCE", "file_model", 1)).toEqual(["CC_chibang"]);
    expect(ganchosExigidos("FLYSWORD_ESSENCE", "file_model", 2)).toEqual(["CC_feijian"]);
    expect(ganchosExigidos("WINGMANWING_ESSENCE", "file_model", 0)).toEqual(["CC_chibang"]);
    expect(ganchosExigidos("DECORATION_ESSENCE", "file_model", 0)).toEqual([]);
    expect(pastaSugerida("WINGMANWING_ESSENCE")).toContain("通用装备");
  });
  it("pele no corpo: caminhos do binário", () => {
    expect(ehPeleNoCorpo("ARMOR_ESSENCE") && ehPeleNoCorpo("FASHION_ESSENCE") && !ehPeleNoCorpo("WEAPON_ESSENCE")).toBe(true);
    expect(caminhosDaPele("紫金胸甲", 0, "男通用")[0]).toBe("Models\\Players\\装备\\男\\紫金胸甲\\男通用紫金胸甲一级.ski");
    expect(caminhosDaPele("精灵裙手套", 1, "妖精")[2]).toBe("Models\\Players\\装备\\女\\精灵裙手套\\妖精精灵裙手套三级.ski");
    expect(pelesDoCorpo("武侠男")).toEqual(["Models\\Players\\形象\\武侠男\\躯干\\武侠男.ski", "Models\\Players\\形象\\武侠男\\头\\武侠男头.ski"]);
  });
});
