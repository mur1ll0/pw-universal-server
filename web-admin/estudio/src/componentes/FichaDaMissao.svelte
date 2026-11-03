<script lang="ts">
  /* B392: uma missão do tasks.data no Estúdio (as rotas dos Editores, B255-B270: `GET/PUT missoes/{id}`, `clonar`).
     Abas: Geral (nome com cor, nível, tempo, repetição), Quem dá e recebe (NPCs), Objetivo (monstros e itens — os
     vetores que o escritor aceita, com o teto do cliente), Textos (descrição, aceitar, recusar, tributo), Diálogos (os
     cinco `talk_proc` da missão no editor de conversa, em modo de missão — `lib/missao.ts` converte em operações),
     Prêmio e Etapas. Grava tudo num PUT, no rascunho; o servidor relê o arquivo antes de aceitar. */
  import { clonarMissao, editarMissao, lerMissao, type Registro } from "../lib/api";
  import { BIT_DE_FUNCAO, type ConversaModelo, type Funcao, type OpcaoModelo } from "../lib/conversa";
  import { avisar, estudio, parametros } from "../lib/estado.svelte";
  import { CAMPOS_DO_PREMIO, CAMPOS_GERAIS, DIALOGOS, modeloDoDialogo, operacoesDoDialogo, type Operacao } from "../lib/missao";
  import EditorDeConversa from "./EditorDeConversa.svelte";
  import EditorDeTexto from "./EditorDeTexto.svelte";
  import Icone from "./Icone.svelte";
  import SeletorDeEspaco from "./SeletorDeEspaco.svelte";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { id, aoAbrirOutra }: { id: number; aoAbrirOutra?: (id: number) => void } = $props();
  const editavel = $derived(!!estudio.rascunho);

  let m = $state<any>(null);
  let erro = $state("");
  let aba = $state("geral");
  let gravando = $state(false);
  // valores editados
  let gerais = $state<Record<string, number | boolean | string>>({});
  let premio = $state<Record<string, number>>({});
  let textos = $state<Record<string, string>>({});
  let vetores = $state<Record<string, any[]>>({});
  let dialogoAberto = $state(0);
  /** operações de diálogo já prontas (cada diálogo grava pelo próprio botão do editor) */
  const nomesLocais = $state<Record<number, string>>({});

  async function carregar() {
    erro = "";
    try {
      const x = await lerMissao(id);
      m = x;
      gerais = { name: x.ficha.name, ...Object.fromEntries(CAMPOS_GERAIS.map((c) => [c.campo, x.ficha[c.campo]])),
        npc_que_entrega: x.ficha.npc_que_entrega, npc_que_premia: x.ficha.npc_que_premia };
      premio = Object.fromEntries(CAMPOS_DO_PREMIO.map((c) => [c.campo, Number(x.ficha.rewards?.[c.campo] ?? 0)]));
      textos = { ...x.textos };
      vetores = { itens_exigidos: structuredClone(x.ficha.itens_exigidos ?? []), itens_entregues: structuredClone(x.ficha.itens_entregues ?? []),
        monster_kills: structuredClone(x.ficha.monster_kills ?? []) };
    } catch (e) { erro = (e as Error).message; }
  }
  $effect(() => { void id; void estudio.rascunho; carregar(); });

  const nomeDe = (v: number) => (v ? nomesLocais[v] ?? m?.nomes?.[String(v)]?.nome ?? `id ${v}` : "nenhum");
  const indiceDe = (v: number) => m?.nomes?.[String(v)]?.indice as number | undefined;
  const teto = (nome: string) => (m?.vetores as [string, number][] | undefined)?.find((v) => v[0] === nome)?.[1] ?? 0;

  const mudancas = $derived.by(() => {
    if (!m) return {} as Record<string, unknown>;
    const c: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(gerais)) if (v !== (k === "name" ? m.ficha.name : m.ficha[k])) c[k] = v;
    for (const [k, v] of Object.entries(premio)) if (v !== Number(m.ficha.rewards?.[k] ?? 0)) c[k] = v;
    for (const [k, v] of Object.entries(textos)) if (v !== m.textos[k]) c[k] = v;
    for (const [k, v] of Object.entries(vetores)) if (JSON.stringify(v) !== JSON.stringify(m.ficha[k] ?? [])) c[k] = v;
    return c;
  });
  const quantas = $derived(Object.keys(mudancas).length);

  async function gravar(extra: Record<string, unknown> = {}) {
    const campos = { ...mudancas, ...extra };
    if (!Object.keys(campos).length) { avisar("aviso", "Nada mudou."); return; }
    gravando = true;
    try {
      await editarMissao(id, campos);
      avisar("ok", `Missão ${id} gravada no rascunho (${Object.keys(campos).join(", ")}).`);
      await carregar();
    } catch (e) { avisar("erro", (e as Error).message); throw e; }
    finally { gravando = false; }
  }
  async function clonar() {
    try {
      const r = await clonarMissao(id);
      const novo = r.resultado?.id ?? r.id;
      avisar("ok", `Missão ${novo} criada (cópia de ${id}, com as etapas). Ela só aparece em jogo depois de entrar no serviço de um NPC.`);
      if (novo) aoAbrirOutra?.(novo);
    } catch (e) { avisar("erro", (e as Error).message); }
  }

  /* ---- diálogos ---- */
  const ROTULOS_DE_FUNCAO: Record<string, string> = {
    NPC_TALK: "Conversa", NPC_GIVE_TASK: "Dar a missão", NPC_COMPLETE_TASK: "Concluir a missão", NPC_GIVE_TASK_MATTER: "Dar item de missão",
    TALK_RETURN: "Voltar", TALK_EXIT: "Sair", TALK_GIVEUP_TASK: "Desistir da missão",
  };
  const funcoes = $derived<Funcao[]>(((m?.funcoes_de_opcao ?? []) as [number, string][]).map(([n, nome]) => ({ codigo: (BIT_DE_FUNCAO + n) >>> 0, rotulo: ROTULOS_DE_FUNCAO[nome] ?? nome })));
  const modelos = $derived<ConversaModelo[]>((m?.dialogos ?? []).map((d: any) => modeloDoDialogo(d)));
  async function gravarDialogo(k: number, novo: ConversaModelo, novas: Set<OpcaoModelo>) {
    let ops: Operacao[];
    try { ops = operacoesDoDialogo(k, modelos[k], novo, novas); }
    catch (e) { avisar("erro", (e as Error).message); throw e; }
    if (!ops.length) { avisar("aviso", "Nada mudou no diálogo."); return; }
    await gravar({ dialogos: ops });
  }

  /* ---- escolhas ---- */
  let escolhendo = $state<null | { tipo: "npc"; campo: string } | { tipo: "item" | "monstro"; vetor: string; linha: number | null }>(null);
  function escolher(r: Registro) {
    const e = escolhendo;
    if (!e || !r.id) return;
    nomesLocais[r.id] = r.nome;
    if (e.tipo === "npc") { gerais[e.campo] = r.id; return; }
    const l = vetores[e.vetor];
    if (e.tipo === "monstro") {
      if (e.linha == null) l.push({ monstro: r.id, quantidade: 1, item_que_cai: 0, quantidade_do_item: 0, item_comum: false, chance_do_item: 0, nivel_do_matador: false, dps: 0, dph: 0 });
      else l[e.linha].monstro = r.id;
    } else {
      if (e.linha == null) l.push({ id: r.id, comum: true, quantidade: 1, probabilidade: 1, validade: 0 });
      else l[e.linha].id = r.id;
    }
  }
  const ABAS = [["geral", "Geral"], ["quem", "Quem dá e recebe"], ["objetivo", "Objetivo"], ["textos", "Textos"], ["dialogos", "Diálogos"], ["premio", "Prêmio"], ["etapas", "Etapas"]];
  const TEXTOS = [["descricao", "Descrição (no diário)"], ["ok", "Ao aceitar"], ["nao", "Ao recusar"], ["tributo", "Tributo"]];
