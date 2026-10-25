<script lang="ts">
  /* B383: seletor do efeito do refino — só os efeitos de tratador `refine_*` (`Estudio::refinos`), cada um descrito
     ("Refino: +7 de ataque físico no +1; em +12 soma 262"), com a tabela dos 12 níveis ao passar e o uso por família.
     O GS não restringe a família (`get_item_refine_addon`, `itemdataman.cpp:205-228`): "serve nesta família" = alguma
     peça da mesma família já usa o efeito no rascunho. */
  import Modal from "./Modal.svelte";
  import { descreverRefino, filtrarRefinos, NOME_DA_FAMILIA, refinos, serveNaFamilia, usoDoRefino, type EfeitoDeRefino, type Familia } from "../lib/refino";

  let { familia, atual = 0, aoFechar, aoEscolher }:
    { familia: Familia | null; atual?: number; aoFechar: () => void; aoEscolher: (e: EfeitoDeRefino) => void } = $props();

  let lista = $state<EfeitoDeRefino[]>([]);
  let erro = $state("");
  let busca = $state("");
  let soDaFamilia = $state(true);
  let carregando = $state(true);
  refinos().then((r) => (lista = r.efeitos)).catch((e) => (erro = (e as Error).message)).finally(() => (carregando = false));

  const vistos = $derived(filtrarRefinos(lista, busca, familia, soDaFamilia && !!familia));
</script>

<Modal titulo="Efeito do refino" largo {aoFechar}>
  <p class="mini">Só os efeitos de refino do servidor (refine_*). O valor do nível é param × fator ({"1; 2; 3,05; 4,3; 5,75; 7,55; 9,95; 13; 17,05; 22,3; 29; 37,5"}, equip_item.cpp:208-223) e substitui o do nível anterior.</p>
  <div class="linha">
    <input class="campo" placeholder="Filtrar: ataque, HP, defesa, esquiva, resistências, nome ou id" bind:value={busca} aria-label="Filtrar" />
    {#if familia}<label class="mini"><input type="checkbox" bind:checked={soDaFamilia} /> só os que {NOME_DA_FAMILIA[familia]} usam</label>{/if}
  </div>
  {#if erro}<p class="erro">{erro}</p>
  {:else if carregando}<p class="suave">Carregando…</p>
  {:else}
    <p class="mini">{vistos.length} de {lista.length} efeitos de refino</p>
    <ul class="escolhas">
      {#each vistos as e (e.id)}
        <li>
          <button type="button" class:ativo={e.id === atual} onclick={() => { aoEscolher(e); aoFechar(); }}>
            <span class="cab"><strong>{descreverRefino(e)}</strong>
              {#if familia}<span class="selo" class:fora={!serveNaFamilia(e, familia)}>{serveNaFamilia(e, familia) ? `serve em ${NOME_DA_FAMILIA[familia]}` : `sem uso em ${NOME_DA_FAMILIA[familia]}`}</span>{/if}
            </span>
            <span class="mini">{e.nome} · id {e.id} · {e.tratador} · param {e.param} · {usoDoRefino(e)}</span>
            <span class="niveis mini">{#each e.niveis as v, k (k)}<span>+{k + 1}: {v}</span>{/each}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</Modal>

<style>
  .linha { display: flex; gap: 10px; align-items: center; margin-bottom: 8px; }
  .linha input.campo { flex: 1; }
  .escolhas { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 60vh; overflow: auto; }
  .escolhas button { width: 100%; text-align: left; padding: 6px 8px; border-radius: 8px; border: 1px solid var(--borda); background: var(--fundo-2);
    color: var(--texto); font: inherit; cursor: pointer; display: flex; flex-direction: column; gap: 2px; }
  .escolhas button:hover { border-color: var(--ouro); }
  .escolhas button.ativo { border-color: var(--ouro); background: var(--ouro-suave); }
  .cab { display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; }
  .selo { font-size: 11px; padding: 1px 6px; border-radius: 999px; background: var(--ouro-suave); }
  .selo.fora { background: transparent; border: 1px solid var(--borda); opacity: .75; }
  .niveis { display: flex; flex-wrap: wrap; gap: 4px 10px; }
</style>
