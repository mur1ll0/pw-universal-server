<script lang="ts">
  import { urlDoIcone } from "../lib/api";
  // `discreto`: sem ícone (ou fora do atlas de itens, como o de NPC) não desenha nada.
  let { hex, nome = "", tamanho = 32, discreto = false }: { hex: string | null | undefined; nome?: string; tamanho?: number; discreto?: boolean } = $props();
  let falhou = $state(false);
  $effect(() => { hex; falhou = false; });
</script>

{#if hex && !falhou}
  <img class="icone-item" src={urlDoIcone(hex)} alt={nome} width={tamanho} height={tamanho} onerror={() => (falhou = true)} />
{:else if !discreto}
  <span class="icone-item vazio" style:width="{tamanho}px" style:height="{tamanho}px" title={hex ? "ícone fora do atlas" : "sem ícone"}>?</span>
{/if}

<style>
  .icone-item { border-radius: 6px; image-rendering: pixelated; flex: none; background: var(--fundo-2); }
  .vazio { display: inline-grid; place-items: center; color: var(--texto-3); border: 1px dashed var(--borda-forte); font-size: 12px; }
</style>
