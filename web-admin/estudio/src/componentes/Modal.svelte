<script lang="ts">
  import type { Snippet } from "svelte";
  import Icone from "./Icone.svelte";

  let { titulo, aoFechar, largo = false, children, rodape }:
    { titulo: string; aoFechar: () => void; largo?: boolean; children: Snippet; rodape?: Snippet } = $props();

  function tecla(e: KeyboardEvent) {
    if (e.key === "Escape") aoFechar();
  }
</script>

<svelte:window onkeydown={tecla} />

<div class="fundo" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) aoFechar(); }}>
  <div class="janela" class:largo role="dialog" aria-modal="true" aria-label={titulo}>
    <header>
      <h2>{titulo}</h2>
      <button class="botao fantasma" type="button" onclick={aoFechar} aria-label="Fechar"><Icone nome="fechar" /></button>
    </header>
    <div class="corpo">{@render children()}</div>
    {#if rodape}<footer>{@render rodape()}</footer>{/if}
  </div>
</div>

<style>
  .fundo { position: fixed; inset: 0; background: rgba(0, 0, 0, .55); display: grid; place-items: center; z-index: 50; padding: 16px; }
  .janela { background: var(--superficie); border: 1px solid var(--borda); border-radius: var(--raio-g); box-shadow: var(--sombra);
    width: min(560px, 100%); max-height: calc(100vh - 32px); display: flex; flex-direction: column; }
  .janela.largo { width: min(900px, 100%); }
  header { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--borda); }
  h2 { margin: 0; font-size: 16px; }
  .corpo { padding: 16px; overflow: auto; }
  footer { padding: 12px 16px; border-top: 1px solid var(--borda); display: flex; gap: 8px; justify-content: flex-end; flex-wrap: wrap; }
</style>
