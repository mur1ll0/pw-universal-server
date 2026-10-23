import { describe, expect, it } from "vitest";
import { camposDaBox, rotuloGerado } from "./catalogo";

describe("camposDaBox", () => {
  const nomes = ["ID", "defence_low", "magic_defences_1_low", "magic_defences_1_high", "magic_defences_2_low", "price"];
  it("nome solto só se a tabela tiver", () =>
    expect(camposDaBox({ titulo: "", campos: ["price", "require_reputation"] }, nomes)).toEqual(["price"]));
  it("expressão na ordem do layout, sem repetir", () =>
    expect(camposDaBox({ titulo: "", campos: ["defence_low", "/^magic_defences_[0-9]+_(low|high)$/", "defence_low"] }, nomes))
      .toEqual(["defence_low", "magic_defences_1_low", "magic_defences_1_high", "magic_defences_2_low"]));
});

describe("rotuloGerado", () => {
  it("defesa mágica pelo elemento (金木水火土)", () => expect(rotuloGerado("magic_defences_4_high")).toBe("Defesa de Fogo (máx.)"));
  it("campo do ExpTypes", () => expect(rotuloGerado("mp_launch")).toBe("MP para levantar voo"));
  it("refino do voo", () => expect(rotuloGerado("refine_3_cost")).toBe("Refino 3: custo"));
  it("desconhecido", () => expect(rotuloGerado("xyz")).toBeNull());
});
