import { describe, expect, it } from "vitest";
import { comBit, FILTROS_DE_NPC, grausDoVetor, ligado, SERVICOS_DO_NPC, SIMPLES, SIMPLES2, vetorDosGraus, EXPLICACOES_DO_NPC } from "./npc";
import { CATEGORIAS_DE_SERVICO } from "./servicos";

import bruto156 from "../../../../specs/elements_layouts/v156.json?raw";
import bruto7 from "../../../../specs/elements_layouts/v7.json?raw";

type Layout = { tables: { name: string; fields: { name: string }[] }[] };
const campos = (bruto: string, tabela: string) =>
  new Set((JSON.parse(bruto) as Layout).tables.find((t) => t.name === tabela)?.fields.map((f) => f.name) ?? []);
const tabelas = (bruto: string) => new Set((JSON.parse(bruto) as Layout).tables.map((t) => t.name));

describe("NPC: campos e serviços conferidos nos layouts", () => {
  const npc156 = campos(bruto156, "NPC_ESSENCE"), npc7 = campos(bruto7, "NPC_ESSENCE");
  const t156 = tabelas(bruto156), t7 = tabelas(bruto7);

  it("toda vaga de serviço é campo do NPC_ESSENCE v156 e aponta para uma tabela que existe", () => {
    const semTabelaNoV7: string[] = [];
    for (const s of SERVICOS_DO_NPC) {
      expect(npc156.has(s.campo), s.campo).toBe(true);
      expect(t156.has(s.tabela), s.tabela).toBe(true);
      if (npc7.has(s.campo) && !t7.has(s.tabela)) semTabelaNoV7.push(s.campo);
    }
    // Medido: o NPC_ESSENCE do v7 tem `id_cross_server_service`, mas o v7 não tem a tabela NPC_CROSS_SERVER_SERVICE —
    // no 1.2.6 a vaga não tem o que vincular (a tela esconde vaga sem tabela).
    expect(semTabelaNoV7).toEqual(["id_cross_server_service"]);
  });
  it("cobre todos os id_*_service do NPC_ESSENCE v156", () => {
    const nossos = new Set(SERVICOS_DO_NPC.map((s) => s.campo));
    const doLayout = [...npc156].filter((n) => /^id_.*_service$/.test(n));
    expect(doLayout.filter((n) => !nossos.has(n))).toEqual([]);
  });
  it("campos explicados existem; o v7 não tem o segundo campo de serviços simples", () => {
    for (const n of Object.keys(EXPLICACOES_DO_NPC)) expect(npc156.has(n), n).toBe(true);
    expect(npc7.has("combined_services")).toBe(true);
    expect(npc7.has("combined_services2")).toBe(false);
    for (const f of FILTROS_DE_NPC) expect(npc156.has(f.campo), f.campo).toBe(true);
  });
  it("as categorias de serviço cobrem todas as tabelas NPC_*_SERVICE do v156", () => {
    const nossas = new Set(CATEGORIAS_DE_SERVICO.flatMap((c) => c.tabelas));
    const doLayout = [...t156].filter((t) => /^NPC_.*_SERVICE$/.test(t));
    expect(doLayout.filter((t) => !nossas.has(t))).toEqual([]);
  });
  it("bits sem repetição e dentro de 32", () => {
    for (const l of [SIMPLES, SIMPLES2]) {
      const b = l.map((x) => x.bit);
      expect(new Set(b).size).toBe(b.length);
      expect(b.every((x) => x >= 0 && x < 32)).toBe(true);
    }
  });
});

describe("bits e direção", () => {
  it("liga e desliga o bit 31 sem perder o sinal do int32", () => {
    const v = comBit(0, 31, true);
    expect(v).toBe(-2147483648);
    expect(ligado(v, 31)).toBe(true);
    expect(comBit(v, 31, false)).toBe(0);
    expect(comBit(0x200, 10, true)).toBe(0x600);
  });
  it("graus ↔ vetor como o GS (atan2(z, x))", () => {
    expect(grausDoVetor(1, 0)).toBe(0);
    expect(grausDoVetor(0, 1)).toBe(90);
    expect(grausDoVetor(-1, 0)).toBe(180);
    expect(grausDoVetor(0, -1)).toBe(270);
    const v = vetorDosGraus(135);
    expect(grausDoVetor(v.x, v.z)).toBe(135);
  });
});
