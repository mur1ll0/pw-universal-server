import { describe, expect, it } from "vitest";
import { CATEGORIAS, TABELAS_DE_ITEM } from "./catalogo";
import { chancesDoRefino, gruposDeItens, ondeAPedraServe, recargaDoRemedio, regraDoRemedio, renascerDaMina, SUBCATEGORIAS_DE_ITENS } from "./itens";

import bruto156 from "../../../../specs/elements_layouts/v156.json?raw";
import bruto7 from "../../../../specs/elements_layouts/v7.json?raw";

/* Os layouts das duas versões (o mesmo arquivo que o pw-recursos usa). */
const layout = (bruto: string) => new Set<string>(JSON.parse(bruto).tables.map((t: { name: string }) => t.name));
const v156 = layout(bruto156);
const v7 = layout(bruto7);

describe("grupos de Itens", () => {
  const equipaveis = new Set(CATEGORIAS.find((c) => c.chave === "equipaveis")!.subcategorias.map((s) => s.tabela));
  it("cobre todas as tabelas de item que não são Equipáveis, mais as minas", () => {
    const esperadas = [...TABELAS_DE_ITEM].filter((t) => !equipaveis.has(t)).sort();
    const nossas = SUBCATEGORIAS_DE_ITENS.map((s) => s.tabela).filter((t) => t !== "MINE_ESSENCE").sort();
    expect(nossas).toEqual(esperadas);
  });
  it("toda tabela existe no v156, e a que não é só do 1.5.5 existe no v7", () => {
    for (const s of SUBCATEGORIAS_DE_ITENS) {
      expect(v156.has(s.tabela), s.tabela).toBe(true);
      if (!s.so) expect(v7.has(s.tabela), s.tabela).toBe(true);
      if (s.so === "1.5.5") expect(v7.has(s.tabela), `${s.tabela} marcada só 1.5.5 mas existe no v7`).toBe(false);
      for (const t of [s.maior, s.menor].filter(Boolean) as string[]) expect(v156.has(t) && (s.so || v7.has(t)), t).toBe(true);
    }
  });
  it("1.2.6 não lista o que só o 1.5.5 tem; ordem e nomes dos grupos", () => {
    const g126 = gruposDeItens("1.2.6");
    expect(g126.flatMap((g) => g.subs).some((s) => s.tabela === "FACTION_MATERIAL_ESSENCE")).toBe(false);
    expect(gruposDeItens("1.5.5").flatMap((g) => g.subs).some((s) => s.tabela === "FACTION_MATERIAL_ESSENCE")).toBe(true);
    expect(g126.map((g) => g.grupo).slice(0, 4)).toEqual(["Materiais", "Poções e remédios", "Pergaminhos", "Livros de habilidade"]);
    expect(g126.some((g) => g.grupo === "Cartas e Astrolábio")).toBe(false);
  });
  it("chaves únicas", () => expect(new Set(SUBCATEGORIAS_DE_ITENS.map((s) => s.chave)).size).toBe(SUBCATEGORIAS_DE_ITENS.length));
});

describe("remédio (generate_medicine, item_potion.cpp)", () => {
  const base = { require_level: 0, cool_time: 5000, hp_add_total: 0, hp_add_time: 0, mp_add_total: 0, mp_add_time: 0 };
  it("vida no tempo: total / tempo por segundo, divisão inteira", () => {
    const r = regraDoRemedio({ ...base, id_major_type: 1794, hp_add_total: 100, hp_add_time: 3 });
    expect(r.linhas[0]).toContain("33 por segundo");
    expect(r.avisos).toEqual([]);
    expect(r.linhas[0]).toContain("no total 99");
  });
  it("menos de 1 por segundo é recusado pelo GS (ASSERT)", () =>
    expect(regraDoRemedio({ ...base, id_major_type: 1802, mp_add_total: 2, mp_add_time: 5 }).avisos.length).toBe(1));
  it("vida e mana na hora ignora os tempos", () =>
    expect(regraDoRemedio({ ...base, id_major_type: 1810, hp_add_total: 50, mp_add_total: 20 }).linhas[0]).toContain("na hora 50 de vida e 20 de mana"));
  it("categoria que não é remédio não gera item", () => expect(regraDoRemedio({ ...base, id_major_type: 9 }).avisos[0]).toContain("não é de remédio"));
  it("recarga abaixo de 1000 é ×100", () => { expect(recargaDoRemedio(30)).toBe(3000); expect(recargaDoRemedio(1500)).toBe(1500); });
});

describe("pedra (IsStoneFit, item.cpp:332-353)", () => {
  it("máscara 0: arma e armadura, não acessório", () => expect(ondeAPedraServe(0)).toEqual({ arma: true, armadura: true, acessorio: false }));
  it("bit 1 tira a arma; colar libera acessório", () => expect(ondeAPedraServe(0x1 | 0x4)).toEqual({ arma: false, armadura: true, acessorio: true }));
});

describe("refino (refine_table + RandSelect)", () => {
  const soma = (c: number[]) => c.reduce((a, b) => a + b, 0);
  it("+0 sem talismã: 50% sucesso, 50% nada muda (o 0,7 passa de 1 e é cortado)", () => {
    const c = chancesDoRefino(0);
    expect(c[0]).toBeCloseTo(0.5); expect(c[1]).toBeCloseTo(0.5); expect(soma(c)).toBeCloseTo(1);
  });
  it("+3 sem talismã: 30% sucesso, 70% zera", () => {
    const c = chancesDoRefino(3);
    expect(c[0]).toBeCloseTo(0.3); expect(c[3]).toBeCloseTo(0.7);
  });
  it("talismã soma no sucesso e na chance de cair só um", () => {
    const c = chancesDoRefino(3, { ext_succeed_prob: 0.1, ext_reserved_prob: 0.2, fail_reserve_level: 0, fail_ext_succeed_prob: [] });
    expect(c.map((x) => Math.round(x * 100))).toEqual([40, 0, 20, 40]);
  });
  it("falhar não tira nível: sucesso do nível, o resto nada muda", () => {
    const t = { ext_succeed_prob: 0, ext_reserved_prob: 0, fail_reserve_level: 1, fail_ext_succeed_prob: [0.5, 0.25, 0.25, 0.2, 0.2, 0.2, 0.2, 0.2, 0.15, 0.1, 0.05, 0.02] };
    const c = chancesDoRefino(1, t);
    expect(c[0]).toBeCloseTo(0.25); expect(c[1]).toBeCloseTo(0.75); expect(c[3]).toBeCloseTo(0);
  });
  it("ajuste acima de 1 é preso a 1", () =>
    expect(chancesDoRefino(5, { ext_succeed_prob: 3, ext_reserved_prob: 0, fail_reserve_level: 0, fail_ext_succeed_prob: [] })[0]).toBeCloseTo(1));
});

describe("área de recurso", () => {
  it("renascer = 15 + dwRefreshTime, nunca menos de 15", () => {
    expect(renascerDaMina(0)).toBe(15); expect(renascerDaMina(45)).toBe(60); expect(renascerDaMina(-30)).toBe(15);
  });
});

describe("rota da edição pela tabela", async () => {
  const { categoriaDaTabela } = await import("./catalogo");
  it("item, mina, equipável e monstro", () => {
    expect(categoriaDaTabela("MEDICINE_ESSENCE")).toBe("itens");
    expect(categoriaDaTabela("MINE_ESSENCE")).toBe("itens");
    expect(categoriaDaTabela("WEAPON_ESSENCE")).toBe("equipaveis");
    expect(categoriaDaTabela("MONSTER_ESSENCE")).toBe("monstros");
  });
});
