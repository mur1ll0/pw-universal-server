<script lang="ts">
  /* Lista de cartões (efeito, %) que somam 100%. Mexer num controle reparte o resto entre os outros (`ajustar`); a
     unidade gravada é a do arquivo (`float` 0–1) e a regra é a do servidor (`lib/chances.ts`). */
  import type { Registro } from "../lib/api";
  import { acrescentar, ajustar, porcentagens, remover, type Cartao } from "../lib/chances";
  import Icone from "./Icone.svelte";
  import IconeItem from "./IconeItem.svelte";
  import { abrirItemEmNovaAba, esconderDica, mostrarDica } from "../lib/dica.svelte";
  import { avisar } from "../lib/estado.svelte";
  import SeletorDeEfeito from "./SeletorDeEfeito.svelte";
  import SeletorDeEspaco from "./SeletorDeEspaco.svelte";

  // `tabela`: com ela, o seletor lista só os efeitos que servem na família da peça (B368, `lib/efeitos.ts`).
  let { titulo, explica = "", espaco, tabela = "", maximo, linhas, editavel, aoMudar }:
    { titulo: string; explica?: string; espaco: string; tabela?: string; maximo: number; linhas: Cartao[]; editavel: boolean; aoMudar: (l: Cartao[]) => void } = $props();

  const pct = $derived(porcentagens(linhas.map((l) => l.peso)));
  let seletor = $state<null | { indice: number | null }>(null);

  function mover(i: number, permil: number) {
    const pesos = ajustar(linhas.map((l) => l.peso), i, permil / 1000);
    aoMudar(linhas.map((l, k) => ({ ...l, peso: pesos[k] })));
  }
  function tirar(i: number) {
    const pesos = remover(linhas.map((l) => l.peso), i);
    aoMudar(linhas.filter((_, k) => k !== i).map((l, k) => ({ ...l, peso: pesos[k] })));
  }
  function escolher(r: Pick<Registro, "id" | "nome"> & { texto?: string; icone?: string | null }) {
    if (!seletor || r.id == null) return;
    const descricao = r.texto || undefined;
    const icone = r.icone ?? null;
    if (seletor.indice === null) {
      const pesos = acrescentar(linhas.map((l) => l.peso));
      aoMudar([...linhas, { id: r.id, peso: 0, nome: r.nome, descricao, icone }].map((l, k) => ({ ...l, peso: pesos[k] })));
    } else {
      const i = seletor.indice;
      aoMudar(linhas.map((l, k) => (k === i ? { ...l, id: r.id!, nome: r.nome, descricao, icone } : l)));
    }
  }
  const fmt = (x: number) => x.toLocaleString("pt-BR", { maximumFractionDigits: 2 });
</script>

<section class="caixa">
  <h3>{titulo} <span class="mini">{linhas.length}/{maximo}</span></h3>
  {#if explica}<p class="mini">{explica}</p>{/if}
  <ul class="cartoes">
    {#each linhas as l, i (i)}
      <li class="cartao" role="group" onmouseenter={(e) => { if (espaco !== "ADDON") mostrarDica(l.id, l.nome, e.currentTarget as HTMLElement); }}
        onmouseleave={esconderDica}>
        <button class="nome" type="button" disabled={!editavel} title={espaco === "ADDON" ? "Trocar" : "Trocar o item"} onclick={() => (seletor = { indice: i })}>
          {#if espaco !== "ADDON"}<IconeItem hex={l.icone} nome={l.nome} tamanho={28} discreto />{/if}
          <span class="textos"><span>{l.descricao || l.nome || "(sem nome)"}</span>
            <span class="mini">{l.descricao ? `${l.nome} · ` : ""}id {l.id}{l.extra ? ` · ${l.extra}` : ""}</span></span>
        </button>
        <input type="range" min="0" max="1000" step="1" value={Math.round(pct[i] * 10)} disabled={!editavel || linhas.length < 2}
          aria-label="Chance de {l.descricao || l.nome}" oninput={(e) => mover(i, Number((e.target as HTMLInputElement).value))} />
        <span class="pct">{fmt(pct[i])}%</span>
        {#if espaco !== "ADDON"}
          <button class="botao fantasma pequeno" type="button" aria-label="Abrir o item em nova aba" title="Abrir o item em nova aba (edição no Estúdio)"
            onclick={() => abrirItemEmNovaAba(l.id).catch((e) => avisar("erro", (e as Error).message))}><Icone nome="abrir" tamanho={16} /></button>
          <button class="botao fantasma pequeno" type="button" disabled={!editavel} aria-label="Trocar o item" title="Trocar o item" onclick={() => (seletor = { indice: i })}><Icone nome="editar" tamanho={16} /></button>
        {/if}
        <button class="botao fantasma pequeno" type="button" disabled={!editavel} aria-label="Remover" title="Tirar da lista" onclick={() => tirar(i)}><Icone nome="lixeira" tamanho={16} /></button>
      </li>
    {:else}
      <li class="suave">Nenhum.</li>
    {/each}
  </ul>
  <div class="linha">
    <button class="botao pequeno" type="button" disabled={!editavel || linhas.length >= maximo} onclick={() => (seletor = { indice: null })}><Icone nome="mais" tamanho={16} /> Adicionar</button>
    {#if linhas.length}<span class="mini">total {fmt(pct.reduce((s, x) => s + x, 0))}%</span>{/if}
  </div>
</section>

{#if seletor}
  {#if tabela}
    <SeletorDeEfeito titulo="{titulo}: escolher efeito" {tabela} aoEscolher={escolher} aoFechar={() => (seletor = null)} />
  {:else}
    <SeletorDeEspaco titulo="{titulo}: escolher efeito" {espaco} aoEscolher={escolher} aoFechar={() => (seletor = null)} />
  {/if}
{/if}

<style>
  .cartoes { list-style: none; margin: 0 0 10px; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .cartao { display: grid; grid-template-columns: minmax(0, 1fr) minmax(90px, 160px) 64px auto auto auto; align-items: center; gap: 8px;
    padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .textos { display: flex; flex-direction: column; min-width: 0; }
  .nome { display: flex; flex-direction: row; gap: 8px; align-items: center; background: none; border: 0; color: var(--texto); font: inherit; cursor: pointer; text-align: left; min-width: 0; }
  .nome span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; }
  .nome:disabled { cursor: default; }
  .pct { text-align: right; font-variant-numeric: tabular-nums; }
  input[type="range"] { width: 100%; accent-color: var(--ouro); }
</style>
