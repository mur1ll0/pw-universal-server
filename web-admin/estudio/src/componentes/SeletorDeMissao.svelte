<script lang="ts">
  /* B392: escolher uma missão do tasks.data do realm (`GET missoes`, B255): busca por id ou nome, só as de topo ou todas. */
  import { listarMissoes, type LinhaDeMissao } from "../lib/api";
  import { avisar } from "../lib/estado.svelte";
  import Modal from "./Modal.svelte";
  import TextoDoJogo from "./TextoDoJogo.svelte";

  let { titulo = "Escolher missão", aoEscolher, aoFechar }: { titulo?: string; aoEscolher: (m: LinhaDeMissao) => void; aoFechar: () => void } = $props();
  let busca = $state("");
  let topo = $state(true);
  let pagina = $state(0);
  let r = $state<{ total: number; linhas: LinhaDeMissao[] } | null>(null);
  let carregando = $state(false);
  let espera: ReturnType<typeof setTimeout> | undefined;

  async function buscar() {
    carregando = true;
    try { const x = await listarMissoes(busca.trim(), topo && !busca.trim(), pagina, 60); r = { total: x.total, linhas: x.linhas }; }
    catch (e) { avisar("erro", (e as Error).message); }
    finally { carregando = false; }
  }
  $effect(() => { void topo; void pagina; buscar(); });
  function digitar() { clearTimeout(espera); espera = setTimeout(() => { pagina = 0; buscar(); }, 300); }
</script>

<Modal {titulo} {aoFechar} largo>
  <div class="linha">
    <input class="campo busca" placeholder="id ou nome da missão" bind:value={busca} oninput={digitar} aria-label="Buscar missão" />
    <label class="mini"><input type="checkbox" bind:checked={topo} onchange={() => (pagina = 0)} /> só as de topo (sem busca)</label>
    <span class="mini">{carregando ? "Carregando (a primeira vez lê o tasks.data inteiro)…" : r ? `${r.total} missão(ões)` : ""}</span>
  </div>
  <ul>
    {#each r?.linhas ?? [] as m (m.id)}
      <li><button type="button" onclick={() => { aoEscolher(m); aoFechar(); }}>
        <span class="id">{m.id}</span>
        <span class="nome"><TextoDoJogo texto={m.nome || "(sem nome)"} padrao="var(--texto)" linha /></span>
        <span class="mini">nível {m.nivel_min}–{m.nivel_max}{m.filhas ? ` · ${m.filhas} etapa(s)` : ""}{m.mae ? ` · etapa de ${m.mae}` : ""}</span>
      </button></li>
    {/each}
  </ul>
  {#snippet rodape()}
    <button class="botao" type="button" disabled={pagina === 0} onclick={() => pagina--}>Anterior</button>
    <span class="mini">página {pagina + 1} de {Math.max(1, Math.ceil((r?.total ?? 0) / 60))}</span>
    <button class="botao" type="button" disabled={!r || (pagina + 1) * 60 >= r.total} onclick={() => pagina++}>Próxima</button>
  {/snippet}
</Modal>

<style>
  .busca { flex: 1; min-width: 220px; }
  ul { list-style: none; margin: 10px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 55vh; overflow: auto; }
  button { width: 100%; display: grid; grid-template-columns: 70px minmax(0, 1fr) auto; gap: 10px; align-items: center; text-align: left; padding: 6px 8px;
    border-radius: 8px; border: 1px solid var(--borda); background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; }
  button:hover { border-color: var(--ouro); }
  .id { font-variant-numeric: tabular-nums; color: var(--texto-3); }
  .nome { min-width: 0; }
</style>
