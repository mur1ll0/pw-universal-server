<script lang="ts">
  /* B392: busca, criação e edição de serviços de NPC. Esquerda: categorias (`lib/servicos.ts`) com quantos registros
     cada tabela tem; meio: busca (id, nome ou o que o serviço lista — item vendido, receita) e a lista com quantos NPCs
     usam cada um (`GET servicos/{T}`, `Estudio::servicos`); direita: o editor (`EditorDeServico`).
     Com `vinculo` (aberto de dentro de um NPC) o rodapé vincula o serviço escolhido ao NPC — ou uma cópia só para ele,
     quando o serviço é compartilhado. Novo = clone do primeiro registro da tabela com as listas zeradas; Duplicar =
     clone do escolhido. Tudo no rascunho (`POST tabelas/{T}/{i}/clonar` + `PUT`). */
  import { clonar, ficha as lerFicha, listarServicos, salvar, tabelas, type ServicoListado } from "../lib/api";
  import { avisar, estudio } from "../lib/estado.svelte";
  import { CATEGORIAS_DE_SERVICO, GRUPOS_DE_CATEGORIA, NOME_DA_TABELA, type CategoriaDeServico, type Vinculo } from "../lib/servicos";
  import { SIMPLES } from "../lib/npc";
  import EditorDeServico from "./EditorDeServico.svelte";
  import Icone from "./Icone.svelte";
  import TextoDoJogo from "./TextoDoJogo.svelte";


  let { categoriaInicial = "loja", tabelaInicial = "", indiceInicial = null, vinculo = null, aoAbrir }:
    { categoriaInicial?: string; tabelaInicial?: string; indiceInicial?: number | null; vinculo?: Vinculo | null;
      aoAbrir?: (tabela: string, indice: number) => void } = $props();
  const editavel = $derived(!!estudio.rascunho);

  let contagem = $state<Record<string, number>>({});
  tabelas().then((t) => (contagem = Object.fromEntries(t.tabelas.map((x) => [x.nome, x.registros])))).catch(() => {});
  const categorias = $derived(CATEGORIAS_DE_SERVICO.filter((c) => c.simples || !Object.keys(contagem).length || c.tabelas.some((t) => t in contagem)));

  const inicial = CATEGORIAS_DE_SERVICO.find((c) => (tabelaInicial ? c.tabelas.includes(tabelaInicial) : c.chave === categoriaInicial)) ?? CATEGORIAS_DE_SERVICO[1];
  let cat = $state<CategoriaDeServico>(inicial);
  let tabela = $state<string>(tabelaInicial || inicial.tabelas[0] || "");
  let busca = $state("");
  let uso = $state<"" | "em_uso" | "sem_uso">("");
  let pagina = $state(0);
  let total = $state(0);
  let lista = $state<ServicoListado[]>([]);
  let escolhido = $state<{ indice: number; id: number; nome: string; npcs: number } | null>(indiceInicial != null ? { indice: indiceInicial, id: 0, nome: "", npcs: 0 } : null);
  let ocupado = $state(false);
  const POR_PAGINA = 40;

  async function buscar() {
    if (!tabela) { lista = []; total = 0; return; }
    try {
      const r = await listarServicos(tabela, busca.trim(), uso, pagina, POR_PAGINA);
      lista = r.registros;
      total = r.total;
      if (escolhido && !escolhido.id) {
        const x = r.registros.find((s) => s.indice === escolhido!.indice);
        if (x) escolhido = { indice: x.indice, id: x.id, nome: x.nome, npcs: x.npcs };
      }
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { void tabela; void uso; void pagina; void estudio.rascunho; buscar(); });
  let espera: ReturnType<typeof setTimeout> | undefined;
  function digitar() { clearTimeout(espera); espera = setTimeout(() => { pagina = 0; buscar(); }, 300); }
  function trocarCategoria(c: CategoriaDeServico) { cat = c; tabela = c.tabelas[0] ?? ""; pagina = 0; escolhido = null; busca = ""; }
  function abrir(s: ServicoListado) { escolhido = { indice: s.indice, id: s.id, nome: s.nome, npcs: s.npcs }; aoAbrir?.(tabela, s.indice); }

  const resumo = (s: ServicoListado) => {
    const p: string[] = [];
    if (s.resumo.abas) p.push(`${s.resumo.abas} aba(s)`);
    if (s.resumo.itens) p.push(`${s.resumo.itens} ${tabela === "NPC_MAKE_SERVICE" ? "receita(s)" : "item(ns)"}`);
    if (s.resumo.destinos) p.push(`${s.resumo.destinos} destino(s)`);
    if (s.resumo.lista) p.push(`${s.resumo.lista} na lista`);
    return p.join(" · ");
  };

  /** Clona e devolve o registro novo; com `zerar`, as listas do clone ficam vazias (abas, itens, destinos, missões). */
  async function criarClone(indice: number, nome: string, zerar: boolean) {
    const c = await clonar(tabela, indice);
    const novo = c.resultado;
    const campos: Record<string, number | string> = { Name: nome };
    if (zerar) {
      const f = await lerFicha(tabela, novo.indice);
      for (const x of f.campos) {
        if (x.nome === "ID" || x.nome === "Name" || !/\d/.test(x.nome)) continue;
        campos[x.nome] = x.tipo === "int32" || x.tipo === "float" ? 0 : "";
      }
      if (f.campos.some((x) => x.nome === "num_targets")) campos.num_targets = 0;
    }
    await salvar(tabela, novo.indice, campos);
    return novo;
  }
  async function novo() {
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para criar."); return; }
    ocupado = true;
    try {
      const primeiro = (await listarServicos(tabela, "", "", 0, 1)).registros[0];
      if (!primeiro) throw new Error(`${tabela} não tem registro para servir de base.`);
      const n = await criarClone(primeiro.indice, `Novo ${cat.nome}`.slice(0, 31), true);
      avisar("ok", `Serviço novo id ${n.id} no rascunho (cópia vazia do ${primeiro.id}).`);
      busca = String(n.id); pagina = 0; await buscar();
      escolhido = { indice: n.indice, id: n.id, nome: `Novo ${cat.nome}`, npcs: 0 };
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }
  async function duplicar(soParaONpc = false) {
    if (!escolhido) return;
    if (!editavel) { avisar("aviso", "Escolha um rascunho no alto para duplicar."); return; }
    ocupado = true;
    try {
      const nome = soParaONpc && vinculo ? `${escolhido.nome}`.slice(0, 31) : `${escolhido.nome} (cópia)`.slice(0, 31);
      const n = await criarClone(escolhido.indice, nome, false);
      avisar("ok", `Cópia id ${n.id} no rascunho.`);
      if (soParaONpc && vinculo) { vinculo.aoVincular(tabela, n.id, nome); return; }
      busca = String(n.id); pagina = 0; await buscar();
      escolhido = { indice: n.indice, id: n.id, nome, npcs: 0 };
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }
</script>

<div class="painel">
  <!-- 1. Categorias -->
  <nav class="categorias" aria-label="Categorias de serviço">
    {#each GRUPOS_DE_CATEGORIA as g (g)}
      {@const lista_g = categorias.filter((c) => c.grupo === g)}
      {#if lista_g.length}
        <span class="grupo">{g}</span>
        {#each lista_g as c (c.chave)}
          <button type="button" class:on={cat.chave === c.chave} onclick={() => trocarCategoria(c)}>
            <span>{c.nome}</span><span class="q">{c.simples ? "simples" : c.tabelas.reduce((a, t) => a + (contagem[t] ?? 0), 0)}</span>
          </button>
        {/each}
      {/if}
    {/each}
  </nav>

  {#if cat.simples}
    <!-- Serviço simples: um bit do NPC -->
    <section class="simples">
      <h3>{cat.nome}</h3>
      <p class="mini">{cat.explica}</p>
      <ul>
        {#each cat.simples.bits as b (b)}
          {@const s = SIMPLES.find((x) => x.bit === b)}
          <li>
            <span><strong>{s?.nome ?? `bit ${b}`}</strong> <span class="mini">combined_services bit {b}{s?.atende === "sim" ? " · o servidor atende" : ""}</span></span>
            {#if vinculo?.aoLigarBit}<button class="botao pequeno primario" type="button" onclick={() => vinculo?.aoLigarBit?.(cat.simples!.campo, b)}>Ligar em {vinculo.npc}</button>{/if}
          </li>
        {/each}
      </ul>
      {#if !vinculo}<p class="mini">Ligue na aba Serviços do NPC (seção "Serviços simples").</p>{/if}
    </section>
  {:else}
    <!-- 2. Busca e lista -->
    <section class="lista">
      {#if cat.tabelas.length > 1}
        <div class="sub" role="tablist">
          {#each cat.tabelas.filter((t) => !Object.keys(contagem).length || t in contagem) as t (t)}
            <button type="button" role="tab" aria-selected={tabela === t} class:on={tabela === t} onclick={() => { tabela = t; pagina = 0; escolhido = null; }}>{NOME_DA_TABELA[t] ?? t}</button>
          {/each}
        </div>
      {/if}
      <input class="campo" placeholder="id, nome ou o que ele lista (item, receita)" bind:value={busca} oninput={digitar} aria-label="Buscar serviço" />
      <div class="filtros" role="group" aria-label="Uso">
        {#each [["", "Todos"], ["em_uso", "Em uso"], ["sem_uso", "Sem uso"]] as [v, n] (v)}
          <button type="button" class:on={uso === v} onclick={() => { uso = v as typeof uso; pagina = 0; }}>{n}</button>
        {/each}
        <span class="mini">{total}</span>
      </div>
      <ul>
        {#each lista as s (s.indice)}
          <li><button type="button" class:on={escolhido?.indice === s.indice} onclick={() => abrir(s)}>
            <span class="l1"><span class="nome"><TextoDoJogo texto={s.nome || "(sem nome)"} padrao="var(--texto)" linha /></span><span class="mini">{s.id}</span></span>
            <span class="mini">{resumo(s) || "—"}{s.achado_na_lista ? " · achado no que ele lista" : ""}</span>
            <span class="mini" class:livre={!s.npcs}>{s.npcs ? `${s.npcs} NPC(s): ${s.nomes_dos_npcs.join(", ")}${s.npcs > s.nomes_dos_npcs.length ? "…" : ""}` : "sem uso"}</span>
          </button></li>
        {:else}
          <li class="mini">Nada encontrado.</li>
        {/each}
      </ul>
      <div class="linha">
        <button class="botao pequeno" type="button" disabled={pagina === 0} onclick={() => pagina--}>‹</button>
        <span class="mini">{pagina + 1}/{Math.max(1, Math.ceil(total / POR_PAGINA))}</span>
        <button class="botao pequeno" type="button" disabled={(pagina + 1) * POR_PAGINA >= total} onclick={() => pagina++}>›</button>
        <span class="espaco"></span>
        <button class="botao pequeno" type="button" disabled={!editavel || ocupado} onclick={novo}><Icone nome="mais" tamanho={16} /> Novo</button>
        <button class="botao pequeno" type="button" disabled={!editavel || ocupado || !escolhido} onclick={() => duplicar(false)}><Icone nome="copiar" tamanho={16} /> Duplicar</button>
      </div>
    </section>

    <!-- 3. Editor -->
    <section class="editor">
      {#if escolhido}
        {#key `${tabela}/${escolhido.indice}`}
          <EditorDeServico {tabela} indice={escolhido.indice} aoGravado={() => buscar()} />
        {/key}
      {:else}
        <p class="suave vazio">Escolha um serviço na lista, ou crie um novo.</p>
      {/if}
    </section>
  {/if}
</div>

{#if vinculo && escolhido && !cat.simples}
  <div class="rodape">
    {#if escolhido.npcs > 0}
      <span class="mini aviso">Compartilhado por {escolhido.npcs} NPC(s): mudar o serviço muda em todos.</span>
      <button class="botao" type="button" disabled={!editavel || ocupado} onclick={() => duplicar(true)}><Icone nome="copiar" /> Vincular uma cópia só para {vinculo.npc}</button>
    {/if}
    <button class="botao primario" type="button" disabled={!editavel || ocupado} onclick={() => vinculo?.aoVincular(tabela, escolhido!.id, escolhido!.nome)}>
      <Icone nome="check" /> Vincular a {vinculo.npc}</button>
  </div>
{/if}

<style>
  .painel { display: grid; grid-template-columns: 200px minmax(240px, 300px) minmax(0, 1fr); gap: 12px; min-height: 520px; align-items: start; }
  .categorias { display: flex; flex-direction: column; gap: 2px; position: sticky; top: 0; }
  .grupo { font-size: 10px; text-transform: uppercase; letter-spacing: .08em; color: var(--texto-3); margin: 8px 0 2px; }
  .categorias button { display: flex; justify-content: space-between; gap: 6px; padding: 5px 8px; border-radius: 6px; border: 0; background: none; color: var(--texto-2);
    font: inherit; font-size: 13px; cursor: pointer; text-align: left; }
  .categorias button.on { background: var(--ouro-suave); color: var(--ouro); font-weight: 600; }
  .q { font-variant-numeric: tabular-nums; color: var(--texto-3); font-size: 12px; }
  .lista { display: flex; flex-direction: column; gap: 6px; }
  .sub, .filtros { display: flex; gap: 4px; flex-wrap: wrap; align-items: center; }
  .sub button, .filtros button { padding: 3px 9px; border-radius: 999px; border: 1px solid var(--borda-forte); background: var(--fundo-2); color: var(--texto-2); font: inherit; font-size: 12px; cursor: pointer; }
  .sub button.on, .filtros button.on { border-color: var(--ouro); color: var(--ouro); background: var(--ouro-suave); }
  .lista ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 520px; overflow: auto; }
  .lista li > button { width: 100%; display: flex; flex-direction: column; gap: 2px; text-align: left; padding: 7px 9px; border-radius: 8px; border: 1px solid var(--borda);
    background: var(--superficie); color: var(--texto); font: inherit; cursor: pointer; }
  .lista li > button.on { border-color: var(--ouro); }
  .l1 { display: flex; justify-content: space-between; gap: 6px; }
  .nome { min-width: 0; font-weight: 600; }
  .livre { color: var(--texto-3); font-style: italic; }
  .espaco { flex: 1; }
  .editor { min-width: 0; }
  .vazio { padding: 40px 0; text-align: center; }
  .simples { grid-column: span 2; }
  .simples ul { list-style: none; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .simples li { display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 8px 10px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .rodape { display: flex; gap: 8px; justify-content: flex-end; align-items: center; flex-wrap: wrap; padding-top: 12px; margin-top: 12px; border-top: 1px solid var(--borda); }
  .aviso { color: var(--azul); }
  @media (max-width: 1100px) { .painel { grid-template-columns: 1fr; } .categorias { position: static; flex-direction: row; flex-wrap: wrap; } }
</style>
