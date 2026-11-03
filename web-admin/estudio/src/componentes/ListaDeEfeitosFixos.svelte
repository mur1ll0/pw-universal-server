<script lang="ts">
  /* B370: efeitos fixos de um item (só ids, sem chance): `BIBLE_ESSENCE.id_addons_N`, `PROJECTILE_ESSENCE.id_addonN`,
     `POKER_ESSENCE.addon_N`. Grava compactado (ids no começo, zeros no fim), como as listas com chance. */
  import type { EfeitoFixo } from "../lib/chances";
  import Icone from "./Icone.svelte";
  import SeletorDeEfeito from "./SeletorDeEfeito.svelte";

  let { titulo, tabela, maximo, linhas, editavel, aoMudar }:
    { titulo: string; tabela: string; maximo: number; linhas: EfeitoFixo[]; editavel: boolean; aoMudar: (l: EfeitoFixo[]) => void } = $props();
  let seletor = $state<null | { indice: number | null }>(null);
</script>

<section class="caixa">
  <h3>{titulo} <span class="mini">{linhas.length}/{maximo}</span></h3>
  <ul class="cartoes">
    {#each linhas as l, i (i)}
      <li class="cartao">
        <button class="nome" type="button" disabled={!editavel} title="Trocar o efeito" onclick={() => (seletor = { indice: i })}>
          <span>{l.descricao || l.nome || "(sem nome)"}</span><span class="mini">{l.descricao ? `${l.nome} · ` : ""}id {l.id}</span>
        </button>
        <button class="botao fantasma pequeno" type="button" disabled={!editavel} aria-label="Remover" onclick={() => aoMudar(linhas.filter((_, k) => k !== i))}><Icone nome="lixeira" tamanho={16} /></button>
      </li>
    {:else}
      <li class="suave">Nenhum.</li>
    {/each}
  </ul>
  <button class="botao pequeno" type="button" disabled={!editavel || linhas.length >= maximo} onclick={() => (seletor = { indice: null })}><Icone nome="mais" tamanho={16} /> Adicionar</button>
</section>

{#if seletor}
  <SeletorDeEfeito titulo="{titulo}: escolher efeito" {tabela} aoFechar={() => (seletor = null)}
    aoEscolher={(e) => {
      const novo = { id: e.id, nome: e.nome, descricao: e.texto || undefined };
      const i = seletor?.indice;
      aoMudar(i == null ? [...linhas, novo] : linhas.map((l, k) => (k === i ? novo : l)));
    }} />
{/if}

<style>
  .cartoes { list-style: none; margin: 0 0 10px; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .cartao { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border: 1px solid var(--borda); border-radius: 10px; background: var(--fundo-2); }
  .nome { flex: 1; display: flex; flex-direction: column; align-items: flex-start; background: none; border: 0; color: var(--texto); font: inherit; cursor: pointer; text-align: left; min-width: 0; }
  .nome:disabled { cursor: default; }
</style>
