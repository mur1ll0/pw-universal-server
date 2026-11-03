/* B392: missões (tasks.data) no Estúdio. Os diálogos de missão são `talk_proc` dentro do arquivo (cinco por missão:
   entrega, não qualificado, entrega de item, execução, prêmio — `tasks_editavel::DIALOGOS`) e o servidor só aceita
   operações sobre eles (`Estudio::editar_missao`, campo "dialogos", B266/B270), aplicadas em ordem:
     {dialogo, janela?, opcao?, texto}                         → título, texto de janela ou de botão
     {dialogo, acao: "nova_opcao", janela, texto, destino: {janela: texto} | {funcao: n}, param?}
                                                               → botão no fim da janela; "janela" cria a janela nova
                                                                 (id = maior + 1, sem botões, `acrescentar_opcao`)
     {dialogo, acao: "remover_opcao", janela, opcao}           → tira o botão e, se ele abre janela, ela e as filhas
     {dialogo, acao: "criar_raiz", texto}                      → a primeira janela (id 1) de um diálogo vazio
   `janela` e `opcao` são posições (índices) no estado do arquivo no momento da operação. Aqui a edição livre do
   `EditorDeConversa` (modo de missão) vira essa lista, simulando o arquivo para as posições baterem. */

import { BIT_DE_FUNCAO, type ConversaModelo, type JanelaModelo, type OpcaoModelo } from "./conversa";

export const DIALOGOS = [
  { chave: "entrega", nome: "Ao oferecer a missão" },
  { chave: "nao_qualificado", nome: "Quando não pode pegar" },
  { chave: "entrega_de_item", nome: "Ao entregar o item de missão" },
  { chave: "execucao", nome: "Durante a missão" },
  { chave: "premio", nome: "Ao entregar (prêmio)" },
];

export interface DialogoDoArquivo { id_talk: number; texto: string; janelas: { id: number; pai: number; texto: string; opcoes: { id: number; texto: string; param: number }[] }[] }

/** Opção com a chave da posição original ("janela:opção"), para reconhecer o que foi tirado. */
export type OpcaoComChave = OpcaoModelo & { chave?: string };

export function modeloDoDialogo(d: DialogoDoArquivo): ConversaModelo {
  return {
    titulo: d.texto,
    janelas: d.janelas.map((j, ji) => ({
      id: j.id,
      id_pai: j.pai === 0xffffffff || j.pai === -1 ? -1 : j.pai,
      texto: j.texto,
      opcoes: j.opcoes.map((o, oi) => ({ id: o.id >>> 0, texto: o.texto, param: o.param, chave: `${ji}:${oi}` } as OpcaoComChave)),
    })),
  };
}

export type Operacao = Record<string, unknown>;

