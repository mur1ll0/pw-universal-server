<script lang="ts">
  /* B392: aba Conversa do NPC. Em cima, a janela do NPC como o cliente monta (`DlgNPC.cpp:339-1135`): a fala inicial
     (`hello_msg`, do NPC) e as linhas — cujo texto é o `Name` de cada serviço (editável aqui, com o aviso de serviço
     compartilhado). Embaixo, as conversas ligadas: a do serviço de fala (`NPC_TALK_SERVICE.id_dialog`) e a "antes de
     abrir" de cada serviço com `id_dialog` (`DlgNPC.cpp:2854-3023`), editadas no `EditorDeConversa` (TALK_PROC, B240).
     Dar conversa a quem não tem: conversa nova + serviço de fala novo (clone com o `id_dialog` dela) na vaga do NPC. */
  import { conversaPorId, ficha as lerFicha, funcoesDeConversa, listarServicos, clonar, novaConversa, salvar, salvarConversa,
    type Conversa, type FuncaoDeConversa } from "../lib/api";
  import type { ConversaModelo } from "../lib/conversa";
  import { avisar, estudio } from "../lib/estado.svelte";
  import { ligado, SERVICOS_DO_NPC, SIMPLES, SIMPLES2, type LinhaDoMenu, type Vinculado } from "../lib/npc";
  import EditorDeConversa from "./EditorDeConversa.svelte";
  import EditorDeTexto from "./EditorDeTexto.svelte";
  import Icone from "./Icone.svelte";
  import JanelaDoNpc from "./JanelaDoNpc.svelte";

  let { nome, valores = $bindable(), vinculados, aoVincular }:
    { nome: string; valores: Record<string, number | string>; vinculados: Vinculado[]; aoVincular: (campo: string, tabela: string, id: number, nome: string, indice: number) => void } = $props();
  const editavel = $derived(!!estudio.rascunho);

  let funcoes = $state<FuncaoDeConversa[]>([]);
  funcoesDeConversa().then((f) => (funcoes = f)).catch(() => {});

  /** Os serviços ligados, lidos: nome (texto do botão), id_dialog e quantos registros os usam. */
  interface Lido extends Vinculado { dialogo: number | null; usado: number; nomeEditado: string }
  let lidos = $state<Lido[]>([]);
  async function lerServicos() {
    const r: Lido[] = [];
    for (const v of vinculados) {
      try {
        const f = await lerFicha(v.tabela, v.indice);
        const d = f.campos.find((c) => c.nome === "id_dialog");
        r.push({ ...v, nome: f.nome, nomeEditado: f.nome, dialogo: d ? Number(d.valor ?? 0) : null, usado: f.usado_por.length });
      } catch { r.push({ ...v, dialogo: null, usado: 0, nomeEditado: v.nome }); }
    }
    lidos = r;
  }
  $effect(() => { void vinculados; void estudio.rascunho; lerServicos(); });

  const ordem = (campo: string) => SERVICOS_DO_NPC.findIndex((s) => s.campo === campo);
  const linhas = $derived.by((): LinhaDoMenu[] => {
    const l: LinhaDoMenu[] = [];
    const cs = Number(valores.combined_services ?? 0), cs2 = Number(valores.combined_services2 ?? 0);
    for (const s of SIMPLES) if (ligado(cs, s.bit)) l.push({ texto: s.nome, tipo: "simples" });
    if ("combined_services2" in valores) for (const s of SIMPLES2) if (ligado(cs2, s.bit)) l.push({ texto: s.nome, tipo: "simples" });
    for (const x of [...lidos].sort((a, b) => ordem(a.campo) - ordem(b.campo))) {
      const def = SERVICOS_DO_NPC.find((s) => s.campo === x.campo);
      if (def && (def.menu === "linha" || def.menu === "condicional")) l.push({ texto: x.nomeEditado || def.nome, tipo: "servico", chave: x.campo, nota: def.menu === "condicional" ? def.explica : "" });
    }
    if (lidos.some((x) => x.campo.startsWith("id_task_"))) l.push({ texto: "(as missões que ele dá e recebe)", tipo: "missao", nota: "CheckTaskService, DlgNPC.cpp:1100" });
    l.push({ texto: "Sair", tipo: "sair" });
    return l;
  });

  /* ---- conversa aberta ---- */
  let fonte = $state<string | null>(null); // campo do serviço
  let conversa = $state<Conversa | null>(null);
  let carregando = $state(false);
  const lidoDaFonte = $derived(lidos.find((x) => x.campo === fonte));
  async function abrirConversa(campo: string) {
    fonte = campo;
    conversa = null;
    const x = lidos.find((y) => y.campo === campo);
    if (!x?.dialogo) return;
    carregando = true;
    try { conversa = await conversaPorId(x.dialogo); if (!conversa) avisar("aviso", `A conversa ${x.dialogo} não existe no elements.`); }
    catch (e) { avisar("erro", (e as Error).message); }
    finally { carregando = false; }
  }
  const modelo = (c: Conversa): ConversaModelo => ({ titulo: c.texto, janelas: c.janelas.map((j) => ({ id: j.id, id_pai: j.id_pai, texto: j.texto, opcoes: j.opcoes.map((o) => ({ id: o.id >>> 0, texto: o.texto, param: o.param })) })) });

  async function gravarConversa(m: ConversaModelo) {
    if (!conversa) return;
    try {
      await salvarConversa(conversa.indice, { texto: m.titulo, janelas: m.janelas.map((j) => ({ ...j, opcoes: j.opcoes.map((o) => ({ id: o.id >>> 0, texto: o.texto, param: o.param })) })) });
      avisar("ok", `Conversa ${conversa.id} gravada no rascunho.`);
    } catch (e) { avisar("erro", (e as Error).message); throw e; }
  }

  /** Conversa nova (ou cópia da atual) ligada ao serviço da vaga; sem serviço de fala, cria um. */
  async function darConversa(campo: string, copiar = false) {
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto."); return; }
    try {
      const nova = await novaConversa(copiar && conversa ? conversa.indice : undefined);
      const x = lidos.find((y) => y.campo === campo);
      if (x) {
        await salvar(x.tabela, x.indice, { id_dialog: nova.id });
        avisar("ok", `Conversa ${nova.id} criada e ligada a "${x.nome}".`);
      } else {
        // sem serviço de fala: um novo, clone do primeiro NPC_TALK_SERVICE
        const base = (await listarServicos("NPC_TALK_SERVICE", "", "", 0, 1)).registros[0];
        if (!base) throw new Error("O elements não tem NPC_TALK_SERVICE para servir de base.");
        const c = await clonar("NPC_TALK_SERVICE", base.indice);
        await salvar("NPC_TALK_SERVICE", c.resultado.indice, { Name: "Conversar", id_dialog: nova.id });
        aoVincular("id_talk_service", "NPC_TALK_SERVICE", c.resultado.id, "Conversar", c.resultado.indice);
        avisar("ok", `Conversa ${nova.id} e serviço de fala ${c.resultado.id} criados. Grave o NPC para ligar.`);
      }
      await lerServicos();
      await abrirConversa(campo);
    } catch (e) { avisar("erro", (e as Error).message); }
  }

  async function gravarNome(x: Lido) {
    try { await salvar(x.tabela, x.indice, { Name: x.nomeEditado }); avisar("ok", `Texto do botão gravado em "${x.nomeEditado}".`); await lerServicos(); }
    catch (e) { avisar("erro", (e as Error).message); }
  }
  const temDialogo = (x: Lido) => x.dialogo !== null;
