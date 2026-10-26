<script lang="ts">
  /* Busca de uma categoria: subcategoria → lista paginada (id ou nome, com ícone) → Abrir / Clonar / Novo.
     "Novo" (decisão de 2026-10-23) clona o registro de menor id da subcategoria e abre a edição com o nome "Novo …".
     Tudo no rascunho (`POST tabelas/{T}/{i}/clonar`, o mesmo dos Editores). */
  import { clonar, listar, salvar, tabelas, type Registro } from "../lib/api";
  import type { Categoria } from "../lib/catalogo";
  import { avisar, estudio, ir } from "../lib/estado.svelte";
  import Icone from "./Icone.svelte";
  import IconeItem from "./IconeItem.svelte";
  import Modal from "./Modal.svelte";

  let { categoria, aoFechar }: { categoria: Categoria; aoFechar: () => void } = $props();

  const POR_PAGINA = 40;
  let existentes = $state<Set<string>>(new Set());
  let subs = $derived(categoria.subcategorias.filter((s) => (!s.so || s.so === estudio.versao) && (!existentes.size || existentes.has(s.tabela))));
  let chave = $state(categoria.subcategorias[0]?.chave ?? "");
  let sub = $derived(subs.find((s) => s.chave === chave) ?? subs[0]);
  const grupos = $derived.by(() => {
    const g = new Map<string, typeof subs>();
    for (const x of subs) g.set(x.grupo ?? "", [...(g.get(x.grupo ?? "") ?? []), x]);
    return [...g.entries()];
  });
  let busca = $state("");
  let pagina = $state(0);
  let total = $state(0);
  let registros = $state<Registro[]>([]);
  let escolhido = $state<Registro | null>(null);
  let ocupado = $state(false);

  tabelas().then((t) => (existentes = new Set(t.tabelas.map((x) => x.nome)))).catch(() => {});

  let espera: ReturnType<typeof setTimeout> | undefined;
  async function buscar() {
    if (!sub) return;
    try {
      const r = await listar(sub.tabela, busca.trim(), pagina, POR_PAGINA);
      registros = r.registros;
      total = r.total;
      if (escolhido && !registros.some((x) => x.indice === escolhido!.indice)) escolhido = null;
    } catch (e) { avisar("erro", (e as Error).message); }
  }
  $effect(() => { void sub; void pagina; void estudio.rascunho; buscar(); });
  function digitar() { clearTimeout(espera); espera = setTimeout(() => { pagina = 0; buscar(); }, 250); }

  const paginas = $derived(Math.max(1, Math.ceil(total / POR_PAGINA)));
  const editavel = $derived(!!estudio.rascunho);

  function abrir(r: Registro | null = escolhido, extra: Record<string, string | number> = {}) {
    if (!r || !sub) return;
    if (!sub.pronta) { avisar("aviso", `${sub.nome}: tela própria em breve. Edite nos Editores.`); return; }
    // A tabela é lida antes de fechar: fechado o modal, as derivações dele deixam de valer.
    const destino = `${categoria.chave}/${sub.tabela}/${r.indice}`;
    aoFechar();
    ir(destino, extra);
  }

  async function clonarEscolhido(r: Registro | null = escolhido, renomear?: string) {
    if (!r || !sub) return;
    if (!editavel) { avisar("aviso", "Escolha ou crie um rascunho no alto para clonar."); return; }
    ocupado = true;
    try {
      const c = await clonar(sub.tabela, r.indice);
      if (renomear) await salvar(sub.tabela, c.resultado.indice, { Name: renomear });
      avisar("ok", `Registro novo com id ${c.resultado.id} no rascunho.`);
      // "Novo": a edição abre com as listas de efeitos limpas (só na tela; gravar confirma).
      abrir({ ...r, indice: c.resultado.indice, id: c.resultado.id }, renomear ? { novo: 1 } : {});
    } catch (e) { avisar("erro", (e as Error).message); }
    finally { ocupado = false; }
  }

  async function novo() {
    if (!sub) return;
    // Base: o registro de menor id da subcategoria (até os 200 primeiros do arquivo).
    try {
      const r = await listar(sub.tabela, "", 0, 200);
      const base = r.registros.filter((x) => x.id).sort((a, b) => (a.id ?? 0) - (b.id ?? 0))[0];
      if (!base) { avisar("aviso", "Subcategoria sem registro para servir de base."); return; }
      await clonarEscolhido(base, `Novo (${sub.nome})`);
    } catch (e) { avisar("erro", (e as Error).message); }
  }
