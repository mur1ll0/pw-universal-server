<script lang="ts">
  /* B392: área Missões — o tasks.data do realm (missões que não dependem de NPC também). Lista à esquerda (busca por id
     ou nome, só as de topo ou todas, `GET missoes`), ficha à direita (`FichaDaMissao`). Rota `#/missoes/<id>`. */
  import { listarMissoes, type LinhaDeMissao } from "../lib/api";
  import { avisar, estudio, ir, parametros, rota } from "../lib/estado.svelte";
  import FichaDaMissao from "../componentes/FichaDaMissao.svelte";
  import Icone from "../componentes/Icone.svelte";
  import TextoDoJogo from "../componentes/TextoDoJogo.svelte";

  let aberta = $state<number | null>(rota.caminho[1] ? Number(rota.caminho[1]) : null);
  let busca = $state("");
  let topo = $state(true);
  let pagina = $state(0);
  let r = $state<{ total: number; missoes: number; de_topo: number; linhas: LinhaDeMissao[] } | null>(null);
  let carregando = $state(false);
  const POR_PAGINA = 80;

  async function buscar() {
    carregando = true;
    try { const x = await listarMissoes(busca.trim(), topo && !busca.trim(), pagina, POR_PAGINA); r = { total: x.total, missoes: x.missoes, de_topo: x.de_topo, linhas: x.linhas }; }
    catch (e) { avisar("erro", (e as Error).message); }
    finally { carregando = false; }
  }
  $effect(() => { void topo; void pagina; void estudio.rascunho; buscar(); });
  let espera: ReturnType<typeof setTimeout> | undefined;
  function digitar() { clearTimeout(espera); espera = setTimeout(() => { pagina = 0; buscar(); }, 300); }
  function abrir(id: number) {
    aberta = id;
    history.replaceState(null, "", `#/missoes/${id}?${parametros()}`);
  }
</script>

<div class="barra">
  <button class="botao" type="button" onclick={() => ir("")}><Icone nome="voltar" /> Voltar</button>
  <h2>Missões</h2>
  <span class="mini">{r ? `${r.missoes} missões, ${r.de_topo} de topo` : ""}</span>
</div>

<div class="layout">
  <aside class="caixa lista">
    <input class="campo" placeholder="id ou nome" bind:value={busca} oninput={digitar} aria-label="Buscar missão" />
    <label class="mini"><input type="checkbox" bind:checked={topo} onchange={() => (pagina = 0)} /> só as de topo (as etapas abrem pela missão)</label>
    <span class="mini">{carregando ? "Carregando (a primeira vez lê o tasks.data inteiro)…" : r ? `${r.total} na lista` : ""}</span>
    <ul>
      {#each r?.linhas ?? [] as m (m.id)}
        <li><button type="button" class:on={aberta === m.id} onclick={() => abrir(m.id)}>
          <span class="l1"><TextoDoJogo texto={m.nome || "(sem nome)"} padrao="var(--texto)" linha /><span class="mini">{m.id}</span></span>
          <span class="mini">nível {m.nivel_min}–{m.nivel_max}{m.filhas ? ` · ${m.filhas} etapa(s)` : ""}</span>
        </button></li>
      {/each}
    </ul>
    <div class="linha">
      <button class="botao pequeno" type="button" disabled={pagina === 0} onclick={() => pagina--}>‹</button>
      <span class="mini">{pagina + 1}/{Math.max(1, Math.ceil((r?.total ?? 0) / POR_PAGINA))}</span>
      <button class="botao pequeno" type="button" disabled={!r || (pagina + 1) * POR_PAGINA >= r.total} onclick={() => pagina++}>›</button>
    </div>
  </aside>
  <section class="caixa ficha">
    {#if aberta}
      {#key aberta}<FichaDaMissao id={aberta} aoAbrirOutra={abrir} />{/key}
    {:else}
      <p class="suave">Escolha uma missão na lista.</p>
    {/if}
  </section>
</div>

<style>
  .barra { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin-bottom: 14px; }
  h2 { margin: 0; font-size: 18px; }
  .layout { display: grid; grid-template-columns: minmax(260px, 340px) minmax(0, 1fr); gap: 14px; align-items: start; }
  .lista { display: flex; flex-direction: column; gap: 6px; position: sticky; top: 70px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 3px; max-height: 62vh; overflow: auto; }
  li button { width: 100%; display: flex; flex-direction: column; gap: 2px; text-align: left; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--borda);
    background: var(--fundo-2); color: var(--texto); font: inherit; cursor: pointer; }
  li button.on { border-color: var(--ouro); background: var(--ouro-suave); }
  .l1 { display: flex; justify-content: space-between; gap: 6px; min-width: 0; }
  .ficha { min-width: 0; }
  @media (max-width: 960px) { .layout { grid-template-columns: 1fr; } .lista { position: static; } }
</style>