</script>

<div class="topo">
  <section class="caixa">
    <h3>Fala inicial</h3>
    <EditorDeTexto rotulo="hello_msg" valor={String(valores.hello_msg ?? "")} limite={255} linhas={4} desabilitado={!editavel} previa={false}
      aoMudar={(v) => (valores.hello_msg = v)} />
    <p class="mini">Grava com o NPC (botão no alto). Até 255 caracteres (o campo tem 256 com o fim), contando os códigos de cor.</p>

    <h3 class="sep">Texto de cada botão</h3>
    <p class="mini">O texto é o nome do serviço: mudar aqui muda em todos os NPCs que usam o mesmo serviço.</p>
    <ul class="botoes">
      {#each [...lidos].sort((a, b) => ordem(a.campo) - ordem(b.campo)) as x (x.campo)}
        {@const def = SERVICOS_DO_NPC.find((s) => s.campo === x.campo)}
        <li>
          <span class="mini tp">{def?.nome ?? x.campo}{def?.menu === "na_loja" ? " · dentro da loja, sem linha" : def?.menu === "missoes" ? " · lista de missões" : ""}</span>
          <div class="linha">
            <div class="texto"><EditorDeTexto rotulo="" valor={x.nomeEditado} limite={31} umaLinha previa={false} desabilitado={!editavel} aoMudar={(v) => (x.nomeEditado = v)} /></div>
            <button class="botao pequeno" type="button" disabled={!editavel || x.nomeEditado === x.nome} onclick={() => gravarNome(x)}>Gravar</button>
          </div>
          {#if x.usado > 1}<span class="mini aviso">compartilhado por {x.usado} registros</span>{/if}
        </li>
      {:else}
        <li class="mini">Nenhum serviço ligado.</li>
      {/each}
    </ul>
  </section>
  <section class="caixa">
    <h3>Como o jogador vê</h3>
    <JanelaDoNpc nome={nome} fala={String(valores.hello_msg ?? "")} {linhas}
      aoClicar={(l) => { if (l.chave) abrirConversa(l.chave); }} />
    <p class="mini">Ordem do cliente: serviços simples, serviços (na ordem fixa do cliente), missões, Sair. Algumas linhas só aparecem em certa condição
      (passe o mouse). Clique numa linha de serviço para abrir a conversa dela.</p>
  </section>
</div>

<section class="caixa">
  <h3>Conversas deste NPC</h3>
  <div class="fontes">
    {#if !lidos.some((x) => x.campo === "id_talk_service")}
      <button class="fonte vazia" type="button" disabled={!editavel} onclick={() => darConversa("id_talk_service")}>
        <Icone nome="mais" tamanho={16} /> Dar uma conversa a este NPC <span class="mini">(serviço de fala novo)</span></button>
    {/if}
    {#each lidos.filter(temDialogo) as x (x.campo)}
      <button class="fonte" class:on={fonte === x.campo} type="button" onclick={() => abrirConversa(x.campo)}>
        <Icone nome="conversa" tamanho={16} />
        <span>{x.campo === "id_talk_service" ? "Conversa (serviço de fala)" : `Antes de abrir: ${x.nome}`}</span>
        <span class="mini">{x.dialogo ? `conversa ${x.dialogo}` : "sem conversa"}</span>
      </button>
    {/each}
  </div>

  {#if fonte}
    {#if carregando}<p class="suave">Carregando a conversa…</p>
    {:else if conversa}
      {#if (conversa.usado_por?.length ?? 0) > 1}
        <p class="aviso-caixa">Esta conversa é usada por {conversa.usado_por?.length} registros ({conversa.usado_por?.slice(0, 4).map((u) => u.nome || u.tabela).join(", ")}…): mudar muda em todos.
          <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => darConversa(fonte!, true)}><Icone nome="copiar" tamanho={16} /> Fazer uma cópia só para este serviço</button></p>
      {/if}
      {#key conversa.indice}
        <EditorDeConversa modelo={modelo(conversa)} {funcoes} {editavel} nomeNaPrevia={nome} aoSalvar={gravarConversa} />
      {/key}
    {:else if lidoDaFonte && !lidoDaFonte.dialogo}
      <p class="suave">"{lidoDaFonte.nome}" não mostra conversa antes de abrir.
        <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => darConversa(fonte!)}><Icone nome="mais" tamanho={16} /> Criar conversa</button></p>
    {/if}
  {:else}
    <p class="mini">Escolha uma conversa acima (ou clique numa linha da janela do NPC).</p>
  {/if}
</section>

<style>
  .topo { display: grid; grid-template-columns: minmax(0, 1fr) minmax(280px, 380px); gap: 14px; align-items: start; margin-bottom: 14px; }
  .sep { margin-top: 18px !important; }
  .botoes { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .botoes li { display: flex; flex-direction: column; gap: 3px; }
  .tp { text-transform: uppercase; letter-spacing: .04em; font-size: 10px; }
  .texto { flex: 1; min-width: 0; }
  .aviso { color: var(--azul); }
  .fontes { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
  .fonte { display: inline-flex; gap: 8px; align-items: center; padding: 8px 12px; border-radius: 10px; border: 1px solid var(--borda-forte); background: var(--fundo-2);
    color: var(--texto); font: inherit; cursor: pointer; }
  .fonte.on { border-color: var(--ouro); background: var(--ouro-suave); }
  .fonte.vazia { border-style: dashed; color: var(--texto-2); }
  .aviso-caixa { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; padding: 8px 10px; border: 1px solid var(--azul); border-radius: 10px; background: var(--azul-suave); font-size: 13px; }
  @media (max-width: 1000px) { .topo { grid-template-columns: 1fr; } }
</style>
