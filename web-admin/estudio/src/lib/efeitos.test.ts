import { describe, expect, it } from "vitest";
import { filtrarEfeitos, type EfeitoDisponivel } from "./efeitos";

const lista: EfeitoDisponivel[] = [
  { id: 1286, nome: "W_SK01", texto: "Mordaz: Aumenta seu poder de ataque físico.", habilidade: "Garra" },
  { id: 753, nome: "A_MD00", texto: "Ataque físico +2", habilidade: "" },
  { id: 12, nome: "A_C12", texto: "Crítico +1%", habilidade: "" },
];

describe("filtrarEfeitos", () => {
  it("vazio devolve todos", () => expect(filtrarEfeitos(lista, " ")).toHaveLength(3));
  it("pelo texto do cliente, sem caixa", () => expect(filtrarEfeitos(lista, "mordaz").map((e) => e.id)).toEqual([1286]));
  it("pelo nome da habilidade", () => expect(filtrarEfeitos(lista, "garra").map((e) => e.id)).toEqual([1286]));
  it("pelo nome interno", () => expect(filtrarEfeitos(lista, "a_md").map((e) => e.id)).toEqual([753]));
  it("id exato antes de parte do id", () => expect(filtrarEfeitos(lista, "12").map((e) => e.id)).toEqual([12]));
  it("parte do id quando não há exato", () => expect(filtrarEfeitos(lista, "28").map((e) => e.id)).toEqual([1286]));
  it("texto que aparece em vários", () => expect(filtrarEfeitos(lista, "ataque").map((e) => e.id)).toEqual([1286, 753]));
});