</script>

{#if erro}<p class="erro">{erro}</p>
{:else if !m}<p class="suave">Carregando a missão (a primeira vez lê o tasks.data inteiro)…</p>
{:else}
  <div class="missao">
    <div class="topo">
      <div class="titulo">
        <span class="mini">Missão {id}{m.mae ? ` · etapa de ${m.mae.id}` : ""}</span>
        <h2><TextoDoJogo texto={String(gerais.name ?? "")} padrao="var(--texto)" /></h2>
      </div>
      <span class="espaco"></span>
      {#if quantas}<span class="mini">{quantas} mudança(s) por gravar</span>{/if}
      <button class="botao" type="button" disabled={!quantas || gravando} onclick={carregar}>Descartar</button>
      <button class="botao" type="button" disabled={!editavel || !!m.mae} onclick={clonar} title={m.mae ? "Clone a missão de topo" : "Copia com as etapas"}><Icone nome="copiar" /> Clonar</button>
      <button class="botao primario" type="button" disabled={!editavel || !quantas || gravando} onclick={() => gravar()}><Icone nome="check" /> Gravar missão</button>
    </div>

    <div class="abas" role="tablist">
      {#each ABAS as [k, n] (k)}<button type="button" role="tab" aria-selected={aba === k} class:on={aba === k} onclick={() => (aba = k)}>{n}</button>{/each}
    </div>

    {#if aba === "geral"}
      <section class="caixa">
        <EditorDeTexto rotulo="Nome" valor={String(gerais.name ?? "")} limite={29} umaLinha desabilitado={!editavel} aoMudar={(v) => (gerais.name = v)} />
        <div class="grade-campos">
          {#each CAMPOS_GERAIS as c (c.campo)}
            <label><span>{c.rotulo}</span>
              {#if c.tipo === "sim"}
                <select class="campo" disabled={!editavel} value={gerais[c.campo] ? "1" : "0"} onchange={(e) => (gerais[c.campo] = (e.target as HTMLSelectElement).value === "1")}>
                  <option value="0">não</option><option value="1">sim</option></select>
              {:else}
                <input class="campo" type="number" disabled={!editavel} value={gerais[c.campo]} oninput={(e) => (gerais[c.campo] = Number((e.target as HTMLInputElement).value))} />
              {/if}
            </label>
          {/each}
        </div>
        <p class="mini">Só estes campos se editam no lugar (tasks_editavel::campos); o resto da ficha fica nos Editores → Missões.</p>
      </section>
    {:else if aba === "quem"}
      <section class="caixa">
        {#each [["npc_que_entrega", "Quem dá a missão"], ["npc_que_premia", "Quem recebe (premia)"]] as [c, rot] (c)}
          {@const v = Number(gerais[c] ?? 0)}
          <div class="npc">
            <span class="mini">{rot}</span>
            <strong>{nomeDe(v)}</strong> <span class="mini">{v || ""}</span>
            <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = { tipo: "npc", campo: c })}>Trocar</button>
            {#if v && indiceDe(v) != null}<a class="botao pequeno fantasma" href="#/npcs/NPC_ESSENCE/{indiceDe(v)}?{parametros()}" target="_blank" rel="noopener">Abrir o NPC</a>{/if}
          </div>
        {/each}
        <p class="mini">Isto é o que a missão diz. Para o jogador ver a missão no NPC, ela também precisa estar no serviço "Dar missão"/"Receber missão" dele
          (aba Missões do NPC).</p>
      </section>
    {:else if aba === "objetivo"}
      <section class="caixa">
        <h3>Monstros a matar <span class="mini">{vetores.monster_kills.length} de {teto("monster_kills")}</span></h3>
        <ul class="linhas">
          {#each vetores.monster_kills as x, k (k)}
            <li>
              <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = { tipo: "monstro", vetor: "monster_kills", linha: k })}>{nomeDe(x.monstro)}</button>
              <label class="mini">quantidade <input class="campo n" type="number" min="0" disabled={!editavel} value={x.quantidade} oninput={(e) => (x.quantidade = Number((e.target as HTMLInputElement).value))} /></label>
              {#if x.item_que_cai}<span class="mini">cai {nomeDe(x.item_que_cai)} × {x.quantidade_do_item} ({Math.round(x.chance_do_item * 100)}%)</span>{/if}
              <button class="botao pequeno fantasma" type="button" disabled={!editavel} aria-label="Tirar" onclick={() => vetores.monster_kills.splice(k, 1)}><Icone nome="lixeira" tamanho={16} /></button>
            </li>
          {/each}
        </ul>
        <button class="botao pequeno" type="button" disabled={!editavel || vetores.monster_kills.length >= teto("monster_kills")}
          onclick={() => (escolhendo = { tipo: "monstro", vetor: "monster_kills", linha: null })}><Icone nome="mais" tamanho={16} /> Monstro</button>
        {#each [["itens_exigidos", "Itens que o jogador precisa ter"], ["itens_entregues", "Itens que a missão entrega ao começar"]] as [v, rot] (v)}
          <h3 class="sep">{rot} <span class="mini">{vetores[v].length} de {teto(v)}</span></h3>
          <ul class="linhas">
            {#each vetores[v] as x, k (k)}
              <li>
                <button class="botao pequeno" type="button" disabled={!editavel} onclick={() => (escolhendo = { tipo: "item", vetor: v, linha: k })}>{nomeDe(x.id)}</button>
                <label class="mini">quantidade <input class="campo n" type="number" min="0" disabled={!editavel} value={x.quantidade} oninput={(e) => (x.quantidade = Number((e.target as HTMLInputElement).value))} /></label>
                <button class="botao pequeno fantasma" type="button" disabled={!editavel} aria-label="Tirar" onclick={() => vetores[v].splice(k, 1)}><Icone nome="lixeira" tamanho={16} /></button>
              </li>
            {/each}
          </ul>
          <button class="botao pequeno" type="button" disabled={!editavel || vetores[v].length >= teto(v)} onclick={() => (escolhendo = { tipo: "item", vetor: v, linha: null })}><Icone nome="mais" tamanho={16} /> Item</button>
        {/each}
        <p class="mini">Tetos do cliente (TaskInterface.h): 3 monstros e 10 itens. Lugares a alcançar e o resto do objetivo ficam nos Editores → Missões.</p>
      </section>
    {:else if aba === "textos"}
      <section class="caixa textos">
        {#each TEXTOS as [k, rot] (k)}
          <EditorDeTexto rotulo={rot} valor={textos[k] ?? ""} limite={0} linhas={k === "descricao" ? 6 : 3} desabilitado={!editavel} aoMudar={(v) => (textos[k] = v)} />
        {/each}
      </section>
    {:else if aba === "dialogos"}
      <div class="abas sub" role="tablist">
        {#each DIALOGOS as d, k (d.chave)}
          <button type="button" role="tab" aria-selected={dialogoAberto === k} class:on={dialogoAberto === k} onclick={() => (dialogoAberto = k)}>
            {d.nome} <span class="mini">{modelos[k]?.janelas.length ?? 0}</span></button>
        {/each}
      </div>
      {#if modelos[dialogoAberto]}
        {#key `${id}/${dialogoAberto}/${JSON.stringify(m.dialogos[dialogoAberto]).length}`}
          <EditorDeConversa modelo={modelos[dialogoAberto]} {funcoes} {editavel} estruturaLivre={false} nomeNaPrevia={nomeDe(Number(gerais.npc_que_entrega ?? 0))}
            rotuloDoSalvar="Gravar diálogo" aoSalvar={(c, novas) => gravarDialogo(dialogoAberto, c, novas)} />
        {/key}
        {#if !modelos[dialogoAberto].janelas.length}<p class="mini">Diálogo vazio: "+ Janela raiz" cria a primeira janela (id 1); as outras nascem de botões.</p>{/if}
      {/if}
    {:else if aba === "premio"}
      <section class="caixa">
        <div class="grade-campos">
          {#each CAMPOS_DO_PREMIO as c (c.campo)}
            <label><span>{c.rotulo}</span><input class="campo" type="number" disabled={!editavel} value={premio[c.campo]} oninput={(e) => (premio[c.campo] = Number((e.target as HTMLInputElement).value))} />
              {#if c.campo === "nova_missao" && premio.nova_missao}<button class="botao pequeno fantasma" type="button" onclick={() => aoAbrirOutra?.(premio.nova_missao)}>abrir {premio.nova_missao}</button>{/if}</label>
          {/each}
        </div>
        <h3 class="sep">Itens de prêmio</h3>
        {#each m.ficha.rewards?.grupos_de_itens ?? [] as g, k (k)}
          <p class="mini">Grupo {k + 1}{g.sorteia_um ? " (sorteia um)" : " (todos)"}: {g.itens.map((i: any) => `${nomeDe(i.id)} × ${i.quantidade}${g.sorteia_um ? ` (${Math.round(i.probabilidade * 100)}%)` : ""}`).join(", ")}</p>
        {:else}<p class="mini">Sem itens.</p>{/each}
        <p class="mini">Os grupos de itens se editam nos Editores → Missões (formato próprio, B265).</p>
      </section>
    {:else if aba === "etapas"}
      <section class="caixa">
        {#if m.mae}<p>Etapa de: <button class="botao pequeno" type="button" onclick={() => aoAbrirOutra?.(m.mae.id)}>{m.mae.id} · {m.mae.nome}</button></p>{/if}
        <h3>Etapas (filhas) <span class="mini">{m.filhas.length}</span></h3>
        <ul class="linhas">
          {#each m.filhas as f (f.id)}<li><button class="botao pequeno" type="button" onclick={() => aoAbrirOutra?.(f.id)}>{f.id}</button> <TextoDoJogo texto={f.nome ?? "(não existe)"} padrao="var(--texto)" /></li>
          {:else}<li class="mini">Sem etapas.</li>{/each}
        </ul>
        {#if m.pre_missoes.length}<h3 class="sep">Antes, precisa ter feito</h3>
          <ul class="linhas">{#each m.pre_missoes as f (f.id)}<li><button class="botao pequeno" type="button" onclick={() => aoAbrirOutra?.(f.id)}>{f.id}</button> {f.nome}</li>{/each}</ul>{/if}
      </section>
    {/if}
  </div>
{/if}

{#if escolhendo}
  {@const e = escolhendo}
  <SeletorDeEspaco titulo={e.tipo === "npc" ? "Escolher NPC" : e.tipo === "monstro" ? "Escolher monstro" : "Escolher item"} espaco="ESSENCE"
    categoriaInicial={e.tipo === "npc" ? "NPC_ESSENCE" : e.tipo === "monstro" ? "MONSTER_ESSENCE" : ""} soItens={e.tipo === "item"}
    aoFechar={() => (escolhendo = null)} aoEscolher={escolher} />
{/if}

<style>
  .missao { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .topo { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .titulo h2 { margin: 2px 0 0; font-size: 18px; }
  .espaco { flex: 1; }
  .abas { display: flex; gap: 4px; flex-wrap: wrap; border-bottom: 1px solid var(--borda); }
  .abas button { padding: 8px 12px; border: 0; border-bottom: 2px solid transparent; background: none; color: var(--texto-2); font: inherit; cursor: pointer; }
  .abas button.on { color: var(--texto); border-color: var(--ouro); font-weight: 600; }
  .abas.sub { border: 0; }
  .abas.sub button { border: 1px solid var(--borda-forte); border-radius: 8px; padding: 4px 10px; }
  .abas.sub button.on { border-color: var(--ouro); background: var(--ouro-suave); }
  .npc { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; padding: 8px 0; border-bottom: 1px solid var(--borda); }
  .linhas { list-style: none; margin: 6px 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .linhas li { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .n { width: 80px; height: 28px; }
  .sep { margin-top: 16px !important; }
  .textos { display: flex; flex-direction: column; gap: 14px; }
</style>
