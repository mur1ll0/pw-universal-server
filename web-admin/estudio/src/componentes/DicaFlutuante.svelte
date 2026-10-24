<script lang="ts">
  import { dica, partes } from "../lib/dica.svelte";
</script>

{#if dica.visivel}
  <div class="dica" style:left="{dica.x}px" style:bottom="{dica.base}px" role="tooltip">
    {#each dica.linhas as l, k (k)}
      <p>{#each partes(l) as p, j (j)}<span style:color="#{p.cor}">{p.texto}</span>{/each}</p>
    {/each}
  </div>
{/if}

<style>
  /* O mesmo visual da dica do painel (`static/painel.css:259-263`), escuro como as janelas do cliente. */
  .dica { position: fixed; z-index: 90; max-width: 340px; max-height: calc(100vh - 16px); overflow: hidden; padding: 8px 10px; border-radius: 6px; background: rgba(10, 12, 18, .96);
    border: 1px solid #5a4a26; box-shadow: 0 6px 20px rgba(0, 0, 0, .5); font-size: 12px; line-height: 1.45; color: #fff; pointer-events: none; }
  p { margin: 0; white-space: pre-wrap; }
  p:first-child { font-size: 13px; font-weight: 600; }
</style>
