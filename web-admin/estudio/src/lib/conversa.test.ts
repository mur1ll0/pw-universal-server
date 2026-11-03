import { describe, expect, it } from "vitest";
import { arvore, novaJanela, removerJanela, TALK_EXIT, validar, type ConversaModelo } from "./conversa";
import funcoesJson from "../../../../specs/estudio/funcoes_de_conversa.json";

const funcoes = (funcoesJson as { funcoes: { codigo: number; rotulo: string }[] }).funcoes.map((f) => ({ codigo: f.codigo, rotulo: f.rotulo }));

const base = (): ConversaModelo => ({
  titulo: "Ferreiro",
  janelas: [
    { id: 1, id_pai: -1, texto: "Olá", opcoes: [{ id: 2, texto: "Minério?", param: 0 }, { id: TALK_EXIT, texto: "Até", param: 0 }] },
    { id: 2, id_pai: 1, texto: "Da mina", opcoes: [{ id: 3, texto: "E?", param: 0 }] },
    { id: 3, id_pai: 2, texto: "Fim", opcoes: [{ id: TALK_EXIT, texto: "Ok", param: 0 }] },
  ],
});

describe("conversa (talk_proc)", () => {
  it("árvore pela janela pai", () => {
    expect(arvore(base()).map((x) => [x.janela.id, x.nivel])).toEqual([[1, 0], [2, 1], [3, 2]]);
  });
  it("as funções do arquivo de funções existem e a saída é TALK_EXIT", () => {
    expect(funcoes.some((f) => f.codigo === TALK_EXIT)).toBe(true);
    expect(validar(base(), funcoes)).toEqual([]);
  });
  it("janela nova com id livre e remover leva as filhas e os botões que apontavam", () => {
    const c = base();
    const id = novaJanela(c, 1, "nova");
    expect(id).toBe(4);
    expect(removerJanela(c, 2)).toBe(2);
    expect(c.janelas.map((j) => j.id)).toEqual([1, 4]);
    expect(c.janelas[0].opcoes.map((o) => o.id)).toEqual([TALK_EXIT]);
    expect(validar(c, funcoes)).toEqual([]);
  });
  it("valida como o servidor: pai inexistente, destino inexistente, texto longo", () => {
    const c = base();
    c.janelas[1].id_pai = 9;
    c.janelas[0].opcoes[0].id = 7;
    c.janelas[2].opcoes[0].texto = "x".repeat(64);
    const p = validar(c, funcoes);
    expect(p.some((x) => x.includes("pai 9"))).toBe(true);
    expect(p.some((x) => x.includes("janela 7"))).toBe(true);
    expect(p.some((x) => x.includes("64 unidades"))).toBe(true);
  });
});
