<script lang="ts">
  /* B392: escolher uma habilidade do catálogo do realm (`GET habilidades`, a mesma busca da lista de habilidades do monstro). */
  import { buscarHabilidades, type Habilidade } from "../lib/api";
  import { avisar } from "../lib/estado.svelte";
  import Modal from "./Modal.svelte";

  let { titulo = "Escolher habilidade", aoEscolher, aoFechar }: { titulo?: string; aoEscolher: (h: Habilidade) => void; aoFechar: () => void } = $props();
  let busca = $state("");
  let lista = $state<Habilidade[]>([]);
  let espera: ReturnType<typeof setTimeout> | undefined;
  async function buscar() {
    try { lista = (await buscarHabilidades(busca.trim())).habilidades.slice(0, 120); }
    catch (e) { avisar("erro", (e as Error).message); }
  }
  buscar();
  function digitar() { clearTimeout(espera); espera = setTimeout(buscar, 300); }
</script>

<Modal {titulo} {aoFechar} largo>
  <input class="campo busca" placeholder="id ou nome" bind:value={busca} oninput={digitar} aria-label="Buscar habilidade" />
  <ul>
    {#each lista as h (h.id)}
      <li><button type="button" onclick={() => { aoEscolher(h); aoFechar(); }}>
        <span class="id">{h.id}</span><span>{h.nome}</span><span class="mini">nível máx. {h.max_level}</span>
      </button></li>
    {/each}
  </ul>
</Modal>

<style>
  .busca { width: 100%; }
  ul { list-style: none; margin: 10px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 55vh; overflow: auto; }
  button { width: 100%; display: grid; grid-template-columns: 70px 1fr auto; gap: 10px; text-align: left; padding: 6px 8px; border-radius: 8px;
    border: 1px solid var(--borda); background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; }
  button:hover { border-color: var(--ouro); }
  .id { color: var(--texto-3); font-variant-numeric: tabular-nums; }
</style>
