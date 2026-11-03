import { describe, expect, it } from "vitest";
import {
  FILTRO_INICIAL, bitsParaDias, cabeEmWchar, campoParaData, contarPorAba, dataParaCampo, diasParaBits, filtrarVitrine,
  goldEmCentavos, linhaParaOferta, precoCheio, precoEmGold, selo, validadeEmTexto, type Oferta,
} from "./gshop";

const o = (indice: number, aba: number, sub: number, extra: Partial<Oferta> = {}): Oferta => ({
  ...linhaParaOferta([indice, aba, sub, 1000 + indice, 1, `Oferta ${indice}`, 100, `Item ${indice}`, null, 0, 0, 0, 1, "", 100]),
  ...extra,
});

describe("vitrine da G-Shop", () => {
  const lista = [
    o(3, 0, 1), o(0, 0, 0, { selo: 1 }), o(1, 0, 1, { selo: 7, brinde: 5 }), o(2, 1, 0, { vip: 2, selo: 15 }),
    o(4, 0, 0, { preco: 0, opcoes: 0, mostrado: 0, nome: "Desligada" }),
  ];

  it("ordem do arquivo (goods_index) e filtro pela aba", () => {
    const f = { ...FILTRO_INICIAL, ordem: "arquivo" as const };
    expect(filtrarVitrine(lista, f).map((x) => x.indice)).toEqual([0, 1, 3, 4]);
    expect(filtrarVitrine(lista, { ...f, sub: 1 }).map((x) => x.indice)).toEqual([1, 3]);
    expect(filtrarVitrine(lista, { ...f, aba: null }).map((x) => x.indice)).toEqual([0, 1, 2, 3, 4]);
  });

  it("ordem da vitrine do cliente: relâmpago, recomendado, promoção, novo, resto; desativada no fim", () => {
    const l = [o(0, 0, 0), o(1, 0, 0, { selo: 1 }), o(2, 0, 0, { selo: 3 }), o(3, 0, 0, { mostrado: 0 }), o(4, 0, 0, { selo: 13 }), o(5, 0, 0, { selo: 2 }), o(6, 0, 0, { selo: 3 })];
    expect(filtrarVitrine(l, FILTRO_INICIAL).map((x) => x.indice)).toEqual([4, 2, 6, 5, 1, 0, 3]);
  });

  it("busca por nome, id do item e índice", () => {
    const f = { ...FILTRO_INICIAL, aba: null, ordem: "arquivo" as const };
    expect(filtrarVitrine(lista, { ...f, busca: "oferta 2" }).map((x) => x.indice)).toEqual([2]);
    expect(filtrarVitrine(lista, { ...f, busca: "1003" }).map((x) => x.indice)).toEqual([3]);
    expect(filtrarVitrine(lista, { ...f, busca: "#4" }).map((x) => x.indice)).toEqual([4]);
  });

  it("filtra selo (descontos e VIP em grupo), VIP, brinde, desativadas e problemas", () => {
    const f = { ...FILTRO_INICIAL, aba: null, ordem: "arquivo" as const };
    expect(filtrarVitrine(lista, { ...f, selo: 4 }).map((x) => x.indice)).toEqual([1]);
    expect(filtrarVitrine(lista, { ...f, selo: 14 }).map((x) => x.indice)).toEqual([2]);
    expect(filtrarVitrine(lista, { ...f, soVip: true }).map((x) => x.indice)).toEqual([2]);
    expect(filtrarVitrine(lista, { ...f, soBrinde: true }).map((x) => x.indice)).toEqual([1]);
    expect(filtrarVitrine(lista, { ...f, desativadas: false, ordem: "arquivo" }).map((x) => x.indice)).toEqual([0, 1, 2, 3]);
    expect(filtrarVitrine(lista, { ...f, soComProblema: true }, { "3": { erros: ["x"], avisos: [] } }).map((x) => x.indice)).toEqual([3]);
  });

  it("conta ofertas por aba e subaba", () => {
    const c = contarPorAba(lista);
    expect([c.get("0"), c.get("0/1"), c.get("1/0")]).toEqual([4, 2, 1]);
  });
});

describe("unidades do cliente", () => {
  it("preço em centavos como GetCashText", () => {
    expect([precoEmGold(1490), precoEmGold(1250), precoEmGold(5)]).toEqual(["14,90", "12,50", "0,05"]);
    expect([precoEmGold(100000), goldEmCentavos("12,5"), goldEmCentavos("1.000"), goldEmCentavos("x")]).toEqual(["1.000", 1250, 100000, null]);
  });

  it("selo e preço cheio do desconto", () => {
    expect([selo(1).curto, selo(3).curto, selo(7).curto, selo(15).curto]).toEqual(["NOVO", "HOT", "-60%", "VIP 2"]);
    expect([precoCheio(400, 7), precoCheio(400, 1)]).toEqual([1000, null]);
  });

  it("validade, datas em GMT e dias marcados", () => {
    expect([validadeEmTexto(0), validadeEmTexto(604800), validadeEmTexto(90000)]).toEqual(["permanente", "7 d", "1 d 1 h"]);
    expect(dataParaCampo(1700000000)).toBe("2023-11-14T22:13");
    expect(campoParaData("2023-11-14T22:13")).toBe(1699999980);
    expect([dataParaCampo(0), campoParaData("")]).toEqual(["", 0]);
    expect(bitsParaDias(0b1000001, false)).toEqual([0, 6]);
    expect(diasParaBits([0, 30])).toBe(0x40000001);
    expect(diasParaBits(bitsParaDias(0x7fffffff, true))).toBe(0x7fffffff);
  });

  it("texto que cabe no wchar", () => {
    expect([cabeEmWchar("a".repeat(31), 32), cabeEmWchar("a".repeat(32), 32), cabeEmWchar("😀".repeat(16), 32)]).toEqual([true, false, false]);
  });
});
