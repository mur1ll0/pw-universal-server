<script lang="ts">
  import { CATEGORIAS, type Categoria } from "../lib/catalogo";
  import Icone from "../componentes/Icone.svelte";
  import ModalBusca from "../componentes/ModalBusca.svelte";

  let aberta = $state<Categoria | null>(null);
</script>

<section class="categorias">
  {#each CATEGORIAS as c (c.chave)}
    <button class="categoria" type="button" disabled={!c.pronta} onclick={() => (aberta = c)}>
      <Icone nome={c.icone} tamanho={44} />
      <span class="nome">{c.nome}</span>
      {#if !c.pronta}<span class="mini">em breve</span>{/if}
    </button>
  {/each}
</section>
<p class="suave">O Estúdio grava pelas mesmas rotas dos <a href="/">Editores</a>, no rascunho escolhido no alto. O que não está aqui continua nos Editores.</p>

{#if aberta}
  <ModalBusca categoria={aberta} aoFechar={() => (aberta = null)} />
{/if}

<style>
  .categorias { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 14px; margin: 8px 0 16px; }
  .categoria { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; height: 160px;
    background: var(--superficie); border: 1px solid var(--borda); border-radius: var(--raio-g); color: var(--texto); font: inherit; cursor: pointer; }
  .categoria:hover:not(:disabled) { border-color: var(--ouro); color: var(--ouro); }
  .categoria:disabled { opacity: .5; cursor: not-allowed; }
  .nome { font-size: 16px; font-weight: 600; }
</style>