/** As operações que levam o diálogo `antes` (do arquivo) ao `depois` (editado). `novas` = botões criados na edição. */
export function operacoesDoDialogo(dialogo: number, antes: ConversaModelo, depois: ConversaModelo, novas: Set<OpcaoModelo>): Operacao[] {
  const ops: Operacao[] = [];
  const ehNova = (o: OpcaoModelo) => novas.has(o) || !(o as OpcaoComChave).chave;
  if (depois.titulo !== antes.titulo) ops.push({ dialogo, texto: depois.titulo });

  // Diálogo vazio que ganhou janela: a raiz primeiro (id 1).
  const trabalho: JanelaModelo[] = antes.janelas.map((j) => ({ ...j, opcoes: j.opcoes.map((o) => ({ ...o })) }));
  if (!trabalho.length && depois.janelas.length) {
    const raiz = depois.janelas.find((j) => j.id_pai === -1) ?? depois.janelas[0];
    ops.push({ dialogo, acao: "criar_raiz", texto: raiz.texto });
    trabalho.push({ id: 1, id_pai: -1, texto: raiz.texto, opcoes: [] });
    if (raiz.id !== 1) throw new Error("a janela raiz de um diálogo vazio é criada com o id 1");
  }

  // 1. Textos das janelas e botões que já existiam (posições do arquivo antes de qualquer mudança de estrutura).
  antes.janelas.forEach((ja, ji) => {
    const jd = depois.janelas.find((x) => x.id === ja.id);
    if (!jd) return;
    if (jd.texto !== ja.texto) ops.push({ dialogo, janela: ji, texto: jd.texto });
    ja.opcoes.forEach((oa, oi) => {
      const chave = (oa as OpcaoComChave).chave;
      const od = jd.opcoes.find((x) => (x as OpcaoComChave).chave === chave && !novas.has(x));
      if (od && od.texto !== oa.texto) ops.push({ dialogo, janela: ji, opcao: oi, texto: od.texto });
    });
  });

  // 2. Botões tirados, um a um, simulando o arquivo (tirar um botão que abre janela tira ela e as filhas).
  const presentes = new Set(depois.janelas.flatMap((j) => j.opcoes.filter((o) => !ehNova(o)).map((o) => (o as OpcaoComChave).chave)));
  for (;;) {
    let achou = false;
    for (let ji = 0; ji < trabalho.length && !achou; ji++) {
      const oi = trabalho[ji].opcoes.findIndex((o) => !presentes.has((o as OpcaoComChave).chave));
      if (oi < 0) continue;
      ops.push({ dialogo, acao: "remover_opcao", janela: ji, opcao: oi });
      const [o] = trabalho[ji].opcoes.splice(oi, 1);
      if (((o.id >>> 0) & BIT_DE_FUNCAO) === 0) {
        const sai = new Set([o.id]);
        let mudou = true;
        while (mudou) { mudou = false; for (const j of trabalho) if (!sai.has(j.id) && sai.has(j.id_pai)) { sai.add(j.id); mudou = true; } }
        for (let k = trabalho.length - 1; k >= 0; k--) if (sai.has(trabalho[k].id)) trabalho.splice(k, 1);
      }
      achou = true;
    }
    if (!achou) break;
  }

  // 3. Botões novos: primeiro nas janelas que já existem, depois nas janelas novas (na ordem de criação = id).
  const ordem = [...depois.janelas].sort((a, b) => Number(!trabalho.some((t) => t.id === a.id)) - Number(!trabalho.some((t) => t.id === b.id)) || a.id - b.id);
  for (const jd of ordem) {
    for (const o of jd.opcoes.filter(ehNova)) {
      const ji = trabalho.findIndex((t) => t.id === jd.id);
      if (ji < 0) throw new Error(`a janela ${jd.id} não existe no arquivo quando o botão "${o.texto}" é criado`);
      const funcao = ((o.id >>> 0) & BIT_DE_FUNCAO) !== 0;
      if (funcao) {
        ops.push({ dialogo, acao: "nova_opcao", janela: ji, texto: o.texto, destino: { funcao: (o.id >>> 0) - BIT_DE_FUNCAO }, param: o.param });
        trabalho[ji].opcoes.push({ ...o });
      } else {
        const alvo = depois.janelas.find((x) => x.id === o.id);
        if (!alvo) throw new Error(`o botão "${o.texto}" aponta para a janela ${o.id}, que não existe`);
        const nid = Math.max(0, ...trabalho.map((t) => t.id)) + 1;
        if (nid !== alvo.id || trabalho.some((t) => t.id === o.id)) {
          throw new Error(`no diálogo de missão um botão só pode abrir janela nova (o arquivo cria a de id ${nid})`);
        }
        ops.push({ dialogo, acao: "nova_opcao", janela: ji, texto: o.texto, destino: { janela: alvo.texto } });
        trabalho[ji].opcoes.push({ ...o });
        trabalho.push({ id: nid, id_pai: jd.id, texto: alvo.texto, opcoes: [] });
      }
    }
  }
  return ops;
}

/* ---- campos da ficha (os editáveis no lugar, `tasks_editavel::campos`) ---- */
export const CAMPOS_GERAIS: { campo: string; rotulo: string; tipo: "texto" | "numero" | "sim" }[] = [
  { campo: "min_level", rotulo: "Nível mínimo", tipo: "numero" },
  { campo: "max_level", rotulo: "Nível máximo", tipo: "numero" },
  { campo: "limite_de_tempo", rotulo: "Tempo limite (s, 0 = sem)", tipo: "numero" },
  { campo: "espera", rotulo: "Espera (s)", tipo: "numero" },
  { campo: "frequencia", rotulo: "Frequência", tipo: "numero" },
  { campo: "pode_desistir", rotulo: "Pode desistir", tipo: "sim" },
  { campo: "pode_repetir", rotulo: "Pode repetir", tipo: "sim" },
  { campo: "entrega_automatica", rotulo: "Entrega automática", tipo: "sim" },
];
export const CAMPOS_DO_PREMIO: { campo: string; rotulo: string }[] = [
  { campo: "money", rotulo: "Dinheiro" }, { campo: "exp", rotulo: "Experiência" }, { campo: "sp", rotulo: "SP" },
  { campo: "reputation", rotulo: "Reputação" }, { campo: "nova_missao", rotulo: "Missão seguinte" },
];
