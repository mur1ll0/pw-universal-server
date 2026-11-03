<script lang="ts">
  /* Caixas que montam uma máscara de bits (classes: `1 << classe`; regras do item: `ITEM_PROC_TYPE_*`). */
  import Modal from "./Modal.svelte";

  let { titulo, opcoes, valor, explica = "", aoAplicar, aoFechar }:
    { titulo: string; opcoes: [number, string][]; valor: number; explica?: string; aoAplicar: (v: number) => void; aoFechar: () => void } = $props();

  let atual = $state(valor >>> 0);
  const conhecidos = opcoes.reduce((m, [b]) => m | b, 0) >>> 0;
  const desconhecidos = $derived((atual & ~conhecidos) >>> 0);
  const marcar = (b: number, sim: boolean) => { atual = (sim ? atual | b : atual & ~b) >>> 0; };
</script>

<Modal {titulo} {aoFechar}>
  {#if explica}<p class="mini">{explica}</p>{/if}
  <div class="opcoes">
    {#each opcoes as [b, nome] (b)}
      <label><input type="checkbox" checked={(atual & b) !== 0} onchange={(e) => marcar(b, (e.target as HTMLInputElement).checked)} /> {nome}</label>
    {/each}
  </div>
  <div class="linha acoes">
    <button class="botao pequeno" type="button" onclick={() => (atual = (atual | conhecidos) >>> 0)}>Todas</button>
    <button class="botao pequeno" type="button" onclick={() => (atual = (atual & ~conhecidos) >>> 0)}>Nenhuma</button>
    <span class="mini">valor: {atual} (0x{atual.toString(16)}){desconhecidos ? ` · bits fora da lista mantidos: 0x${desconhecidos.toString(16)}` : ""}</span>
  </div>
  {#snippet rodape()}
    <button class="botao" type="button" onclick={aoFechar}>Cancelar</button>
    <button class="botao primario" type="button" onclick={() => { aoAplicar(atual | 0); aoFechar(); }}>Aplicar</button>
  {/snippet}
</Modal>

<style>
  .opcoes { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 6px 12px; }
  label { display: flex; align-items: center; gap: 6px; }
  .acoes { margin-top: 12px; }
</style>
