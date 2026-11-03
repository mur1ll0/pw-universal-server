/* B392: a conversa (`talk_proc`, `CCommon/ExpTypes.h:3872`) no editor do Estúdio. A mesma estrutura serve ao TALK_PROC
   do elements (`pw_data_loader::conversas`, B240) e aos cinco diálogos de cada missão (`tasks_editavel`, B266/B270):
   janelas em árvore (`id_pai` −1 = raiz) e opções de 63 caracteres; opção com o bit alto no id é uma função do jogo
   (`enum SERVICE_TYPE`, `ExpTypes.h:36-102`), sem ele aponta para uma janela. As regras de `validar` são as do servidor
   (`estudio.rs::conversa_do_json`). */

export const BIT_DE_FUNCAO = 0x80000000;
export const TEXTO_DA_OPCAO = 63; // `conversas.rs::texto_fixo`: 64 unidades com o fim
export const TALK_EXIT = 0x80000012;
export const TALK_RETURN = 0x80000011;

export interface OpcaoModelo { id: number; texto: string; param: number }
export interface JanelaModelo { id: number; id_pai: number; texto: string; opcoes: OpcaoModelo[] }
export interface ConversaModelo { titulo: string; janelas: JanelaModelo[] }
export interface Funcao { codigo: number; rotulo: string }

export const ehFuncao = (id: number) => (id >>> 0) >= BIT_DE_FUNCAO;

export function copiar(c: ConversaModelo): ConversaModelo {
  return { titulo: c.titulo, janelas: c.janelas.map((j) => ({ ...j, opcoes: j.opcoes.map((o) => ({ ...o })) })) };
}

/** As janelas em ordem de árvore: cada raiz seguida das filhas (profundidade para recuar no fluxo). Janela cujo pai
    não existe aparece como raiz (o servidor recusa ao gravar; a tela mostra o aviso). */
export function arvore(c: ConversaModelo): { janela: JanelaModelo; nivel: number }[] {
  const ids = new Set(c.janelas.map((j) => j.id));
  const filhas = new Map<number, JanelaModelo[]>();
  const raizes: JanelaModelo[] = [];
  for (const j of c.janelas) {
    if (j.id_pai === -1 || !ids.has(j.id_pai) || j.id_pai === j.id) raizes.push(j);
    else filhas.set(j.id_pai, [...(filhas.get(j.id_pai) ?? []), j]);
  }
  const r: { janela: JanelaModelo; nivel: number }[] = [];
  const vistos = new Set<number>();
  const ir = (j: JanelaModelo, nivel: number) => {
    if (vistos.has(j.id)) return;
    vistos.add(j.id);
    r.push({ janela: j, nivel });
    for (const f of filhas.get(j.id) ?? []) ir(f, nivel + 1);
  };
  raizes.forEach((j) => ir(j, 0));
  for (const j of c.janelas) if (!vistos.has(j.id)) ir(j, 0); // ciclos: aparecem soltos
  return r;
}

export const proximoId = (c: ConversaModelo) => Math.max(0, ...c.janelas.map((j) => j.id)) + 1;

/** Janela nova filha de `pai` (−1 = raiz). Devolve o id. */
export function novaJanela(c: ConversaModelo, pai: number, texto = ""): number {
  const id = proximoId(c);
  c.janelas.push({ id, id_pai: pai, texto, opcoes: [{ id: TALK_EXIT, texto: "Até logo", param: 0 }] });
  return id;
}

/** Remove a janela e as filhas (pela árvore) e as opções que apontavam para elas. Devolve quantas janelas saíram. */
export function removerJanela(c: ConversaModelo, id: number): number {
  const sai = new Set<number>([id]);
  let mudou = true;
  while (mudou) {
    mudou = false;
    for (const j of c.janelas) if (!sai.has(j.id) && sai.has(j.id_pai)) { sai.add(j.id); mudou = true; }
  }
  c.janelas = c.janelas.filter((j) => !sai.has(j.id));
  for (const j of c.janelas) j.opcoes = j.opcoes.filter((o) => ehFuncao(o.id) || !sai.has(o.id));
  return sai.size;
}

/** Os problemas que o servidor recusaria (ou que o jogador veria). */
export function validar(c: ConversaModelo, funcoes: Funcao[]): string[] {
  const p: string[] = [];
  if (!c.janelas.length) p.push("A conversa precisa de ao menos uma janela.");
  if (c.titulo.length > TEXTO_DA_OPCAO) p.push(`Título com ${c.titulo.length} unidades (cabem ${TEXTO_DA_OPCAO}).`);
  const ids = new Set<number>();
  for (const j of c.janelas) {
    if (ehFuncao(j.id)) p.push(`Janela ${j.id}: id com o bit alto (reservado às funções).`);
    if (ids.has(j.id)) p.push(`Id de janela repetido: ${j.id}.`);
    ids.add(j.id);
  }
  for (const j of c.janelas) {
    if (j.id_pai !== -1 && !ids.has(j.id_pai)) p.push(`Janela ${j.id}: a janela pai ${j.id_pai} não existe.`);
    j.opcoes.forEach((o, k) => {
      if (o.texto.length > TEXTO_DA_OPCAO) p.push(`Janela ${j.id}, botão ${k + 1}: ${o.texto.length} unidades (cabem ${TEXTO_DA_OPCAO}).`);
      if (ehFuncao(o.id)) { if (!funcoes.some((f) => f.codigo === o.id)) p.push(`Janela ${j.id}, botão ${k + 1}: função ${o.id.toString(16)} desconhecida.`); }
      else if (!ids.has(o.id)) p.push(`Janela ${j.id}, botão ${k + 1}: aponta para a janela ${o.id}, que não existe.`);
    });
    if (!j.opcoes.length) p.push(`Janela ${j.id}: sem botões — o jogador só sai pelo X.`);
  }
  return p;
}

/** O que o botão faz, em palavras. */
export function destino(o: OpcaoModelo, funcoes: Funcao[]): string {
  if (!ehFuncao(o.id)) return `vai para a janela ${o.id}`;
  const f = funcoes.find((x) => x.codigo === (o.id >>> 0));
  return f ? f.rotulo : `função ${(o.id >>> 0).toString(16)}`;
}

/** Funções que levam um parâmetro útil (o id de uma missão, `NPC_GIVE_TASK`/`NPC_COMPLETE_TASK`/`NPC_GIVE_TASK_MATTER`). */
export const FUNCOES_COM_MISSAO = new Set([0x80000006, 0x80000007, 0x80000008]);
