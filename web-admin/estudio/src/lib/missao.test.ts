import { describe, expect, it } from "vitest";
import { copiar, novaJanela, TALK_EXIT, type OpcaoModelo } from "./conversa";
import { modeloDoDialogo, operacoesDoDialogo } from "./missao";

const arquivo = () => ({
  id_talk: 0, texto: "Oferta",
  janelas: [
    { id: 1, pai: 0xffffffff, texto: "Quer ajudar?", opcoes: [{ id: 2, texto: "Conte mais", param: 0 }, { id: 0x80000006, texto: "Aceito", param: 2104 }] },
    { id: 2, pai: 1, texto: "Lobos!", opcoes: [{ id: TALK_EXIT, texto: "Depois", param: 0 }] },
  ],
});

describe("diálogo de missão → operações do tasks.data", () => {
  it("só textos: uma operação por texto mudado, nas posições do arquivo", () => {
    const a = modeloDoDialogo(arquivo());
    const d = copiar(a);
    d.titulo = "Oferta nova";
    d.janelas[1].texto = "^ff0000Lobos!";
    d.janelas[0].opcoes[1].texto = "Aceito!";
    expect(operacoesDoDialogo(0, a, d, new Set())).toEqual([
      { dialogo: 0, texto: "Oferta nova" },
      { dialogo: 0, janela: 0, opcao: 1, texto: "Aceito!" },
      { dialogo: 0, janela: 1, texto: "^ff0000Lobos!" },
    ]);
  });
  it("tirar o botão que abre janela tira a janela; o botão novo vai na posição atual", () => {
    const a = modeloDoDialogo(arquivo());
    const d = copiar(a);
    d.janelas[0].opcoes.splice(0, 1);
    d.janelas = d.janelas.filter((j) => j.id !== 2);
    d.janelas[0].opcoes.push({ id: TALK_EXIT, texto: "Sair", param: 0 });
    const novas = new Set<OpcaoModelo>([d.janelas[0].opcoes[1]]);
    expect(operacoesDoDialogo(3, a, d, novas)).toEqual([
      { dialogo: 3, acao: "remover_opcao", janela: 0, opcao: 0 },
      { dialogo: 3, acao: "nova_opcao", janela: 0, texto: "Sair", destino: { funcao: 18 }, param: 0 },
    ]);
  });
  it("botão para janela nova cria a janela com o texto dela; botões dentro dela vêm depois", () => {
    const a = modeloDoDialogo(arquivo());
    const d = copiar(a);
    const id = novaJanela(d, 2, "A toca fica ao norte");
    d.janelas[1].opcoes.push({ id, texto: "Onde?", param: 0 });
    const novas = new Set<OpcaoModelo>([d.janelas[1].opcoes[1], ...d.janelas.find((j) => j.id === id)!.opcoes]);
    expect(id).toBe(3);
    expect(operacoesDoDialogo(0, a, d, novas)).toEqual([
      { dialogo: 0, acao: "nova_opcao", janela: 1, texto: "Onde?", destino: { janela: "A toca fica ao norte" } },
      { dialogo: 0, acao: "nova_opcao", janela: 2, texto: "Até logo", destino: { funcao: 18 }, param: 0 },
    ]);
  });
  it("diálogo vazio: cria a raiz", () => {
    const a = modeloDoDialogo({ id_talk: 0, texto: "", janelas: [] });
    const d = copiar(a);
    novaJanela(d, -1, "Volte depois");
    const novas = new Set<OpcaoModelo>(d.janelas[0].opcoes);
    expect(operacoesDoDialogo(1, a, d, novas)).toEqual([
      { dialogo: 1, acao: "criar_raiz", texto: "Volte depois" },
      { dialogo: 1, acao: "nova_opcao", janela: 0, texto: "Até logo", destino: { funcao: 18 }, param: 0 },
    ]);
  });
});