</script>

<Modal titulo="{categoria.nome} · buscar" aoFechar={aoFechar} largo>
  <div class="linha">
    <label class="suave" for="sub">Subcategoria</label>
    <select id="sub" class="campo" bind:value={chave} onchange={() => { pagina = 0; escolhido = null; }}>
      {#if subs.some((s) => s.grupo)}
        <!-- Itens: subcategorias agrupadas como o jogador entende (lib/itens.ts). -->
        {#each grupos as [g, lista] (g)}
          <optgroup label={g}>{#each lista as s (s.chave)}<option value={s.chave}>{s.nome}</option>{/each}</optgroup>
        {/each}
      {:else}
        {#each subs as s (s.chave)}<option value={s.chave}>{s.nome}{s.pronta ? "" : " (Editores)"}</option>{/each}
      {/if}
    </select>
    <input class="campo busca" placeholder="id ou nome" bind:value={busca} oninput={digitar} aria-label="Buscar por id ou nome" />
  </div>
  {#if sub?.oQueE}<p class="mini">{sub.tabela} · {sub.oQueE}</p>{/if}
  {#if sub && !sub.pronta}
    <p class="mini">{sub.nome} ({sub.tabela}) ainda não tem tela própria no Estúdio: a lista serve para achar o id; a edição é nos Editores.</p>
  {/if}
  <ul class="lista" role="listbox" aria-label="Registros">
    {#each registros as r (r.indice)}
      <li>
        <button type="button" class:ativo={escolhido?.indice === r.indice} role="option" aria-selected={escolhido?.indice === r.indice}
          onclick={() => (escolhido = r)} ondblclick={() => abrir(r)}>
          <IconeItem hex={r.icone} nome={r.nome} />
          <span class="nome">{r.nome || `(registro ${r.indice})`}</span>
          <span class="mini">id {r.id ?? "—"}</span>
        </button>
      </li>
    {:else}
      <li class="suave">Nada encontrado.</li>
    {/each}
  </ul>
  <div class="linha paginas">
    <span class="mini">{total} registro(s) · página {pagina + 1} de {paginas}</span>
    <button class="botao pequeno" type="button" disabled={pagina === 0} onclick={() => pagina--}>‹</button>
    <button class="botao pequeno" type="button" disabled={pagina + 1 >= paginas} onclick={() => pagina++}>›</button>
  </div>

  {#snippet rodape()}
    {#if !editavel}<span class="mini">Clonar e Novo pedem um rascunho.</span>{/if}
    <button class="botao" type="button" disabled={!editavel || ocupado || !sub?.pronta} onclick={novo}><Icone nome="mais" /> Novo</button>
    <button class="botao" type="button" disabled={!escolhido || !editavel || ocupado || !sub?.pronta} onclick={() => clonarEscolhido()}><Icone nome="copiar" /> Clonar</button>
    <button class="botao primario" type="button" disabled={!escolhido || !sub?.pronta} onclick={() => abrir()}><Icone nome="abrir" /> Abrir</button>
  {/snippet}
</Modal>

<style>
  .busca { flex: 1; min-width: 160px; }
  .lista { list-style: none; margin: 12px 0 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 6px; }
  .lista button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--borda);
    background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; text-align: left; }
  .lista button:hover { border-color: var(--borda-forte); }
  .lista button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .nome { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .paginas { justify-content: flex-end; margin-top: 8px; }
</style>
