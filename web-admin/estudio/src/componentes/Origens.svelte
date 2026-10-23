<script lang="ts">
  /* B369: quatro cards com a origem do item — dropado de, recompensa de, vendido por, forjado por — pela rota
     `GET estudio/origens/{id}` (`crates/pw-recursos/src/estudio.rs::origens_do_item`), no rascunho escolhido. Cada
     monstro, NPC, loja, receita e forja abre nos Editores em outra aba. */
  import { linkDosEditores, origensDoItem, type Origens, type RefDeOrigem } from "../lib/api";
  import IconeItem from "./IconeItem.svelte";

  let { id }: { id: number } = $props();
  let o = $state<Origens | null>(null);
  let erro = $state("");
  $effect(() => {
    o = null; erro = "";
    if (id > 0) origensDoItem(id).then((x) => (o = x)).catch((e) => (erro = (e as Error).message));
  });

  // Nomes de missão e de NPC trazem os códigos de cor do cliente (`^RRGGBB`): fora, para a lista.
  const limpo = (t: string | null | undefined) => (t ?? "").replace(/\^[0-9a-fA-F]{6}/g, "").trim();
  const pct = (x: number | null | undefined) => (x == null ? "" : `${(x * 100).toLocaleString("pt-BR", { maximumFractionDigits: 3 })}%`);
  const paginas = (p: { pagina: number; titulo: string }[]) => p.map((x) => x.titulo || `página ${x.pagina}`).join(", ");
  const vendas = $derived(o ? o.vendido_por.length + o.loja_gold.length : 0);
</script>

{#snippet ref(r: RefDeOrigem, extra = "")}
  <a class="ref" href={linkDosEditores(r.tabela, r.indice)} target="_blank" rel="noopener" title="Abrir nos Editores: {r.tabela} id {r.id}">
    <IconeItem hex={r.icone} nome={r.nome} tamanho={24} discreto />
    <span class="nome">{limpo(r.nome) || `(sem nome)`}</span>
    <span class="mini">{extra ? `${extra} · ` : ""}id {r.id}</span>
  </a>
{/snippet}

<div class="origens">
  <section class="caixa">
    <h3>Dropado de {#if o}<span class="mini">{o.dropado_de.length}</span>{/if}</h3>
    {#if erro}<p class="erro">{erro}</p>{:else if !o}<p class="suave">Carregando…</p>
    {:else if !o.dropado_de.length}<p class="suave">Nenhum monstro dropa este item.</p>
    {:else}
      <ul>{#each o.dropado_de as m (m.indice)}<li>{@render ref(m, `chance na lista ${pct(m.chance)}${m.linhas > 1 ? ` (${m.linhas} linhas)` : ""}`)}</li>{/each}</ul>
      <p class="mini">Chance = peso do item na lista de drops do monstro (MONSTER_ESSENCE.drop_matters).</p>
    {/if}
  </section>

  <section class="caixa">
    <h3>Recompensa de {#if o}<span class="mini">{o.recompensa_de.length}</span>{/if}</h3>
    {#if erro}<p class="erro">{erro}</p>{:else if !o}<p class="suave">Carregando… (o tasks.data é grande)</p>
    {:else if !o.recompensa_de.length}<p class="suave">Nenhuma missão dá este item.</p>
    {:else}
      <ul>
        {#each o.recompensa_de as m, k (k)}
          <li class="linha-simples">
            <span class="nome">{limpo(m.nome) || "(sem nome)"}</span>
            <span class="mini">{m.quando} · {m.quantidade}×{m.sorteio ? " · sorteado no grupo" : ""} · missão {m.id}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="caixa">
    <h3>Vendido por {#if o}<span class="mini">{vendas}</span>{/if}</h3>
    {#if erro}<p class="erro">{erro}</p>{:else if !o}<p class="suave">Carregando…</p>
    {:else if !vendas}<p class="suave">Nenhuma loja vende este item.</p>
    {:else}
      <ul>
        {#each o.loja_gold as g (`${g.arquivo}/${g.indice}`)}
          <li class="linha-simples">
            <span class="nome">Loja Gold · {limpo(g.nome)}</span>
            <span class="mini">{g.arquivo} · oferta {g.indice} · {g.quantidade}× · {g.preco?.toLocaleString("pt-BR")} (buy_0_price, valor do arquivo)</span>
          </li>
        {/each}
        {#each o.vendido_por as l (l.indice)}
          <li>
            {#each l.npcs as n (n.indice)}{@render ref(n, `loja ${limpo(l.nome)}`)}{:else}<span class="mini">nenhum NPC tem esta loja</span>{/each}
            <div class="sub">{@render ref(l, paginas(l.paginas))}</div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="caixa">
    <h3>Forjado por {#if o}<span class="mini">{o.forjado_por.length}</span>{/if}</h3>
    {#if erro}<p class="erro">{erro}</p>{:else if !o}<p class="suave">Carregando…</p>
    {:else if !o.forjado_por.length}<p class="suave">Nenhuma receita produz este item.</p>
    {:else}
      <ul>
        {#each o.forjado_por as r (r.indice)}
          <li>
            {#each r.forjas as f (f.indice)}
              {#each f.npcs as n (n.indice)}{@render ref(n, `forja ${limpo(f.nome)}`)}{:else}<span class="mini">forja {f.nome}: nenhum NPC a tem</span>{/each}
              <div class="sub">{@render ref(f, paginas(f.paginas))}</div>
            {:else}
              <span class="mini">nenhuma forja tem esta receita</span>
            {/each}
            <div class="sub">{@render ref(r, `receita${r.chance != null ? ` · chance ${pct(r.chance)}` : ""}`)}</div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .origens { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 14px; margin-top: 14px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; max-height: 340px; overflow: auto; }
  li { display: flex; flex-direction: column; gap: 3px; padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .ref { display: flex; align-items: center; gap: 8px; color: var(--texto); text-decoration: none; min-width: 0; }
  .ref:hover .nome { color: var(--ouro); }
  .nome { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ref .mini { margin-left: auto; white-space: nowrap; }
  .sub { padding-left: 32px; font-size: 13px; }
  .linha-simples { gap: 2px; }
</style>
